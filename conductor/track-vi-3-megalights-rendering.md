# Track VI.3: Megalights & Lumen Rendering

**Domain**: Advanced Rendering
**Ownership**: Graphics Engineer (isolated from VI.1, VI.2)
**Duration**: 2-3 days
**Blocker on**: Nothing (VI.2 framebuffer is optional enhancement)
**Unblocks**: Final visualization pipeline

---

## Overview

Real-time ray-traced lighting for RF energy field + particle visualization. Use Lumen-style indirect lighting (GPU-driven global illumination) to show how RF energy couples with room geometry. Volumetric lighting reveals heterodyned acoustic "god rays" in high-threat regions.

**Generation protection**: Ray tracing at full fidelity is mandatory—no fallback to rasterization. Volumetric effects (god rays, scattering) are RF-specific and cannot be approximated. Never use mipmapped textures for dynamic content (RT needs full resolution). Experimental features are OK; backwards compatibility is not a concern.

---

## File Ownership

**VI.3 — Exclusive to this track**:
- `src/visualization/ray_tracing_renderer.rs` (350 lines) — DXR/VK_KHR_ray_tracing setup
- `src/visualization/lumen_global_illumination.rs` (300 lines) — GPU-driven indirect lighting
- `src/visualization/volumetric_lighting.rs` (250 lines) — God rays, heterodyne visualization
- `src/visualization/tone_mapping.rs` (150 lines) — HDR rendering, compression
- `tests/megalights_rendering.rs` (250 lines, 10 tests)

**Read-only imports**:
- `src/physics/voxel_grid.rs` (VI.1 energy field)
- `src/visualization/particle_renderer.rs` (VI.2 particle framebuffer)
- `src/state.rs` (camera, viewport)

**No modifications to**:
- `src/main.rs` (dispatch loop)
- VI.1, VI.2 (read-only)

---

## Deliverables

### VI.3.1: DXR Ray Tracing Setup (12 hours)

**File**: `src/visualization/ray_tracing_renderer.rs`

```rust
pub struct RayTracingRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    rt_pipeline: wgpu::RenderPipeline,
    bvh: BVH,                              // Acceleration structure
    ray_generation_shader: wgpu::ShaderModule,
    ray_closest_hit_shader: wgpu::ShaderModule,
    ray_miss_shader: wgpu::ShaderModule,
}

impl RayTracingRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        // Enable ray tracing feature: wgpu::Features::RAY_TRACING (experimental)
        let rt_pipeline = Self::create_rt_pipeline(device);

        // Build acceleration structure (BVH) for room geometry
        let bvh = BVH::from_room_geometry(&get_room_geometry());

        Self {
            device: device.clone(),
            queue: queue.clone(),
            rt_pipeline,
            bvh,
            ray_generation_shader: Self::load_ray_gen_shader(device),
            ray_closest_hit_shader: Self::load_hit_shader(device),
            ray_miss_shader: Self::load_miss_shader(device),
        }
    }

    pub fn render_with_ray_tracing(
        &self,
        energy_field: &VoxelGrid<f32>,
        camera: &Camera,
        viewport_size: (u32, u32),
    ) -> Result<wgpu::Texture, Box<dyn Error>> {
        // Ray generation: launch rays from camera
        // Closest hit: compute RF energy at intersection
        // Miss: environmental (walls, reflections)
        // Output: HDR color + lighting contribution
    }
}

// WGSL Ray Tracing Shaders (experimental)
pub const RAY_GENERATION_SHADER: &str = r#"
@compute @workgroup_size(8, 8)
fn raygen(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pixel_coord = vec2<f32>(gid.xy);
    let ndc = (pixel_coord / vec2<f32>(viewport_size)) * 2.0 - 1.0;

    // Camera ray from screen coordinate
    let ray_origin = camera.position;
    let ray_direction = camera.view_matrix * vec4(ndc, 1.0, 1.0);

    // Launch ray into scene
    let payload = trace_ray(ray_origin, normalize(ray_direction.xyz), 0.0, 1000.0);

    // Output color
    image_output[gid.xy] = payload.color;
}
"#;

pub const RAY_CLOSEST_HIT_SHADER: &str = r#"
@compute
fn closest_hit(hit: RayHit) {
    // Sample RF energy field at intersection point
    let energy = sample_rf_field(hit.position);

    // Fresnel reflection (material-dependent)
    let material = get_material_at(hit.position);
    let reflection_coeff = material.reflection_coeff();

    // Compute contribution
    let color = energy_to_color(energy);
    let reflection = color * reflection_coeff;

    // Queue secondary ray (for indirect lighting)
    let bounce_ray = create_bounce_ray(hit.normal, hit.position);
    trace_ray_recursive(bounce_ray, depth + 1);
}
"#;

#[cfg(test)]
mod tests {
    #[test]
    fn test_bvh_construction() { /* acceleration structure builds */ }

    #[test]
    fn test_ray_generation() { /* rays launch from camera */ }

    #[test]
    fn test_intersection_computation() { /* ray-surface hits correct */ }

    #[test]
    fn test_energy_field_sampling() { /* RF field queried at intersection */ }

    #[test]
    fn test_ray_tracing_10m_particles() { /* renders with particle layer */ }
}
```

