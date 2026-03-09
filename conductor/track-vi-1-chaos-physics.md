# Track VI.1: Chaos Physics Engine (RF-Matter Interaction)

**Domain**: Physics Simulation
**Ownership**: Physics Engineer (isolated from VI.2, VI.3)
**Duration**: 2-3 days
**Blocker on**: Nothing (physics is independent)
**Unblocks**: VI.2 (particle collision), VI.3 (lighting integration)

---

## Overview

Simulate how RF energy couples with physical materials and human presence. Model heterodyned acoustic fields interacting with room geometry, body shapes, and material properties. Output: 3D scalar field (energy density per voxel) that feeds particle emission and lighting.

**Generation protection**: RF-matter coupling must be phase-aware (not just magnitude). Heterodyne frequency mixing is critical—do not approximate. Material properties (hardness, roughness, wetness) directly modulate wave absorption/reflection. Never skip the body-interaction model (human presence fundamentally changes RF propagation).

---

## File Ownership

**VI.1 — Exclusive to this track**:
- `src/physics/rf_propagation.rs` (350 lines) — Wave equation solving, phase tracking
- `src/physics/material_absorption.rs` (200 lines) — Per-material RF absorption curves
- `src/physics/heterodyne_mixer.rs` (250 lines) — Frequency mixing, sideband generation
- `src/physics/body_interaction_model.rs` (200 lines) — Human presence effect on RF
- `src/physics/voxel_grid.rs` (180 lines) — 3D scalar field representation
- `tests/chaos_physics.rs` (300 lines, 15 tests)

**Read-only imports**:
- `src/analysis/pattern_library.rs` (Track K interface: load_pattern_library)
- `src/ml/pose_frame.rs` (Track I interface: human skeleton)
- `src/state.rs` (room geometry, frequency targets)

**No modifications to**:
- `src/main.rs` (dispatch loop)
- `src/visualization/` (VI.2, VI.3 handle rendering)

---

## Deliverables

### VI.1.1: RF Wave Propagation (16 hours)

**File**: `src/physics/rf_propagation.rs`

```rust
pub struct RFWavePropagation {
    grid: VoxelGrid<Complex<f32>>,  // Complex amplitude per voxel (phase-aware)
    frequency_hz: f32,
    wavelength_m: f32,
    speed_of_light: f32,
}

impl RFWavePropagation {
    pub fn new(grid_size: usize, freq_hz: f32) -> Self {
        let speed_of_light = 3e8;
        let wavelength = speed_of_light / freq_hz;
        Self {
            grid: VoxelGrid::new(grid_size),
            frequency_hz: freq_hz,
            wavelength_m: wavelength,
            speed_of_light,
        }
    }

    /// Solve wave equation: ∇²E = -k²E (Helmholtz equation)
    /// Using finite-difference frequency-domain (FDFD)
    pub fn solve_wave_equation(
        &mut self,
        source_position: (f32, f32, f32),
        source_amplitude: f32,
        material_grid: &VoxelGrid<Material>,
    ) -> Result<(), Box<dyn Error>> {
        let k = 2.0 * std::f32::consts::PI / self.wavelength_m;  // Wave number

        // FDFD solver: iterative (conjugate gradient or BiCG)
        // Set boundary condition: plane wave at source
        // Solve for field at all voxels
        // Track phase (not just magnitude)
    }

    /// Get field magnitude (energy density) and phase at position
    pub fn field_at(&self, pos: (f32, f32, f32)) -> (f32, f32) {
        let complex = self.grid.sample(pos);
        (complex.norm(), complex.arg())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_wave_equation_solver() { /* field satisfies Helmholtz */ }

    #[test]
    fn test_phase_continuity() { /* phase gradients smooth */ }

    #[test]
    fn test_energy_conservation() { /* power budget closes */ }

    #[test]
    fn test_plane_wave_source() { /* spherical wavefront at distance */ }

    #[test]
    fn test_high_frequency_accuracy() { /* 2.4 GHz wavelength = 12.5cm */ }
}
```

