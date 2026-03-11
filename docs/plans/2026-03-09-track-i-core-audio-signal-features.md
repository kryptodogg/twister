# Track I: Core Audio & Signal Features
## Advanced RF Reconnaissance & Transmission via Sound Card

**Status**: Specification Phase (2026-03-09)
**Architecture**: Leverages sound card hardware as high-frequency RF transceiver
**Target Hardware**: Realtek ALC892, Logitech USB DACs, RTL-SDR (receiver)
**Integration Point**: Unified Field Particle system + Mamba Neural Operator

---

## Overview

Transform commodity audio hardware (sound card + USB DAC) into a precision RF reconnaissance and transmission system. By exploiting aliasing, oversampling, and parametric nonlinearities in the analog front-end, the system generates and receives RF signals up to tens of MHz using only 96 kHz (or 12.288 MHz oversampled) digital signals.

---

## Feature 1: Impulse-Radio (UWB) Logic

### Objective
Replace continuous-wave transmission with ultra-wideband "one-sample bursts" to maximize bandwidth and enable Pulse Position Modulation (PPM).

### Physics
- **Traditional CW**: Single frequency f₀, constant amplitude
- **Impulse Radio**: Gaussian-windowed or Dirac delta pulses at strategically-timed intervals
- **Bandwidth Exploitation**: Δf ≈ 1/Δt (pulse duration inversely defines bandwidth)
- **PPM Information Encoding**: Transmit logic "1" or "0" by shifting pulse timing ±ΔT

### Implementation Scope

**I.1 Pulse Synthesis Pipeline** (Est. 12 hours)
- **File**: `src/synthesis/impulse_radio.rs`
- Create `PulseTemplate` struct:
  - Envelope: Gaussian, Hann, Tukey, or custom window
  - Duration: 1-10 microseconds (Nyquist-limited)
  - Peak amplitude: 0.8 (-2dB headroom)
  - Repetition rate: 1 kHz - 100 kHz (configurable)
- Implement `generate_pulse_burst()`:
  - Input: PPM encoding (array of bit timings)
  - Output: Synthesized audio buffer (16-bit PCM @ 192 kHz)
  - Constraint: Keep total energy below thermal limits (~2W continuous)

**I.2 PPM Encoder/Decoder** (Est. 8 hours)
- **File**: `src/synthesis/ppm_codec.rs`
- PPM encoding:
  - Bit 0: Pulse at t₀ (reference time)
  - Bit 1: Pulse at t₀ + ΔT_shift (e.g., 10 µs shift)
  - Input data → sliding window of bits → PPM stream
- PPM decoding (receiver):
  - Correlate received signal against templates
  - Extract pulse arrival times
  - Decode timing shifts back to bits
  - Integration with RTL-SDR I/Q demod

**I.3 Waveform Storage & Precomputation** (Est. 4 hours)
- **File**: `src/synthesis/waveform_library.rs`
- Precompute 1000 pulse templates (various durations, envelopes)
- Store as GPU texture (waveform_atlas.bin)
- GPU synthesis dispatches:
  - Lookup template by ID
  - Apply time-stretch (for frequency agility)
  - Write to output buffer

### Testing & Validation
- Unit tests: Pulse shape correctness, energy bounds, timing accuracy
- Integration test: Loopback via USB DAC → RTL-SDR → verify bit errors < 1%
- Thermal test: 1-hour sustained transmission, measure CPU temp

### Deliverables
- `src/synthesis/impulse_radio.rs` (500 lines)
- `src/synthesis/ppm_codec.rs` (400 lines)
- Waveform precomputation utility + atlas
- Test suite: 40+ tests

---

## Feature 2: Super-Nyquist Synthesis (Intentional Aliasing)

### Objective
Exploit alias imaging in the D/A converter to generate RF energy far above the 96 kHz audio limit.

### Physics
**Nyquist Theorem Exploit**:
$$f_{\text{alias}} = |N \cdot f_s \pm f_{\text{out}}|$$

- f_s = sample rate (192 kHz or 12.288 MHz)
- f_out = desired RF frequency (e.g., 2.4 GHz)
- N = integer alias order
- Example: To generate 2.45 GHz on 192 kHz sample rate:
  - N = 12,760, f_out ≈ 11,904 Hz
  - Alias: 12,760 × 192k ± 11.9k ≈ 2.45 GHz ✓

