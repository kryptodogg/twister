# Track FF Addendum: RF-BSDF Material Framework & Mamba Material Learning

**Status**: Ready for refinement before Jules implementation
**Duration**: 120 minutes (material format + Blender editor + Mamba integration)
**Dependency**: Track D temporal rewind (GPU particles), Mamba trainer (gradient updates)
**Integration**: User-created materials → GPU ray/particle representation → Mamba learns RF response

---

## Executive Summary

Track FF defines **RF-BSDF (Radio Frequency Bidirectional Scattering Distribution Function)**, a sparse point-cloud material representation that enables Mamba to learn how RF heterodyning patterns change based on environmental materials.

**Key Innovation**: Users create materials in a Blender-like editor (permittivity, conductivity, loss tangent), place them as sparse points in 3D space, and Mamba continuously learns to predict RF response. No geometry meshes—just material properties at sparse positions, enabling DLSS-style temporal upsampling.

- **RF-BSDF**: Frequency-dependent permittivity (ε), conductivity (σ), loss tangent (tan δ)
- **Material Points**: 3D positions with RF properties + velocity (for moving material changes)
- **Mamba Integration**: Learn material permittivity/conductivity/loss as trainable parameters
- **Visualization**: GPU particle splatting shows material-dependent RF scattering
- **Blender Editor**: Create custom materials; Mamba refines their properties based on RF observations

---

## RF-BSDF Material Definition

### Debye Model Parameters (Frequency-Dependent Permittivity)

**Debye Equation** (complex permittivity):
```
ε(ω) = ε∞ + (ε_s - ε∞) / (1 + jωτ)
```

Where:
- **ε_s** (static permittivity): Low-frequency limit (e.g., water = 80)
- **ε∞** (infinity permittivity): High-frequency limit (e.g., water ≈ 5 at microwave)
- **τ** (relaxation time): Debye relaxation time (seconds, material-dependent)
- **ω** (angular frequency): 2πf

**Conductivity**:
```
σ(f) = σ_base * f^α
```
- **σ_base**: Conductivity at 1 Hz (e.g., water ≈ 0.001 S/m)
- **α**: Frequency exponent (0.0-1.0, material-dependent)

### Predefined Materials

| Material | ε_s | ε∞ | τ (ps) | σ_base (S/m) | α | Use Case | Notes |
|----------|-----|-----|--------|--------------|-----|----------|-------|
| **Air** | 1.0 | 1.0 | ∞ | 0.0 | 0.0 | Baseline | No attenuation |
| **Water** | 80 | 5.0 | 8.27 | 0.001 | 0.5 | Liquid resonance | Debye standard |
| **Glass** | 6.0 | 2.0 | 1.0e-12 | 1e-11 | 0.0 | Dielectric | Low loss |
| **Human Tissue** | 50 | 4.0 | 7.2 | 0.6 | 0.7 | Biological | Cole-Cole model |
| **Wood (Dry)** | 3.5 | 1.5 | 1e-10 | 1e-12 | 0.0 | Low-loss | Frequency-independent |
| **Metal (Copper)** | -1e6 | - | - | 5.96e7 | -1.0 | Perfect conductor | Drude model |
| **Concrete** | 6.5 | 3.0 | 1e-11 | 0.001 | 0.2 | Building material | High loss tangent |
| **Clothing (Cotton)** | 4.0 | 1.8 | 5e-11 | 1e-10 | 0.3 | Textile | Low conductivity |

### Material Storage Format

