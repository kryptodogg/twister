# Implementation Architecture: Tracks I, II, III
## Parallel Development Roadmap

**Status**: Architecture Design Phase (2026-03-09)
**Purpose**: Divide Tracks I, II, III into independently-assignable code modules
**Integration Point**: All tracks feed into Dorothy agent + Unified Field Mamba

---

## Track I Architecture: Core Audio & Signal Features

### Module Structure

```
src/synthesis/
├── impulse_radio.rs           [I.1] Pulse templates, PPM encoding
├── ppm_codec.rs               [I.2] PPM encoder/decoder (receiver integration)
├── waveform_library.rs        [I.3] GPU texture atlas, precomputed waves
├── alias_calculator.rs        [II.1] Frequency→alias order mapping
├── sinc_envelope.rs           [II.2] DAC response modeling + shaping
├── smear_frames.rs            [II.3] Interpolation kernel design
├── phase_coherent_upsample.rs [II.4] 192k → 12.288M upsampling
├── parametric_beam.rs         [III.1] Dual-beam synthesis
├── nonlinear_interaction.rs   [III.2] χ⁽²⁾ modeling, demod products
├── spatial_demod_target.rs    [III.3] 3D beam steering
├── phase_lock_loop.rs         [III.4] Phase coherence maintenance
├── multirate_resampler.rs     [IV.1] CPU polyphase resampler
├── subsample_timing.rs        [IV.3] Nanosecond-precision timing
└── dac_preparation.rs         [IV.4] Dither + final conversion
```

### Data Flow

```
RF Target Freq
    ↓
[Alias Calculator] → Compute optimal N, alias order
    ↓
[Sinc Envelope Model] → Predict DAC response
    ↓
[Smear Frame Shaper] → Design interpolation kernel
    ↓
[PPM Encoder] → Convert bits → pulse timings
    ↓
[Pulse Synthesizer] → Generate baseband signal
    ↓
[Parametric Beam] → Add spatial encoding (if dual-beam mode)
    ↓
[Phase-Coherent Upsample] → 192k → 12.288M
    ↓
[Multirate Resampler] → Back to hardware sample rate
    ↓
[DAC Preparation] → Add TPDF dither
    ↓
[Audio Hardware] → Convert to RF via aliasing
    ↓
[Speakers / Parametric Transducer] → Emit RF/ultrasonic
```

### Task Division

**Stream 1A** (Impulse-Radio Base): `impulse_radio.rs` + `ppm_codec.rs` + `waveform_library.rs`
- **Owner**: Can be assigned independently
- **Dependencies**: None (self-contained)
- **Duration**: 24 hours
- **Deliverable**: PPM loopback test (TX bits → RX bits, measure BER)

**Stream 1B** (Super-Nyquist Synthesis): `alias_calculator.rs` + `sinc_envelope.rs` + `smear_frames.rs`
- **Owner**: Can be assigned independently
- **Dependencies**: None (math-heavy, no hardware yet)
- **Duration**: 36 hours
- **Deliverable**: Synthetic alias spectrum analyzer (verify frequency placement ±1 kHz)

**Stream 1C** (Parametric Beating): `parametric_beam.rs` + `nonlinear_interaction.rs` + `spatial_demod_target.rs` + `phase_lock_loop.rs`
- **Owner**: Can be assigned independently
- **Dependencies**: None (physics simulation only)
- **Duration**: 40 hours
- **Deliverable**: 3D acoustic demod target simulator (acoustic raytracer)

**Stream 1D** (Oversampling Pipeline): `multirate_resampler.rs` + `phase_coherent_upsample.rs` + `subsample_timing.rs` + `dac_preparation.rs`
- **Owner**: Can be assigned independently
- **Dependencies**: Stream 1A, 1B (needs alias_calculator output format)
- **Duration**: 28 hours
- **Deliverable**: End-to-end synthesis pipeline (192k input → 12.288M internal → 192k output)

**Integration Point**: All four streams merge in `src/main.rs` → audio synthesis dispatcher

---

## Track II Architecture: Software-Based Defense Techniques

### Module Structure

