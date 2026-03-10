# Track FF: Point Cloud Material Format (RT/DLSS Gains via Sparse Material Distribution)

**Status**: Ready for implementation
**Duration**: 5-7 days
**Owner**: Materials & Visualization (Overlaps VI.1 physics + VI.3 rendering)
**Integration**: Phase 3 Point Mamba learns material properties; VI.1 physics simulates through materials; VI.3 ray traces sparse material cloud

---

## Executive Summary

Traditional ray tracing requires dense geometry (meshes). **Point cloud materials** decouple RF simulation from geometry: each spatial point carries material properties (permittivity, conductivity, loss). This enables:

- **Sparse sampling**: Only store materials where they exist (glass of water = ~1000 points, not millions of triangles)
- **Mamba learning**: Model predicts material_id from RF response patterns
- **RT/DLSS gains**: Ray trace sparse cloud (1-5ms), temporal upsampling (10-40ms total)
- **User creation**: Blender-style material editor—define custom RF-BSDF, let Mamba refine it
- **Real-time adaptation**: Material properties learned/updated as RF response observed

**Key insight**: The point cloud IS the material distribution. No separate geometry model needed.

---

## Data Structure: Point Cloud with Materials

### File Format (Binary + Metadata)

```
Point Cloud Material File Format (.pcm)
┌─────────────────────────────────────┐
│ Header (64 bytes)                   │
├─────────────────────────────────────┤
│ Format version (u32)                │ 1
│ Point count (u32)                   │ e.g., 50000
│ Material count (u32)                │ e.g., 12
│ Timestamp (u64 micros)              │ Last update
│ Reserved (20 bytes)                 │ Future expansion
├─────────────────────────────────────┤
│ Materials Section                   │
│ (material_count × 256 bytes each)   │
├─────────────────────────────────────┤
│ Points Section                      │
│ (point_count × variable size)       │
└─────────────────────────────────────┘
```

### Material Definition (256 bytes per material)

```rust
#[repr(C)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaterialDef {
    // Identification
    pub material_id: u32,               // 0-11 (12 material slots)
    pub name: [u8; 32],                 // "water", "glass", "human_tissue", etc.

    // Dielectric properties (frequency-dependent)
    pub permittivity_static: f32,       // ε₀ at 0 Hz (e.g., 80.0 for water)
    pub permittivity_infinity: f32,     // ε∞ at high frequency
    pub relaxation_time_ps: f32,        // Debye relaxation time (picoseconds)

    // Conductivity (Ohm⁻¹·m⁻¹)
    pub conductivity_base: f32,         // σ at 1 MHz
    pub conductivity_frequency_exp: f32,// Frequency exponent (0.0-1.0)

    // Loss characteristics
    pub loss_tangent_1ghz: f32,         // tan(δ) at 1 GHz
    pub absorption_coefficient: f32,    // α (dB/cm) per GHz

    // Physical properties
    pub density_kg_m3: f32,             // Mass density
    pub acoustic_impedance: f32,        // Mechanical impedance (for sound)

    // Scattering (RF-BSDF)
    pub roughness: f32,                 // [0.0, 1.0] surface roughness
    pub anisotropy: f32,                // [0.0, 1.0] directional dependence

    // Thermal properties (future)
    pub thermal_conductivity: f32,      // W·m⁻¹·K⁻¹
    pub specific_heat: f32,             // J·kg⁻¹·K⁻¹

    // Mamba learning state
    pub confidence: f32,                // [0.0, 1.0] how well material is known
    pub last_updated_micros: u64,       // Timestamp of last Mamba update
    pub version: u32,                   // Material definition version

    pub reserved: [u8; 64],             // Padding for future properties
}
```

### Point Definition (Variable size, ~40 bytes typical)