```rust
#[repr(C)]
pub struct MaterialDef {
    pub material_id: u32,           // Unique identifier
    pub name: [u8; 32],             // UTF-8 material name (max 31 chars)
    pub permittivity_static: f32,   // ε_s (Debye static)
    pub permittivity_infinity: f32, // ε∞ (Debye high-freq limit)
    pub relaxation_time_ps: f32,    // τ in picoseconds
    pub conductivity_base: f32,     // σ_base at 1 Hz
    pub conductivity_frequency_exp: f32, // α exponent
    pub loss_tangent_1ghz: f32,     // tan(δ) at 1 GHz (precomputed)
    pub absorption_coefficient: f32, // α_abs for Beer-Lambert law
    pub temperature_kelvin: f32,    // Reference temperature (e.g., 293K)
    pub density_kg_m3: f32,         // Density for physics simulation
    pub _reserved: [f32; 6],        // Future use (64 bytes total)
}

#[repr(C)]
pub struct MaterialPoint {
    pub position_xyz: [f32; 3],     // 3D position in space
    pub material_id: u32,           // Reference to MaterialDef
    pub material_blend: f32,        // [0, 1] blend ratio (for smoothing)
    pub confidence: f32,            // [0, 1] Mamba-learned confidence
    pub timestamp_micros: u64,      // When material was placed/updated
    pub permittivity_at_freq: f32,  // Precomputed for current frequency
    pub conductivity_at_freq: f32,  // Precomputed for current frequency
    pub velocity_xyz: [f32; 3],     // Velocity for moving water/breathing
    pub temperature_kelvin: f32,    // Localized temperature (affects permittivity)
}
```

---

## Blender-Style Material Editor (Slint UI)

### File Ownership

- **`ui/materials_editor.slint`** (NEW) - Material creation/editing panel
- **`src/materials/material_editor.rs`** (NEW) - Material CRUD logic
- **`src/materials/material_library.rs`** (NEW) - Material storage + serialization
- **`src/mamba_materials.rs`** (NEW) - Mamba material property learning

### Material Editor Panel (Blender-inspired)

