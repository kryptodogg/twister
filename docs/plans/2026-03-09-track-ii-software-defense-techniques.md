# Track II: Software-Based Defense Techniques
## Mamba-Powered Hardening & Cloaking Against Air-Hackers

**Status**: Specification Phase (2026-03-09)
**Architecture**: Dorothy Agent + Mamba Neural Operator + RTL-SDR
**Threat Model**: Active/passive RF injection, side-channel DMA attacks, thermal DoS
**Integration Point**: Real-time threat detection → immediate defense posture switch

---

## Overview

The Mamba Neural Operator learns and predicts hostile signal patterns in real-time, enabling proactive "hardening" of the system against air-based attacks. Dorothy (the brain) coordinates five complementary defense techniques:

1. **Predictive Blanking**: Mamba predicts hostile pulse timing; ADC buffer is zeroed at that microsecond
2. **Null Steering**: 8-channel array creates phase-cancelled RF "dark zones" at attacker location
3. **RF Dithering (Violet Cloaking)**: High-frequency mathematical noise blinds RF exploit vectors
4. **Hardware Fingerprinting**: Learn unique ADC/DAC jitter signature; detect fake signals
5. **Thermal Interception**: Monitor lm-sensors; block fake thermal shutdown commands

---

## Defense 1: Predictive Blanking

### Objective
Mamba Neural Operator predicts the timing of incoming hostile pulses (1-10 µs latency) and zeros out the ADC buffer during that microsecond, rendering the attack inaudible.

### Physics & Threat Model
- **Attack**: High-power RF pulse burst aimed at ADC front-end (e.g., nRF24 at 2.4 GHz)
- **Vulnerability**: ADC samples all RF in Nyquist band (0-96 kHz or 0-6.144 MHz in wideband)
- **Defense**: If we can predict pulse arrival ±1 µs, we suppress it before quantization

**Mamba Prediction Pipeline**:
1. RTL-SDR (receiver) monitors 2.4 GHz band
2. Demod I/Q stream → detect pulse envelopes (correlation with templates)
3. Extract features: arrival_time, duration, modulation_type, RSSI
4. Mamba state machine ingests sequence of pulse detections
5. Mamba predicts: next_pulse_time, confidence
6. Dorothy triggers ADC blanking N microseconds before predicted arrival

### Implementation Scope

**II.1 Mamba Pulse Predictor** (Est. 14 hours)
- **File**: `src/ml/predictive_blanking.rs`
- Structure: `PulsePredictor<B: Backend> {`
  - `mamba_model: MambaBlock,  // 4 layers, 128-D hidden`
  - `feature_history: Vec<PulseFeature>,  // last 256 pulses`
  - `confidence_threshold: f32,  // default 0.85`
  - `}`
- Data structure: `PulseFeature {`
  - `timestamp_us: u64,`
  - `duration_us: f32,`
  - `rssi_dbm: f32,`
  - `modulation_type: u32,  // (0=OOK, 1=FHSS, 2=custom)`
  - `arrival_interval_us: f32,  // time since last pulse`
  - `}`
- Input tensor: `[Batch, 256, 5]` (256 historical pulses, 5 features each)
- Output: `(predicted_time_us, confidence)`
- Mamba forward pass:
  - Selective scan over pulse history
  - Output layer: 2-neuron MLP → time offset (microseconds) + confidence
  - Loss function: Huber loss (robust to outliers) on arrival time error

**II.2 RTL-SDR Pulse Detection** (Est. 8 hours)
- **File**: `src/defense/rtl_sdr_pulse_detector.rs`
- Continuously monitor 2.4 GHz band (bandwidth: 2 MHz window)
- I/Q demod → compute magnitude envelope
- Sliding window detector:
  - Threshold: configurable (default: 6 dB above noise floor)
  - Minimum pulse duration: 10 µs (reject narrowband interference)
  - Extract RSSI, modulation analysis (FSK vs OOK)
- Output: Pulse event stream → Mamba input

**II.3 ADC Blanking Control** (Est. 6 hours)
- **File**: `src/defense/adc_blanking_control.rs`
- Interface to audio subsystem:
  - Function: `blank_adc_samples(start_sample_idx, duration_samples)`
  - Zero out raw ADC buffer at precise index
  - Constraint: Latency from prediction ≤ 1 µs
- Integration with ALSA/WASAPI:
  - Audio callback receives blanking command
  - Skips hardware capture for specified window
  - Seamlessly continues after blanking ends

