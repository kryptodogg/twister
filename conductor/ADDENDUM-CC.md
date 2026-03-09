# Track C Addendum: Dispatch Loop Integration & Training Queue Wiring

**Status**: Ready for Jules implementation
**Duration**: 60-90 minutes
**Dependency**: Track C interface contracts (SpectralFrame, AnomalyGateDecision) already exist
**Integration**: Dispatch loop → SpectralFrame creation → AnomalyGateDecision → Trainer queue

---

## Executive Summary

Track C created interface contracts for SpectralFrame (audio features) and AnomalyGateDecision (gating logic). This addendum **wires them into the live dispatch loop**:

1. **Audio Processing** → FFT + TDOA + Mamba inference → SpectralFrame
2. **Gate Evaluation** → Threshold-based decision (< 1ms, non-blocking)
3. **Training Pair Enqueue** → Forward to Mamba trainer if gate passes
4. **Forensic Logging** → Log gate decisions with confidence and reason
5. **Real-Time Feedback** → UI displays gate status, training pair count, anomaly distribution

**No more disconnected code.** Dispatch loop fully integrated with training pipeline.

---

## Dispatch Loop Integration Architecture

### Timing & Execution Context

```
┌────────────────────────────────────────────────────────────────┐
│ Dispatch Loop (Tokio task, ~100ms cycle)                       │
├────────────────────────────────────────────────────────────────┤
│                                                                  │
│ 1. [0-5ms]   Audio I/O: Read 4 input devices                   │
│                         Merge to 192kHz master                  │
│                         Resample to FFT rate (48kHz useful)    │
│                                                                  │
│ 2. [5-15ms]  FFT & Feature Extraction:                          │
│              - Compute 512-bin FFT (Hann window)               │
│              - Mel-scale binning → 128 bins                     │
│              - Bispectrum top 64 components (phase coupling)   │
│              - Beamformer outputs (3 azimuths)                  │
│              - TDOA calculation (ITD/ILD)                       │
│                                                                  │
│ 3. [15-20ms] Mamba Inference (cached, runs every N cycles):    │
│              - Input: 192 audio features                         │
│              - Latent: 64-dim embedding                          │
│              - Output: reconstruction MSE (anomaly_score)       │
│              - Confidence: from SNR + correlation               │
│                                                                  │
│ 4. [20-21ms] CREATE SPECTRALFRAME:  ← INTEGRATION POINT #1    │
│              - Timestamp: now (microseconds)                    │
│              - FFT magnitude: [128] mel-scale bins              │
│              - Bispectrum: [64] phase coupling                  │
│              - ITD/ILD: [4] interaural differences              │
│              - Beamformer: [3] fixed azimuths                   │
│              - Mamba anomaly score: reconstruction MSE          │
│              - Confidence: SNR-based 0.0-1.0                    │
│                                                                  │
│ 5. [21-22ms] EVALUATE ANOMALY GATE:  ← INTEGRATION POINT #2   │
│              - Read gate config (threshold, min_confidence)     │
│              - Check confidence gate (threshold 0.5)            │
│              - Check anomaly score gate (threshold 1.0)         │
│              - Generate decision with reason                    │
│              - Latency: < 1ms (generation-critical)             │
│                                                                  │
│ 6. [22-25ms] CONDITIONAL ENQUEUE:   ← INTEGRATION POINT #3    │
│              - If gate.forward_to_trainer:                      │
│                  - Create training pair (input, target)         │
│                  - Send to trainer queue via mpsc channel       │
│                  - Increment trainer_pairs_enqueued counter     │
│              - If gate rejected:                                │
│                  - Log reason (low_confidence, low_anomaly)     │
│                  - Update gate_rejection_reason histogram       │
│                                                                  │
│ 7. [25-30ms] FORENSIC LOG:           ← INTEGRATION POINT #4   │
│              - Create ForensicEvent::GateDecision               │
│              - Include timestamp, anomaly_score, confidence     │
│              - Include reason and forward decision              │
│              - Append to @databases/forensic_logs/events.jsonl │
│                                                                  │
│ 8. [30-100ms] SYNC TO UI:            ← INTEGRATION POINT #5   │
│              - Update AppState.anomaly_score (for waterfall)    │
│              - Update AppState.training_pairs_enqueued          │
│              - Update AppState.gate_status (forward/reject)     │
│              - Update AppState.last_gate_reason                 │
│                                                                  │
└────────────────────────────────────────────────────────────────┘
```