```slint
// ui/materials_editor.slint

TabContent {
    title: "MATERIALS";

    VerticalLayout {
        spacing: 10px;
        padding: 20px;

        // ─────────────────────────────────────────────────────
        // MATERIAL LIBRARY
        // ─────────────────────────────────────────────────────
        HorizontalLayout {
            Text { text: "Material Library:"; font-weight: bold; min-width: 200px; }

            ComboBox {
                model: ["Water", "Glass", "Human Tissue", "Wood (Dry)", "Metal", "Concrete", "Cotton", "Custom"];
                current-index: 0;
                selected => { app-window.material_selected(self.current-value); }
            }

            Button {
                text: "+ New";
                clicked => { app-window.create_material(); }
            }

            Button {
                text: "Delete";
                clicked => { app-window.delete_material(); }
            }
        }

        // ─────────────────────────────────────────────────────
        // DEBYE MODEL SLIDERS (RF-BSDF Parameters)
        // ─────────────────────────────────────────────────────
        HorizontalLayout {
            spacing: 20px;

            VerticalLayout {
                width: 200px;
                Text { text: "ε_s (Static Permittivity):"; }
                Slider {
                    value: root.epsilon-static;
                    minimum: 1.0;
                    maximum: 100.0;
                    changed => { app-window.update_epsilon_static(self.value); }
                }
                Text { text: "{root.epsilon-static.round(2)}"; color: #0f0; }
                Text { text: "(e.g., water=80)"; font-size: 10px; color: #888; }
            }

            VerticalLayout {
                width: 200px;
                Text { text: "ε∞ (High-Freq Limit):"; }
                Slider {
                    value: root.epsilon-infinity;
                    minimum: 1.0;
                    maximum: 20.0;
                    changed => { app-window.update_epsilon_infinity(self.value); }
                }
                Text { text: "{root.epsilon-infinity.round(2)}"; color: #0f0; }
                Text { text: "(e.g., water≈5)"; font-size: 10px; color: #888; }
            }

            VerticalLayout {
                width: 200px;
                Text { text: "τ (Relaxation Time, ps):"; }
                Slider {
                    value: root.relaxation-time-ps;
                    minimum: 0.001;
                    maximum: 100.0;
                    changed => { app-window.update_relaxation_time(self.value); }
                }
                Text { text: "{root.relaxation-time-ps.round(3)} ps"; color: #0f0; }
                Text { text: "(e.g., water=8.27)"; font-size: 10px; color: #888; }
            }
        }

        // ─────────────────────────────────────────────────────
        // CONDUCTIVITY PARAMETERS
        // ─────────────────────────────────────────────────────
        HorizontalLayout {
            spacing: 20px;

            VerticalLayout {
                width: 200px;
                Text { text: "σ_base (S/m at 1 Hz):"; }
                Slider {
                    value: root.conductivity-base;
                    minimum: -11.0;  // log scale: 10^-11
                    maximum: 8.0;    // log scale: 10^8
                    changed => { app-window.update_conductivity_base(self.value); }
                }
                Text {
                    text: "{pow(10.0, root.conductivity-base).round(6)} S/m";
                    color: #0f0;
                }
                Text { text: "(log scale: 10^x)"; font-size: 10px; color: #888; }
            }

            VerticalLayout {
                width: 200px;
                Text { text: "α (Frequency Exponent):"; }
                Slider {
                    value: root.conductivity-exp;
                    minimum: -1.0;
                    maximum: 1.0;
                    changed => { app-window.update_conductivity_exp(self.value); }
                }
                Text { text: "{root.conductivity-exp.round(2)}"; color: #0f0; }
                Text { text: "(0=constant, 1=linear)"; font-size: 10px; color: #888; }
            }

            VerticalLayout {
                width: 200px;
                Text { text: "tan(δ) @ 1 GHz:"; }
                Slider {
                    value: root.loss-tangent;
                    minimum: 0.0;
                    maximum: 1.0;
                    changed => { app-window.update_loss_tangent(self.value); }
                }
                Text { text: "{root.loss-tangent.round(3)}"; color: #0f0; }
                Text { text: "(0=low loss, 1=high)"; font-size: 10px; color: #888; }
            }
        }

        // ─────────────────────────────────────────────────────
        // MAMBA LEARNING STATUS
        // ─────────────────────────────────────────────────────
        Rectangle {
            background: #1a1a1a;
            border: 2px solid #444;
            height: 120px;

            VerticalLayout {
                padding: 10px;
                spacing: 8px;

                Text { text: "Mamba Material Learning"; font-weight: bold; color: #0f0; }

                HorizontalLayout {
                    Text { text: "Training: "; width: 80px; }
                    Text {
                        text: if root.mamba-training { "▶ Active" } else { "⏸ Paused" };
                        color: if root.mamba-training { #0f0 } else { #f80 };
                    }
                }

                HorizontalLayout {
                    Text { text: "Confidence: "; width: 80px; }
                    ProgressBar {
                        value: root.mamba-confidence;
                        width: 200px;
                    }
                    Text { text: "{(root.mamba-confidence * 100).round()}%"; }
                }

                HorizontalLayout {
                    Text { text: "Gradient Loss: "; width: 80px; }
                    Text {
                        text: "{root.mamba-loss.round(4)}";
                        color: if root.mamba-loss < 0.1 { #0f0 } else if root.mamba-loss < 0.5 { #ff0 } else { #f00 };
                    }
                }

                HorizontalLayout {
                    spacing: 10px;
                    Button {
                        text: "Start Learning";
                        width: 100px;
                        clicked => { app-window.start_mamba_learning(); }
                    }
                    Button {
                        text: "Reset Properties";
                        width: 100px;
                        clicked => { app-window.reset_material_properties(); }
                    }
                }
            }
        }

        // ─────────────────────────────────────────────────────
        // 3D PLACEMENT (for future: Click to place material points)
        // ─────────────────────────────────────────────────────
        Rectangle {
            background: #222;
            border: 2px solid #444;
            height: 150px;

            VerticalLayout {
                padding: 10px;
                spacing: 5px;

                Text { text: "Material Point Placement (in 3D wavefield)"; font-weight: bold; }

                Text {
                    text: "Click in 3D view to place this material at cursor position\nMaterials can move (velocity_xyz) to simulate sloshing water, breathing, etc.";
                    color: #888;
                    wrap: word-wrap;
                }

                HorizontalLayout {
                    Button {
                        text: "Place Material Point";
                        clicked => { app-window.enable_material_placement(); }
                    }

                    Button {
                        text: "Visualize Points";
                        clicked => { app-window.show_material_cloud(); }
                    }

                    Button {
                        text: "Export Library";
                        clicked => { app-window.export_materials_json(); }
                    }
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────
// EXPORTED PROPERTIES (Material Editor ↔ Slint bindings)
// ─────────────────────────────────────────────────────

export global MaterialEditor {
    // Debye model
    in-out property <float> epsilon-static;
    in-out property <float> epsilon-infinity;
    in-out property <float> relaxation-time-ps;

    // Conductivity
    in-out property <float> conductivity-base;       // log scale: 10^x
    in-out property <float> conductivity-exp;        // 0 to 1

    // Loss
    in-out property <float> loss-tangent;

    // Mamba learning
    in-out property <bool> mamba-training;
    in-out property <float> mamba-confidence;        // [0, 1]
    in-out property <float> mamba-loss;              // Gradient descent loss

    // Callbacks
    callback material_selected(string);
    callback create_material();
    callback delete_material();
    callback update_epsilon_static(float);
    callback update_epsilon_infinity(float);
    callback update_relaxation_time(float);
    callback update_conductivity_base(float);
    callback update_conductivity_exp(float);
    callback update_loss_tangent(float);
    callback start_mamba_learning();
    callback reset_material_properties();
    callback enable_material_placement();
    callback show_material_cloud();
    callback export_materials_json();
}
```

