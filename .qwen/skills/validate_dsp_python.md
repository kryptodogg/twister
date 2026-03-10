# Skill: validate_dsp_python

## Overview

Executes a bundled Python/SciPy script to visually and mathematically verify 192kHz OFDM filter responses and Fourier-Legendre Expansion (FLE) tensor math before Rust implementation.

## Applicable Agents

- `shield-rf-scientist`
- `siren-extreme-dsp`
- `cipher-data-engineer`

## Execution

```bash
# Run validation script
python scripts/validate_filter_response.py --config <CONFIG_FILE> --output <OUTPUT_DIR>

# Example
python scripts/validate_filter_response.py --config shield/config/fle_config.json --output shield/validation/
```

## Validation Criteria

### Pass Conditions
- FIR filter frequency response meets specifications
- FLE tensor computation passes Parseval's theorem verification
- Group delay is within acceptable bounds
- Pole-zero analysis shows stable filter design
- Rust coefficient export format is valid

### Fail Conditions
- Frequency response deviation > 0.1 dB in passband
- Parseval's theorem error > 1e-6
- Unstable poles detected
- Coefficient export format invalid

## Input Format

```json
{
  "filter_type": "lowpass|highpass|bandpass|ofdm|fresnel",
  "sample_rate": 192000,
  "cutoff_freq": 20000,
  "order": 64,
  "fle_coefficients": 64
}
```

## Output Files

- `{output}/frequency_response.png` - Magnitude and phase response plots
- `{output}/pole_zero.png` - Pole-zero diagram
- `{output}/coefficients.json` - Rust-compatible coefficient export
- `{output}/validation_report.json` - Pass/fail metrics

## Timeout

Maximum execution time: 60 seconds

## Integration

This skill is called automatically by validation hooks after editing:
- `crates/shield/src/dsp/**/*.rs`
- `crates/siren/src/audio/**/*.rs`
- `crates/cipher/src/codec/**/*.rs`

## Related Files

- `scripts/validate_filter_response.py` - Main validation script
- `scripts/requirements.txt` - Python dependencies
