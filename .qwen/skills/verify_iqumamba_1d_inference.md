# Skill: verify_iqumamba_1d_inference

## Overview

Validates IQUMamba-1D inference pipeline for RF signal reconstruction. Checks tensor shapes, bilinear discretization, Hermitian loss, and Burn WGPU backend integration.

## Applicable Agents

- `crystal-ball-reconstruction`
- `train-state-space-ml`
- `tri-modal-defense-specialist`

## Execution

```bash
# Run IQUMamba inference validation
python scripts/validate_iqumamba.py --model <MODEL_PATH> --input <IQ_SAMPLES> --expected <GROUND_TRUTH>

# Example: Validate RF-Vim prediction
python scripts/validate_iqumamba.py --model domains/cognitive/crystal_ball/models/rf_vim.onnx --input test_data/iq_samples.npy --expected test_data/material_properties.json
```

## Validation Criteria

### Pass Conditions
- Input shape: [B, 2, L] where L = 32768 (I/Q stacked as real channels)
- Patch embedding: Conv1d(kernel=32, stride=16) → [B, N, D] where N = L/16
- Selective SSM: State dimension = 16, heads = 8
- Output: [ε', ε'', σ, α] (permittivity real/imag, conductivity, roughness)
- Inference latency: < 1 ms on WGPU backend
- SI-SDR improvement: ≥ 10 dB

### Fail Conditions
- Zero-order hold (ZOH) discretization detected
- Complex tensor input (should be real-stacked I/Q)
- Missing Hermitian loss term
- Inference latency > 1 ms
- SI-SDR improvement < 10 dB

## Detection Patterns

The validator detects IQUMamba components by:
- Module names: `SelectiveSSM`, `RF_Vim`, `BilinearDiscretization`
- Tensor operations: `conv1d`, `patch_embed`, `state_update`
- Loss functions: `hermitian_inner_product`, `phase_coherence_loss`

## Output Format

```json
{
  "model": "domains/cognitive/crystal_ball/models/rf_vim.onnx",
  "input_shape": [1, 2, 32768],
  "tests": [
    {
      "name": "patch_embedding",
      "input_shape": [1, 2, 32768],
      "output_shape": [1, 2048, 512],
      "status": "PASS"
    },
    {
      "name": "selective_ssm_block_1",
      "state_dim": 16,
      "heads": 8,
      "discretization": "bilinear",
      "status": "PASS"
    },
    {
      "name": "output_head",
      "predictions": {
        "epsilon_real": 78.2,
        "epsilon_imag": 5.1,
        "conductivity": 0.52,
        "roughness": 0.15
      },
      "ground_truth": {
        "epsilon_real": 78.0,
        "epsilon_imag": 5.0,
        "conductivity": 0.50,
        "roughness": 0.14
      },
      "mae_percent": 2.3,
      "status": "PASS"
    },
    {
      "name": "inference_latency",
      "measured_us": 850,
      "target_us": 1000,
      "status": "PASS"
    },
    {
      "name": "si_sdr_improvement",
      "input_si_sdr_db": -5.2,
      "output_si_sdr_db": 6.8,
      "improvement_db": 12.0,
      "target_db": 10.0,
      "status": "PASS"
    }
  ],
  "summary": {
    "total": 5,
    "passed": 5,
    "failed": 0,
    "mae_percent": 2.3,
    "inference_latency_us": 850,
    "si_sdr_improvement_db": 12.0
  }
}
```

## IQUMamba-1D Architecture

```
Input: [B, 2, 32768] (I/Q stacked)
  ↓
PatchEmbedding (Conv1d: 2→512, kernel=32, stride=16)
  ↓
[B, 2048, 512] (N patches, D dimensions)
  ↓
RF-Vim Block × 6:
  - SelectiveSSM (state_dim=16, heads=8)
  - Bilinear discretization (Tustin transform)
  - Gated output projection
  ↓
Prediction Heads:
  - ε' (permittivity real)
  - ε'' (permittivity imag)
  - σ (conductivity)
  - α (roughness)
```

## Bilinear Discretization (Tustin Transform)

```python
# Continuous-time SSM: h'(t) = A·h(t) + B·x(t)
# Bilinear discretization:
k = ω₀ × cot(ω₀ × Δt / 2)
A_d = (I + A/k) × (I - A/k)⁻¹
B_d = B × (2/k) × (I - A/k)⁻¹

# State update:
h[t] = A_d × h[t-1] + B_d × x[t]
```

## Hermitian Inner Product Loss

```python
# For complex-valued predictions
def hermitian_loss(y_pred, y_true):
    # Hermitian inner product: ⟨a, b⟩_H = a^H × b
    inner_product = torch.sum(torch.conj(y_pred) * y_true)
    # Loss = 1 - normalized inner product (phase coherence)
    loss = 1 - inner_product / (||y_pred|| × ||y_true||)
    return loss
```

## Timeout

Maximum execution time: 60 seconds

## Integration

This skill is called automatically by validation hooks after editing:
- `domains/cognitive/crystal_ball/src/ml/**/*.rs`
- `domains/cognitive/train/src/iqumamba/**/*.rs`
- Any file containing `SelectiveSSM` or `RF_Vim` modules

## Related Files

- `scripts/validate_iqumamba.py` - Main IQUMamba validator
- `domains/cognitive/crystal_ball/models/` - Pre-trained models
- `domains/cognitive/train/src/iqumamba/` - IQUMamba implementation

## References

- Gu & Dao, "Mamba: Linear-Time Sequence Modeling", arXiv 2023
- Dao & Gu, "Transformers are SSMs", ICML 2024
- "IQUMamba-1D: Complex-Valued State Space Models for RF Reconstruction", Project Oz Internal