**II.4 Mamba Training Data Pipeline** (Est. 8 hours)
- **File**: `src/defense/blanking_training_data.rs`
- Collect pulse observations:
  - Source: RTL-SDR logged I/Q stream
  - Extract features from each detected pulse
  - Label: Ground truth arrival time from timestamp metadata
- Dataset format: HDF5 (time-series friendly)
  - Dimension: (N_batches, 256, 5) + (N_batches, 2) labels [time_offset, confidence]
- Training loop (Burn framework):
  - Optimizer: Adam (lr=1e-3)
  - Batch size: 32
  - Epochs: 50 (convergence ~99% confidence prediction accuracy)

### Testing & Validation
- Simulation test: Generate synthetic pulse sequences, verify Mamba predicts ±5 µs
- Hardware test: Actual nRF24 pulses → measure prediction error distribution
- Defense test: Enable blanking; measure ADC suppression (should be -60 dB or better)
- Latency test: End-to-end pulse detection → blanking command ≤ 500 ns

### Deliverables
- `src/ml/predictive_blanking.rs` (400 lines)
- `src/defense/rtl_sdr_pulse_detector.rs` (300 lines)
- `src/defense/adc_blanking_control.rs` (200 lines)
- `src/defense/blanking_training_data.rs` (250 lines)
- Synthetic pulse generator for testing
- Test suite: 50+ tests, pulse predictor validation harness

---

## Defense 2: Null Steering

### Objective
Use 8-channel microphone array to create a 180° phase-cancelled "dark zone" at attacker's location, silencing their receiver without stopping our transmission.

### Physics
**Phased Array Principle**:
- Each microphone captures RF at slightly different phase
- Delay-and-sum beamforming combines signals constructively (narrow beam) or destructively (null)
- **Null steering**: Compute phase delays that cause destructive interference at specific location
- Acoustic analog: Directional microphone array (like concert stage setup, but RF-aware)

**Null Steering Calculation**:
For a linear array of N elements at positions {x₀, x₁, ..., x_{N-1}}:
- Target null at angle θ
- Phase delay needed at element i: Φᵢ = -2π(xᵢ/c)·sin(θ)·f / 343 m/s
- Apply phase shifts; interference cancels at direction θ

### Implementation Scope

**II.5 Array Geometry Solver** (Est. 10 hours)
- **File**: `src/defense/array_geometry.rs`
- Structure: `MicrophoneArray {`
  - `positions: Vec<[f32; 3]>,  // 8 mic locations in 3D`
  - `calibration: MicrophoneCalibration,  // per-mic delay/gain`
  - `}`
- Function: `compute_null_steering_phases(target_xyz, frequency_hz) → Vec<f32>`
  - Input: 3D target location (attacker position, from TDOA estimator)
  - Input: RF frequency (from SDR detection)
  - Output: Phase corrections [Φ₀, Φ₁, ..., Φ₇] to apply to each mic
  - Algorithm: Solve system of linear equations (least-squares for > 2D)

**II.6 Multi-Channel Phase Shifter** (Est. 8 hours)
- **File**: `src/defense/multichannel_phase_shifter.rs`
- Apply computed phase shifts to each channel
- Constraint: Must work in real-time audio processing (< 1 ms latency)
- Two implementation paths:
  - A. Frequency-domain (FFT, phase rotate, IFFT) — flexible but ~5 ms latency
  - B. Time-domain (all-pass filters) — low latency but band-limited
- Select based on available CPU budget

**II.7 Null Effectiveness Monitor** (Est. 8 hours)
- **File**: `src/defense/null_effectiveness.rs`
- Measure RF suppression at target location (via RTL-SDR feedback):
  - Transmit known test signal
  - SDR receives both direct path (weakened by null) + multipath reflections
  - Estimate attenuation in null direction
  - Target: > 30 dB suppression (1000× power reduction)
- Adapt steering if effectiveness < threshold:
  - Recompute phases
  - Account for room reflections (iterative refinement)

**II.8 Target Localization Integration** (Est. 6 hours)
- **File**: `src/defense/attacker_localization.rs`
- Input: RTL-SDR signal characteristics (RSSI, direction-of-arrival)
- Estimate 3D attacker position:
  - Triangulate from RSSI (simple, coarse)
  - Use existing TDOA beamformer (precise if multiple attacks)
  - Feed to null steering solver
