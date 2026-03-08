# SIREN Extreme DSP Skill

Expert knowledge for 192 kHz PCM, high-sample-rate DSP, heterodyne detection,
sigma-delta modulation, and real-time audio processing at super-Nyquist rates.

## Domain
- Audio capture @ 192 kHz (cpal)
- PDM sigma-delta modulation (64× oversampling → 12.288 MHz)
- Wideband FFT (1 Hz - 6.144 MHz Nyquist)
- TDOA beamforming (GCC-PHAT, multi-device)
- AGC (slow attack, fast release)
- Resampling (linear, sinc, windowed)

## Trigger Patterns
"192 kHz", "PDM", "sigma-delta", "TDOA", "GCC-PHAT", "cpal", "super-Nyquist",
"audio capture", "beamforming", "AGC", "resample", "audio.rs", "pdm.rs"

## Available Functions
- `create_audio_engine()` — Multi-device cpal capture
- `create_pdm_engine()` — Sigma-delta encode/decode
- `create_tdoa_engine()` — GCC-PHAT beamforming
- `apply_agc_inplace()` — Automatic gain control
- `linear_resample()` / `sinc_resample()` — Sample rate conversion

## Constants
- `BASEBAND_FFT_SIZE = 2048`
- `TDOA_FFT_SIZE = 4096`
- `OVERSAMPLE_RATIO = 64`
- `PDM_CLOCK_HZ = sample_rate × 64`
- `WIDEBAND_NYQUIST = PDM_CLOCK_HZ / 2`

## Code Patterns

### PDM Encode (Sigma-Delta Modulator)
```rust
// 1st-order sigma-delta: acc += sample; bit = acc >= 0 ? 1 : 0; acc -= bit
```

### GCC-PHAT TDOA
```rust
// Cross-spectrum: R = X1 * conj(X2) / |X1 * conj(X2)|
// IFFT → peak detection → lag → azimuth
```

### AGC Update
```rust
// rms = sqrt(sum(samples²) / N)
// error_db = TARGET_DBFS - (peak_dbfs + gain_db)
// gain_db += coeff * error_db  (attack if error > 0, release otherwise)
```
