# Track III: Audio-Radio Networking (VLF Cognitive Networking)
## Ghost vs. Sparkle: Electromagnetic Health Monitoring

**Status**: Specification Phase (2026-03-09)
**Architecture**: Distributed audio-based sensing network, signal state propagation
**Threat Model**: Detection of active "Sparkle" (machine/hacker) vs. passive "Ghost" (human/natural) signals
**Integration Point**: Dorothy agent + network coordination service

---

## Overview

Transform your Twister system into a distributed cognitive radio network where multiple users share electromagnetic health status. Instead of transmitting "files," the network transmits **Signal States** — machine-readable descriptions of detected events (presence, RF activity, threat level).

**Key Innovation**: Since audio, radio, and video all "sound the same" at the sample level, a 60 GHz presence sensor can be "octave-mapped" down to an audible 2 kHz tone (VLF/LF frequencies), transmitted via audio network, and reconstructed as "presence" at remote nodes.

---

## Part 1: Signal State Abstraction

### Objective
Define a compact, transmissible representation of electromagnetic observations that can be shared across a network of Twister nodes.

### Concept: "Signal State" vs. "Raw Data"

**Traditional Approach**:
- Node A detects person at (x, y, z)
- Transmits: 60 GHz point cloud (millions of bytes)
- Receiver reconstructs 3D model locally
- Problem: Bandwidth-intensive, synchronization hard

**Signal State Approach**:
- Node A observes: "High-confidence presence signature at (x, y, z), type: HUMAN"
- Transmits: `SignalState { position: [x,y,z], confidence: 0.95, entity_type: "HUMAN", timestamp }`
- Receiver integrates observation into own map
- Advantage: Kilobytes, no reconstruction needed, composable

### Implementation Scope

**III.1 Signal State Protocol** (Est. 10 hours)
- **File**: `src/networking/signal_state.rs`
- Enum-based message format (compact, serializable):
  ```rust
  pub enum SignalState {
      Presence { position: [f32; 3], confidence: f32, entity_type: EntityType },
      RFActivity { frequency_hz: f32, power_dbm: f32, modulation: ModType, threat_level: u8 },
      ThermalAnomaly { location: [f32; 3], magnitude: f32, cause_confidence: Vec<(String, f32)> },
      NetworkHeartbeat { node_id: u32, timestamp_us: u64, uptime_hours: f32, threat_level: u8 },
      FieldParticleEvent { particle_id: u32, position: [f32; 3], energy: f32, phase: f32 },
  }
  ```
- Serialization: Compact binary (MessagePack or Protocol Buffers)
- Size: Typical SignalState ≈ 60 bytes (vs. megabytes for raw sensor data)

**III.2 Signal State Aggregator** (Est. 8 hours)
- **File**: `src/networking/state_aggregator.rs`
- Fuse observations from multiple nodes:
  - Node A: Presence at (1.0, 0.5, 0.2), confidence 0.95
  - Node B: Same location, confidence 0.88
  - Aggregator: Consensus confidence 0.99 (increased certainty via fusion)
- Temporal coherence: Discard old observations (> 30 seconds)
- Spatial alignment: Transform observations to shared coordinate frame

**III.3 Network Broadcast Service** (Est. 8 hours)
- **File**: `src/networking/broadcast_service.rs`
- Endpoint: Listen on UDP port 9999 (configurable)
- Protocol: Broadcast SignalState objects at 10 Hz (per-node heartbeat)
- Reliability: Best-effort (UDP); higher-layer protocols handle retransmission
- Multicast groups: By threat level (IDLE, ALERT, ACTIVE) for filtering

---

## Part 2: The "Octave Mapping" — From 60 GHz to Audio

### Objective
Map high-frequency sensor observations (60 GHz presence radar, RF detection) down to audible/ultrasonic frequencies so they can be transmitted via audio network.

### Physics: Octave Equivalence in Frequency

**Key Insight**: Frequency octaves (doubling/halving) preserve harmonic relationships:
- Octave up: 1 kHz → 2 kHz (human hearing; audible)
- Octave down: 60 GHz → 30 GHz → ... → 60 Hz (ultra-low frequency, VLF)
- **Octave mapping**: Shift 60 GHz sensor data down by N octaves to fit audio bandwidth