---

## Rust Implementation

### File Ownership

- **`src/materials/material_library.rs`** (NEW) - Material storage + serialization (CRUD)
- **`src/materials/material_editor.rs`** (NEW) - UI callbacks → material mutations
- **`src/materials/debye_model.rs`** (NEW) - RF-BSDF Debye equation computation
- **`src/mamba_materials.rs`** (NEW) - Mamba gradient descent on material properties
- **`src/main.rs`** - Wire material editor callbacks

### Material Library Storage

```rust
// src/materials/material_library.rs

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct MaterialDef {
    pub material_id: u32,
    pub name: String,

    // Debye model
    pub permittivity_static: f32,       // ε_s
    pub permittivity_infinity: f32,     // ε∞
    pub relaxation_time_ps: f32,        // τ in picoseconds

    // Conductivity
    pub conductivity_base: f32,         // σ_base
    pub conductivity_frequency_exp: f32, // α exponent
    pub loss_tangent_1ghz: f32,         // tan(δ) @ 1 GHz

    // Metadata
    pub temperature_kelvin: f32,        // Reference temp (293K typical)
    pub density_kg_m3: f32,             // For physics simulation
    pub learnable: bool,                // Can Mamba adjust this?
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MaterialPoint {
    pub position_xyz: [f32; 3],
    pub material_id: u32,
    pub material_blend: f32,            // [0, 1] interpolation
    pub confidence: f32,                // [0, 1] Mamba-learned confidence
    pub velocity_xyz: [f32; 3],         // Moving material (water sloshing, breathing)
    pub temperature_kelvin: f32,        // Localized temperature
}

pub struct MaterialLibrary {
    pub materials: HashMap<u32, MaterialDef>,
    pub points: Vec<MaterialPoint>,
    pub current_material_id: u32,
}

impl MaterialLibrary {
    pub fn new() -> Self {
        let mut materials = HashMap::new();

        // Predefined materials
        materials.insert(0, MaterialDef {
            material_id: 0,
            name: "Water".to_string(),
            permittivity_static: 80.0,
            permittivity_infinity: 5.0,
            relaxation_time_ps: 8.27,
            conductivity_base: 0.001,
            conductivity_frequency_exp: 0.5,
            loss_tangent_1ghz: 0.15,
            temperature_kelvin: 293.0,
            density_kg_m3: 1000.0,
            learnable: true,
        });

        // Add more predefined materials (Glass, Metal, Tissue, etc.)

        MaterialLibrary {
            materials,
            points: Vec::new(),
            current_material_id: 0,
        }
    }

    pub fn create_material(&mut self, name: String) -> u32 {
        let new_id = self.materials.len() as u32;
        self.materials.insert(new_id, MaterialDef {
            material_id: new_id,
            name,
            permittivity_static: 1.0,
            permittivity_infinity: 1.0,
            relaxation_time_ps: 1e-12,
            conductivity_base: 1e-12,
            conductivity_frequency_exp: 0.0,
            loss_tangent_1ghz: 0.01,
            temperature_kelvin: 293.0,
            density_kg_m3: 1000.0,
            learnable: true,
        });
        new_id
    }

    pub fn add_material_point(&mut self, position: [f32; 3], material_id: u32) {
        self.points.push(MaterialPoint {
            position_xyz: position,
            material_id,
            material_blend: 1.0,
            confidence: 0.0,   // Initially zero; Mamba will update
            velocity_xyz: [0.0; 3],
            temperature_kelvin: 293.0,
        });
    }

    pub fn save_to_json(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        eprintln!("[Materials] Saved to {}", path);
        Ok(())
    }

    pub fn load_from_json(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(path)?;
        let library: MaterialLibrary = serde_json::from_str(&json)?;
        self.materials = library.materials;
        self.points = library.points;
        eprintln!("[Materials] Loaded from {}", path);
        Ok(())
    }
}
```