**Generation protection**:
- ✅ Phase-aware (Complex<f32>, not magnitude-only)
- ✅ Helmholtz equation solver (full wave physics)
- ❌ DON'T use geometric optics approximation (breaks at room scale)
- ❌ DON'T drop phase information (heterodyne mixing requires it)

---

### VI.1.2: Material Absorption Model (8 hours)

**File**: `src/physics/material_absorption.rs`

```rust
#[derive(Clone)]
pub struct Material {
    pub name: String,
    pub hardness: f32,           // 0.0-1.0 (0 = soft/absorbent, 1 = hard/reflective)
    pub roughness: f32,           // 0.0-1.0 (diffuse scattering)
    pub wetness: f32,             // 0.0-1.0 (water content, affects permittivity)
    pub permittivity: f32,        // ε_r (relative permittivity)
    pub conductivity: f32,        // σ (siemens per meter)
}

impl Material {
    /// Attenuation coefficient: α = ω * sqrt(εμ/2) * sqrt(1 + (σ/ωε)² - 1)
    /// Based on material loss tangent: tan(δ) = σ / (ωε)
    pub fn attenuation_coeff(&self, frequency_hz: f32) -> f32 {
        let omega = 2.0 * std::f32::consts::PI * frequency_hz;
        let epsilon_0 = 8.854e-12;
        let mu_0 = 4.0 * std::f32::consts::PI * 1e-7;

        let permittivity = self.permittivity * epsilon_0;
        let tan_delta = self.conductivity / (omega * permittivity);

        // Simplified: higher wetness → higher conductivity → more loss
        omega * (permittivity * mu_0).sqrt() * (1.0 + tan_delta * tan_delta).sqrt()
    }

    /// Reflection coefficient: R = |E_reflected / E_incident|
    /// Fresnel equations for normal incidence
    pub fn reflection_coeff(&self, frequency_hz: f32) -> f32 {
        // Z_material = sqrt(μ/ε) [impedance]
        // R = |(Z_material - Z_free) / (Z_material + Z_free)|
        let attenuation = self.attenuation_coeff(frequency_hz);
        // Simplified: hardness scales R linearly
        self.hardness.min(1.0)
    }

    /// Scattering coefficient: depends on roughness
    pub fn scattering_coeff(&self) -> f32 {
        self.roughness  // 0 = specular, 1 = diffuse
    }
}

pub struct MaterialGrid {
    grid: VoxelGrid<Material>,
}

impl MaterialGrid {
    pub fn from_room_geometry(room: &RoomGeometry) -> Self {
        // Populate voxel grid with materials based on room layout
        // Walls: drywall (moderate loss)
        // Floor/ceiling: concrete (high loss)
        // Human body: muscle tissue (very high loss, ~55% water)
    }

    pub fn attenuate_wave(&self, wave: Complex<f32>, distance: f32, material: &Material) -> Complex<f32> {
        let loss = (-material.attenuation_coeff(self.frequency_hz) * distance).exp();
        wave * loss
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_drywall_absorption() { /* ~10-20% loss per 10cm */ }

    #[test]
    fn test_water_absorption() { /* high loss at microwave frequencies */ }

    #[test]
    fn test_reflection_coeff_hardness() { /* hardness → R */ }

    #[test]
    fn test_scattering_roughness() { /* roughness → diffuse */ }
}
```

**Generation protection**:
- ✅ Material properties grounded in physics (permittivity, conductivity)
- ✅ Frequency-dependent absorption (critical for RF)
- ❌ DON'T use constant absorption (frequency scaling is real)
- ❌ DON'T ignore water content (human body is 60% water; dramatically changes RF)

---

### VI.1.3: Heterodyne Mixer (10 hours)

**File**: `src/physics/heterodyne_mixer.rs`

