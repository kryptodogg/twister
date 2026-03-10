# Skill: validate_heterodyne_kernel

## Overview

Validates Dorothy heterodyne WGPU compute kernel for correct complex I/Q mixing, RDNA 2 Wave32 alignment, and frequency shift accuracy.

## Applicable Agents

- `dorothy-heterodyne-specialist`
- `shield-rf-scientist`
- `rdna2-compute-specialist`

## Execution

```bash
# Run heterodyne kernel validation
python scripts/validate_heterodyne.py --shader <SHADER_PATH> --test_freq <FREQ_HZ> --shift <SHIFT_HZ>

# Example: Validate 1 MHz tone shifted by 500 kHz
python scripts/validate_heterodyne.py --shader assets/shaders/dorothy/heterodyne.wgsl --test_freq 1000000 --shift 500000
```

## Validation Criteria

### Pass Conditions
- Complex mixing math: I_out = I_in·cos(θ) - Q_in·sin(θ), Q_out = I_in·sin(θ) + Q_in·cos(θ)
- Workgroup size: @workgroup_size(32, 1, 1) for Wave32
- 128-byte aligned HeterodynePayload struct
- Frequency shift accuracy: < 0.01 Hz error
- Phase error: < 0.1° per sample

### Fail Conditions
- Incorrect mixing formula (sign errors)
- @workgroup_size(64, ...) without Wave64 justification
- HeterodynePayload not 128-byte aligned
- Frequency error > 0.01 Hz
- Lookup table precision insufficient

## Detection Patterns

The validator detects heterodyne implementations by:
- Function names: `heterodyne_`, `complex_mix`, `frequency_shift`
- Variable patterns: `I_in`, `Q_in`, `I_out`, `Q_out`, `lo_phase`
- Mathematical patterns: `cos(lo_phase)`, `sin(lo_phase)`, `I * cos`

## Output Format

```json
{
  "shader": "assets/shaders/dorothy/heterodyne.wgsl",
  "test_frequency_hz": 1000000,
  "shift_hz": 500000,
  "tests": [
    {
      "name": "complex_mixing_math",
      "expected_formula": "I_out = I_in*cos(θ) - Q_in*sin(θ); Q_out = I_in*sin(θ) + Q_in*cos(θ)",
      "found_formula": "I_out = I_in*cos(θ) - Q_in*sin(θ); Q_out = I_in*sin(θ) + Q_in*cos(θ)",
      "status": "PASS"
    },
    {
      "name": "workgroup_size",
      "expected": [32, 1, 1],
      "found": [32, 1, 1],
      "status": "PASS"
    },
    {
      "name": "payload_alignment",
      "struct": "HeterodynePayload",
      "size_bytes": 128,
      "alignment_bytes": 128,
      "status": "PASS"
    },
    {
      "name": "frequency_accuracy",
      "expected_output_hz": 500000,
      "measured_output_hz": 500000.003,
      "error_hz": 0.003,
      "target_hz": 0.01,
      "status": "PASS"
    },
    {
      "name": "phase_accuracy",
      "expected_phase_error_deg": 0.0,
      "measured_phase_error_deg": 0.05,
      "target_deg": 0.1,
      "status": "PASS"
    }
  ],
  "summary": {
    "total": 5,
    "passed": 5,
    "failed": 0,
    "frequency_error_hz": 0.003,
    "phase_error_deg": 0.05
  }
}
```

## Heterodyne Math Reference

```wgsl
// Complex I/Q mixing (frequency shift)
// Input: I_in + j·Q_in at frequency f_in
// LO: cos(2π·f_lo·t) + j·sin(2π·f_lo·t)
// Output: I_out + j·Q_out at f_in ± f_lo

fn heterodyne_mix(I_in: f32, Q_in: f32, lo_phase: f32) -> vec2<f32> {
    let cos_lo = cos(lo_phase);
    let sin_lo = sin(lo_phase);
    
    // Complex multiplication: (I + jQ) × (cos + j·sin)
    let I_out = I_in * cos_lo - Q_in * sin_lo;
    let Q_out = I_in * sin_lo + Q_in * cos_lo;
    
    return vec2<f32>(I_out, Q_out);
}
```

## HeterodynePayload Struct

```rust
#[repr(C, align(128))]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct HeterodynePayload {
    pub lo_phase_increment: f32,  // 4 bytes: Δφ per sample
    pub lo_phase_current: f32,    // 4 bytes: Current LO phase
    pub shift_direction: u32,     // 4 bytes: 0=up, 1=down
    pub folding_mode: u32,        // 4 bytes: Nyquist zone
    
    pub _pad: [u32; 28],          // 112 bytes padding
}                                    // Total: 128 bytes
```

## RDNA 2 Optimization Rules

1. **Wave32 workgroup**: `@workgroup_size(32, 1, 1)` for native execution
2. **LDS for LO phase**: Share LO phase across workgroup via `var<workgroup>`
3. **Subgroup broadcast**: Use `subgroupBroadcast` for LO phase distribution
4. **Register pressure**: Keep < 24 VGPRs per thread
5. **128-byte alignment**: All uniform structs aligned to cache line

## Timeout

Maximum execution time: 30 seconds

## Integration

This skill is called automatically by validation hooks after editing:
- `assets/shaders/dorothy/heterodyne.wgsl`
- `domains/spectrum/dorothy/src/heterodyne.rs`
- Any file containing `heterodyne_` or `complex_mix` functions

## Related Files

- `scripts/validate_heterodyne.py` - Main heterodyne validator
- `domains/spectrum/dorothy/src/heterodyne.rs` - Rust host code
- `conductor/tracks/dorothy_active_heterodyne/plan.md` - Implementation plan

## References

- Lyons, "Understanding Digital Signal Processing", 3rd ed., Prentice Hall
- "Complex Mixing for Software Defined Radio", Analog Devices AN-924
- "RDNA 2 Optimization Guide", AMD Developer Technologies
