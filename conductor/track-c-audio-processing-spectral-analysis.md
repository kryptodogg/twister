# Track C: Audio Processing & Spectral Analysis Pipeline

**Ownership**: Audio Signal Processing Engineer (exclusive ownership of src/processing/)
**Duration**: 4-6 days
**Integration Point**: Feeds Track B (training) with pre-processed features, feeds Track D (TDOA) with spectral data
**Critical Dependency**: Track A (signal ingestion) for raw audio input
**Files Modified**: src/vbuffer.rs (owns spec computation), src/spectral_features.rs (NEW)

---

## Strategic Context

Track C is the **signal processing middleware** between raw audio input (Track A) and both the Mamba trainer (Track B) and spatial localization (Track D).

**User's Vision**: Transform raw multichannel audio into structured feature representations that reveal RF attack patterns.

---

## C.1: Real-Time FFT & V-Buffer Management (1.5 days)

### Objective
Compute 512-bin FFT on 192 kHz audio, maintain rolling history buffer, expose spectral data to downstream consumers.

### Current Implementation (Existing)
- **Location**: `src/vbuffer.rs`
- **Status**: Partially implemented, needs completion
- **Provides**: `VBuffer` struct with rolling FFT history
- **Missing**: Full integration into dispatch loop, complete test coverage

### Enhancements Needed

**New Methods**:
```rust
pub impl VBuffer {
    /// Get magnitude spectrum at specific time offset
    /// Returns: [f32; 512] log-magnitude bins
    pub fn get_spectrum_at_offset(&self, time_offset_frames: i32) -> [f32; 512] { ... }

    /// Get magnitude at specific bin and time
    pub fn get_bin_magnitude(&self, bin: usize, time_offset: i32) -> f32 { ... }

    /// Compute spectral centroid (weighted frequency center)
    pub fn compute_spectral_centroid(&self) -> f32 { ... }

    /// Detect spectral peaks (local maxima above threshold)
    pub fn find_spectral_peaks(&self, threshold_db: f32) -> Vec<(usize, f32)> { ... }

    /// Compute spectral flatness (entropy metric)
    pub fn compute_spectral_flatness(&self) -> f32 { ... }
}
```

**File**: Enhance existing `src/vbuffer.rs` (~100 lines added)

### Performance Target
- **Latency**: FFT computed every ~100ms
- **Memory**: 1024 frames × 512 bins × 4 bytes = ~2 MB rolling buffer
- **CPU**: ~5% single-core usage (FFT + peak detection)

### Tests (5 tests, add to existing test suite)
- Test 1: FFT correctness (known tone)
- Test 2: Spectral centroid computation
- Test 3: Peak detection above threshold
- Test 4: Spectral flatness bounds
- Test 5: V-buffer rolling (old frames discarded correctly)

---

## C.2: Spectral Feature Extraction (2 days)

### Objective
Convert raw 512-bin FFT into structured feature vectors that reveal RF attack characteristics.

### Feature Set

**Per-FFT-frame features** (~250-300 dimensions):

```
Magnitude Features:
  ├─ FFT bins [0:512] log-magnitude (512 dims)
  │  (but typically collapsed to 64 or 128 via mel-binning for efficiency)
  ├─ Spectral centroid (1 dim) - "where is energy concentrated?"
  ├─ Spectral spread (1 dim) - "how spread out is energy?"
  └─ Spectral flatness (1 dim) - "is it noise or tone?"

Cepstral Features:
  ├─ MFCC (Mel-Frequency Cepstral Coefficients) [0:13] (13 dims)
  └─ Delta-MFCC (rate of change) (13 dims)

Harmonic Features:
  ├─ Fundamental frequency estimate (1 dim)
  ├─ Harmonic energy ratio (1 dim)
  ├─ Spectral sparsity (1 dim) - "how many peaks?"
  └─ Harmonic clarity (1 dim) - confidence in fundamental

Heterodyne Features:
  ├─ Intermodulation products (top-N peaks, e.g., 8 dims)
  ├─ Phase coherence (1 dim) - RF modulation indicator
  └─ Modulation index (1 dim) - how deep is AM/FM?

Total: ~150-200 dimensions (can scale up/down for memory/latency)
```

### Files to Create

**New:**
- `src/spectral_features.rs` (400 lines)
  ```rust
  pub struct SpectralFeatureExtractor {
      mel_fb: MelFilterbank,            // 128 mel-bins
      dct: DCTTransform,                // For MFCC computation
      harmonic_detector: HarmonicDetector,
      heterodyne_analyzer: HeterodynAnalyzer,
  }

  pub struct SpectralFrame {
      pub timestamp_us: u64,
      pub mel_spectrum: [f32; 128],     // Mel-scaled magnitude
      pub mfcc: [f32; 13],
      pub delta_mfcc: [f32; 13],
      pub spectral_centroid: f32,
      pub spectral_spread: f32,
      pub spectral_flatness: f32,
      pub fundamental_hz: f32,
      pub harmonic_clarity: f32,
      pub phase_coherence: f32,         // RF modulation indicator
      pub intermod_peaks: [f32; 8],     // Top 8 heterodyne products
      pub modulation_index: f32,
  }

  impl SpectralFeatureExtractor {
      pub fn new() -> Self { ... }

      /// Extract all spectral features from FFT magnitude spectrum
      pub fn extract(&self, magnitude_spectrum: &[f32; 512]) -> SpectralFrame { ... }

      /// Convert to dense feature vector [f32; 240] for ML
      pub fn to_feature_vector(&self, frame: &SpectralFrame) -> [f32; 240] { ... }
  }
  ```