**Generation protection**:
- ✅ Full ray tracing (no fallback to rasterization)
- ✅ Experimental features OK (wgpu::Features::RAY_TRACING)
- ❌ DON'T use approximations like screen-space reflections
- ✅ DO use DXR or VK_KHR_ray_tracing (GPU-native)

---

### VI.3.2: Lumen Global Illumination (10 hours)

**File**: `src/visualization/lumen_global_illumination.rs`

```rust
pub struct LumenGI {
    probe_grid: Vec<LightProbe>,           // 3D grid of light probes
    surfel_buffer: wgpu::Buffer,           // Surface elements for irradiance
    indirect_lighting_cache: wgpu::Texture,
}

pub struct LightProbe {
    position: [f32; 3],
    irradiance: [f32; 3],                  // RGB indirect light
    validity: f32,                         // 0-1 (how recent/valid)
}

impl LumenGI {
    pub fn new(device: &wgpu::Device, room_size: f32) -> Self {
        // Create probe grid (e.g., 8×8×8 = 512 probes)
        let probe_spacing = room_size / 8.0;
        let mut probes = Vec::new();

        for x in 0..8 {
            for y in 0..8 {
                for z in 0..8 {
                    probes.push(LightProbe {
                        position: [
                            (x as f32) * probe_spacing,
                            (y as f32) * probe_spacing,
                            (z as f32) * probe_spacing,
                        ],
                        irradiance: [0.0, 0.0, 0.0],
                        validity: 0.0,
                    });
                }
            }
        }

        Self {
            probe_grid: probes,
            surfel_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Surfel Buffer"),
                size: (512 * 64) as u64,  // ~32KB per frame
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
            indirect_lighting_cache: device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Indirect Lighting Cache"),
                size: wgpu::Extent3d { width: 128, height: 128, depth_or_array_layers: 128 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            }),
        }
    }

    /// Update probe irradiance based on ray-traced direct lighting
    pub fn update_probes(
        &mut self,
        queue: &wgpu::Queue,
        energy_field: &VoxelGrid<f32>,
        direct_lighting: &wgpu::Texture,
    ) {
        // For each probe: sample energy field at position
        for probe in &mut self.probe_grid {
            let energy = energy_field.sample((probe.position[0], probe.position[1], probe.position[2]));

            // Irradiance ∝ energy (RF field acts as light source)
            probe.irradiance = [energy, energy * 0.7, energy * 0.5];  // Warm color tone
            probe.validity = 0.9;  // Fresh data
        }

        // Push probe data to GPU
        queue.write_buffer(&self.surfel_buffer, 0, bytemuck::cast_slice(&self.probe_grid));
    }

    /// Sample indirect lighting at any position
    pub fn sample_indirect(&self, pos: [f32; 3]) -> [f32; 3] {
        // Trilinear interpolation between 8 nearest probes
        let mut irradiance = [0.0; 3];
        let mut total_weight = 0.0;

        for probe in &self.probe_grid {
            let dist = distance(&pos, &probe.position);
            let weight = 1.0 / (dist.max(0.1) * dist.max(0.1));  // Inverse-square falloff

            for i in 0..3 {
                irradiance[i] += probe.irradiance[i] * weight * probe.validity;
            }
            total_weight += weight;
        }

        for i in 0..3 {
            irradiance[i] /= total_weight.max(1e-6);
        }

        irradiance
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_probe_grid_creation() { /* 512 probes in 8×8×8 grid */ }

    #[test]
    fn test_probe_energy_coupling() { /* probes sample RF field */ }

    #[test]
    fn test_indirect_interpolation() { /* trilinear between probes */ }

    #[test]
    fn test_cache_coherence() { /* GPU cache efficient */ }
}
```