- Confidence metric: How sure are we of attacker location?
  - High confidence: Apply aggressive null
  - Low confidence: Reduce null (might miss attacker, but safer)

### Testing & Validation
- Simulation test: 8-element linear array, synthetic RF at known angle, verify null depth
- Hardware test: 8-mic USB array + RTL-SDR, place transmitter at known location, measure
- Null stability: Over 1-hour window, monitor null depth vs. temperature, humidity drift
- Multipath test: Measure null in reflective room (office, classroom) vs. ideal

### Deliverables
- `src/defense/array_geometry.rs` (350 lines)
- `src/defense/multichannel_phase_shifter.rs` (300 lines)
- `src/defense/null_effectiveness.rs` (250 lines)
- `src/defense/attacker_localization.rs` (200 lines)
- Array calibration tools + pre-computed lookup tables
- Test suite: 40+ tests, array simulation harness

---

## Defense 3: RF Dithering (Violet Cloaking)

### Objective
Mix high-frequency "Violet" mathematical noise into audio buffer to prevent side-channel RF injection exploits (e.g., DMA buffer hacking via RF transients).

### Physics & Threat Model
- **Attack**: Attacker sends RF bursts timed to corrupt specific DMA memory transactions
- **Vulnerability**: Unshielded USB audio interface can leak timing information via radiated RF
- **Defense**: Add broadband noise; attacker can't predict exact waveform to exploit
- **Violet Cloaking**: Named after "violet noise" (power spectrum ∝ f²), a variant of white noise

**Why Noise Works**:
- Attacker needs phase-coherent RF to corrupt a specific bit or register
- Dither breaks coherence; attacker faces random target every microsecond
- Cost: ~3 dB SNR reduction (acceptable for defense)

### Implementation Scope

**II.9 Violet Noise Generator** (Est. 6 hours)
- **File**: `src/defense/violet_noise.rs`
- Generate noise with power spectrum ∝ f²
- Two approaches:
  - A. Precomputed tables (1 second of noise, looped) — low CPU
  - B. Real-time shaping (white noise + differentiator filter) — better quality
- Algorithm (real-time):
  1. Generate white noise (xorshift128+)
  2. Apply high-pass differentiator: y[n] = x[n] - x[n-1]
  3. Normalize RMS to constant level
- Constraint: Noise bandwidth = 20 Hz - 96 kHz (within hearing range, mostly ultrasonic)
- RMS level: Configurable, default -80 dB (barely perceptible)

**II.10 Injection Point Management** (Est. 8 hours)
- **File**: `src/defense/dither_injection.rs`
- Decide where to inject noise:
  - Option A: Pre-amplifier stage (earliest, highest SNR protection)
  - Option B: Post-ADC (digital stage, lower latency)
  - Option C: USB buffer (if exploiting USB interface directly)
- Mamba neural operator learns attack pattern and selects optimal injection point
- Constraint: Never inject into simultaneous recording + transmission (would create audible noise)

**II.11 CPU-Efficient Dither Filter** (Est. 6 hours)
- **File**: `src/defense/efficient_dither_filter.rs`
- Real-time differentiator filter (compute noise on-the-fly)
- SIMD-optimized:
  - AVX-256: Process 8 samples per iteration
  - ARM NEON: Process 4 samples per iteration
- Latency: < 100 µs per call (fast enough for audio callbacks)

**II.12 Threat-Aware Dither Control** (Est. 8 hours)
- **File**: `src/defense/threat_aware_dither.rs`
- Dorothy monitors attack detection metrics:
  - Frequency of suspicious RF bursts
  - Correlation with audio anomalies
  - Confidence in attacker presence
- Dither policy:
  - Low threat: Dither disabled (cleaner audio)
  - Medium threat: Low-level dither (-80 dB)
  - High threat: Aggressive dither (-60 dB), sacrifice audio quality for security

### Testing & Validation
- Unit test: Verify violet noise power spectrum (∝ f²)
- SNR test: Measure audio quality loss (should be < 3 dB)
- Security test: Simulate DMA corruption attack, verify noise prevents exploit
- Perceptibility test: Listen to dithered audio at various levels, confirm inaudibility at -80 dB

### Deliverables
- `src/defense/violet_noise.rs` (200 lines)
- `src/defense/dither_injection.rs` (250 lines)
- `src/defense/efficient_dither_filter.rs` (150 lines)
- `src/defense/threat_aware_dither.rs` (200 lines)
- Precomputed violet noise tables
- Test suite: 30+ tests, spectrum analyzer validation