- `src/spectral_features/mel_filterbank.rs` (120 lines)
  - Mel-scale frequency binning (physics: log-frequency matches human hearing)
  - Maps 512 linear bins → 128 mel bins
  - Pre-computed filterbank matrix

- `src/spectral_features/harmonic_detector.rs` (150 lines)
  - Find fundamental frequency via autocorrelation or peak detection
  - Estimate harmonic clarity (confidence)
  - Identify harmonic series (f0, 2*f0, 3*f0, ...)

- `src/spectral_features/heterodyne_analyzer.rs` (130 lines)
  - Detect RF heterodyne products (frequency mixing artifacts)
  - Compute phase coherence across frames (indicates artificial modulation)
  - Measure modulation index (AM/FM depth)

### Tests (8 tests)
- Test 1: Mel-filterbank correctness (known tone)
- Test 2: MFCC computation
- Test 3: Spectral centroid
- Test 4: Harmonic detection (tone vs. noise)
- Test 5: Phase coherence (sinusoid should have high coherence)
- Test 6: Heterodyne detection (mixed frequencies)
- Test 7: Feature vector shape and bounds
- Test 8: Performance: extract 100 frames in < 10ms

### Integration (src/main.rs dispatch loop)

```rust
// Every FFT frame (~100ms)
let spectrum = vbuffer.get_current_spectrum();
let spectral_frame = spectral_extractor.extract(&spectrum);
let feature_vector = spectral_extractor.to_feature_vector(&spectral_frame);

// Feed to downstream consumers:
// 1. Track B (training): enqueue feature_vector for Mamba
// 2. Track D (TDOA): use phase_coherence for spatial confidence
// 3. Forensic logging: log spectral_frame with timestamp
```

---

## C.3: Multi-Channel Spectral Correlation (1.5 days)

### Objective
Correlate spectral features across 4 audio devices (C925e, Rear Pink, Rear Blue, RTL-SDR) to detect common RF attack signatures.

### Design

```
Device 0 Spectrum:  [S0_bin0, S0_bin1, ..., S0_bin511]
Device 1 Spectrum:  [S1_bin0, S1_bin1, ..., S1_bin511]
Device 2 Spectrum:  [S2_bin0, S2_bin1, ..., S2_bin511]
Device 3 Spectrum:  [S3_bin0, S3_bin1, ..., S3_bin511]

Cross-Correlation: Compute cos_sim(S_i, S_j) for all pairs
  ├─ High correlation at specific frequency bins → common RF source
  ├─ Low correlation → independent noise on each device
  └─ Partial correlation → device-specific processing artifacts

Output: (bin_index, correlation_matrix, confidence)
  Example: "Bin 256 (12 kHz) has 0.89 correlation across all devices"
         → Strong evidence of RF attack (not environmental)
```

### Files to Create

**New:**
- `src/spectral_features/multichannel_correlation.rs` (120 lines)
  ```rust
  pub struct MultiChannelSpectralCorrelator {
      device_count: usize,
  }

  impl MultiChannelSpectralCorrelator {
      /// Compute cross-correlation across all device pairs
      pub fn correlate_devices(
          &self,
          spectra: &[[f32; 512]; 4],  // One spectrum per device
      ) -> SpectralCorrelation {
          // Return correlation matrix + peak bins
      }
  }

  pub struct SpectralCorrelation {
      pub correlation_matrix: [[f32; 4]; 4],  // 4x4 pairwise correlations
      pub high_corr_bins: Vec<(usize, f32)>,  // Bins with > 0.7 correlation
      pub is_rf_attack: bool,                 // Heuristic: > 2 bins correlated?
      pub confidence: f32,
  }
  ```

### Tests (3 tests)
- Test 1: Identical spectra → correlation = 1.0
- Test 2: Uncorrelated noise → correlation ≈ 0.0
- Test 3: Single RF tone across devices → high correlation, attack detected

---

## C.4: Real-Time Anomaly Detection Gate (1 day)

### Objective
Decide whether current frame warrants forwarding to Mamba training or forensic logging (prevents noise from flooding trainer).

### Decision Logic