```rust
#[repr(C)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaterialPoint {
    // Spatial position
    pub position_xyz: [f32; 3],         // Meters from origin

    // Material reference
    pub material_id: u32,               // Index into MaterialDef array (0-11)
    pub material_blend: f32,            // [0.0, 1.0] blend towards next material
    pub next_material_id: u32,          // For blending (composite materials)

    // Confidence and metadata
    pub confidence: f32,                // [0.0, 1.0] how certain this material is
    pub timestamp_micros: u64,          // When this point was created/updated

    // RF properties (cache for speed)
    pub permittivity_at_freq: f32,      // ε(f) at detection frequency
    pub conductivity_at_freq: f32,      // σ(f) at detection frequency

    // Physics state
    pub velocity_xyz: [f32; 3],         // Meters/second (for liquid/deformable)
    pub temperature_kelvin: f32,        // For thermal effects

    // Optional: Derived quantities (for visualization)
    pub attenuation_db_per_cm: f32,     // Precomputed attenuation
    pub group_velocity_ratio: f32,      // vg/c (for dispersion)
}
```

---

## Material Database (Built-in Library)

### Predefined Materials (Immutable Baseline)

```rust
pub struct MaterialLibrary {
    pub materials: HashMap<String, MaterialDef>,
}

impl MaterialLibrary {
    pub fn default() -> Self {
        let mut lib = MaterialLibrary {
            materials: HashMap::new(),
        };

        // Water (room temperature, ~25°C)
        lib.materials.insert("water".to_string(), MaterialDef {
            material_id: 0,
            name: *b"water\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            permittivity_static: 80.0,      // At 0 Hz
            permittivity_infinity: 4.8,     // At ∞ Hz
            relaxation_time_ps: 8.3,        // Debye relaxation (~8.3 ps)
            conductivity_base: 0.05,        // Pure water ~0.05 S/m
            conductivity_frequency_exp: 0.5,
            loss_tangent_1ghz: 0.15,
            absorption_coefficient: 0.02,   // dB/cm/GHz
            density_kg_m3: 1000.0,
            acoustic_impedance: 1.48e6,
            roughness: 0.0,                 // Smooth surface
            anisotropy: 0.0,                // Isotropic
            thermal_conductivity: 0.6,
            specific_heat: 4200.0,
            confidence: 0.99,               // Well-characterized
            last_updated_micros: 0,
            version: 1,
            reserved: [0; 64],
        });

        // Glass (borosilicate, ~25°C)
        lib.materials.insert("glass".to_string(), MaterialDef {
            material_id: 1,
            name: *b"glass\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            permittivity_static: 6.0,
            permittivity_infinity: 5.8,
            relaxation_time_ps: 1e-3,       // Weak dispersion
            conductivity_base: 1e-11,       // Essentially insulator
            conductivity_frequency_exp: 1.0,
            loss_tangent_1ghz: 0.001,       // Very low loss
            absorption_coefficient: 0.0001,
            density_kg_m3: 2230.0,
            acoustic_impedance: 1.26e7,
            roughness: 0.05,                // Polished but slightly rough
            anisotropy: 0.0,
            thermal_conductivity: 1.2,
            specific_heat: 840.0,
            confidence: 0.98,
            last_updated_micros: 0,
            version: 1,
            reserved: [0; 64],
        });

        // Human tissue (simplified: ~60% water)
        lib.materials.insert("human_tissue".to_string(), MaterialDef {
            material_id: 2,
            name: *b"human_tissue\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            permittivity_static: 50.0,      // ~60% water effect
            permittivity_infinity: 4.0,
            relaxation_time_ps: 10.0,       // Longer than pure water
            conductivity_base: 0.5,         // Conductive due to ions
            conductivity_frequency_exp: 0.4,
            loss_tangent_1ghz: 0.2,
            absorption_coefficient: 0.03,
            density_kg_m3: 1050.0,
            acoustic_impedance: 1.54e6,
            roughness: 0.3,                 // Rough surface
            anisotropy: 0.1,                // Weakly anisotropic (fiber-like)
            thermal_conductivity: 0.5,
            specific_heat: 3500.0,
            confidence: 0.85,               // Simplified model
            last_updated_micros: 0,
            version: 1,
            reserved: [0; 64],
        });

        // Wood (pine, dry)
        lib.materials.insert("wood".to_string(), MaterialDef {
            material_id: 3,
            name: *b"wood\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            permittivity_static: 3.5,
            permittivity_infinity: 3.4,
            relaxation_time_ps: 1e-2,
            conductivity_base: 1e-3,        // Low conductivity
            conductivity_frequency_exp: 0.8,
            loss_tangent_1ghz: 0.01,
            absorption_coefficient: 0.005,
            density_kg_m3: 500.0,
            acoustic_impedance: 2.4e6,
            roughness: 0.4,
            anisotropy: 0.2,                // Grain structure
            thermal_conductivity: 0.12,
            specific_heat: 1500.0,
            confidence: 0.90,
            last_updated_micros: 0,
            version: 1,
            reserved: [0; 64],
        });

        // Metal (aluminum)
        lib.materials.insert("metal".to_string(), MaterialDef {
            material_id: 4,
            name: *b"metal\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            permittivity_static: -1e6,      // Drude model (negative permittivity)
            permittivity_infinity: 1.0,
            relaxation_time_ps: 0.1,        // Short relaxation
            conductivity_base: 3.8e7,       // Highly conductive
            conductivity_frequency_exp: 0.0, // Frequency-independent
            loss_tangent_1ghz: 1e-4,        // Minimal loss (good conductor)
            absorption_coefficient: 0.0,    // Full reflection
            density_kg_m3: 2700.0,
            acoustic_impedance: 1.73e7,
            roughness: 0.2,                 // Polished but not perfect
            anisotropy: 0.0,
            thermal_conductivity: 237.0,
            specific_heat: 900.0,
            confidence: 0.95,
            last_updated_micros: 0,
            version: 1,
            reserved: [0; 64],
        });

        // Add more as needed (plastic, foam, concrete, fabric, etc.)

        lib
    }
}
```

