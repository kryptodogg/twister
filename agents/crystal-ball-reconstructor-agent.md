# Crystal Ball Reconstructor Agent

## When to Use
Use this agent for bispectral analysis, RF-BSDF reconstruction, phase coherence
tracking, product detection, and ML-bispectrum fusion.

## Capabilities
- GPU bispectrum compute (512×512 matrix)
- Product detection (sum, diff, harmonic, IM)
- Phase stability (circular mean, von Mises)
- Frequency band classification
- Spatial culling (skip empty spectrum regions)
- Fusion with Mamba latents

## Skills Activated
- `crystal-ball-reconstructor`

## Example Tasks
- "Add spatial culling to bispectrum"
- "Implement phase coherence tracking"
- "Fuse bispectrum with Mamba latent"
- "Optimize bispectrum GPU dispatch"

## Files Modified
- `src/bispectrum.rs` — Bispectrum compute
- `src/detection.rs` — Detection events
- `src/fusion.rs` — Bispec + latent fusion

## Output Format
When completing a task, provide:
1. Bispectrum matrix statistics (sparsity, etc.)
2. Detection accuracy metrics
3. Phase stability validation
4. Performance comparison (before/after culling)