```
Input: SpectralFrame
  ├─ Spectral flatness > 0.7 (noise)? → SKIP (just background)
  ├─ Phase coherence > 0.4 (RF modulation)? → FORWARD (likely attack)
  ├─ Harmonic clarity > 0.6 (tonal)? → FORWARD (structured signal)
  ├─ MultiChannel correlation > 0.7? → FORWARD (coordinated attack)
  └─ Energy above threshold? → FORWARD

Output: (decision, confidence_score)
```

### File: Add to `src/spectral_features.rs`

```rust
pub struct AnomalyGate {
    thresholds: AnomalyThresholds,
    detection_history: VecDeque<bool>,  // Last 100 frames
}

pub struct AnomalyThresholds {
    pub spectral_flatness_max: f32,     // > this = skip noise
    pub phase_coherence_min: f32,       // > this = forward (RF)
    pub harmonic_clarity_min: f32,      // > this = forward (tonal)
    pub multichannel_corr_min: f32,     // > this = forward (attack)
    pub energy_threshold_db: f32,       // > this = forward
}

impl AnomalyGate {
    pub fn should_forward(&mut self, frame: &SpectralFrame) -> (bool, f32) {
        let mut forward = false;
        let mut confidence = 0.0;

        if frame.phase_coherence > self.thresholds.phase_coherence_min {
            forward = true;
            confidence = frame.phase_coherence;
        }

        if frame.harmonic_clarity > self.thresholds.harmonic_clarity_min {
            forward = true;
            confidence = confidence.max(frame.harmonic_clarity);
        }

        self.detection_history.push_back(forward);
        (forward, confidence)
    }
}
```

### Tests (3 tests)
- Test 1: Background noise → skip
- Test 2: RF tone → forward
- Test 3: Threshold calibration (avoid false positives)

---

## Integration Flow

```
Track A (Signal Ingestion)
  Raw audio (4 devices @ 192 kHz)
       ↓
Track C.1 (FFT & V-Buffer)
  512-bin FFT per device
       ↓
Track C.2 (Spectral Features)
  240-D feature vectors per device
       ↓
Track C.3 (Multi-Channel Correlation)
  Detect coordinated attacks across devices
       ↓
Track C.4 (Anomaly Gate)
  Decide: forward to trainer or discard?
       ↓
   ├─→ Track B (Training): Feature vectors → Mamba encoder
   ├─→ Track D (TDOA): Phase coherence + multichannel corr → spatial confidence
   └─→ Forensic Log: All frames logged with timestamp + features
```

---

## Performance Budget

| Component | Time | Memory | Notes |
|-----------|------|--------|-------|
| FFT (512-bin) | 3 ms | 4 KB | Once per 100ms frame |
| Mel-binning | 1 ms | 128 × 4 B | 128-D output |
| MFCC | 2 ms | 13 × 4 B | Standard speech processing |
| Harmonic detection | 2 ms | 10 KB | Autocorrelation |
| Heterodyne analysis | 1 ms | 8 × 4 B | Top-N peaks |
| Multichannel correlation | 3 ms | 16 × 4 B | 4×4 matrix |
| Anomaly gate | < 1 ms | 100 × 1 B | History buffer |
| **Total per frame** | **~13 ms** | **~50 KB** | Can process in real-time |

**Throughput**: 1 frame every 100ms → 10 frames/second, well within budget

---

## Tests & Verification

```bash
# All Track C tests
cargo test spectral_features --lib -- --nocapture
cargo test vbuffer --lib -- --nocapture

# Integration: audio flows through pipeline
cargo run --release
# Expected logs:
#   [VBuffer] FFT frame 0: 10 bins above -20 dB
#   [Spectral] Feature vector computed: 240-D
#   [Anomaly] Forward to trainer: confidence=0.82
```

---

## Success Criteria

✅ **C.1 Complete**:
- V-buffer rolling history working
- FFT computed correctly (test against known tones)
- All 5 tests passing

✅ **C.2 Complete**:
- 240-D feature vectors extracted from FFT
- Spectral centroid, flatness, MFCC all computed
- All 8 tests passing

✅ **C.3 Complete**:
- Cross-device correlation detected
- RF attacks identified via coordinated peaks
- All 3 tests passing

✅ **C.4 Complete**:
- Anomaly gate filters noise effectively
- False positive rate < 5%
- All 3 tests passing

✅ **Integration Complete**:
- Audio flows A → C → B/D
- ~13ms latency per frame (real-time)
- 0 errors, 0 compilation warnings in Track C code

---

## File Ownership

```
src/vbuffer.rs                              - Enhance existing (C.1)
src/spectral_features.rs                    - Main extraction logic (C.2, C.4)
src/spectral_features/mel_filterbank.rs     - Frequency binning (C.2)
src/spectral_features/harmonic_detector.rs  - Tonality analysis (C.2)
src/spectral_features/heterodyne_analyzer.rs - RF modulation (C.2)
src/spectral_features/multichannel_correlation.rs - Multi-device (C.3)

tests/spectral_features_integration.rs      - All integration tests
```

**No conflicts**: All new files in src/spectral_features/ namespace, v-buffer enhancements are additive (no breaking changes).