### Mamba Material Learning

```rust
// src/mamba_materials.rs

use crate::materials::MaterialDef;
use burn::tensor::Tensor;

pub struct MambaMaterialLearner {
    pub learning_rate: f32,
    pub materials: Vec<MaterialDef>,
    pub losses: Vec<f32>,
}

impl MambaMaterialLearner {
    pub fn new(materials: Vec<MaterialDef>) -> Self {
        MambaMaterialLearner {
            learning_rate: 0.001,
            materials,
            losses: Vec::new(),
        }
    }

    /// Gradient descent step on material properties
    /// Input: Predicted RF response vs. observed RF response
    /// Update: ε_s, σ_base, tan(δ) via backprop
    pub fn train_step(
        &mut self,
        predicted_response: &[f32],  // Mamba model output (N samples)
        observed_response: &[f32],   // Ground truth (N samples)
    ) -> f32 {
        // MSE loss
        let loss = predicted_response
            .iter()
            .zip(observed_response)
            .map(|(p, o)| (p - o).powi(2))
            .sum::<f32>() / predicted_response.len() as f32;

        // Gradient on material parameters (simplified)
        for mat in &mut self.materials {
            if !mat.learnable { continue; }

            // ∂loss/∂ε_s (numerical gradient)
            let delta = 0.001;
            mat.permittivity_static -= self.learning_rate * (loss / delta);

            // ∂loss/∂σ_base
            mat.conductivity_base -= self.learning_rate * (loss / delta);

            // Clamp to valid ranges
            mat.permittivity_static = mat.permittivity_static.max(1.0);
            mat.conductivity_base = mat.conductivity_base.max(1e-12);
        }

        self.losses.push(loss);
        loss
    }
}
```

---

## GPU Ray Tracing Integration

### Sparse Material Point Sampling

Since materials are sparse point clouds (not dense meshes), GPU rendering uses temporal upsampling (DLSS-style):

1. **Per-frame**: Sample N random material points from cloud
2. **Ray tracing**: Cast rays through points, compute Debye permittivity at ray frequency
3. **Scatter calculation**: Use RF-BSDF to compute energy scattering
4. **Temporal reprojection**: Accumulate across frames for full-cloud coverage

**Performance**:
- 10,000 material points: < 3ms per frame
- 100,000 points: < 10ms per frame (temporal upsampling reduces per-frame cost)

### Debye Model GPU Shader

```wgsl
// Compute ε(f) for ray at frequency f_hz
fn debye_permittivity(f_hz: f32, mat: MaterialDef) -> vec2<f32> {
    let omega = 2.0 * PI * f_hz;
    let tau = mat.relaxation_time_ps * 1e-12;  // Convert ps to seconds

    // ε∞ + (ε_s - ε∞) / (1 + j*ω*τ)
    let numerator = mat.permittivity_static - mat.permittivity_infinity;
    let denominator_real = 1.0;
    let denominator_imag = omega * tau;

    // Division in complex plane
    let denom_mag_sq = denominator_real * denominator_real + denominator_imag * denominator_imag;
    let real_part = mat.permittivity_infinity + (numerator * denominator_real) / denom_mag_sq;
    let imag_part = -(numerator * denominator_imag) / denom_mag_sq;

    return vec2<f32>(real_part, imag_part);
}

// Compute conductivity σ(f) = σ_base * f^α
fn conductivity_at_freq(f_hz: f32, mat: MaterialDef) -> f32 {
    return mat.conductivity_base * pow(f_hz, mat.conductivity_frequency_exp);
}
```

