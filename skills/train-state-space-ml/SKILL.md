# Train State Space ML Skill

Burn 0.21 IQUMamba-1D, cubecl-wgpu backend, selective state-space (S6) blocks,
bilinear discretization, latent space training, phase-coherent embeddings.

## Domain
- Burn ML framework (v0.21-pre1+, WGPU backend)
- S6 selective state-space blocks
- Bilinear discretization (Ā = exp(ΔA), B̄ discretization)
- IQUMamba-1D architecture
- Graph-aware encoders (Neo4j fusion)
- Latent space training (32-dim, phase-coherent)
- Training pipelines (synthetic + recorded data)

## Trigger Patterns
"Burn", "S6", "Mamba", "state-space", "latent", "autoencoder", "training",
"bilinear", "IQUMamba", "WGPU backend", "mamba.rs"

## Available Functions
- `create_iqumamba_encoder()` — S6-based encoder
- `train_mamba()` — Training loop with losses
- `encode_with_graph_context()` — Graph-aware inference
- `load_from_hf_hub()` — Hugging Face model loading
- `bilinear_discretize()` — S6 discretization (Ā, B̄)

## Constants
- `D_MODEL = 256` (embedding dimension)
- `N_LAYERS = 4` (S6 blocks)
- `D_STATE = 16` (SSM state size)
- `D_LATENT = 32` (output embedding)
- `INPUT_BINS = 2048`

## Code Patterns

### S6 Block Structure
```rust
struct S6Block {
    in_proj: Linear,      // d_model → d_inner
    conv1d: Conv1d,       // Local mixing
    ssm_proj: Linear,     // d_inner → d_state × d_model
    x_proj: Linear,       // Timestep projection
    out_proj: Linear,     // d_inner → d_model
    norm: LayerNorm,
    a_param: Tensor,      // Learned SSM A
}
```

### Bilinear Discretization
```rust
// Δ = softplus(delta_proj)
// Ā = exp(Δ × A)
// B̄ = (Δ × A)⁻¹ × (exp(Δ × A) - I) × Δ × B
```

### SSM Recurrence (Parallel Scan)
```rust
// h_t = Ā × h_{t-1} + B̄ × x_t
// Use associative scan for GPU parallelism
```
