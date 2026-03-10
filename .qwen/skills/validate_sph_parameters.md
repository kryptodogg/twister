# Skill: validate_sph_parameters

## Overview

Validates Smoothed Particle Hydrodynamics (SPH) parameters for Aether fluid simulation. Ensures kernel functions, density evaluation, and pressure solve are physically correct and GPU-efficient.

## Applicable Agents

- `aether-fluid-specialist`
- `physics-mathematician`
- `rdna2-compute-specialist`

## Execution

```bash
# Run SPH parameter validation
python scripts/validate_sph.py --config <CONFIG_JSON> --particle_count <N>

# Example: Validate 1M particle simulation
python scripts/validate_sph.py --config domains/physics/aether/config/sph_params.json --particle_count 1000000
```

## Validation Criteria

### Pass Conditions
- Kernel normalization: ∫W(r,h)dr = 1
- Density evaluation: ρ₀ = 1000 kg/m³ (water) within 1%
- Pressure solve: Ideal gas law P = k(ρ - ρ₀) stable
- Smoothing radius: h = 0.2m (configurable)
- CFL condition: Δt < h / c_sound satisfied
- GPU performance: > 30 FPS at 1M particles

### Fail Conditions
- Kernel not normalized (integral ≠ 1)
- Density error > 1% from rest density
- Pressure instability (exploding particles)
- CFL violation (Δt too large)
- GPU performance < 30 FPS at 1M particles

## Detection Patterns

The validator detects SPH implementations by:
- Function names: `sph_density`, `sph_pressure`, `sph_forces`
- Kernel names: `poly6`, `spiky`, `viscosity`
- Variable patterns: `smoothing_radius`, `rest_density`, `gas_constant`

## Output Format

```json
{
  "config": "domains/physics/aether/config/sph_params.json",
  "particle_count": 1000000,
  "tests": [
    {
      "name": "kernel_normalization",
      "kernel": "Poly6",
      "expected_integral": 1.0,
      "computed_integral": 0.9998,
      "error_percent": 0.02,
      "status": "PASS"
    },
    {
      "name": "density_evaluation",
      "rest_density_kg_m3": 1000.0,
      "computed_density_kg_m3": 1002.3,
      "error_percent": 0.23,
      "target_percent": 1.0,
      "status": "PASS"
    },
    {
      "name": "pressure_stability",
      "max_pressure_pa": 2500.0,
      "min_pressure_pa": 980.0,
      "stable": true,
      "status": "PASS"
    },
    {
      "name": "cfl_condition",
      "smoothing_radius_m": 0.2,
      "sound_speed_m_s": 50.0,
      "max_dt_s": 0.004,
      "actual_dt_s": 0.001,
      "satisfied": true,
      "status": "PASS"
    },
    {
      "name": "gpu_performance",
      "particle_count": 1000000,
      "frame_time_ms": 28.5,
      "fps": 35.1,
      "target_fps": 30.0,
      "status": "PASS"
    }
  ],
  "summary": {
    "total": 5,
    "passed": 5,
    "failed": 0,
    "density_error_percent": 0.23,
    "fps": 35.1
  }
}
```

## SPH Kernel Functions

```wgsl
// Poly6 kernel (density evaluation)
// W_poly6(r, h) = 315 / (64πh⁹) × (h² - r²)³ for 0 ≤ r ≤ h
fn kernel_poly6(r: f32, h: f32) -> f32 {
    let coeff = 315.0 / (64.0 * PI * pow(h, 9));
    let term = h * h - r * r;
    return coeff * term * term * term;
}

// Spiky kernel (pressure gradient)
// W_spiky(r, h) = 15 / (2πh⁶) × (h - r)³ for 0 ≤ r ≤ h
fn kernel_spiky(r: f32, h: f32) -> f32 {
    let coeff = 15.0 / (2.0 * PI * pow(h, 6));
    let term = h - r;
    return coeff * term * term * term;
}

// Viscosity kernel (Laplacian)
// W_viscosity(r, h) = 15 / (2πh⁶) × (-r³/2h + r² - h²/2) for 0 ≤ r ≤ h
fn kernel_viscosity(r: f32, h: f32) -> f32 {
    let coeff = 15.0 / (2.0 * PI * pow(h, 6));
    let term = -pow(r, 3) / (2.0 * h) + r * r - h * h / 2.0;
    return coeff * term;
}
```

## SPH Governing Equations

```
Density evaluation:
  ρ(xᵢ) = Σⱼ mⱼ · W(xᵢ - xⱼ, h)

Pressure (ideal gas law):
  Pᵢ = k · (ρᵢ - ρ₀)

Pressure force:
  F_pressure = -Σⱼ mⱼ · (Pᵢ/ρᵢ² + Pⱼ/ρⱼ²) · ∇W(xᵢ - xⱼ, h)

Viscosity force:
  F_viscosity = μ · Σⱼ mⱼ · (vⱼ - vᵢ) / ρⱼ · ∇²W(xᵢ - xⱼ, h)

External forces:
  F_external = m · g (gravity) + F_vortex + F_collision
```

## Default SPH Parameters

```json
{
  "dt": 0.001,
  "rest_density": 1000.0,
  "gas_constant": 2000.0,
  "viscosity": 250.0,
  "smoothing_radius": 0.2,
  "gravity": [0.0, -9.81, 0.0],
  "kernel_poly6": 315.0 / (64.0 * PI * h^9),
  "kernel_spiky": 15.0 / (2.0 * PI * h^6),
  "kernel_viscosity": 15.0 / (2.0 * PI * h^6)
}
```

## CFL Condition

```
Δt < h / c_sound

where:
  h = smoothing radius
  c_sound = sqrt(∂P/∂ρ) = sqrt(k) (speed of sound in fluid)

For h = 0.2m, k = 2000:
  c_sound = sqrt(2000) ≈ 45 m/s
  Δt < 0.2 / 45 ≈ 0.0044 s

Recommended: Δt = 0.001 s (4× safety margin)
```

## Timeout

Maximum execution time: 60 seconds

## Integration

This skill is called automatically by validation hooks after editing:
- `domains/physics/aether/src/shaders/sph*.wgsl`
- `domains/physics/aether/src/container.rs`
- Any file containing `sph_` or `kernel_poly6` functions

## Related Files

- `scripts/validate_sph.py` - Main SPH validator
- `domains/physics/aether/src/shaders/sph_solver.wgsl` - SPH compute shader
- `domains/physics/aether/config/sph_params.json` - Default parameters

## References

- Müller et al., "Particle-Based Fluid Simulation for Interactive Applications", SCA 2003
- Ihmsen et al., "Smoothed Particle Hydrodynamics", VRIPHYS 2011
- "SPH Tutorial for Real-Time Applications", NVIDIA Developer