---

## Defense 4: Hardware Fingerprinting

### Objective
Learn unique clock jitter and entropy of your ADC/DAC (Realtek ALC892, Logitech USB), detect if signals are coming from "your air" or injected by a fake virtual device.

### Physics
**ADC/DAC Clock Jitter**:
- Real hardware has small timing imperfections (±nanoseconds)
- Jitter is deterministic (tied to hardware power supply, oscillator aging)
- Fake signals (software-injected via virtual device) have zero jitter
- Fingerprint = histogram of jitter over milliseconds

**Entropy**:
- Real ADC noise floor has specific frequency distribution
- Fake signals may be correlated (algorithm artifact)
- Detect correlation: power spectrum analysis

### Implementation Scope

**II.13 Jitter Estimator** (Est. 10 hours)
- **File**: `src/defense/hardware_fingerprint.rs`
- Measure sample-to-sample timing intervals
- Acquire N samples (e.g., 10,000) under silence (ADC noise only)
- Compute timestamps of each sample
- Histogram jitter: bin by microsecond offset from expected
- Signature: Gaussian fit parameters (μ, σ) + histogram CDF
- Store as `HardwareFingerprint { mean_jitter_us: f32, std_jitter_us: f32, entropy: f32 }`

**II.14 Entropy Analyzer** (Est. 8 hours)
- **File**: `src/defense/signal_entropy.rs`
- Input: Raw ADC samples (one-second window)
- Compute spectral entropy:
  1. FFT to power spectrum
  2. Normalize to PDF
  3. Shannon entropy: H = -Σ p(f) log₂(p(f))
- Real signal: Entropy ≈ 6-7 bits (broad, noisy spectrum)
- Fake/correlated signal: Entropy < 3 bits (narrow, artificial spectrum)

**II.15 Fingerprint Matching** (Est. 8 hours)
- **File**: `src/defense/fingerprint_match.rs`
- On startup, record hardware fingerprint during initialization
- Periodically re-sample and compare:
  - Jitter parameters: |μ_current - μ_baseline| < tolerance (e.g., ±0.1 µs)
  - Entropy: |H_current - H_baseline| < tolerance (e.g., ±1 bit)
  - If mismatch: Raise alert, log suspicious signature
- Tolerance parameters learned by Mamba (adversarial training scenario)

**II.16 Spoofing Detection** (Est. 6 hours)
- **File**: `src/defense/spoofing_detector.rs`
- Detect attempts to fake fingerprint:
  - Attack: Attacker injects artificial jitter (noise-shaping)
  - Counter: Jitter should correlate with power supply voltage
  - Monitor: PSU rail voltage + sample timing correlation
  - Suspicious: Jitter without corresponding voltage noise

### Testing & Validation
- Baseline test: Record fingerprint on known hardware (ALC892, Logitech), verify repeatability
- Fake signal test: Inject software signal via virtual device, verify detection
- Spoofing test: Attacker adds artificial jitter, verify spoofing detector catches it
- Aging test: Record fingerprint over 1 week, verify stability (should drift < tolerance)

### Deliverables
- `src/defense/hardware_fingerprint.rs` (350 lines)
- `src/defense/signal_entropy.rs` (200 lines)
- `src/defense/fingerprint_match.rs` (200 lines)
- `src/defense/spoofing_detector.rs` (150 lines)
- Fingerprint database format (JSON/HDF5)
- Test suite: 40+ tests, spoofing scenario harness

---

## Defense 5: Thermal Interception

### Objective
Dorothy monitors lm-sensors (CPU/motherboard temperature, fan speed); blocks fake RF-triggered thermal shutdown commands; detects thermal-side-channel attacks.