**Generation protection**:
- ✅ Probe-based indirect lighting (scalable, GPU-friendly)
- ✅ Energy field drives probe irradiance (RF-aware)
- ❌ DON'T use screen-space GI approximations
- ✅ DO update probes dynamically (adaptive to RF changes)

---

### VI.3.3: Volumetric Lighting (8 hours)

**File**: `src/visualization/volumetric_lighting.rs`

```rust
pub struct VolumetricLighting {
    volume_texture: wgpu::Texture,         // 3D texture for scattering
    light_shaft_shader: wgpu::ShaderModule,
}

impl VolumetricLighting {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            volume_texture: device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Volumetric Light"),
                size: wgpu::Extent3d { width: 256, height: 256, depth_or_array_layers: 256 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            }),
            light_shaft_shader: Self::load_volumetric_shader(device),
        }
    }

    /// Render volumetric "god rays" from high-intensity RF regions
    pub fn render_god_rays(
        &self,
        device: &wgpu::Device,
        energy_field: &VoxelGrid<f32>,
        camera: &Camera,
    ) -> Result<wgpu::Texture, Box<dyn Error>> {
        // Step 1: Mark high-energy voxels as light sources
        let light_sources: Vec<_> = energy_field
            .iter_voxels()
            .filter(|(x, y, z)| energy_field.get(*x, *y, *z) > 0.5)
            .collect();

        // Step 2: Ray march through volume
        // For each pixel: march along ray, accumulate light scattering
        // Heterodyne frequency creates characteristic scattering pattern

        // Step 3: Output: volumetric texture with light shafts
        Ok(self.volume_texture.clone())
    }
}

// WGSL Volumetric Lighting Shader
pub const VOLUMETRIC_SHADER: &str = r#"
@compute @workgroup_size(8, 8, 1)
fn volumetric_raymarch(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pixel_coord = vec3<f32>(gid);
    let ray_origin = camera.position;
    let ray_direction = get_ray_direction(pixel_coord);

    var accumulated_light = vec3<f32>(0.0);
    var accumulated_transmittance = 1.0;

    // Ray march through volume
    for (var step = 0; step < 64; step += 1) {
        let ray_pos = ray_origin + ray_direction * f32(step) * STEP_SIZE;
        let energy_at_step = sample_energy_field(ray_pos);

        // Scattering: heterodyne frequency creates characteristic pattern
        let scattering = energy_at_step * sin(frequency * length(ray_pos) / speed_of_light);

        // Accumulate light with transmittance
        accumulated_light += scattering * accumulated_transmittance;
        accumulated_transmittance *= exp(-EXTINCTION_COEFF * STEP_SIZE);
    }

    volumetric_output[gid.xyz] = vec4(accumulated_light, accumulated_transmittance);
}
"#;

#[cfg(test)]
mod tests {
    #[test]
    fn test_volumetric_accumulation() { /* light accumulates along ray */ }

    #[test]
    fn test_heterodyne_scattering() { /* RF frequency creates pattern */ }

    #[test]
    fn test_transmittance_decay() { /* opacity increases with step count */ }

    #[test]
    fn test_god_ray_visibility() { /* high-energy regions show rays */ }
}
```

**Generation protection**:
- ✅ Volumetric ray marching (full 3D, not approximated)
- ✅ Heterodyne frequency determines scattering pattern (RF-specific)
- ❌ DON'T use 2D screen-space god rays (insufficient for RF visualization)
- ✅ DO accumulate transmittance physically (light propagates, scatters)

---

### VI.3.4: HDR Tone Mapping (6 hours)

**File**: `src/visualization/tone_mapping.rs`