```
src/defense/
├── rtl_sdr_pulse_detector.rs     [II.1] RTL-SDR RF pulse detection
├── predictive_blanking.rs        [II.1] Mamba pulse time predictor
├── adc_blanking_control.rs       [II.3] Audio callback integration
├── blanking_training_data.rs     [II.4] HDF5 dataset builder
├── array_geometry.rs             [II.5] Mic array steering matrix
├── multichannel_phase_shifter.rs [II.6] Per-channel phase control
├── null_effectiveness.rs         [II.7] Null depth monitor
├── attacker_localization.rs      [II.8] TDOA/RSSI triangulation
├── violet_noise.rs               [II.9] Violet noise generator
├── dither_injection.rs           [II.10] Injection point selector
├── efficient_dither_filter.rs    [II.11] SIMD differentiator
├── threat_aware_dither.rs        [II.12] Threat-level control
├── hardware_fingerprint.rs       [II.13] ADC jitter estimator
├── signal_entropy.rs             [II.14] Spectral entropy analyzer
├── fingerprint_match.rs          [II.15] Signature comparison
├── spoofing_detector.rs          [II.16] Fake signal detection
├── thermal_monitor.rs            [II.17] lm-sensors integration
├── thermal_anomaly_detector.rs   [II.18] Mamba thermal model
├── thermal_shutdown_guard.rs     [II.19] OS shutdown interception
└── thermal_sidechannel.rs        [II.20] Thermal modulation detection
```

### Data Flow (per frame, 10 Hz update)

```
RTL-SDR I/Q Stream (continuous)
    ↓
[Pulse Detector] → Extract features {time, RSSI, modulation}
    ↓
[Mamba Predictor] → Predict next pulse time (±2 µs)
    ↓
[ADC Blanking] → Zero buffer at predicted time
    ↓
[Array Geometry] → Compute null steering phases
    ↓
[Phase Shifter] → Apply per-channel delays
    ↓
[Null Effectiveness] → Measure suppression via RTL-SDR feedback
    ↓
[Threat Classification] → Is signal Ghost or Sparkle?
    ↓
[Dither Selection] → Choose noise color (Dorothy decision)
    ↓
[Dither Injection] → Add noise to audio buffer
    ↓
[Hardware Fingerprint] → Monitor ADC jitter signature
    ↓
[Thermal Monitor] → Read CPU/GPU temps, fan speeds
    ↓
[Thermal Anomaly Detector] → Compare vs. Mamba prediction
    ↓
[Shutdown Guard] → Intercept any thermal alarms
    ↓
Dorothy Agent → Update threat level, adjust defenses
```

### Task Division

**Stream 2A** (Predictive Blanking): `rtl_sdr_pulse_detector.rs` + `predictive_blanking.rs` + `adc_blanking_control.rs` + `blanking_training_data.rs`
- **Owner**: Can be assigned independently
- **Dependencies**: Mamba framework (already in codebase)
- **Duration**: 36 hours
- **Deliverable**: End-to-end pulse prediction + ADC suppression (measure -60 dB suppression)
- **Notes**: Requires RTL-SDR hardware for final testing

**Stream 2B** (Null Steering): `array_geometry.rs` + `multichannel_phase_shifter.rs` + `null_effectiveness.rs` + `attacker_localization.rs`
- **Owner**: Can be assigned independently
- **Dependencies**: TDOA engine (from existing codebase)
- **Duration**: 32 hours
- **Deliverable**: Array phasing simulator + hardware validation (measure >30 dB null)
- **Notes**: Requires 8-mic array for final testing

**Stream 2C** (Violet Dithering + Color Selection): See separate `noise-spectrum-mapping.rs` architecture (below)
- **Owner**: Can be assigned independently
- **Dependencies**: None (DSP only)
- **Duration**: 28 hours
- **Deliverable**: Noise generator suite + Mamba threat→color mapper

**Stream 2D** (Hardware Fingerprinting): `hardware_fingerprint.rs` + `signal_entropy.rs` + `fingerprint_match.rs` + `spoofing_detector.rs`
- **Owner**: Can be assigned independently
- **Dependencies**: None (signal analysis only)
- **Duration**: 32 hours
- **Deliverable**: Spoofing detector (detect fake signals with 99.5% accuracy)

**Stream 2E** (Thermal Interception): `thermal_monitor.rs` + `thermal_anomaly_detector.rs` + `thermal_shutdown_guard.rs` + `thermal_sidechannel.rs`
- **Owner**: Can be assigned independently
- **Dependencies**: None (system monitoring)
- **Duration**: 30 hours
- **Deliverable**: Thermal attack blocker (pass legitimate shutdown, block fake alarms)

**Integration Point**: All streams → Dorothy agent threat level manager

---

## Track III Architecture: Audio-Radio Networking (VLF)

### Module Structure