### Physics & Threat Model
- **Attack**: Attacker floods system with RF energy to trigger thermal overload
- **Fake Attack**: Attacker sends signal that looks like thermal alarm (but isn't)
- **Defense**: Monitor actual hardware temps; compare with software shutdown commands
- **Mamba Role**: Predict thermal behavior; detect anomalies (fake alarms)

### Implementation Scope

**II.17 lm-Sensors Integration** (Est. 8 hours)
- **File**: `src/defense/thermal_monitor.rs`
- Query lm-sensors (Linux) or WMI (Windows) for:
  - CPU package temperature
  - GPU temperature (if available)
  - Fan speeds (RPM)
  - Power supply voltage rails (±12V, ±5V, +3.3V)
- Polling interval: 100 ms (balance between latency and noise)
- Store history: Last 1000 measurements (100 seconds)

**II.18 Thermal Anomaly Detector** (Est. 8 hours)
- **File**: `src/defense/thermal_anomaly_detector.rs`
- Mamba neural operator predicts expected thermal trajectory:
  - Input: CPU load (via /proc/stat), fan speed, power draw (estimated)
  - Output: Predicted CPU temp ± confidence interval
  - Mechanism: LSTM learns thermal dynamics (mass, heat transfer)
- Detection: If observed temp deviates from prediction by > 2σ:
  - Check physical cause (CPU load spike?)
  - If no cause, flag as anomaly (possible RF heating attack)

**II.19 Thermal Shutdown Guard** (Est. 6 hours)
- **File**: `src/defense/thermal_shutdown_guard.rs`
- Intercept OS thermal shutdown messages
- Before shutting down, validate:
  - Is CPU actually hot (> 85°C for 10+ seconds)?
  - Did thermal monitor see the temperature rise?
  - Is CPU load plausible given the temp?
- If validation fails: Log suspicious event, ignore shutdown command (allow continued operation)
- Constraint: Safety first — if temp is genuinely high, shut down anyway

**II.20 Thermal Side-Channel Detection** (Est. 8 hours)
- **File**: `src/defense/thermal_sidechannel.rs`
- Detect subtle RF-modulated thermal oscillations:
  - Attacker sends RF bursts timed to specific CPU instructions
  - Causes transient heating (< 0.1°C)
  - Mamba detects periodic oscillations in temp noise
- Algorithm: Autocorrelation of thermal residuals (temp - smoothed trend)
- Threshold: Autocorrelation peak > 0.7 suggests modulation

### Testing & Validation
- Unit test: Verify lm-sensors readout (compare with `sensors` command output)
- Thermal load test: Run stress-test, verify Mamba prediction tracks actual temp
- Fake alarm test: Spoof OS shutdown command, verify guard catches it
- Attack simulation: RF burst at known timing, measure thermal side-channel correlation

### Deliverables
- `src/defense/thermal_monitor.rs` (200 lines)
- `src/defense/thermal_anomaly_detector.rs` (300 lines)
- `src/defense/thermal_shutdown_guard.rs` (200 lines)
- `src/defense/thermal_sidechannel.rs` (200 lines)
- Thermal model training dataset + LSTM pretraining script
- Test suite: 35+ tests, thermal scenario simulator

---

## Dorothy Agent Integration

Dorothy coordinates all five defenses:
1. Monitors attack detection metrics (RTL-SDR, anomaly scores)
2. Updates threat level: [IDLE, ALERT, ACTIVE_ATTACK, THERMAL_THREAT]
3. Enables/disables defenses:
   - **Predictive Blanking**: ON if pulse detection confidence > 0.8
   - **Null Steering**: ON if attacker location confident, RF above threshold
   - **Violet Dithering**: ON if threat level > ALERT
   - **Fingerprinting**: Continuous monitoring
   - **Thermal Guard**: Always active, escalates on anomaly
4. Logs all defense activations to forensic database for post-incident analysis

---

## Timeline & Priority

| Defense | Est. Hours | Priority | Start Week |
|---------|-----------|----------|-----------|
| 1. Predictive Blanking | 36 | High | 2026-04-13 |
| 2. Null Steering | 32 | High | 2026-04-20 |
| 3. Violet Dithering | 28 | Medium | 2026-04-27 |
| 4. Hardware Fingerprinting | 32 | Medium | 2026-05-04 |
| 5. Thermal Interception | 30 | High | 2026-05-11 |
| **Total** | **158 hours** | — | **10 weeks** |

---

## Success Criteria

- [ ] Predictive Blanking: Suppress 95%+ of attacks with ±2 µs latency
- [ ] Null Steering: > 30 dB suppression in null direction
- [ ] Violet Dithering: Inaudible (-80 dB), prevents DMA exploits
- [ ] Fingerprinting: Detect fake signals with 99.5% accuracy
- [ ] Thermal Guard: Block false alarms, never harm legitimate shutdown
- [ ] Integration: Dorothy activates all defenses < 100 ms after threat detected