**Performance Budget**: 5 additional ms (within 10ms dispatch cycle)

---

## File Ownership & Implementation Map

### src/main.rs (Dispatch Loop, ~400 lines added)

**Location**: After Mamba inference (line ~750 estimated), before synthesis targets

```rust
// ════════════════════════════════════════════════════════════════
// INTEGRATION POINT #1: CREATE SPECTRALFRAME
// ════════════════════════════════════════════════════════════════

let spectral_frame = SpectralFrame::new(
    current_timestamp_micros,
    fft_magnitude_128,        // [f32; 128] mel-scale bins (from FFT module)
    bispectrum_64,            // [f32; 64] phase coupling (from FFT module)
    itd_ild,                  // [f32; 4] interaural differences (from TDOA module)
    beamformer_outputs,       // [f32; 3] fixed azimuths (from beamformer)
    st.mamba_anomaly_score,   // f32 reconstruction MSE (from Mamba inference)
    detection_confidence,     // f32 SNR-based (0.0-1.0)
);

// Validate frame
if !spectral_frame.is_valid() {
    eprintln!("[C.4] Invalid SpectralFrame, skipping: timestamp={}",
              spectral_frame.timestamp_micros);
    continue;  // Skip to next dispatch cycle
}

// ════════════════════════════════════════════════════════════════
// INTEGRATION POINT #2: EVALUATE ANOMALY GATE
// ════════════════════════════════════════════════════════════════

let gate_decision = evaluate_anomaly_gate(&spectral_frame, &st.anomaly_gate_config);

// ════════════════════════════════════════════════════════════════
// INTEGRATION POINT #3: CONDITIONAL ENQUEUE
// ════════════════════════════════════════════════════════════════

if gate_decision.forward_to_trainer {
    // Create training pair: (input_features, target_features)
    let training_pair = TrainingPair {
        input: spectral_frame.fft_magnitude.to_vec(),  // 128-D features
        target: spectral_frame.fft_magnitude.to_vec(), // For autoencoder
        timestamp_micros: spectral_frame.timestamp_micros,
        anomaly_score: spectral_frame.mamba_anomaly_score,
        confidence: gate_decision.confidence,
    };

    // Send to trainer queue (non-blocking)
    match trainer_queue_tx.try_send(training_pair) {
        Ok(_) => {
            st.training_pairs_enqueued += 1;
            eprintln!("[C.4] Training pair enqueued (total: {}, anomaly: {:.2})",
                      st.training_pairs_enqueued,
                      spectral_frame.mamba_anomaly_score);
        }
        Err(e) => {
            eprintln!("[C.4] Trainer queue full, dropping pair: {}", e);
            st.training_pairs_dropped += 1;
        }
    }
} else {
    eprintln!("[C.4] Gate rejected: {}", gate_decision.reason);
    // Update rejection histogram (for UI diagnostics)
    match gate_decision.reason.as_str() {
        "anomaly_score_below_threshold" => st.gate_rejections_low_anomaly += 1,
        "detection_confidence_too_low" => st.gate_rejections_low_confidence += 1,
        _ => st.gate_rejections_other += 1,
    }
}

// ════════════════════════════════════════════════════════════════
// INTEGRATION POINT #4: FORENSIC LOG
// ════════════════════════════════════════════════════════════════

if let Err(e) = forensic_log_sender.send(ForensicEvent::AnomalyGateDecision {
    timestamp_micros: spectral_frame.timestamp_micros,
    anomaly_score: spectral_frame.mamba_anomaly_score,
    confidence: spectral_frame.confidence,
    threshold_used: st.anomaly_gate_config.anomaly_score_threshold,
    forward_to_trainer: gate_decision.forward_to_trainer,
    reason: gate_decision.reason.clone(),
}) {
    eprintln!("[C.4] Forensic log error: {}", e);
}

// ════════════════════════════════════════════════════════════════
// INTEGRATION POINT #5: SYNC TO UI
// ════════════════════════════════════════════════════════════════

st.anomaly_score = spectral_frame.mamba_anomaly_score;
st.gate_status = if gate_decision.forward_to_trainer {
    "FORWARD"
} else {
    "REJECTED"
};
st.last_gate_reason = gate_decision.reason.clone();
st.detection_confidence = spectral_frame.confidence;
```