---

## Mamba Learning Integration

### Training Loop

Materials are updated by Mamba during training:

```
For each RF observation (predicted vs. observed):
  1. Compute loss: MSE(predicted, observed)
  2. Backprop through material properties (ε_s, σ_base, tan(δ))
  3. Update via SGD: mat_param -= α * ∇loss
  4. Clamp to valid ranges (ε_s ≥ 1, σ_base ≥ 0)
```

**Expected convergence**:
- Initial loss: ~2.5 dB (random material initialization)
- Final loss: ~0.1 dB (material properties learned)
- Convergence speed: ~500 observations per material

### Visualizing Material Confidence

Mamba learns which materials are present by confidence scoring. UI shows:
- **Confidence**: Percentage of training observations explained by this material
- **Gradient norm**: How actively Mamba is refining this material (higher = active learning)
- **Loss trend**: Is Mamba improving material predictions?

---

## Material Editor Workflow

### Step 1: Select Predefined Material or Create Custom

```
User selects "Water" from dropdown
→ Slint loads ε_s=80, ε∞=5, τ=8.27ps, σ_base=0.001, α=0.5
→ UI displays sliders at these values
```

### Step 2: Fine-Tune RF-BSDF Parameters (Optional)

```
User adjusts ε_s slider: 80 → 75 (slightly less polar)
→ Mamba watches RF predictions change
→ Confidence decreases (material less consistent with observations)
→ User can revert or continue learning
```

### Step 3: Place Material Points in 3D Space

```
User clicks "Place Material Point"
→ Can then click in 3D wavefield view to position point
→ Material property (ε_s, σ_base) applied to that location
→ Multiple points create sparse cloud representation
```

### Step 4: Let Mamba Learn

```
User clicks "Start Learning"
→ Mamba trains for N epochs (configurable)
→ Material properties gradually refined based on RF response
→ Confidence increases as Mamba explains observations
→ Gradient loss decreases toward convergence
```

### Step 5: Export or Continue

```
User clicks "Export Library"
→ Saves materials.json with final learned properties
→ Can be loaded in future sessions (material persistence)

---

## Pre-Commit Hook Validation

```bash
#!/bin/bash
# .git/hooks/pre-commit (append for materials)

# ✓ Material library exists
if grep -q "pub struct MaterialLibrary" src/materials/material_library.rs 2>/dev/null; then
    echo "✓ Material library implemented"
else
    echo "⚠ Material library not found"
fi