---

## Material Creation & Editing (Blender-Style UI)

### File Ownership

- **`src/materials/material_editor.rs`** (NEW) - Slint UI for material creation
- **`src/materials/material_library.rs`** (NEW) - Material storage, loading, blending
- **`src/materials/material_blender.rs`** (NEW) - Composite material interpolation

### Slint UI for Material Creation

```slint
// ui/materials.slint

export component MaterialEditor {
    property <string> material-name;
    property <float> permittivity-static: 50.0;
    property <float> permittivity-infinity: 4.0;
    property <float> conductivity: 0.5;
    property <float> loss-tangent: 0.2;
    property <float> roughness: 0.3;

    VerticalLayout {
        spacing: 15px;
        padding: 20px;

        HorizontalLayout {
            Text { text: "Material Name:"; }
            TextInput {
                text <=> material-name;
                placeholder-text: "e.g., water, custom_foam";
            }
        }

        // Permittivity
        HorizontalLayout {
            Text { text: "Permittivity (static):"; }
            Slider {
                minimum: 1.0;
                maximum: 100.0;
                value <=> permittivity-static;
            }
            Text { text: "{permittivity-static.round()}"; }
        }

        // Conductivity
        HorizontalLayout {
            Text { text: "Conductivity (S/m):"; }
            Slider {
                minimum: -3.0;  // Log scale
                maximum: 8.0;
                value <=> conductivity;
            }
            Text { text: "{10^conductivity}"; }
        }

        // Loss tangent
        HorizontalLayout {
            Text { text: "Loss tangent:"; }
            Slider {
                minimum: 0.0;
                maximum: 1.0;
                value <=> loss-tangent;
            }
            Text { text: "{loss-tangent}"; }
        }

        // Roughness
        HorizontalLayout {
            Text { text: "Roughness:"; }
            Slider {
                minimum: 0.0;
                maximum: 1.0;
                value <=> roughness;
            }
            Text { text: "{roughness}"; }
        }

        // Buttons
        HorizontalLayout {
            Button {
                text: "Save Material";
                clicked => {
                    app-window.save_material(
                        material-name,
                        permittivity-static,
                        permittivity-infinity,
                        conductivity,
                        loss-tangent,
                        roughness
                    );
                }
            }

            Button {
                text: "Load Preset";
                clicked => {
                    app-window.load_material_preset();
                }
            }

            Button {
                text: "Test with RT";
                clicked => {
                    app-window.test_material_raytracing();
                }
            }
        }

        // Preview
        Text {
            text: "Material preview (ray traced):";
        }
        Rectangle {
            background: #222;
            border: 1px solid #444;
            min-height: 200px;
            // Ray-traced preview renders here
        }
    }
}
```