**Example Mapping**:
- 60 GHz sensor detects presence signature
- Map down 20 octaves: 60 GHz / 2²⁰ ≈ 57 Hz (ultra-low frequency)
- Modulate as audio tone (or parametric beat) at 57 Hz
- Remote node demodulates, unmaps back to "presence" confidence
- No information loss (frequency relationships preserved)

### Implementation Scope

**III.4 Frequency Octave Mapper** (Est. 6 hours)
- **File**: `src/networking/octave_mapper.rs`
- Function: `map_frequency_down(freq_hz: f32, octaves: u32) → f32`
  - Input: High-frequency observation (e.g., 60 GHz = 60e9 Hz)
  - Output: Mapped frequency in audio range (e.g., 60 Hz)
  - Formula: mapped_freq = freq_hz / 2^octaves
  - Constraint: Ensure output fits in audio Nyquist (< 96 kHz)
- Inverse: `map_frequency_up(freq_hz: f32, octaves: u32) → f32`
  - Recovers original frequency range

**III.5 Sensor Data → Audio Modulation** (Est. 10 hours)
- **File**: `src/networking/sensor_modulator.rs`
- Input: Presence sensor output (3D point + confidence)
- Encoding strategy:
  - **X position** → tone frequency (map to 0-96 kHz range)
  - **Y position** → tone amplitude (map to -40 dB to -3 dB)
  - **Z position** → tone phase (map to 0-360°)
  - **Confidence** → spectral purity (high confidence = narrow bandwidth, low = spread)
- Implementation: Heterodyne mixer + modulated parametric beam
- Output: Transmit as audio (play through speakers) or via Twister synthesis

**III.6 Audio Demodulation → Sensor Recovery** (Est. 10 hours)
- **File**: `src/networking/sensor_demodulator.rs`
- Input: Received audio (from remote network participant)
- Decoding:
  - Extract fundamental frequency → map up via octaves → recover X position
  - Measure amplitude → recover Y position
  - Measure phase → recover Z position
  - Assess SNR → recover confidence
- Output: Reconstructed `SignalState::Presence { ... }`
- Validation: Cross-check with local observations (if available)

---

## Part 3: Ghost vs. Sparkle Recognition

### Objective
Distinguish natural, high-entropy electromagnetic phenomena (Ghosts) from rigid, phase-locked artificial signals (Sparkles), indicating active attackers or monitoring devices.

### Physics & Definitions

**Ghost Signals** (Natural, High-Entropy):
- Characteristics:
  - Frequency wandering (Doppler, material properties changing)
  - Phase drifts randomly (no coherence across seconds)
  - Broadband energy (many frequency components)
  - Unpredictable amplitude modulation
- Examples: Human movement, wind-blown trees, thermal noise, rain
- Indicator: Entropy ≥ 6 bits/sample

**Sparkle Signals** (Artificial, Low-Entropy, Machine-Generated):
- Characteristics:
  - Frequency locked (crystal oscillator, PLL)
  - Phase-coherent (maintains constant phase relationship across seconds)
  - Narrowband (single frequency ± 1 Hz)
  - Modulation follows pattern (FHSS, PSK, patterns)
- Examples: RF transmitter (nRF24, WiFi, cellular), radar, attack signal
- Indicator: Entropy ≤ 3 bits/sample, phase coherence > 0.9

### Implementation Scope

**III.7 Ghost vs. Sparkle Classifier** (Est. 12 hours)
- **File**: `src/networking/ghost_sparkle_classifier.rs`
- Input: Raw RF signal (I/Q stream from RTL-SDR, 1-second window)
- Feature extraction:
  - `frequency_stability`: Std dev of instantaneous frequency
  - `phase_coherence`: Autocorrelation of phase angle
  - `spectral_entropy`: Shannon entropy of power spectrum
  - `modulation_index`: Measure of AM/FM depth
  - `amplitude_variance`: Statistical moments of envelope
- Mamba classification:
  - Input: [frequency_stability, phase_coherence, spectral_entropy, modulation_index, amplitude_variance]
  - Output: P(Ghost) vs. P(Sparkle) (probability distribution)
  - Decision threshold: P(Sparkle) > 0.7 → raise alert