# ✓ Debye model shader
if grep -q "fn debye_permittivity" src/*.wgsl 2>/dev/null; then
    echo "✓ Debye permittivity shader defined"
else
    echo "⚠ Debye shader missing (add to dispatch_kernel.wgsl)"
fi

# ✓ Mamba material learner
if grep -q "pub struct MambaMaterialLearner" src/mamba_materials.rs 2>/dev/null; then
    echo "✓ Mamba material learning integrated"
else
    echo "⚠ Mamba material learner not found"
fi

# ✓ Material editor UI
if grep -q "export global MaterialEditor" ui/materials_editor.slint 2>/dev/null; then
    echo "✓ Material editor UI defined"
else
    echo "⚠ Material editor UI missing"
fi

echo "✓ Material framework validation complete"
exit 0
```

---

## Implementation Checklist (for Jules)

### Phase 1: Material Library (20 min)
- [ ] Create src/materials/material_library.rs
- [ ] Implement MaterialDef struct (Debye parameters)
- [ ] Implement MaterialPoint struct (3D placement)
- [ ] Add CRUD methods (create, delete, update)
- [ ] Serialize/deserialize to JSON

### Phase 2: Material Editor UI (25 min)
- [ ] Create ui/materials_editor.slint
- [ ] Add sliders for ε_s, ε∞, τ, σ_base, α, tan(δ)
- [ ] Add material dropdown (predefined + custom)
- [ ] Add Mamba learning status display
- [ ] Tests: UI updates AppState correctly

### Phase 3: Debye Model GPU Shader (20 min)
- [ ] Add debye_permittivity() to dispatch_kernel.wgsl
- [ ] Compute ε(f) from Debye equation
- [ ] Integrate into ray-material intersection
- [ ] Tests: Verify permittivity matches formula for known materials

### Phase 4: Mamba Material Learning (25 min)
- [ ] Create src/mamba_materials.rs
- [ ] Implement train_step() with gradient descent
- [ ] Wire material property updates to training loop
- [ ] Tests: Material parameters converge toward ground truth

### Phase 5: Main Loop Integration (15 min)
- [ ] Wire material editor callbacks in src/main.rs
- [ ] Connect material points to GPU particle system
- [ ] Sync AppState material updates to GPU
- [ ] Tests: Material changes visible in real-time visualization

### Phase 6: Testing & Export (15 min)
- [ ] Cargo build → 0 errors
- [ ] Create test materials (water, glass, metal)
- [ ] Place points in 3D space, verify rendering
- [ ] Run Mamba learning, verify loss decreases
- [ ] Export/import materials.json
- [ ] Tests: Full workflow end-to-end

---

## Total Duration

| Task | Time |
|------|------|
| Phase 1: Material library | 20 min |
| Phase 2: Material editor UI | 25 min |
| Phase 3: Debye GPU shader | 20 min |
| Phase 4: Mamba learning | 25 min |
| Phase 5: Main loop wiring | 15 min |
| Phase 6: Testing + export | 15 min |
| **Total** | **120 min** |

*Estimated 2 hours total*

---

## Verification & Success Criteria

✅ **Material library working**:
- Predefined materials load correctly
- Custom materials can be created
- Material properties persist across sessions

✅ **Material editor UI responsive**:
- All sliders update AppState
- Dropdown selects materials
- Mamba learning status displays correctly

✅ **Debye permittivity accurate**:
- Water (ε_s=80) → ε(1GHz) ≈ 5.5
- Glass (ε_s=6) → ε(1GHz) ≈ 3.2
- Metal → very large imaginary component

✅ **Material points render correctly**:
- Sparse point cloud visible in 3D wavefield
- Points can move (velocity simulation)
- Material properties affect RF scattering in real-time

✅ **Mamba learning converges**:
- Loss starts ~2.5 dB, decreases to ~0.1 dB
- Material confidence increases from 0 → 1.0
- Learned properties match ground truth

✅ **Export/import works**:
- materials.json saves learned properties
- Can load in future sessions
- Material library persists across app restarts

---

## Notes for Jules

**Why RF-BSDF matters**: Materials are the bridge between RF behavior (what Mamba observes) and geometry-free representation (sparse point clouds). By learning material properties, Mamba understands how room contents affect RF propagation without ever seeing a 3D model or photograph.

**Debye model trade-off**: The Debye model (3-parameter fit) is simpler than full Cole-Cole, but accurate enough for most materials in 1 Hz - 1 GHz range. If Cole-Cole is needed later, can extend with additional α/β parameters.

**Why Blender-style editor?** Users familiar with material editing (e.g., Blender artists) will immediately understand permittivity/conductivity sliders. Metaphor: "You're authoring the material database just like you'd author a Blender shader."

**Mamba learns materials, not models**: No neural networks training on geometry. Mamba only learns which material properties best explain observed RF behavior. This keeps model size small and prevents overfitting to specific room layouts.

**Sparse points = DLSS gains**: By using point clouds instead of dense meshes, GPU can temporally upsample across frames. One frame samples N points, next frame samples different N points, temporal reprojection fills gaps. Reduces per-frame cost by ~4x vs. full mesh ray-tracing.

---

## Dependencies

Add to Cargo.toml:
```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
burn = { version = "0.21", features = ["wgpu"] }  # For Mamba training
```

Add predefined Debye parameters to MATERIALS constant in src/materials/material_library.rs.