---

## Ray Tracing Through Sparse Materials

### RT/DLSS Pipeline

```
1. Point Cloud Material Distribution [1-5ms]
   - Sample materials at spatial grid (e.g., 100×100×100 voxels)
   - Interpolate material properties between points
   - Compute local ε(f), σ(f), α(f) per voxel

2. Ray Casting [10-40ms]
   - Trace rays through material voxels
   - Compute refraction/reflection at material boundaries
   - Accumulate attenuation through lossy materials
   - Cache results in temporal buffer

3. Temporal Upsampling (DLSS-style) [5-10ms]
   - Use previous frame + current frame results
   - Reprojection to account for camera motion
   - Reduce noise via temporal filtering
   - Output high-resolution ray trace result

4. Visualization [5-10ms]
   - Tone map to RGB (blue=transparent, red=opaque)
   - Composite with Gaussian splatting
   - Display in ANALYSIS tab
```

### Rust Implementation (Compute Shader)

```wgsl
// src/visualization/material_raytracer.wgsl

@group(0) @binding(0) var<storage, read> point_cloud: array<MaterialPoint>;
@group(0) @binding(1) var<storage, read> materials: array<MaterialDef>;
@group(0) @binding(2) var<storage, read_write> rt_buffer: array<vec4<f32>>;

@compute @workgroup_size(256)
fn raytrace_materials(
    @builtin(global_invocation_id) global_id: vec3u
) {
    let ray_idx = global_id.x;
    if (ray_idx >= arrayLength(&rt_buffer)) { return; }

    // Ray origin and direction (from camera)
    let ray_origin = get_camera_position();
    let ray_dir = get_ray_direction(ray_idx);

    // Trace through point cloud materials
    var accumulated_color = vec3<f32>(0.0);
    var accumulated_attenuation = 1.0;

    // Intersect ray with material points (spatial acceleration structure)
    let intersections = find_material_intersections(ray_origin, ray_dir);

    for (var i: u32 = 0u; i < arrayLength(&intersections); i++) {
        let point_idx = intersections[i];
        let point = point_cloud[point_idx];

        // Get material properties
        let material = materials[point.material_id];
        let permittivity = point.permittivity_at_freq;
        let conductivity = point.conductivity_at_freq;

        // Compute interaction
        let refraction_ratio = 1.0 / sqrt(permittivity);
        let attenuation = exp(-point.attenuation_db_per_cm / 20.0);

        accumulated_attenuation *= attenuation;

        // Material color (for visualization)
        // Blue: transparent (low permittivity)
        // Red: opaque (high permittivity + conductivity)
        let material_color = mix(
            vec3<f32>(0.0, 0.0, 1.0),  // Blue
            vec3<f32>(1.0, 0.0, 0.0),  // Red
            permittivity / 100.0
        );

        accumulated_color += material_color * accumulated_attenuation;
    }

    // Output
    rt_buffer[ray_idx] = vec4<f32>(accumulated_color, accumulated_attenuation);
}
```

---

## Mamba Material Learning

### Material Property Refinement

As Mamba observes RF responses, it learns material properties:

```rust
// src/ml/material_learning.rs

pub struct MaterialLearner {
    pub materials: Vec<MaterialDef>,
    pub confidence_history: Vec<f32>,  // Mamba's confidence per observation
}

impl MaterialLearner {
    /// Update material properties based on Mamba inference
    pub async fn refine_material(
        &mut self,
        material_id: u32,
        observed_rf_response: f32,
        predicted_rf_response: f32,
        mamba_confidence: f32,
    ) {
        let error = observed_rf_response - predicted_rf_response;

        // Gradient descent: adjust permittivity/conductivity
        let learning_rate = 0.01 * mamba_confidence;  // Confidence-weighted

        self.materials[material_id as usize].permittivity_static += learning_rate * error;
        self.materials[material_id as usize].confidence =
            0.95 * self.materials[material_id as usize].confidence +
            0.05 * mamba_confidence;

        // Log refinement
        eprintln!("[Material] Refined {}: ε → {:.2}, confidence → {:.2}",
                  material_id,
                  self.materials[material_id as usize].permittivity_static,
                  self.materials[material_id as usize].confidence);
    }
}
```

