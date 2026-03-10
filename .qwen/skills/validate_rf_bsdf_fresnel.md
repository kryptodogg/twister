# Skill: validate_rf_bsdf_fresnel

## Overview

Validates RF-BSDF Fresnel equation implementations against exact analytical solutions. Ensures complex refractive index arithmetic is correct and Schlick approximations are NEVER used for RF scattering.

## Applicable Agents

- `shield-rf-scientist`
- `tri-modal-defense-specialist`
- `crystal-ball-reconstruction`

## Execution

```bash
# Run Fresnel validation
python scripts/validate_fresnel.py --input <INPUT_FILE> --frequency <FREQ_HZ> --material <MATERIAL_JSON>

# Example: Validate water at 2.4 GHz
python scripts/validate_fresnel.py --input domains/physics/aether/src/shaders/rf_pbr.wgsl --frequency 2400000000 --material materials/water_24ghz.json
```

## Validation Criteria

### Pass Conditions
- Exact Fresnel equations used (NOT Schlick approximation)
- Complex refractive index ñ = n + iκ properly handled
- Reflectance Rs, Rp computed correctly for both polarizations
- Energy conservation: R + T = 1 (within numerical precision)

### Fail Conditions
- Schlick approximation detected (`R = R₀ + (1-R₀)(1-cosθ)⁵`)
- Complex arithmetic errors (missing conjugate, wrong magnitude)
- Reflectance > 1.0 or < 0.0
- Brewster angle incorrect for given material

## Detection Patterns

The validator detects Fresnel implementations by:
- Function names: `fresnel_`, `reflectance_`, `complex_fresnel`
- Variable patterns: `n_complex`, `epsilon_r`, `kappa`, `extinction`
- Mathematical patterns: `cos_theta`, `sin_theta`, `sqrt(1-...)`

## Output Format

```json
{
  "file": "domains/physics/aether/src/shaders/rf_pbr.wgsl",
  "frequency_hz": 2400000000,
  "material": "water",
  "tests": [
    {
      "angle_deg": 0,
      "expected_rs": 0.020,
      "computed_rs": 0.020,
      "expected_rp": 0.020,
      "computed_rp": 0.020,
      "status": "PASS",
      "error_percent": 0.0
    },
    {
      "angle_deg": 45,
      "expected_rs": 0.081,
      "computed_rs": 0.082,
      "expected_rp": 0.005,
      "computed_rp": 0.004,
      "status": "PASS",
      "error_percent": 1.2
    },
    {
      "angle_deg": 90,
      "expected_rs": 1.0,
      "computed_rs": 1.0,
      "expected_rp": 1.0,
      "computed_rp": 1.0,
      "status": "PASS",
      "error_percent": 0.0
    }
  ],
  "summary": {
    "total": 10,
    "passed": 10,
    "failed": 0,
    "max_error_percent": 1.2
  }
}
```

## Exact Fresnel Equations

For complex refractive index ñ₂ = n₂ + iκ₂ and real ñ₁ = n₁:

```
Snell's law (complex):
  ñ₁ sin(θᵢ) = ñ₂ sin(θₜ)

Complex cos(θₜ):
  cos(θₜ) = sqrt(1 - (ñ₁/ñ₂)² sin²(θᵢ))

Fresnel reflectance (s-polarization):
  r_s = (ñ₁ cos(θᵢ) - ñ₂ cos(θₜ)) / (ñ₁ cos(θᵢ) + ñ₂ cos(θₜ))
  R_s = |r_s|²

Fresnel reflectance (p-polarization):
  r_p = (ñ₂ cos(θᵢ) - ñ₁ cos(θₜ)) / (ñ₂ cos(θᵢ) + ñ₁ cos(θₜ))
  R_p = |r_p|²
```

## Material Database

Pre-defined materials at 2.4 GHz:

| Material | ε' | ε'' | n | κ |
|----------|----|-----|---|---|
| Water | 78.0 | 5.0 | 8.83 | 0.28 |
| Concrete | 6.0 | 0.5 | 2.45 | 0.10 |
| Glass | 6.5 | 0.1 | 2.55 | 0.02 |
| Wood (dry) | 2.0 | 0.1 | 1.41 | 0.04 |
| Human tissue | 50.0 | 15.0 | 7.14 | 1.05 |

## Timeout

Maximum execution time: 30 seconds

## Integration

This skill is called automatically by validation hooks after editing:
- `domains/physics/aether/src/shaders/rf_pbr.wgsl`
- `domains/spectrum/shield/src/rf/fresnel.rs`
- Any file containing `fresnel_` or `complex_fresnel` functions

## Related Files

- `scripts/validate_fresnel.py` - Main Fresnel validator
- `scripts/fresnel_reference.json` - Pre-computed reference values
- `domains/physics/aether/src/shaders/rf_math.wgsl` - RF math library

## References

- Born & Wolf, "Principles of Optics", 7th ed., Cambridge University Press
- Balanis, "Advanced Engineering Electromagnetics", Wiley
- "Complex Fresnel Equations for Lossy Media", IEEE Trans. AP, 2019