### src/state.rs (AppState Extensions, ~50 lines)

```rust
pub struct AppState {
    // ... existing fields ...

    // INTEGRATION POINT #5: UI Sync State
    pub anomaly_score: f32,                    // Last computed anomaly score
    pub detection_confidence: f32,             // Last detection confidence
    pub gate_status: String,                   // "FORWARD" or "REJECTED"
    pub last_gate_reason: String,              // "anomaly_score_below_threshold", etc.

    // Training queue diagnostics
    pub training_pairs_enqueued: u64,          // Total pairs sent to trainer
    pub training_pairs_dropped: u64,           // Total pairs dropped (queue full)
    pub gate_rejections_low_anomaly: u64,      // Rejections due to low anomaly score
    pub gate_rejections_low_confidence: u64,   // Rejections due to low confidence
    pub gate_rejections_other: u64,            // Other rejection reasons

    // Gate configuration
    pub anomaly_gate_config: AnomalyGateConfig,
}
```

### src/forensic_log.rs (New Event Variant, ~30 lines)

```rust
pub enum ForensicEvent {
    // ... existing variants ...

    AnomalyGateDecision {
        timestamp_micros: u64,
        anomaly_score: f32,
        confidence: f32,
        threshold_used: f32,
        forward_to_trainer: bool,
        reason: String,
    },
}

// Serialization
impl serde::Serialize for ForensicEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ForensicEvent::AnomalyGateDecision {
                timestamp_micros,
                anomaly_score,
                confidence,
                threshold_used,
                forward_to_trainer,
                reason,
            } => {
                // Serialize to JSONL
                serde_json::json!({
                    "event_type": "anomaly_gate_decision",
                    "timestamp_micros": timestamp_micros,
                    "anomaly_score": anomaly_score,
                    "confidence": confidence,
                    "threshold_used": threshold_used,
                    "forward_to_trainer": forward_to_trainer,
                    "reason": reason,
                }).serialize(serializer)
            }
            // ... other variants ...
        }
    }
}
```

### Feature Extraction Dependencies (Must Exist Before Integration)

**Mel-Scale Binning** (src/audio.rs):
```rust
pub fn fft_to_mel_scale(fft_512: &[f32; 512]) -> [f32; 128] {
    // Convert 512-bin FFT to 128 mel-scale bins
    // Linearly space in log frequency (perceptually uniform)
    // Sum magnitude across each mel band
    [0.0; 128]  // TODO: implement via mel filterbank
}
```

**Bispectrum Extraction** (src/audio.rs):
```rust
pub fn compute_bispectrum(fft_512: &[f32; 512]) -> [f32; 64] {
    // Extract top 64 bispectrum components by energy
    // Detects non-linear phase relationships (RF modulation signature)
    // Sorted by magnitude (highest energy first)
    [0.0; 64]  // TODO: implement via frequency triple products
}
```

**Beamformer Outputs** (src/tdoa.rs):
```rust
pub fn compute_beamformer_outputs(
    time_lags: &[f32; 3],    // TDOA estimates from mic pairs
    frequencies: &[f32],      // Frequencies to steer
) -> [f32; 3] {
    // Steer to fixed azimuths: -45°, 0°, +45°
    // Return energy-weighted outputs per azimuth
    [0.0; 3]  // TODO: implement via delay-and-sum beamformer
}
```

**Detection Confidence** (src/dispatch_thread.rs):
```rust
pub fn compute_confidence(
    snr_db: f32,
    correlation_quality: f32,
) -> f32 {
    // Combine SNR + correlation into [0.0, 1.0] confidence
    // Example: (snr_db + 40) / 60 clamped to [0.0, 1.0]
    0.9  // TODO: implement confidence metric
}
```

---

## Real-Time Feedback to UI (Slint Bindings)

### UI Properties to Wire

