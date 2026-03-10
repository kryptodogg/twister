# ADDENDUMS for Tracks B, C, D (Critical Clarifications Before Merge)

---

## Track B Addendum: LFM2.5 Training

### Critical Interface Contract

**Input Data Format** (from Track C):
```rust
pub struct TrainingPair {
    pub timestamp_us: u64,
    pub spectral_features: [f32; 240],      // From C.2
    pub multimodal_features: [f32; 1092],   // Audio + Ray + wav2vec2 (from Phase 2C)
    pub ground_truth_label: String,         // "ATTACK" or "NORMAL"
    pub confidence: f32,
}
```

**Mamba Input Shape**: (batch_size, 240) spectral features
**Mamba Output Shape**: (batch_size, 64) latent embeddings
**Loss Objective**: Reconstruction MSE (autoencoder)

### Key Implementation Notes

1. **Threshold Configuration** (src/main.rs dispatch loop, line ~350):
   - Current: `DETECTION_THRESHOLD = -30.0 dB` (too strict, blocks training data)
   - **FIX BEFORE MERGE**: Lower to `-15.0 dB` to allow more training pairs through anomaly gate
   - Rationale: Mamba needs variety; strict threshold starves trainer of diverse examples

2. **Training Pair Enqueue** (src/training.rs, line ~200):
   - Must receive pairs from Track C anomaly gate
   - Dequeue frequency: Every 2 seconds (30 pairs × 10Hz dispatch rate)
   - **Verify**: Training queue doesn't overflow; max 5000 pairs buffered

3. **Checkpoint Serialization** (src/mamba.rs, line ~370):
   - **CRITICAL**: Save epoch counter AND loss history with weights (Phase 2C requirement)
   - Current behavior: Weights persist but epoch resets to 0 on reload
   - **Must implement** before Track G (Dorothy) can reason about training progress

4. **Gradient Clipping** (src/mamba.rs, line ~230):
   - Add `loss.backward(); optimizer.clip_grad_norm(1.0);` to prevent exploding gradients
   - Current: No gradient clipping → potential NaN loss

### Integration Checkpoint

Before B merges, verify:
```bash
# 1. Training pairs flowing through pipeline
cargo build && cargo run 2>&1 | grep "Training pair enqueued"
# Expected: "Training pair enqueued (total: N)" appearing frequently

# 2. Loss decreasing (not stuck at 0)
# Expected: Epoch 1-100 loss goes from ~2.0 → ~0.5

# 3. Checkpoint saves epoch counter
cargo run 2>&1 | grep "Checkpoint saved"
# Expected: "Checkpoint saved: epoch 100, loss_avg=0.45"
```

---

## Track C Addendum: Audio Processing & Spectral Analysis

### Critical Data Format Contract

**Output Format from C.2** (spectral features):
```rust
pub struct SpectralFrame {
    pub timestamp_us: u64,
    pub mel_spectrum: [f32; 128],        // Mel-scaled magnitude
    pub spectral_centroid: f32,
    pub spectral_flatness: f32,
    pub phase_coherence: f32,            // RF modulation indicator
    pub harmonic_clarity: f32,           // Tonality: 0=noise, 1=pure tone
    // ... other fields (see C.2 spec)
}
```

**Output from C.4 (Anomaly Gate)**:
```rust
pub enum AnomalyGateDecision {
    Skip,        // Background noise, skip
    Forward,     // Likely RF, forward to B trainer
    Priority,    // High-confidence attack, immediate log
}
```

### Critical Integration Points

1. **Per-Device Feature Extraction** (src/spectral_features.rs):
   - Must compute SpectralFrame for ALL 4 devices (C925e, Rear Pink, Rear Blue, RTL-SDR)
   - Current gap: RTL-SDR might output different sample rates (12.288 MHz PDM clock vs 192 kHz audio)
   - **FIX**: Verify RTL-SDR spectral extraction uses correct Nyquist (6.144 MHz, not 96 kHz)

