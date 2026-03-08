# Crystal Ball Reconstructor Skill

Inverse RF-BSDF neural reconstruction, RF-Vim architectures, selective state-space
models (S6), material property prediction from I/Q measurements, phase coherence,
bispectral analysis.

## Domain
- Bispectrum computation (GPU-accelerated)
- Bicoherence thresholding (per-band)
- Product detection (sum, difference, harmonic, intermodulation)
- Phase stability tracking (circular mean, von Mises)
- Frequency band classification (Infrasound → UpperRF)
- RF-BSDF inverse reconstruction
- Fusion with ML latents

## Trigger Patterns
"bispectrum", "bicoherence", "RF-BSDF", "phase coherence", "intermodulation",
"product detection", "nonlinear coupling", "bispectrum.rs"

## Available Functions
- `compute_bispectrum()` — GPU bispectral matrix (512×512)
- `detect_products()` — Sum/diff/harmonic/IM classification
- `track_phase_stability()` — Circular phase coherence
- `classify_frequency_band()` — Band taxonomy (7 bands)
- `fuse_with_latent()` — Bispec + Mamba fusion
- `spatial_culling()` — Skip empty spectrum regions

## Constants
- `BISPEC_FFT_SIZE = 1024`
- `BISPEC_BINS = 512`
- `BISPEC_MATRIX_SIZE = 262144` (512²)
- `COHERENCE_THRESHOLD = 3.0` (base, per-band varies)
- `MIN_COHERENCE_FRAMES = 10`

## Frequency Bands
| Band | Range | Coherence Threshold |
|------|-------|---------------------|
| Infrasound | 1 Hz - 19 Hz | 2.0 |
| Audio | 20 Hz - 19.999 kHz | 3.0 |
| Ultrasonic | 20 kHz - 95.999 kHz | 4.5 |
| HyperUltrasonic | 96 kHz - 399.999 kHz | 6.0 |
| LowerRF | 400 kHz - 1.535 MHz | 5.0 |
| MidRF | 1.536 MHz - 3.071 MHz | 7.0 |
| UpperRF | 3.072 MHz - 6.144 MHz | 9.0 |

## Code Patterns

### Bispectrum Cell Computation
```wgsl
// B(f1, f2) = X(f1) × X(f2) × conj(X(f1+f2))
// Magnitude: |B|, Phase: arg(B)
```

### Phase Stability (Circular Mean)
```rust
// sc = sum(cos(phase)) / N
// ss = sum(sin(phase)) / N
// stability = sqrt(sc² + ss²)  // 0 = random, 1 = coherent
```

### Spatial Culling
```rust
// Skip cell if |X(f1)| < threshold OR |X(f2)| < threshold OR |X(f1±f2)| < threshold
```