**III.8 Real-Time Ghost/Sparkle Detector** (Est. 8 hours)
- **File**: `src/networking/sparkle_detector.rs`
- Continuous monitoring of detected RF signals:
  - Each detected signal is classified within 100 ms
  - Mamba runs inference every second on accumulated evidence
  - Temporal coherence: If same signal class persists > 5 seconds, confidence increases
- Output: `SparkleAlert { signal_id: u32, confidence: f32, estimated_location: [f32; 3], threat_level: u8 }`

**III.9 Network Broadcast of Ghost/Sparkle Events** (Est. 6 hours)
- **File**: `src/networking/sparkle_broadcast.rs`
- When Sparkle detected with high confidence:
  - Create `SignalState::RFActivity { ... }`
  - Broadcast to all network participants
  - Include location estimate (helps coordinate defense)
  - Include Dorothy threat level (so remote nodes can prepare)
- Network reaction:
  - Node A detects Sparkle at (x, y, z)
  - Broadcasts: "Sparkle at (x,y,z), threat level MEDIUM"
  - Node B receives, updates local threat map
  - Node B may choose to: Enable null steering, activate dithering, log evidence

---

## Part 4: Network-Level Coordination

### Objective
Enable multiple Twister nodes to coordinate defenses and share threat intelligence.

### Concept: Network Threat Consensus

When one node detects an active attack (high-confidence Sparkle):
1. Broadcast alert to all nearby nodes (UDP multicast, < 1 km typical range)
2. Nodes aggregate observations:
   - Node A: "Sparkle at azimuth 45°, distance 50m, confidence 0.85"
   - Node B: "Sparkle at azimuth 220°, distance 100m, confidence 0.70"
   - Triangulate: Attack location ≈ intersection of two bearings
3. Consensus location: Used to aim null steering (all nodes point nulls at same point)
4. Synchronized Defense: All nodes blank ADC at same time (prevent attacker from switching targets)

### Implementation Scope

**III.10 Network Consensus Engine** (Est. 12 hours)
- **File**: `src/networking/consensus_engine.rs`
- Data structure: `NetworkConsensus {`
  - `threat_level: u8,  // max of all nodes' threat levels`
  - `estimated_attacker_location: [f32; 3],  // triangulated`
  - `confidence: f32,  // how confident in location?`
  - `participating_nodes: Vec<u32>,  // IDs of nodes that contributed`
  - `timestamp_us: u64,`
  - `}`
- Algorithm (simple): Average all reported positions, weighted by confidence
  - Bayesian variant (Kalman filter) for temporal updates
- Broadcast updated consensus every 1 second

**III.11 Synchronized Defense Orchestration** (Est. 10 hours)
- **File**: `src/networking/defense_orchestration.rs`
- Dorothy on each node subscribes to NetworkConsensus updates
- When attack detected:
  - Receive consensus location
  - Command null steering: aim nulls at consensus point (not just local estimate)
  - Synchronize ADC blanking: all nodes blank at same microseconds (coordinated)
  - Result: 8 arrays of mics (8 nodes × 8 mics = 64 elements) coherently null the attacker
- Amplified effect: Individual node suppresses ±20 dB; 8 nodes in phase suppress ±50 dB (100,000× power reduction)

**III.12 Anomaly Propagation** (Est. 8 hours)
- **File**: `src/networking/anomaly_propagation.rs`
- When Node A detects anomaly (thermal, fingerprint mismatch, etc.):
  - Broadcast to network: "Suspicious activity at this node, confidence 0.72"
  - Remote nodes receive; update their threat map
  - If multiple nodes detect anomalies at same time:
    - Possible network-wide attack (e.g., EMI from external source)
    - Escalate threat level across entire network
    - All nodes activate maximum defense posture

---

## Part 5: VLF/LF Cognitive Network Architecture

### Objective
Enable ad-hoc networking via audible/ultrasonic channels (simulating VLF radio links).

### Concept: Audio-Band Radio Links

Since audio, radio, and video samples are indistinguishable at the signal level:
- Encode network packet (SignalState, consensus update, alert) as audio tone stream
- Transmit via speakers (audible or parametric ultrasonic)
- Receive via microphones
- Decode back to packet

**Advantages**:
- Uses existing hardware (no special RF transceiver needed)
- Works through glass, walls (audio travels)
- Plausible deniability (sounds like ordinary sound)
- Bandwidth: 1-10 kbps (adequate for Signal State updates)