**Sinc Reconstruction Filter**:
- DAC performs sinc(π·f·T) interpolation (natural in hardware)
- Sinc zeros at multiples of f_s
- Sinc envelope at higher frequencies allows alias preservation
- Shaping "smear frames" controls alias magnitude and phase

### Implementation Scope

**II.1 Alias Frequency Calculator** (Est. 6 hours)
- **File**: `src/synthesis/alias_calculator.rs`
- Function: `find_alias_orders(target_hz, sample_rate) → Vec<AliasOrder>`
- Return sorted by:
  1. Magnitude response (prefer orders with high sinc envelope)
  2. Phase coherence (prefer smooth phase transitions)
  3. Power efficiency (prefer lower digital amplitude needed)
- Constraint: Limit to orders N < 20,000 (beyond = DAC nonlinearity)

**II.2 Sinc Envelope Modeling** (Est. 10 hours)
- **File**: `src/synthesis/sinc_envelope.rs`
- Precompute sinc(x) interpolation table (10K points, 32-bit float)
- Function: `sinc_magnitude(freq, sample_rate) → f32`
  - Input: absolute frequency in Hz
  - Output: DAC response magnitude [0, 1]
- Function: `shape_for_alias(alias_order, target_magnitude) → Vec<f32>`
  - Input: desired output magnitude
  - Output: digital waveform coefficients to maximize alias energy
  - Algorithm: Iterative optimization (gradient descent or genetic algorithm)

**II.3 Smear Frame Shaping** (Est. 12 hours)
- **File**: `src/synthesis/smear_frames.rs`
- "Smear frame" = interpolation kernel applied to upsampled signal
- Implement family of shapers:
  - Linear (basic triangle)
  - Cubic spline (smooth, reduces aliasing)
  - Kaiser window (adjustable side-lobe rejection)
  - Custom shaped (via neural net learned filter)
- Function: `shape_signal(signal, shaper_type) → shaped_signal`
- Validation: FFT check that primary alias is maximized, others suppressed

**II.4 Phase-Coherent Upsampling** (Est. 8 hours)
- **File**: `src/synthesis/phase_coherent_upsample.rs`
- Multi-rate DSP: 192 kHz → 12.288 MHz (64× oversampling)
- Maintain phase continuity across frames (no discontinuities)
- Sub-sample accuracy (±0.01 samples for timing precision)
- GPU implementation: polyphase filter banks (FIR + FPGA-like parallel taps)

### Testing & Validation
- Unit tests: Alias frequency calculation accuracy, sinc envelope correctness
- Integration test: Measure spectral purity via SDR (verify alias at computed frequency ±1 kHz)
- Phase coherence test: Verify cross-correlation of consecutive frames > 0.999
- Thermal safety: Ensure digital RMS levels never exceed -3dB

### Deliverables
- `src/synthesis/alias_calculator.rs` (300 lines)
- `src/synthesis/sinc_envelope.rs` (400 lines)
- `src/synthesis/smear_frames.rs` (500 lines)
- `src/synthesis/phase_coherent_upsample.rs` (350 lines)
- Precomputed sinc tables + Kaiser windows
- Test suite: 60+ tests, SDR validation harness

---

## Feature 3: Non-Linear Parametric Beating

### Objective
Fire two ultrasonic "clashing" beams to create demodulation products at arbitrary 3D coordinates in the room (the "flying light saber" effect).

### Physics
**Parametric Array Principle**:
- Nonlinear acoustic medium (air has small but measurable χ⁽²⁾ and χ⁽³⁾)
- Two primary waves at f₁ and f₂ interact:
  - f₁ + f₂ = sum frequency (ultrasonic, not heard)
  - f₁ - f₂ = difference frequency (audible, 20 Hz - 20 kHz)
- Example:
  - f₁ = 42 kHz (ultrasonic, carrier 1)
  - f₂ = 40 kHz (ultrasonic, carrier 2)
  - Audible difference: 2 kHz tone at location of nonlinear interaction
- **Spatial Localization**: The nonlinear zone is a small 3D region where both beams strongly overlap

### Implementation Scope