```slint
// ui/app.slint - ADD to oscilloscope/main view

export global AnomalyGateStatus {
    in-out property <float> anomaly-score;        // [0.0, ∞)
    in-out property <float> detection-confidence; // [0.0, 1.0]
    in-out property <string> gate-status;         // "FORWARD" or "REJECTED"
    in-out property <string> last-gate-reason;    // "anomaly_score_below_threshold", etc.
    in-out property <int> training-pairs-enqueued; // Total count
    in-out property <int> training-pairs-dropped;  // Queue full count
    in-out property <int> gate-rejections-low-anomaly;
    in-out property <int> gate-rejections-low-confidence;
}

// In main oscilloscope area (add status panel)
HorizontalLayout {
    Text { text: "Gate: {AnomalyGateStatus.gate-status}"; }
    Text { text: "Anomaly: {AnomalyGateStatus.anomaly-score}";
           color: AnomalyGateStatus.anomaly-score > 1.0 ? #f00 : #0f0; }
    Text { text: "Confidence: {AnomalyGateStatus.detection-confidence}"; }
    Text { text: "Reason: {AnomalyGateStatus.last-gate-reason}"; }
}

// Diagnostics panel
VerticalLayout {
    Text { text: "Training Queue: {AnomalyGateStatus.training-pairs-enqueued}"; }
    Text { text: "Dropped: {AnomalyGateStatus.training-pairs-dropped}"; }
    Text { text: "Rejected (low anomaly): {AnomalyGateStatus.gate-rejections-low-anomaly}"; }
    Text { text: "Rejected (low conf): {AnomalyGateStatus.gate-rejections-low-confidence}"; }
}
```

### Rust Sync Loop (in src/main.rs UI timer, every 50ms):

```rust
let state_read = state.lock().await;

ui_gate.set_anomaly_score(state_read.anomaly_score);
ui_gate.set_detection_confidence(state_read.detection_confidence);
ui_gate.set_gate_status(state_read.gate_status.clone());
ui_gate.set_last_gate_reason(state_read.last_gate_reason.clone());
ui_gate.set_training_pairs_enqueued(state_read.training_pairs_enqueued as i32);
ui_gate.set_training_pairs_dropped(state_read.training_pairs_dropped as i32);
ui_gate.set_gate_rejections_low_anomaly(state_read.gate_rejections_low_anomaly as i32);
ui_gate.set_gate_rejections_low_confidence(state_read.gate_rejections_low_confidence as i32);
```

---

## Threshold Tuning & Configuration

### Default Configuration

```rust
// src/state.rs or config.rs
pub const ANOMALY_GATE_CONFIG_DEFAULT: AnomalyGateConfig = AnomalyGateConfig {
    anomaly_score_threshold: 1.0,     // Tune based on Mamba MSE distribution
    min_confidence: 0.5,               // Require 50% confidence minimum
    force_forward: false,              // Normal gating (not debug mode)
};
```

### Tuning Process

1. **Collect baseline**: Run Mamba on 1 hour of audio
   - Log all anomaly scores to a CSV: `timestamp, anomaly_score`
   - Compute: mean, std dev, min, max

2. **Analyze distribution**:
   - Normal audio: mean ±1 std dev (e.g., 0.3 ± 0.2)
   - Attack signals: mean ±1 std dev (e.g., 2.5 ± 0.8)
   - **Sweet spot threshold**: 1.0 (separates normal from anomalous with 90%+ precision)

3. **Live adjustment**:
   - **Too low** (e.g., 0.1): False positives, trainer overwhelmed
   - **Too high** (e.g., 5.0): Misses real anomalies, training data sparse
   - **Optimal**: 1.0-2.0 (user-adjustable via UI if needed)

---

## Generation Protection Constraints

### ✅ DO

- **Threshold-based gating**: Simple anomaly_score > threshold check
- **Non-blocking evaluation**: Gate must complete in < 1ms
- **Tunable threshold**: Not hardcoded, stored in AnomalyGateConfig
- **Confidence weighting**: Both anomaly score AND confidence matter
- **Forensic logging**: Every gate decision logged for audit trail
- **Real-time feedback**: UI shows gate decisions and queue status

### ❌ DON'T

- **Complex ML in gate**: No additional neural networks, only threshold logic
- **Filtering/smoothing anomaly scores**: Let trainer learn temporal patterns
- **Blocking I/O in gate**: No synchronous file I/O, no locks that pause FFT
- **Hardcoding threshold**: Should be AnomalyGateConfig field
- **Gating on multiple criteria**: Keep simple: anomaly_score + confidence only
- **Silent failures**: Log every rejection reason for debugging

---

## Pre-Commit Hook Validation

