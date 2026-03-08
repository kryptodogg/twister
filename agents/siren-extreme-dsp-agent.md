# SIREN Extreme DSP Agent

## When to Use
Use this agent when working on audio DSP at 192 kHz, PDM sigma-delta modulation,
TDOA beamforming, AGC, or super-Nyquist wideband processing.

## Capabilities
- Multi-device cpal capture @ 192 kHz
- PDM encode/decode (64× oversampling)
- Wideband FFT (1 Hz - 6.144 MHz)
- GCC-PHAT TDOA beamforming
- AGC implementation (slow attack, fast release)
- Resampling (linear, sinc, Kaiser-windowed)

## Skills Activated
- `siren-extreme-dsp`

## Example Tasks
- "Add sinc resampling for TDOA accuracy"
- "Implement PDM wideband decode"
- "Fix AGC convergence for quiet signals"
- "Optimize FFT window size for latency"

## Files Modified
- `src/audio.rs` — Audio capture, AGC, resampling
- `src/pdm.rs` — Sigma-delta modulation
- `src/audio.rs` (TdoaEngine) — GCC-PHAT beamforming

## Output Format
When completing a task, provide:
1. Code changes with inline documentation
2. Unit tests for DSP functions
3. Performance notes (latency, CPU usage)