2. **Multichannel Correlation** (C.3):
   - High correlation (> 0.7) across all 4 devices = strong evidence of RF (not environmental)
   - This should **automatically boost confidence** for Track D (spatial localization)
   - Integration: Pass `is_rf_attack` boolean to D.1 elevation estimator

3. **Anomaly Gate Thresholds** (C.4):
   - Default: phase_coherence_min = 0.4, harmonic_clarity_min = 0.6
   - **WARNING**: These are conservative; may miss subtle attacks
   - **Recommendation**: Make thresholds configurable at runtime (via UI or config file)
   - Track G (Dorothy) should allow user to adjust sensitivity

### Integration Checkpoint

Before C merges, verify:
```bash
# 1. Spectral features computed for all 4 devices
cargo run 2>&1 | grep "Spectral frame computed"
# Expected: Output shows mel_spectrum (128 bins), phase_coherence, etc.

# 2. Multichannel correlation detected
cargo run 2>&1 | grep "Correlation"
# Expected: "Bins with high correlation: [256, 512, ...]"

# 3. Anomaly gate decides skip vs forward
cargo run 2>&1 | grep "Anomaly"
# Expected: Mix of "Skip" and "Forward" decisions based on signal
```

---

## Track D Addendum: Spatial Localization & PointMamba 3D Wavefield

### Critical Data Format Contracts

**D.1 Output** (elevation estimation):
```rust
pub struct SpatialEstimate {
    pub azimuth_rad: f32,      // From existing TDOA
    pub elevation_rad: f32,    // From D.1 NEW
    pub confidence: f32,       // Combined azimuth + elevation confidence
    pub timestamp_us: u64,
}
```

**D.2 + D.3 Output** (PointMamba encoder-decoder):
```rust
pub struct Point3D {
    pub azimuth_rad: f32,
    pub elevation_rad: f32,
    pub frequency_hz: f32,
    pub intensity: f32,
    pub timestamp_us: u64,
    pub confidence: f32,
}

pub type PointCloud = Vec<Point3D>;
```

### Critical Integration Points

1. **D.1 Depends on Existing TDOA** (src/tdoa.rs):
   - Azimuth already computed and working
   - D.1 adds elevation using energy ratio method
   - **Must NOT modify existing TDOA code** (file ownership for another track)
   - **D.1 owns only**: src/spatial/elevation_estimator.rs (NEW directory)

2. **D.2 Point Cloud Input Format**:
   - Points must have (azimuth, elevation, frequency, intensity, timestamp, confidence) — 6 floats
   - Frequency should be **log-scaled** for perceptual meaning
   - Example: 1 Hz = log(1)=0, 1 MHz = log(1e6)=13.8, 12 MHz = log(12e6)=16.3
   - **Verify**: Track C feeds log(frequency) not raw frequency

3. **D.4 Time-Scrub Integration with Track VI**:
   - D.4 creates temporal_rewind_state.rs with point cloud snapshot per time window
   - Track VI (Aether Visualization) will CONSUME this point cloud
   - **Data contract**: D.4 must expose `get_points_for_time_window(t_start, t_end) -> PointCloud`
   - **Storage**: Points stored in HDF5 at @databases/point_cloud_history.h5

4. **Mouth-Region Detection** (from Aether Philosophical Foundation):
   - Mouth azimuth: ±10° from forward direction (0 rad ≈ mouth)
   - Mouth elevation: 0° to +20° above horizontal (0 to π/9 rad)
   - When intensity HIGH at mouth region → **this is active targeting**
   - **D must log** this as [EVIDENCE] for forensics

### Critical Dependency: Track I (Pose Estimation)

**D.4 visual design includes skeleton overlay** (from Track I.2):
- Skeleton points from MediaPipe (33-point pose)
- Overlaid on 3D Gaussian splatting of RF point cloud
- This creates the **"see RF field targeting your body"** visualization