```rust
pub struct HeterodyneMixer {
    primary_freq: f32,           // Attack carrier (e.g., 2.4 GHz)
    modulation_freq: f32,        // Audio modulation (e.g., 4 kHz)
    heterodyne_freqs: Vec<f32>,  // Sideband frequencies
}

impl HeterodyneMixer {
    pub fn new(primary_hz: f32, modulation_hz: f32) -> Self {
        let lower_sideband = primary_hz - modulation_hz;
        let upper_sideband = primary_hz + modulation_hz;
        Self {
            primary_freq: primary_hz,
            modulation_freq: modulation_hz,
            heterodyne_freqs: vec![lower_sideband, primary_hz, upper_sideband],
        }
    }

    /// Mix two signals: a(t) * cos(ω₁t) × cos(ω₂t) = 0.5*cos((ω₁-ω₂)t) + 0.5*cos((ω₁+ω₂)t)
    pub fn mix_signals(
        &self,
        rf_field: Complex<f32>,
        audio_modulation: f32,
    ) -> Vec<Complex<f32>> {
        // RF × Audio modulation produces sidebands
        // Primary component (RF)
        // Lower sideband (f_primary - f_audio)
        // Upper sideband (f_primary + f_audio)

        vec![
            rf_field * 0.5 * Complex::new(audio_modulation.cos(), -audio_modulation.sin()),  // Lower
            rf_field,                                                                          // Primary
            rf_field * 0.5 * Complex::new(audio_modulation.cos(), audio_modulation.sin()),   // Upper
        ]
    }

    /// Energy in sideband relative to carrier
    pub fn sideband_efficiency(&self) -> f32 {
        // Modulation index: m = f_audio / f_carrier (for AM)
        let m = self.modulation_freq / self.primary_freq;
        // Sideband power: P_sidebands = (m²/4) * P_carrier
        (m * m / 4.0).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sideband_generation() { /* frequency components correct */ }

    #[test]
    fn test_phase_relationship() { /* sidebands phase-locked to carrier */ }

    #[test]
    fn test_modulation_depth() { /* m < 1 for AM */ }

    #[test]
    fn test_heterodyne_coupling() { /* RF + audio → RF field with amplitude modulation */ }
}
```

**Generation protection**:
- ✅ Heterodyne mixing is frequency-domain critical (FM synthesis requires this)
- ✅ Phase relationship between carrier and sidebands matters
- ❌ DON'T approximate heterodyning as simple multiplication (breaks phase coherence)
- ✅ DO track all three components (lower sideband, carrier, upper sideband)

---

### VI.1.4: Body Interaction Model (8 hours)

**File**: `src/physics/body_interaction_model.rs`

```rust
pub struct HumanBody {
    skeleton: PoseFrame,  // 33 keypoints from Track I
    voxel_map: VoxelGrid<f32>,  // Occupancy per voxel (0.0-1.0)
}

impl HumanBody {
    pub fn from_pose(pose: &PoseFrame) -> Self {
        // Voxelize skeleton + simple cylinders (arms, legs, torso)
        // Muscle tissue: ε_r ≈ 50, σ ≈ 1.0 S/m (very lossy)
        let mut voxel_map = VoxelGrid::new(64);
        for keypoint in &pose.keypoints {
            // Render cylinder from joint to joint
            // Set occupancy to 1.0 for human tissue, 0.0 for air
        }
        Self { skeleton: pose.clone(), voxel_map }
    }

    /// RF field attenuation due to human body (major effect)
    /// Human tissue @ 2.4 GHz: ~50-70% absorption per 10cm
    pub fn attenuate_rf_field(&self, field: Complex<f32>, distance: f32, freq_hz: f32) -> Complex<f32> {
        // Muscle tissue attenuation: ~0.3-0.5 nepers/cm at 2.4 GHz
        let muscle_attenuation_per_m = 35.0;  // Empirical
        let occupancy_factor = self.voxel_map.average_along_path(distance);

        let loss = (-muscle_attenuation_per_m * distance * occupancy_factor).exp();
        field * loss
    }

    /// Position-dependent body shielding
    /// Azimuth facing away from RF source? Shield increases.
    pub fn shielding_factor(&self, rf_azimuth: f32, body_facing: f32) -> f32 {
        let angle_to_rf = (rf_azimuth - body_facing).abs();
        // Facing away (180°): maximum shielding (~0.7)
        // Facing toward (0°): minimum shielding (~0.2)
        0.2 + 0.5 * (-angle_to_rf.abs() / std::f32::consts::PI).exp()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_body_voxelization() { /* cylinders render correctly */ }

    #[test]
    fn test_muscle_attenuation() { /* ~50% loss per 10cm */ }

    #[test]
    fn test_shielding_facing() { /* facing away = higher shielding */ }

    #[test]
    fn test_pose_update() { /* body model follows pose changes */ }
}
```