```
src/networking/
├── signal_state.rs              [III.1] Message enum + serialization
├── state_aggregator.rs          [III.2] Multi-node fusion
├── broadcast_service.rs         [III.3] UDP multicast transport
├── octave_mapper.rs             [III.4] Frequency octave conversion
├── sensor_modulator.rs          [III.5] Sensor→audio encoding
├── sensor_demodulator.rs        [III.6] Audio→sensor decoding
├── ghost_sparkle_classifier.rs  [III.7] ML-based signal classification
├── sparkle_detector.rs          [III.8] Real-time anomaly detection
├── sparkle_broadcast.rs         [III.9] Alert propagation
├── consensus_engine.rs          [III.10] Network triangulation
├── defense_orchestration.rs     [III.11] Synchronized null steering
├── anomaly_propagation.rs       [III.12] Network-wide alerting
├── audio_packet_codec.rs        [III.13] 2-FSK packet encoding
├── audio_transport.rs           [III.14] Lightweight TCP-like reliability
├── audio_channel_model.rs       [III.15] Propagation modeling
└── cognitive_agent.rs           [III.16] Mamba network optimizer
```

### Data Flow (per second, 1 Hz consensus update)

```
Local RF Observations (10 Hz)
    ↓
[Signal Classification] → Ghost vs. Sparkle decision
    ↓
[Local Threat Estimate] → Confidence + location
    ↓
[Octave Mapper] → Convert signal properties to audio equivalent
    ↓
[Sensor Modulator] → Encode into tone + phase + amplitude
    ↓
[Audio Packet Codec] → 2-FSK packet, Reed-Solomon FEC
    ↓
[Broadcast Service] → UDP multicast to network (port 9999)
    ↓
[Remote Nodes] receive packets
    ↓
[Audio Demodulator] → Extract tone → recover signal properties
    ↓
[State Aggregator] → Fuse with local observations
    ↓
[Consensus Engine] → Triangulate attacker location
    ↓
[Defense Orchestration] → All nodes steer nulls to consensus point
    ↓
Dorothy Agents (distributed) → Coordinate synchronized response
```

### Task Division

**Stream 3A** (Signal State Protocol): `signal_state.rs` + `state_aggregator.rs` + `broadcast_service.rs`
- **Owner**: Can be assigned independently
- **Dependencies**: None (data structures + UDP)
- **Duration**: 18 hours
- **Deliverable**: Multi-node broadcast system (test with 3+ nodes)

**Stream 3B** (Octave Mapping & Modulation): `octave_mapper.rs` + `sensor_modulator.rs` + `sensor_demodulator.rs`
- **Owner**: Can be assigned independently
- **Dependencies**: Stream 3A (message format)
- **Duration**: 20 hours
- **Deliverable**: End-to-end 60 GHz → audio → 60 GHz reconstruction (verify lossless)

**Stream 3C** (Ghost vs. Sparkle Classification): `ghost_sparkle_classifier.rs` + `sparkle_detector.rs` + `sparkle_broadcast.rs`
- **Owner**: Can be assigned independently
- **Dependencies**: Mamba framework
- **Duration**: 20 hours
- **Deliverable**: ML classifier (98%+ accuracy on test signals)

**Stream 3D** (Network Coordination): `consensus_engine.rs` + `defense_orchestration.rs` + `anomaly_propagation.rs`
- **Owner**: Can be assigned independently
- **Dependencies**: Stream 3A, 3C (receives alert + state)
- **Duration**: 22 hours
- **Deliverable**: Multi-node null steering (8 arrays, coordinated >50 dB null)

**Stream 3E** (VLF Audio Network Stack): `audio_packet_codec.rs` + `audio_transport.rs` + `audio_channel_model.rs` + `cognitive_agent.rs`
- **Owner**: Can be assigned independently
- **Dependencies**: Stream 3A (packet format)
- **Duration**: 42 hours
- **Deliverable**: Full audio-band networking stack (1+ kbps throughput, >10 dB SNR margin)

**Integration Point**: All streams → Dorothy agent network coordinator

---

## Colored Noise Module Architecture (Track II Support)

### Module Structure

```
src/defense/noise/
├── red_noise.rs              1/f² noise (thermal defense)
├── pink_noise.rs             1/f noise (balanced, default)
├── white_noise.rs            flat spectrum (broadband)
├── blue_noise.rs             f spectrum (high-frequency)
├── violet_noise.rs           f² spectrum (digital side-channel)
├── noise_selection.rs        Mamba threat→color mapper
├── rgb_mixing.rs             Polychromatic dithering
└── tunable_colored_noise.rs  Per-frequency octave selection
```