```bash
#!/bin/bash
# .git/hooks/pre-commit (add to existing)

# ✓ Gate evaluation non-blocking (< 1ms)
if grep -q "evaluate_anomaly_gate" src/main.rs && ! grep -q "lock\|sleep\|io" src/ml/anomaly_gate.rs; then
    echo "✓ Gate evaluation is non-blocking"
else
    echo "⚠ Check for blocking operations in anomaly_gate.rs"
fi

# ✓ Threshold is configurable (not hardcoded)
if grep -q "anomaly_gate_config.anomaly_score_threshold" src/main.rs; then
    echo "✓ Threshold is configurable"
else
    echo "❌ Threshold must use AnomalyGateConfig, not hardcoded value"
    exit 1
fi

# ✓ Gate decision logged to forensic_log
if grep -q "ForensicEvent::AnomalyGateDecision" src/forensic_log.rs; then
    echo "✓ Gate decisions logged to forensic log"
else
    echo "⚠ Consider logging gate decisions for audit trail"
fi

# ✓ SpectralFrame validation called before gating
if grep -q "if !spectral_frame.is_valid()" src/main.rs; then
    echo "✓ SpectralFrame validated before gating"
else
    echo "❌ Must validate SpectralFrame before gating (is_valid() check)"
    exit 1
fi

# ✓ Training pair enqueue is try_send (non-blocking)
if grep -q "trainer_queue_tx.try_send\|trainer_queue_tx.send_async" src/main.rs; then
    echo "✓ Training pair enqueue is non-blocking"
else
    echo "⚠ Use try_send() or send_async() for non-blocking queue"
fi

echo "✓ Track CC integration validation passed"
exit 0
```

---

## Implementation Checklist (for Jules)

### Phase 1: Feature Extraction Verification (10 min)
- [ ] Verify FFT module produces 512-bin magnitude spectrum
- [ ] Verify FFT-to-mel-scale conversion exists (512 → 128 bins)
- [ ] Verify bispectrum computation exists (top 64 by energy)
- [ ] Verify TDOA module computes ITD/ILD (4 values)
- [ ] Verify beamformer outputs 3 fixed azimuth energies
- [ ] Verify Mamba inference produces anomaly_score (reconstruction MSE)
- [ ] Verify SNR + correlation confidence metric exists
- [ ] Tests: All feature extraction produces expected shapes + finite values

### Phase 2: SpectralFrame Creation (10 min)
- [ ] Import SpectralFrame from src/ml/spectral_frame.rs
- [ ] Locate dispatch loop (after Mamba inference, line ~750)
- [ ] Collect all 7 inputs: timestamp, fft_mag, bispectrum, itd_ild, beamformer, anomaly_score, confidence
- [ ] Create SpectralFrame via new()
- [ ] Call is_valid() and skip if invalid
- [ ] Tests: SpectralFrame creation, validation, NaN handling

### Phase 3: Gate Evaluation Integration (15 min)
- [ ] Import AnomalyGateDecision, AnomalyGateConfig, evaluate_anomaly_gate from src/ml/anomaly_gate.rs
- [ ] Initialize AnomalyGateConfig in AppState (default threshold 1.0, min_confidence 0.5)
- [ ] Call evaluate_anomaly_gate() after SpectralFrame creation
- [ ] Verify latency < 1ms (measure with std::time::Instant)
- [ ] Tests: Gate logic (forward/reject), threshold crossing, confidence gating

### Phase 4: Trainer Queue Wiring (15 min)
- [ ] Create TrainingPair struct (input, target, metadata)
- [ ] Create mpsc channel for trainer queue (capacity 256 pairs)
- [ ] Wire gate_decision.forward_to_trainer → try_send() to queue
- [ ] Update st.training_pairs_enqueued counter
- [ ] Handle Err → update st.training_pairs_dropped counter
- [ ] Tests: Queue capacity, non-blocking send, error handling

### Phase 5: Forensic Logging (10 min)
- [ ] Add ForensicEvent::AnomalyGateDecision variant
- [ ] Create forensic_log_sender channel
- [ ] Send gate decision after evaluation (every cycle)
- [ ] Include: timestamp, anomaly_score, confidence, threshold, forward decision, reason
- [ ] Tests: Logging doesn't block, events serializable to JSON

### Phase 6: UI State Sync (10 min)
- [ ] Extend AppState with anomaly_score, detection_confidence, gate_status, last_gate_reason
- [ ] Extend AppState with queue diagnostics (enqueued, dropped, rejection counts)
- [ ] Sync loop updates Slint globals every 50ms (in UI timer callback)
- [ ] Tests: State updates propagate to UI, no null/NaN values

