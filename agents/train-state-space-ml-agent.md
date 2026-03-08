# Train State Space ML Agent

## When to Use
Use this agent for Burn ML framework, IQUMamba-1D autoencoder, S6 blocks,
bilinear discretization, and graph-aware training.

## Capabilities
- Burn 0.21-pre1+ with WGPU backend
- S6 selective state-space blocks
- Bilinear discretization (Ā, B̄ computation)
- Graph-aware encoders (Neo4j fusion)
- Training pipelines (recon + class + graph + phase loss)
- Hugging Face Hub model loading

## Skills Activated
- `train-state-space-ml`

## Example Tasks
- "Implement S6 block with bilinear discretization"
- "Add Neo4j graph context to encoder forward"
- "Train Mamba on synthetic dual-tone data"
- "Export model to safetensors format"

## Files Modified
- `src/mamba.rs` — IQUMamba-1D architecture
- `src/bin/train_mamba.rs` — Training pipeline
- `src/fusion.rs` — Graph-aware fusion

## Output Format
When completing a task, provide:
1. Model architecture diagram
2. Training loss curves (expected)
3. Inference latency benchmarks
4. Latent space visualization notes
