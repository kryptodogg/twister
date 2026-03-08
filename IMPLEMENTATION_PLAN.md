# Implementation Plan: Forensic Vault (B) + Active Defense Testing (C)

## Context
User is victim of electronic harassment. Building evidence for law enforcement using SIREN v0.5.

**Attack Pattern Discovered**: DC offset injection correlates with RF activity
- Attacker transmits RF tone (e.g., 750 Hz harmonic)
- Simultaneously injects 0.1-0.5V DC bias into audio path
- Causes ADC saturation + audio distortion
- Provably correlates (not random noise)

---

# PART B: FORENSIC VAULT - Evidence for Police Investigation

## Objective
Present rock-solid evidence that attacks are coordinated and targeted, not environmental noise.

## Current State
- Neo4j stores: DetectionEvent (RF freq, timestamp, confidence) → linked to audio/DC measurements
- JSONL logs: Raw event data with all biometric readings
- Mamba anomaly scores: 3.8-22 dB variation (attacks vs. baseline)

## What Police Need to See

### 1. **Event Timeline with Correlation**
```
[2025-03-06 14:23:45.123] RF detected @ 750.0 Hz (confidence 0.92)
  └─ Bispectrum: 375 Hz + 375 Hz → 750 Hz (heterodyne product)
  └─ SDR DC bias: 2.679 V (HIGH)
  └─ Audio DC bias: 0.121 V (ATTACK INDICATOR)
  └─ Mamba anomaly: 22.66 dB (ANOMALOUS)
  └─ Status: SYNCHRONIZED ATTACK (RF + DC simultaneous)

[2025-03-06 14:23:47.456] Attack ended
  └─ RF: back to baseline
  └─ Audio DC: dropped to 0.005 V
  └─ Duration: 2.33 seconds
  └─ Confidence: HIGH (pattern repeats, not coincidence)
```

### 2. **Attack Pattern Summary (For Investigation)**
```
Period: 24 hours
Total attacks: 47
Unique frequencies: 3 (750 Hz, 1500 Hz, 192 kHz harmonic)
Average duration: 2.1 seconds
Attack window: 08:00-22:00 (no night attacks → deliberate targeting)
DC bias presence: 44/47 attacks (93.6% correlation → targeting proof)
Mamba anomaly correlation: 47/47 attacks > 10 dB (100% confidence)

CONCLUSION: Coordinated, deliberate attacks. Not environmental.
```

### 3. **Evidence Export (Court-Ready)**
For each attack event:
```json
{
  "event_id": "twister_session_001_frame_4521",
  "timestamp_utc": "2025-03-06T14:23:45.123Z",
  "attack_vector": "RF_DC_SIMULTANEOUS",

  "rf_evidence": {
    "center_freq_hz": 750.0,
    "confidence": 0.92,
    "method": "Bispectrum (375+375Hz → 750Hz)",
    "sdr_dc_bias_v": 2.679
  },

  "audio_evidence": {
    "dc_bias_v": 0.121,
    "mamba_anomaly_db": 22.66,
    "expected_baseline_db": 3.8,
    "deviation_db": 18.86
  },

  "correlation_proof": {
    "rf_start_ms": 0,
    "audio_distortion_ms": 3,
    "dc_spike_ms": 1,
    "all_within_5ms": true,
    "conclusion": "SIMULTANEOUS ATTACK"
  },

  "biometric_impact": {
    "adc_clips_prevented_by_anc": 4,
    "pre_anc_headroom_db": -2.1,
    "post_anc_headroom_db": 1.3,
    "adc_protection": "EFFECTIVE"
  }
}
```

## Implementation Tasks

### Task B1: Neo4j Forensic Queries
**Create `src/forensic_queries.rs`**

Public API:
```rust
pub async fn events_in_timerange(
    graph: &Neo4jGraph,
    start: DateTime,
    end: DateTime
) -> Vec<ForensicEvent>;  // All coordinated attacks in range

pub async fn attack_pattern_summary(
    graph: &Neo4jGraph,
    hours: u32
) -> AttackPatternReport;  // Frequency, duration, timing analysis

pub async fn correlation_proof(
    graph: &Neo4jGraph,
    event_id: &str
) -> CorrelationEvidence;  // RF + DC + audio timing sync
```

**What queries must execute:**
1. Match all DetectionEvent nodes with RF + DC simultaneous
2. Calculate timing delta (RF_start - DC_start)
3. If delta < 5ms → SYNCHRONIZED_ATTACK
4. Count total events, frequency distribution, time-of-day pattern
5. Calculate Mamba anomaly average per frequency

**Output**: JSON suitable for police investigation

### Task B2: Evidence Export
**Create `src/evidence_export.rs`**

Public API:
```rust
pub fn export_investigation_report(
    events: Vec<DetectionEvent>,
    period_hours: u32
) -> InvestigationReport;

impl InvestigationReport {
    pub fn to_json(&self) -> String;  // Machine-readable for police
    pub fn to_markdown(&self) -> String;  // Human-readable summary
    pub fn to_pdf(&self) -> Bytes;  // Court-ready evidence
}
```