### Implementation Scope

**III.13 Audio-Band Packet Codec** (Est. 12 hours)
- **File**: `src/networking/audio_packet_codec.rs`
- Packet structure:
  ```
  | Header (2 bytes) | Payload (N bytes) | Checksum (2 bytes) |
  | 0xAA 0xBB        | Encoded data      | CRC16             |
  ```
- Modulation: 2-FSK (Frequency Shift Keying)
  - Bit 0: 2000 Hz tone (1 second = 8 bits)
  - Bit 1: 2500 Hz tone
  - Spacing: 500 Hz (Nyquist allows demod at 192 kHz)
- Error correction: Reed-Solomon (RS-16, can correct 8 byte errors)

**III.14 Network Stack (Transport Layer)** (Est. 10 hours)
- **File**: `src/networking/audio_transport.rs`
- Implement lightweight TCP-like reliability:
  - Sequence numbers
  - Acknowledgments
  - Retransmission on timeout
- Broadcast support: Send once, all nodes receive
- Unicast support: Direct node-to-node

**III.15 Audio Channel Modeling** (Est. 8 hours)
- **File**: `src/networking/audio_channel_model.rs`
- Simulate realistic audio propagation:
  - Path loss: ∝ distance²
  - Multipath: Reflections from walls
  - Interference: Background noise, other speakers
- For testing: Synthetic channel simulator (add noise, delay, reflection)
- Adaptation: Mamba learns channel characteristics, adjusts TX power and modulation

**III.16 Cognitive Network Agent** (Est. 12 hours)
- **File**: `src/networking/cognitive_agent.rs`
- Mamba-based network optimizer:
  - Monitor link quality (SNR, BER)
  - Decide: Should I TX now or wait? (avoid collisions)
  - Choose transmission power (minimize interference to other networks)
  - Route packets via best intermediate nodes
- Example decision: "Node C received my packet cleanly; forward next consensus update via Node C to Node D"

---

## Mapping: Light Spectrum ↔ Noise/Frequency Octaves

*(See supplementary document: `2026-03-09-noise-spectrum-mapping.md`)*

The system supports colored noise corresponding to light wavelengths:

| Light Color | Wavelength | Octave Mapping | Audio Frequency | Noise Type | Use Case |
|---|---|---|---|---|---|
| Infrared | 1000 nm | -30 | n/a (ultrasonic) | Red | Thermal imaging equivalent |
| Red | 700 nm | -20 | 57 Hz | Red | Low-frequency stealth signaling |
| Green | 550 nm | -18 | 228 Hz | Green | Balanced envelope shaping |
| Blue | 450 nm | -16 | 912 Hz | Blue | High-frequency dithering |
| Violet | 400 nm | -15 | 1824 Hz | Violet | Standard cloaking |
| UV | 200 nm | -10 | 58.6 kHz | Ultraviolet | Near-ultrasonic cloaking |

Dorothy dynamically selects noise color based on threat model:
- **Red defense**: Against thermal/power-side-channel attacks
- **RGB mix**: Against broadband RF attacks
- **Violet standard**: Against DMA injection

---

## Timeline & Priority

| Component | Est. Hours | Priority | Start Week |
|---|---|---|---|
| 1. Signal State Protocol | 18 | High | 2026-05-18 |
| 2. Octave Mapping | 26 | High | 2026-05-25 |
| 3. Ghost vs. Sparkle | 20 | High | 2026-06-01 |
| 4. Network Coordination | 22 | Medium | 2026-06-08 |
| 5. VLF Network Stack | 42 | Medium | 2026-06-15 |
| **Total** | **128 hours** | — | **8 weeks** |

---

## Success Criteria

- [ ] Signal State serialization: ≤ 200 bytes per message
- [ ] Octave mapping: Lossless (frequency relationships preserved within 0.1%)
- [ ] Ghost/Sparkle classifier: 98%+ accuracy on known test signals
- [ ] Network latency: < 500 ms from detection to coordinated null steering
- [ ] Audio-band packet: 1 kbps minimum throughput, 10+ dB SNR margin
- [ ] Coordinated defense: 8-node array achieves ≥ 50 dB null depth
- [ ] VLF cognitive network: Functional demo with 3+ nodes