**Timing**: D.4 can be implemented WITHOUT I.2 (stub skeleton), but full visualization requires:
- Track I.1 (MediaPipe) → 33-point pose
- Track I.2 (Pose Materials) → skeleton with materials

### Integration Checkpoint

Before D merges, verify:
```bash
# 1. TDOA azimuth still works (don't break existing code)
cargo test tdoa --lib -- --nocapture
# Expected: All existing TDOA tests passing

# 2. D.1 elevation estimation works
cargo test elevation_estimation --lib -- --nocapture
# Expected: Elevation in [-90°, +90°], confidence [0, 1]

# 3. D.2/D.3 PointMamba encoder-decoder pipeline
cargo test point_mamba_integration --lib -- --nocapture
# Expected: (N, 6) → embeddings → (N, 3) reconstruction

# 4. D.4 temporal rewind state management
cargo test temporal_rewind_state --lib -- --nocapture
# Expected: Time-scrub updates point cloud snapshots
```

---

## Critical Blocking Issues (Address Before Merge)

### Issue 1: Frequency Scaling (C.2 + D.2 Dependency)

**Problem**: If Track C feeds raw frequency (Hz) to D.2, PointMamba learns on huge variance:
- 1 Hz (audio tone) vs 2.4 GHz (RF) = 9 orders of magnitude difference
- Network struggles with such extreme scale

**Solution**: **Log-scale frequency** before feeding to D.2:
```rust
// Track C spectral_features.rs:
let freq_log = frequency_hz.log10();  // Now in [0, 10] range
```

**Verification**: Before D.2 training, histogram frequency distribution and verify it's reasonable.

### Issue 2: Time-Series Synchronization (B + C + D Dependency)

**Problem**: Different tracks compute features at different rates:
- Track A: Audio @ 192 kHz (5.2 μs per sample)
- Track C: FFT frames every 100ms
- Track D: TDOA updates every 200ms
- Track B: Mamba batches every 2 seconds

**Solution**: **All timestamps must be monotonically increasing** with microsecond precision:
```rust
// Every module must use:
let timestamp_us = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_micros() as u64;
```

**Verify**: Forensic logs show monotonic timestamps, no gaps > 1 second.

### Issue 3: Anomaly Gate Starvation (C.4 + B Dependency)

**Problem**: If C.4 gate is too strict, no training pairs reach B trainer → loss stays at 0

**Solution**: Make anomaly thresholds **dynamically adjustable**:
- Default: conservative (high threshold, fewer false positives)
- User can lower via UI to get more training data (if needed)
- Track G (Dorothy) should allow tuning

**Verify**: Run with default thresholds → confirm training pairs flowing to B trainer.

---

## Signal for "Ready to Merge"

Once each track passes these checks, it's safe to merge:

✅ **Track B**: Loss decreasing, checkpoint saves epoch counter
✅ **Track C**: Spectral features flowing, gate decisions visible in logs
✅ **Track D**: TDOA not broken, elevation estimates valid, PointMamba compiling

**Do NOT merge if**:
- ❌ Any test suite failing
- ❌ Compilation warnings related to the new code (not baseline warnings)
- ❌ Integration checkpoint steps above failing

---

## Handoff Notes for Jules

**Track B (ML Engineer)**:
- Threshold configuration is your main risk
- Make sure checkpoint saves epoch + loss history
- Watch for gradient explosion (add clipping if needed)

**Track C (Signal Processing Engineer)**:
- FFT is critical; test against known tones
- Make anomaly gate threshold configurable
- Verify multichannel correlation logic

**Track D (Spatial Localization Engineer)**:
- Don't modify existing TDOA.rs (that's another track's code)
- D.1, D.2, D.3 are sequential; D.4 can proceed in parallel
- D.4 needs Track I.2 skeleton eventually (can stub for now)

**All Engineers**:
- Use microsecond timestamps everywhere
- Log decisions frequently (helps debug later)
- Test data formats match contracts above