**Generation protection**:
- ✅ Human body is RF-opaque (muscle tissue ~60% water, ~50-70% absorption)
- ✅ Body orientation (facing) affects RF propagation
- ❌ DON'T treat human as invisible (common RF modeling error)
- ✅ DO track body-to-source geometry (shielding depends on relative position)

---

### VI.1.5: Voxel Grid & Field Storage (6 hours)

**File**: `src/physics/voxel_grid.rs`

```rust
pub struct VoxelGrid<T: Clone> {
    data: Vec<T>,
    dimensions: (usize, usize, usize),  // (X, Y, Z)
    voxel_size_m: f32,
}

impl<T: Clone> VoxelGrid<T> {
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![T::default(); size * size * size],
            dimensions: (size, size, size),
            voxel_size_m: 0.1,  // 10cm voxels
        }
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, value: T) {
        let idx = x + y * self.dimensions.0 + z * self.dimensions.0 * self.dimensions.1;
        if idx < self.data.len() {
            self.data[idx] = value;
        }
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> T {
        let idx = x + y * self.dimensions.0 + z * self.dimensions.0 * self.dimensions.1;
        self.data.get(idx).cloned().unwrap_or_default()
    }

    pub fn sample(&self, pos: (f32, f32, f32)) -> T {
        // Trilinear interpolation for continuous positions
    }
}
```

---

## Interface Contract (For VI.2 and VI.3)

**Export from VI.1**:
```rust
pub fn solve_rf_field(
    primary_freq: f32,
    source_pos: (f32, f32, f32),
    body_pose: &PoseFrame,
    room_geometry: &RoomGeometry,
) -> Result<VoxelGrid<Complex<f32>>, Box<dyn Error>> {
    // VI.2 imports this
    // Returns energy density field (can be rendered as particles)
}

pub struct EnergyDensityField {
    pub magnitude_grid: VoxelGrid<f32>,     // |E| per voxel
    pub phase_grid: VoxelGrid<f32>,         // ∠E per voxel
}
```

VI.2 and VI.3 read this interface without modification.

---

## Local Validation

```bash
#!/bin/bash
# Check: Phase-aware (Complex<f32>, not f32)
if ! grep -q "Complex<f32>" src/physics/rf_propagation.rs; then
    echo "❌ ERROR: Wave field must be Complex<f32> (phase-critical)"
    exit 1
fi

# Check: Heterodyne mixer includes all three components
if ! grep -q "lower_sideband\|primary\|upper_sideband" src/physics/heterodyne_mixer.rs; then
    echo "❌ ERROR: Heterodyne mixer missing sideband components"
    exit 1
fi

# Check: Body interaction model included (non-negotiable)
if ! grep -q "human\|body\|muscle\|attenuation" src/physics/body_interaction_model.rs; then
    echo "❌ ERROR: Body interaction model missing"
    exit 1
fi

cargo test chaos_physics --lib -- --nocapture
```

---

## Success Criteria

- [ ] Helmholtz wave equation solver converges
- [ ] Phase information preserved (Complex<f32>)
- [ ] Material absorption frequency-dependent
- [ ] Heterodyne mixing produces correct sidebands
- [ ] Human body attenuates RF correctly (~50-70% loss)
- [ ] Body orientation affects shielding
- [ ] Voxel grid samples smoothly (trilinear interpolation)
- [ ] All 15 tests passing
- [ ] Interface stable (VI.2, VI.3 import without modification)

---

## Notes

**Parallelism**: VI.1 is independent of VI.2 and VI.3 (architecture layer). Can develop all three in parallel.

**Generation protection**: Phase-awareness and body interaction are non-negotiable. Do not approximate.