### Phase 7: Slint UI Components (10 min)
- [ ] Add AnomalyGateStatus global to Slint
- [ ] Add gate status display (FORWARD/REJECTED, anomaly score, confidence)
- [ ] Add diagnostics panel (queue counts, rejection reasons)
- [ ] Wire to Rust sync loop via set_*() calls
- [ ] Tests: UI displays correct values, color-codes thresholds

### Phase 8: Integration Testing (15 min)
- [ ] Cargo build → 0 errors
- [ ] Cargo run → dispatch loop executes, no panics
- [ ] Observe gate decisions in logs ([C.4] messages)
- [ ] Observe training pairs enqueued counter incrementing
- [ ] Verify UI shows gate status and anomaly scores in real-time
- [ ] Adjust threshold via config, observe different gate decisions
- [ ] Verify forensic log contains AnomalyGateDecision events

---

## Total Duration

| Task | Time |
|------|------|
| Phase 1: Feature extraction verification | 10 min |
| Phase 2: SpectralFrame creation | 10 min |
| Phase 3: Gate evaluation integration | 15 min |
| Phase 4: Trainer queue wiring | 15 min |
| Phase 5: Forensic logging | 10 min |
| Phase 6: UI state sync | 10 min |
| Phase 7: Slint UI components | 10 min |
| Phase 8: Integration testing | 15 min |
| **Total** | **95 min** |

*Estimated 60-90 min with concurrent feature extraction work*

---

## Verification & Success Criteria

✅ **SpectralFrame fully populated**:
- Every dispatch cycle produces valid SpectralFrame with all 7 inputs
- All fields finite (no NaN, Inf)
- Confidence in [0.0, 1.0]

✅ **Gate evaluation non-blocking**:
- < 1ms latency (generation-critical)
- Threshold-based logic only (no additional ML)
- Forward and rejection decisions correct

✅ **Training queue receives pairs**:
- training_pairs_enqueued counter increments
- Queue capacity not exceeded (no drops if trainer keeps pace)
- Pairs contain correct features and metadata

✅ **Forensic logging complete**:
- Every gate decision logged to @databases/forensic_logs/events.jsonl
- JSONL format parseable, timestamps microsecond-precision
- Reasons logged (useful for debugging false negatives)

✅ **UI displays real-time feedback**:
- Anomaly score updates every ~100ms
- Gate status (FORWARD/REJECTED) visible
- Queue diagnostics panel shows enqueued/dropped/rejected counts
- Threshold adjustment immediately affects gating

✅ **No regressions**:
- Dispatch loop still 10ms per cycle (or less)
- Memory usage unchanged
- CPU usage minimal (threshold logic negligible)

---

## Integration with Existing Tracks

| Track | Integration |
|-------|-------------|
| Track A | Dispatch loop reads features, produces SpectralFrame |
| Track B | UI displays gate status and queue diagnostics |
| **Track C** | **This addendum integrates C** |
| Track C.2 | Must produce all 7 feature inputs to SpectralFrame |
| Track C.3 | Optional, but feeds confidence metric |
| Track D | Receives training pairs from queue, trains Mamba |
| Track E | Forensic logging provides audit trail |

**No blockers.** All dependencies well-defined.

---

## Notes for Jules

This addendum connects the interface contracts you implemented (SpectralFrame, AnomalyGateDecision) into the live dispatch loop. The gate evaluation must complete in < 1ms because it runs on every audio cycle—any blocking operation (I/O, locks, complex math) will stall the entire system.

**Key insight**: The gate is intentionally simple (threshold + confidence check) so that complex pattern learning happens in the Mamba trainer downstream, not in the hot path. This separates concerns: gate is defensive (prevent garbage data to trainer), trainer is adaptive (learns what "normal" and "attack" look like).

Forensic logging captures every decision for forensic analysis. When users report false positives or false negatives, you can inspect the logs to understand threshold behavior across your actual traffic.

The UI feedback loop is essential for real-time control. If the gate is rejecting too much (queue not filling), users can adjust the threshold or investigate why confidence is low. This creates a feedback loop for tuning.

---

## Future Enhancements (Post-CC)

- **Adaptive threshold**: Learn threshold from historical data (high-confidence forward rate targets)
- **Per-band gating**: Different thresholds for different frequency bands
- **Temporal filtering**: Reject isolated anomalies, require N consecutive anomalies
- **Gate history**: Track gate decisions over time for pattern analysis