```rust
pub fn tone_map_reinhard(linear: [f32; 3], exposure: f32, white_point: f32) -> [f32; 3] {
    // Reinhard tone mapping with white point
    let exposed = [
        linear[0] * exposure,
        linear[1] * exposure,
        linear[2] * exposure,
    ];

    [
        exposed[0] * (1.0 + exposed[0] / (white_point * white_point)) / (1.0 + exposed[0]),
        exposed[1] * (1.0 + exposed[1] / (white_point * white_point)) / (1.0 + exposed[1]),
        exposed[2] * (1.0 + exposed[2] / (white_point * white_point)) / (1.0 + exposed[2]),
    ]
}

pub fn tone_map_aces(linear: [f32; 3]) -> [f32; 3] {
    // Academy Color Encoding System (ACES) tone mapping
    // Better color preservation than Reinhard
    const A: f32 = 2.51;
    const B: f32 = 0.03;
    const C: f32 = 2.43;
    const D: f32 = 0.59;
    const E: f32 = 0.14;

    [
        apply_aces_curve(linear[0], A, B, C, D, E),
        apply_aces_curve(linear[1], A, B, C, D, E),
        apply_aces_curve(linear[2], A, B, C, D, E),
    ]
}

fn apply_aces_curve(x: f32, a: f32, b: f32, c: f32, d: f32, e: f32) -> f32 {
    (x * (a * x + b)) / (x * (c * x + d) + e)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_reinhard_clipping() { /* no negative or > 1 values */ }

    #[test]
    fn test_aces_color_accuracy() { /* preserves hue */ }

    #[test]
    fn test_exposure_compensation() { /* exposure slider works */ }
}
```

---

## Local Validation

```bash
#!/bin/bash
# Check: Ray tracing enabled (not fallback to rasterization)
if grep -q "rasterize\|fallback" src/visualization/ray_tracing_renderer.rs; then
    echo "❌ ERROR: Must use full ray tracing (no fallbacks)"
    exit 1
fi

# Check: Volumetric lighting includes heterodyne frequency
if ! grep -q "heterodyne\|frequency\|scattering" src/visualization/volumetric_lighting.rs; then
    echo "❌ ERROR: Volumetric lighting must include heterodyne scattering pattern"
    exit 1
fi

# Check: Probe-based GI (not screen-space)
if grep -q "screen.space\|SSGI" src/visualization/lumen_global_illumination.rs; then
    echo "❌ ERROR: Must use probe-based GI (not screen-space)"
    exit 1
fi

cargo test megalights_rendering --lib -- --nocapture
```

---

## Success Criteria

- [ ] DXR ray tracing pipeline compiles and runs
- [ ] BVH acceleration structure built for room geometry
- [ ] Ray-material intersections computed correctly
- [ ] Lumen probe grid (8×8×8 = 512 probes) updates dynamically
- [ ] Indirect lighting interpolates smoothly between probes
- [ ] Volumetric ray marching renders god rays in high-energy regions
- [ ] Heterodyne frequency visible in volumetric scattering pattern
- [ ] HDR tone mapping preserves color accuracy
- [ ] All 10 tests passing
- [ ] Frame rate > 30 fps with full pipeline (RT + GI + volumetric)

---

## Notes

**Parallelism**: VI.3 is independent of VI.1, VI.2 (reads their outputs, doesn't block them).

**Experimental features**: Ray tracing, volumetric effects, 3D textures all use wgpu experimental features. Backwards compatibility is not a concern.

**Generation protection**: Full ray tracing is mandatory. Volumetric god rays are RF-specific (heterodyne frequency determines scattering). Probe-based GI is scalable and dynamically updates.

---

## Integration Summary (VI.1 + VI.2 + VI.3)

**Data flow**:
1. **VI.1 (Chaos)**: RF field → Complex amplitude + phase per voxel
2. **VI.2 (Niagara)**: Energy field → Particles (emission, dynamics, mesh shaders)
3. **VI.3 (Megalights)**: Energy field → Ray tracing + GI + volumetric effects

**Output**: Real-time 3D visualization of RF-matter interaction, with:
- Particle dynamics reflecting material properties
- Ray-traced direct lighting from RF field
- Probe-based indirect illumination
- Volumetric god rays from heterodyne coupling

**Performance target**: 30+ fps on RX 6700 XT (full pipeline)

**Generation protection**: No approximations. Full physics at each layer (RF propagation, material response, ray tracing, volumetric effects).