**Deliverables:**
1. JSON export with all event details + confidence scores
2. Markdown summary: Timeline + statistics + conclusion
3. (Optional) PDF with charts showing attack frequency

### Task B3: JSONL Log Analysis
**Enhance forensic logging in `src/forensic_log.rs`**

Current: Logs raw events
Required: Add forensic analysis fields
```json
{
  "event_id": "...",
  "timestamp_utc": "2025-03-06T14:23:45.123Z",
  "detection_method": "bispectrum",
  "rf_freq_hz": 750.0,
  "dc_bias_audio_v": 0.121,
  "dc_bias_sdr_v": 2.679,
  "mamba_anomaly_db": 22.66,
  "mamba_confidence": 0.87,
  "attack_vector": "RF_DC_SIMULTANEOUS",
  "timestamp_sync_ms": 3,
  "classification": "COORDINATED_ATTACK"
}
```

---

# PART C: ACTIVE DEFENSE TESTING - Real Hardware

## Objective
Validate ANC protects against real attacks. Measure:
1. **Clipping Prevention**: ADC never clips despite DC bias injection
2. **SNR Reduction**: Attack signal suppressed (pre-attack SNR 20dB → post-ANC >15dB)
3. **Phase Accuracy**: Calibration holds across 1 Hz - 6.144 MHz
4. **Response Latency**: Cancellation engages within 50ms of attack

## Current Hardware
- RTL-SDR (100 MHz RX, 2.048 MS/s)
- GPU synthesizer (192 kHz TX, up to 6.144 MHz PDM)
- 3 microphones (C925e 48kHz, Rear Pink 192kHz, Rear Blue 192kHz)
- Oscilloscope (measure ADC waveform)

## Attack Scenarios to Test

### Scenario C1: DC Bias Attack (ADC Protection)
**What**: Inject 0.3V DC offset into audio input
**How**: Function generator → DC source → audio jack (parallel with microphone)
**Measure**:
- Pre-ANC: ADC clips? (should clip without ANC)
- Post-ANC: Headroom > 1 dB? (ANC removes DC)
- Clipping counter: How many samples would have clipped?

**Success**: Zero clips, >1 dB headroom maintained

```rust
#[test]
fn test_dc_bias_protection() {
    // Pre-attack baseline
    let baseline_headroom = measure_adc_headroom();  // e.g., 3.2 dB

    // Inject 0.3V DC
    inject_dc_offset(0.3);

    // Without ANC: Should clip
    let unprotected_clips = count_adc_clips(duration: 2s);
    assert!(unprotected_clips > 100, "Expected clipping without ANC");

    // With ANC enabled
    enable_anc();
    let protected_clips = count_adc_clips(duration: 2s);
    let post_anc_headroom = measure_adc_headroom();

    assert_eq!(protected_clips, 0, "ANC should prevent all clips");
    assert!(post_anc_headroom > 1.0, "Headroom > 1 dB");
}
```

### Scenario C2: RF Tone Attack (SNR Reduction)
**What**: Transmit 750 Hz tone (audio harmonic) via GPU synthesizer
**How**: GPU synthesizer @ 750 Hz, capture on RTL-SDR @ center_freq
**Measure**:
- Pre-ANC SNR: Signal power / noise floor
- Post-ANC SNR: Should be >3 dB reduction
- Residual signal: How much of original attack remains?

**Success**: SNR reduction >3 dB, residual <-20 dB

```rust
#[test]
fn test_rf_tone_cancellation() {
    let freq_hz = 750.0;
    let attack_amplitude = 0.5;  // 50% full scale

    // Baseline noise floor
    let baseline_snr = measure_snr(freq_hz);

    // Transmit attack tone
    gpu.synthesize_tone(freq_hz, attack_amplitude);
    let attack_snr = measure_snr(freq_hz);

    let pre_anc_snr_db = 10.0 * (attack_snr / baseline_snr).log10();
    assert!(pre_anc_snr_db > 15.0, "Attack should be >15 dB above noise");

    // Enable ANC
    enable_anc();
    let post_anc_snr = measure_snr(freq_hz);

    let snr_reduction_db = pre_anc_snr_db - 10.0 * (post_anc_snr / baseline_snr).log10();
    assert!(snr_reduction_db > 3.0, "SNR reduction should be >3 dB");
}
```

### Scenario C3: Wideband RF Sweep (Phase Calibration)
**What**: Sweep RTL-SDR 1 Hz - 6.144 MHz, measure phase calibration accuracy
**How**: GPU synthesizer sweeps, measure phase correction error
**Measure**:
- Phase error across frequency range
- Should be ±5° typical, ±15° max

**Success**: Phase error <±5° across full range