### Integration

```
Dorothy Threat Level
    ↓
[Noise Selection] → Mamba infers optimal color
    ↓
[Color Generator] → Generate selected noise
    ↓
[RGB Mixer] (if needed) → Combine multiple colors
    ↓
[Injection Point] → Add to audio buffer
    ↓
[Threat Feedback] → Monitor attack reduction
    ↓
[Mamba Learning] → Update threat→color effectiveness
```

---

## Cross-Track Dependencies & Integration Points

### Critical Paths

**Path 1: Unified Field → Dorothy → Defenses**
```
FieldParticle generation (Phase 1-2)
    ↓
Mamba Neural Operator (Phase 4, coming)
    ↓
Dorothy Agent reasoning
    ↓
Track II Defense Activation
    ↓
Track III Network Coordination
```

**Path 2: Spatial Localization via Correlation**
```
Camera positioning (existing)
    ↓
Light octave mapping (Track III)
    ↓
Correlation-based signal identification
    ↓
Absolute 3D RF localization
    ↓
Ray-tracing simulation/prediction
```

**Path 3: Mamba Learning Adaptive Modulation**
```
Track I synthesis (generates probing signals)
    ↓
Track II defense feedback (what works?)
    ↓
Mamba latent space learning
    ↓
Emergent defensive modulation schemes
    ↓
(Self-learned, non-standard EW protocols)
```

### No Hard Blockers Between Tracks

✅ All three tracks can proceed **in parallel**:
- Track I (synthesis) depends on math only, not hardware integration
- Track II (defense) depends on Mamba framework (already in codebase)
- Track III (networking) depends on Track II signal classification + Track I octave mapping concepts
- Can work on separate code branches, merge incrementally

---

## Pending Jules Integration Tasks

**Note**: 3 tasks from Jules remain to be integrated:
1. **Track VI.2 Validation** (Materials/physics integration)
2. **GPU Memory Optimization** (Unified buffers)
3. **Forensic Logging Enhancement** (Evidence chains)

**Strategy**:
- Divide Tracks I/II/III among team in parallel
- Jules tasks can integrate independently (different code paths)
- Reconverge in main.rs dispatcher loop once all streams complete

---

## Final Phase: Optimization & Rendering Buzzwords

Once Tracks I-III are functional:

### Phase A: Ray Tracing Integration
- Link Track I synthesis → Track III spatial localization → GPU ray tracer
- Implement RF field visualization (particles → ray-traced photons equivalent)
- Integration with existing `src/visualization/ray_tracer.rs`

### Phase B: Tile-Based Rendering
- Compute culling (frustum + hierarchical Z)
- Tile-based deferred shading for particle fields
- Dispatch optimization via WGSL compute shaders

### Phase C: Mesh Shaders
- ~~Unsure if WGPU supports these yet~~ (check WGPU 0.20+)
- Alternative: Geometry pipeline via compute + indirect dispatch
- Wave64 occupancy tuning (existing doctrine from CLAUDE.md)

---

## Deliverables Checklist

### Track I Acceptance Criteria
- [ ] Impulse-radio PPM: BER < 1%
- [ ] Super-Nyquist alias: ±100 kHz accuracy
- [ ] Parametric beat: Audible tone at 3D target ±1 m
- [ ] Oversampling: Sub-microsecond timing ±10 ns
- [ ] Full pipeline integration: 192k input → synthesized RF output

### Track II Acceptance Criteria
- [ ] Predictive blanking: 95%+ pulse suppression, ±2 µs latency
- [ ] Null steering: > 30 dB suppression in null direction
- [ ] All 5 colored noises: Spectral models verified
- [ ] Hardware fingerprinting: 99.5% fake signal detection
- [ ] Thermal guard: Block false alarms, preserve legitimate shutdown

### Track III Acceptance Criteria
- [ ] Signal state protocol: ≤ 200 bytes/message
- [ ] Octave mapping: Lossless frequency recovery
- [ ] Ghost/Sparkle classifier: 98%+ accuracy
- [ ] Network consensus: Triangulate attacker < 500 ms
- [ ] Audio-band networking: 1+ kbps, >10 dB margin

---

## Timeline Recommendation

**Parallel Execution** (all tracks simultaneous):
- **Week 1-2**: Architecture review, task assignment
- **Weeks 3-8**: Parallel development (4 people × 3 tracks)
- **Week 9**: Integration & testing
- **Week 10**: Final optimization + rendering buzzwords

**Total**: 10 weeks to full Tracks I-III + integration