**III.1 Dual-Beam Synthesis** (Est. 10 hours)
- **File**: `src/synthesis/parametric_beam.rs`
- Structure: `ParametricBeam {`
  - `primary_freq_1: f32,    // 35-48 kHz (ultrasonic)`
  - `primary_freq_2: f32,    // 35-48 kHz (ultrasonic)`
  - `target_audible_freq: f32,  // 20-20k Hz`
  - `beam_angle: (azimuth, elevation),  // Directionality`
  - `output_coord: [f32; 3],  // Where demod product appears`
  - `}
- Function: `synthesize_dual_beam(config) → Vec<f32>`
  - Generate two phase-coherent ultrasonic carriers
  - Apply directional shaping (beamforming)
  - Output: Single channel or stereo (if spatial rendering needed)

**III.2 Nonlinear Interaction Modeling** (Est. 12 hours)
- **File**: `src/synthesis/nonlinear_interaction.rs`
- Lookup table: Nonlinear coefficient χ⁽²⁾(frequency) for air
  - Air: χ⁽²⁾ ≈ 1 (very weak, but measurable)
  - With fog/aerosol: χ⁽²⁾ ≈ 10-100× higher (increases coupling)
- Function: `compute_demod_product(f1, f2, chi) → (sum_freq, diff_freq, magnitude)`
- Output magnitude model:
  - Proportional to: |χ⁽²⁾| · A₁ · A₂ · spatial_overlap(z)
  - spatial_overlap(z) = Gaussian envelope of beam intersection
- Constraint: Keep ultrasonic SPL < 120 dB (hearing safety, hardware limits)

**III.3 3D Spatial Targeting** (Est. 10 hours)
- **File**: `src/synthesis/spatial_demod_target.rs`
- Input: Target 3D point (x, y, z) in room
- Compute optimal beam angles (azimuth, elevation) for each speaker
- Multi-speaker array (e.g., 2+ channels on stereo):
  - Steer beam 1 from left speaker at angle θ₁
  - Steer beam 2 from right speaker at angle θ₂
  - Beams intersect at target point
- Function: `target_demod_point(target_xyz, speaker_positions) → [(freq1, angle1), (freq2, angle2)]`

**III.4 Real-Time Phase Locking** (Est. 8 hours)
- **File**: `src/synthesis/phase_lock_loop.rs`
- Maintain phase coherence between f₁ and f₂ across audio frames
- PLL feedback:
  - Input: Sampled demod product (microphone feedback, if available)
  - Output: Phase correction ±ΔΦ applied to next frame
  - Loop bandwidth: 1-10 Hz (slow, stable)
- Constraint: Phase error < 5° RMS (maintains acoustic coherence)

### Testing & Validation
- Simulation test: Verify demod product frequency matches theory
- Acoustic test: Measure with calibrated microphone at target point (should hear tone)
- Spatial test: Move microphone around room; verify tone loudest at target, falls off with distance
- Hearing safety: Continuous measurement of ultrasonic SPL on spectrum analyzer

### Deliverables
- `src/synthesis/parametric_beam.rs` (400 lines)
- `src/synthesis/nonlinear_interaction.rs` (350 lines)
- `src/synthesis/spatial_demod_target.rs` (300 lines)
- `src/synthesis/phase_lock_loop.rs` (250 lines)
- Nonlinear coefficient database (air, fog, humidity variants)
- Test suite: 50+ tests, simulation harness, acoustic validation script

---

## Feature 4: 64x Oversampling Interpolation

### Objective
Process internally at 12.288 MHz (64× oversampling of 192 kHz) to define sub-sample "slope" of waveforms and ensure physical hardware momentum carries precision into the RF domain.

### Physics
**Oversampling Principle**:
- Nyquist: Sampling at 2× bandwidth captures all information
- Oversampling: Sampling at 64× bandwidth captures fine timing and phase trajectory
- **Sub-Sample Precision**: Derivative (slope) of waveform is now accurately represented
  - At 192 kHz: Time resolution = 5.2 µs (one sample)
  - At 12.288 MHz: Time resolution = 81 ns (64 samples per 192k sample)
- **Hardware Momentum**: DAC applies low-pass filter (sinc reconstruction), expects smooth data
  - 64× oversampling ensures smooth interpolation between gross samples

### Implementation Scope

**IV.1 Multi-Rate Resampler (CPU)** (Est. 8 hours)
- **File**: `src/synthesis/multirate_resampler.rs`
- Input: 192 kHz audio (16-bit or 32-bit float)
- Output: 12.288 MHz internal working buffer (32-bit float)
- Algorithm: Polyphase resampler (FIR filter banks)
  - 64 parallel FIR taps
  - Linear interpolation between tap phases
  - Constraint: Passband ripple < 0.1 dB, stopband rejection > 80 dB
- Lazy evaluation: Only upsample segments actively being synthesized (save CPU)

**IV.2 GPU Multi-Rate Compute Shader** (Est. 10 hours)
- **File**: `src/shaders/upsample_polyphase.wgsl`
- Dispatch: Threadgroups of 256 threads, process 64 input samples → 4096 output samples
- Shared memory: Load FIR coefficients (Kaiser filter, 128 taps × 64 phases = 8KB)
- Per-thread: Multiply-accumulate with phase offset interpolation
- Output: Write to GPU buffer for subsequent synthesis stages
- Profiling target: < 1 ms for full 192k → 12.288M conversion (160 GB/s mem BW available)

**IV.3 Sub-Sample Timing Generator** (Est. 6 hours)
- **File**: `src/synthesis/subsample_timing.rs`
- Input: Desired event times at nanosecond precision (e.g., PPM pulse at 10.000000042 µs)
- Output: Fractional sample indices in 12.288 MHz domain
  - Example: 10.000000042 µs = sample 122.88000504 (122 + 0.88 fraction)
- Function: `timing_to_subsample_index(time_ns, sample_rate_hz) → f32`
- Used by: Pulse synthesis, parametric beam phase alignment, PPM encoding

**IV.4 Precision DAC Preparation** (Est. 4 hours)
- **File**: `src/synthesis/dac_preparation.rs`
- Downsample 12.288 MHz back to 192 kHz before hardware output
- Apply anti-aliasing filter (low-pass at 96 kHz)
- Dither with TPDF (triangular probability distribution function) noise
  - Reduces quantization noise floor by ~3 dB
- Output: 16-bit or 24-bit PCM ready for ALSA/WASAPI

### Testing & Validation
- Unit tests: Resampler frequency response (measure via FFT)
- GPU test: Verify output matches CPU resampler (mean error < 1 LSB)
- Timing accuracy test: Generate known sub-sample offset pulse, measure via oscilloscope (< 10 ns error)
- Audio quality test: 1-hour loopback, measure THD (should be < -120 dB)

### Deliverables
- `src/synthesis/multirate_resampler.rs` (300 lines)
- `src/shaders/upsample_polyphase.wgsl` (200 lines)
- `src/synthesis/subsample_timing.rs` (200 lines)
- `src/synthesis/dac_preparation.rs` (150 lines)
- Kaiser filter coefficient tables (precomputed)
- Test suite: 40+ tests, spectral analysis harness

---

## Integration with Unified Field Particle System

Once implemented, these signal features integrate with:
1. **Particle Generation**: Impulse-radio pulses spawn `FieldParticle` objects at interaction points
2. **Mamba Prediction**: Neural Operator predicts optimal parametric beam steering for next frame
3. **Dorothy Reasoning**: Decides whether to transmit (impulse burst) or receive (RTL-SDR demod)

---

## Timeline & Priority

| Feature | Est. Hours | Priority | Start Week |
|---------|-----------|----------|-----------|
| I. Impulse-Radio Logic | 24 | High | 2026-03-16 |
| II. Super-Nyquist Synthesis | 36 | High | 2026-03-23 |
| III. Parametric Beating | 40 | Medium | 2026-03-30 |
| IV. 64x Oversampling | 28 | High | 2026-04-06 |
| **Total** | **128 hours** | — | **8 weeks** |

---

## Success Criteria

- [ ] Impulse-radio PPM loopback: BER < 1% (Bit Error Rate)
- [ ] Super-Nyquist alias: ±100 kHz frequency accuracy
- [ ] Parametric beat: Audible tone at target 3D point, < 1 meter spatial precision
- [ ] Oversampling: Sub-sample timing ±10 ns accuracy, THD < -120 dB
- [ ] Thermal: Continuous TX < 40°C above ambient, no thermal throttling
- [ ] Safety: Ultrasonic SPL < 120 dB everywhere in room