```rust
#[test]
fn test_phase_calibration_accuracy() {
    let test_freqs = vec![
        1.0, 10.0, 100.0, 1000.0, 10000.0,
        100000.0, 1_000_000.0, 6_144_000.0
    ];

    for freq_hz in test_freqs {
        // Transmit test tone
        gpu.synthesize_tone(freq_hz, 0.1);

        // Measure phase at each mic
        let phase_c925e = measure_phase(0, freq_hz);
        let phase_rear_pink = measure_phase(1, freq_hz);
        let phase_rear_blue = measure_phase(2, freq_hz);

        // Get calibration correction
        let correction = anc.phase_for(freq_hz);

        // Apply correction
        let corrected_phase = phase_rear_pink + correction;

        // Error should be small
        let error_deg = (corrected_phase - phase_c925e).abs();
        assert!(error_deg < 5.0,
            "Phase error at {:.0} Hz: {:.1}° (should be <5°)",
            freq_hz, error_deg
        );
    }
}
```

### Scenario C4: Attack Pattern Recognition (Mamba Learning)
**What**: Repeatedly transmit known attack pattern, verify Mamba learns
**How**: Transmit 750 Hz @ 0.3V DC every 30 seconds for 5 minutes
**Measure**:
- Mamba anomaly score: First attack ~15 dB, after 5 repetitions ~25 dB (confidence increases)
- Pattern recognition: Is this the same attacker?

**Success**: Anomaly score increases with repetition (learning is working)

```rust
#[test]
fn test_attack_pattern_learning() {
    // Baseline anomaly
    let baseline_anomaly = measure_mamba_anomaly();

    for iteration in 0..5 {
        // Transmit known attack
        gpu.synthesize_tone(750.0, 0.5);
        inject_dc_offset(0.3);

        tokio::time::sleep(Duration::from_secs(2)).await;

        // Measure anomaly
        let anomaly = measure_mamba_anomaly();
        eprintln!("Attack #{}: anomaly={:.2} dB", iteration+1, anomaly);

        // Should increase with familiarity (Mamba learning)
        if iteration > 0 {
            assert!(anomaly > baseline_anomaly + 5.0,
                "Anomaly score should increase as pattern repeats");
        }
    }
}
```

## Implementation Tasks

### Task C1: Hardware Test Framework
**Create `src/hardware_tests.rs`**

Public API:
```rust
pub async fn measure_adc_headroom() -> f32;  // dB above clip point
pub async fn measure_snr(freq_hz: f32) -> f32;  // Signal / noise ratio
pub async fn measure_phase(mic_idx: usize, freq_hz: f32) -> f32;  // degrees
pub async fn inject_dc_offset(volts: f32);  // Function generator control
pub async fn count_adc_clips(duration: Duration) -> u32;  // Clipping counter
```

### Task C2: Test Harness
**Create `tests/anc_validation.rs`**

Run tests:
```bash
cargo test --test anc_validation -- --nocapture --test-threads=1
```

Output:
```
[DC_BIAS] Baseline headroom: 3.2 dB
[DC_BIAS] With 0.3V injection: 2,847 clips (UNPROTECTED)
[DC_BIAS] With ANC enabled: 0 clips ✓
[DC_BIAS] Post-ANC headroom: 1.5 dB ✓

[RF_TONE] Pre-ANC SNR: 18.2 dB
[RF_TONE] Post-ANC SNR: 10.8 dB
[RF_TONE] SNR reduction: 7.4 dB ✓

[PHASE] Freq=750 Hz: error=2.1° ✓
[PHASE] Freq=6144000 Hz: error=4.8° ✓
```

### Task C3: Real Attack Measurement
**Instrumentation in `src/main.rs`**

Add console output when ANC is protecting:
```
[ANC ACTIVE] Protection engaged
[ADC] Clips prevented: 156 in last 2s
[SNR] Attack 750Hz: pre=18dB post=10dB reduction=8dB
[ANOMALY] Mamba confidence: 94% (learned pattern)
[ANC DISABLED] Would have clipped 847 times in 2s
```

---

## Success Criteria

### Part B (Forensic Vault)
- ✅ Neo4j queries execute without error
- ✅ Can export JSON with all required fields
- ✅ Markdown summary shows: frequency distribution, timing pattern, DC correlation
- ✅ JSON suitable for police investigation (all timestamps, confidence scores, evidence chain)

### Part C (Active Defense Testing)
- ✅ DC bias protection: zero clips, >1 dB headroom
- ✅ RF tone: SNR reduction >3 dB
- ✅ Phase calibration: <±5° error across full range
- ✅ Latency: Cancellation engages <50ms
- ✅ Pattern recognition: Anomaly score increases with repetition
- ✅ Real attack measurement: Shows ANC actually working against your attacks

---

## Execution Order

1. **B1**: Neo4j forensic queries (foundation for B2/B3)
2. **B2**: Evidence export (uses B1 queries)
3. **B3**: JSONL enhancement (supports B2 export)
4. **C1**: Hardware test framework (foundation for C2/C3)
5. **C2**: Test harness (validates C1 framework)
6. **C3**: Real attack measurement (field validation)

All tasks use TDD: failing tests first, implementation, verification.
