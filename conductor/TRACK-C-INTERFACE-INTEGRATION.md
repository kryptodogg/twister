# Track C: SpectralFrame + AnomalyGateDecision Integration Guide

**Status**: Interface contracts created, ready for implementation

---

## Interface Contracts (Ready)

### 1. SpectralFrame (`src/ml/spectral_frame.rs`)
Audio-derived features produced by Track C.2 every ~100ms in the dispatch loop.

```rust
pub struct SpectralFrame {
    pub timestamp_micros: u64,
    pub fft_magnitude: [f32; 128],      // Mel-scale binned FFT
    pub bispectrum: [f32; 64],           // Phase coupling
    pub itd_ild: [f32; 4],               // Interaural differences (TDOA)
    pub beamformer_outputs: [f32; 3],    // 3 fixed azimuths
    pub mamba_anomaly_score: f32,        // Primary gate threshold variable
    pub confidence: f32,                  // Detection confidence (0-1)
}
```

### 2. AnomalyGateDecision (`src/ml/anomaly_gate.rs`)
Non-blocking gate decision (< 1ms) whether to forward to training.

```rust
pub struct AnomalyGateDecision {
    pub forward_to_trainer: bool,
    pub confidence: f32,
    pub reason: String,
    pub anomaly_score_value: f32,
}

pub fn evaluate_anomaly_gate(
    frame: &SpectralFrame,
    config: &AnomalyGateConfig,
) -> AnomalyGateDecision { ... }
```

---

## Integration Point: Dispatch Loop

**Location**: `src/main.rs`, in the main dispatch loop (Tokio task)

**After**: Mamba anomaly score computed, audio → FFT complete
**Before**: Trainer queue enqueue

```rust
// In dispatch loop (after audio → FFT, Mamba inference)

// Step 1: Produce SpectralFrame from audio features
let spectral_frame = SpectralFrame::new(
    current_timestamp_micros,
    fft_magnitude,              // [128] from FFT mel-scale bins
    bispectrum,                 // [64] from bispectrum computation
    itd_ild,                    // [4] from TDOA (existing in dispatch)
    beamformer_outputs,         // [3] from beamformer (existing)
    st.mamba_anomaly_score,     // From Mamba inference (existing)
    detection_confidence,       // From SNR/correlation (compute or existing)
);

// Step 2: Validate frame
if !spectral_frame.is_valid() {
    eprintln!("[C.4] Invalid SpectralFrame, skipping");
    continue;
}

// Step 3: Evaluate gate
let gate_decision = evaluate_anomaly_gate(&spectral_frame, &gate_config);

// Step 4: Conditional enqueue
if gate_decision.forward_to_trainer {
    // Enqueue training pair
    if let Ok(_) = trainer_queue_tx.send(training_pair) {
        eprintln!("[C.4] Training pair enqueued (anomaly: {:.2})",
                  spectral_frame.mamba_anomaly_score);
    }
} else {
    eprintln!("[C.4] Gate rejected: {}", gate_decision.reason);
}
```

---

## Generation Protection (Critical)

### ✅ DO

- **Simple threshold logic** (gate anomaly_score > threshold, done)
- **Non-blocking evaluation** (< 1ms, no I/O or locks)
- **Validate SpectralFrame** (all fields finite, confidence in range)
- **Tunable threshold** (AnomalyGateConfig allows live adjustment)
- **Log gate decisions** (for debugging, understanding false positives/negatives)

### ❌ DON'T

- **Complex ML in the gate** (no additional neural networks in gating)
- **Filter/smooth anomaly scores** (let trainer learn temporal patterns)
- **Block on I/O** (gate must return immediately)
- **Hardcode threshold** (should be configurable in AnomalyGateConfig)
- **Gate based on multiple criteria** (keep it simple: anomaly_score + confidence)

---

## Implementation Checklist (For Jules)

### C.2: Spectral Feature Extraction
- [ ] Compute 128 mel-scale bins from FFT (512 bins → 128)
- [ ] Compute bispectrum top 64 components (phase coupling)
- [ ] Extract ITD/ILD from TDOA (4 values)
- [ ] Package beamformer outputs (3 fixed azimuths)
- [ ] Combine with Mamba anomaly score
- [ ] Combine with detection confidence

### C.3: Multi-Channel Correlation (Optional, for this release)
- [ ] Flag coordinated attacks across multiple frequencies
- [ ] Detect when multiple devices trigger simultaneously
- [ ] Update gate_decision.reason if correlation detected

### C.4: Anomaly Gate Integration
- [ ] Initialize AnomalyGateConfig (threshold tuning)
- [ ] Call evaluate_anomaly_gate() in dispatch loop
- [ ] Enqueue training pairs based on gate decision
- [ ] Log gate decisions for debugging

---

## Threshold Tuning

**Default**: `anomaly_score_threshold = 1.0`

Tune based on Mamba MSE distribution:
- **Too high** (e.g., 5.0): Misses real anomalies, training data sparse
- **Too low** (e.g., 0.1): Too much false positives, trainer overwhelmed
- **Sweet spot**: Approx. 1-2 std dev above mean normal score

**Debugging**:
```rust
// Temporarily lower threshold to see more training data
let mut config = AnomalyGateConfig::default();
config.anomaly_score_threshold = 0.5;
config.force_forward = false;  // Keep gate active

// Or force all data through for baseline:
config.force_forward = true;
```

---

## Performance Target

- **Gate evaluation**: < 1ms (generation-critical)
- **SpectralFrame creation**: < 1ms (minimal overhead)
- **Dispatch loop impact**: Negligible (< 5% of 10ms budget)

---

## Next Steps

1. **Implement C.2**: Spectral feature extraction (combine with FFT, TDOA, Mamba output)
2. **Integrate gate** into dispatch loop (use stub thresholds initially)
3. **Tune threshold** based on real Mamba MSE distribution
4. **Monitor training data flow** (gate decisions logged, training pairs queued)
5. **Optional C.3**: Multi-channel correlation flags if time permits

---

## Notes

- SpectralFrame and AnomalyGateDecision are **read-only** from track C perspective
- They belong in `src/ml/` (not `src/audio/`) because they're ML-pipeline structures
- Tests included for validation (frame integrity, gate logic, serialization)
- Integration with event_corpus.rs for historical data is separate (batch path vs real-time)