---

## File I/O (Load/Save Point Cloud Materials)

```rust
// src/materials/io.rs

pub async fn save_point_cloud_materials(
    points: &[MaterialPoint],
    materials: &[MaterialDef],
    path: &str,
) -> Result<(), Box<dyn Error>> {
    let mut file = tokio::fs::File::create(path).await?;

    // Write header
    let header = [
        1u32,                              // Format version
        points.len() as u32,               // Point count
        materials.len() as u32,            // Material count
        get_current_micros() as u32,       // Timestamp (lower 32 bits)
    ];
    file.write_all(bytemuck::cast_slice(&header)).await?;

    // Write materials
    for material in materials {
        file.write_all(bytemuck::cast_slice(&[*material])).await?;
    }

    // Write points
    for point in points {
        file.write_all(bytemuck::cast_slice(&[*point])).await?;
    }

    file.sync_all().await?;
    Ok(())
}

pub async fn load_point_cloud_materials(
    path: &str,
) -> Result<(Vec<MaterialPoint>, Vec<MaterialDef>), Box<dyn Error>> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut buf = vec![0u8; 16];
    file.read_exact(&mut buf).await?;

    let header: [u32; 4] = bytemuck::cast_slice(&buf).try_into()?;
    let point_count = header[1] as usize;
    let material_count = header[2] as usize;

    // Read materials
    let mut materials = Vec::with_capacity(material_count);
    for _ in 0..material_count {
        let mut mat_buf = vec![0u8; std::mem::size_of::<MaterialDef>()];
        file.read_exact(&mut mat_buf).await?;
        let material: MaterialDef = bytemuck::cast_slice(&mat_buf)[0].clone();
        materials.push(material);
    }

    // Read points
    let mut points = Vec::with_capacity(point_count);
    for _ in 0..point_count {
        let mut point_buf = vec![0u8; std::mem::size_of::<MaterialPoint>()];
        file.read_exact(&mut point_buf).await?;
        let point: MaterialPoint = bytemuck::cast_slice(&point_buf)[0].clone();
        points.push(point);
    }

    Ok((points, materials))
}
```

---

## Integration Points

### Track VI.1 (Physics)
- Read material properties (ε, σ, α) from point cloud
- Solve Helmholtz through material distribution
- Simulate wave refraction/reflection at material boundaries

### Track VI.3 (Rendering)
- Ray trace through sparse materials
- Render material-aware Gaussian splatting
- Visualize permittivity/conductivity as color

### Track D (Temporal Rewind)
- Time-scrub shows material evolution (as Mamba learns)
- Material confidence displayed per point

### Track E (Forensic Logging)
- Log material interactions as evidence
- Snapshot point cloud materials at key events

---

## Success Criteria

✅ **Point cloud format working**:
- Load/save to binary .pcm files
- Material database accessible
- Up to 1M points in memory (12GB system)

✅ **Material editor functional**:
- Create custom materials via UI
- Edit permittivity, conductivity, loss, roughness
- Save to material library

✅ **Ray tracing sparse materials**:
- Trace through point cloud in < 5ms (compute shader)
- Temporal upsampling produces smooth visuals
- RTX/DLSS-style gains (sparse sampling + temporal)

✅ **Mamba material learning**:
- Confidence increases as RF responses predicted better
- Material properties refined through gradient descent
- Learning logged for audit trail

---

## Notes

**Why point cloud, not mesh?**
- Meshes require topology (connectivity, winding order)
- Point clouds just need spatial position + material
- Mamba learns faster on sparse, unstructured data
- Ray tracing naturally handles sparse sampling (DLSS-style upsampling)

**Why not just use existing BSDF models?**
- RF-BSDF is frequency-specific (need curves, not constants)
- User-customizable (Blender-style editor needed)
- Mamba-learnable (properties refine over time)

**RT/DLSS gains?**
- Sparse sampling (1-5ms): Only trace rays through occupied material points
- Temporal upsampling (5-10ms): Reproject + filter across frames
- Total: ~10-15ms for high-quality material ray tracing
- Compare: Dense mesh ray tracing = 50-100ms

