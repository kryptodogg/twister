# Track J: wav2vec2-Burn-wgpu Integration

**Domain**: ML Feature Extraction
**Ownership**: ML Engineer (isolated, no blockers)
**Duration**: 90 minutes
**Blocker on**: Nothing (foundation layer)
**Unblocks**: Track K (TimeGNN training)

---

## Overview

Extract frozen facebook/wav2vec2-base-960h embeddings (768-D per 100ms window) from forensic audio events. Integrate Burn-wgpu for GPU acceleration. Output HDF5 corpus ready for Track K (TimeGNN).

**Generation protection**: wav2vec2 must run at full fidelity (no quantization below float32 for intermediate layers; final embeddings can be fp16). Never skip the frozen encoder—it's the acoustic grounding for multimodal fusion.

---

## File Ownership

**J.1 — Exclusive to this track**:
- `src/ml/wav2vec2_loader.rs` (150 lines) — Load, cache, inference
- `src/ml/multimodal_fusion.rs` (180 lines) — Concatenate audio+ray+wav2vec2 → 1092-D
- `src/ml/event_corpus.rs` (200 lines) — Parse forensic logs, generate HDF5
- `tests/wav2vec2_integration.rs` (250 lines, 10 tests)

**Read-only imports**:
- `src/audio.rs` (resampling utilities)
- `src/forensic_log.rs` (event schema)
- Burn tensor ops (standard library)

**No modifications to**:
- `src/main.rs` (no dispatch loop integration yet; Track K handles that)
- `src/state.rs` (no state mutation; isolated feature extraction)

---

## Deliverables

### J.1: Wav2Vec2 Model Loading (40 min)

**File**: `src/ml/wav2vec2_loader.rs`

```rust
pub struct Wav2Vec2Model<B: Backend> {
    device: Device,
    model: PretrainedWav2Vec2,  // From HF: facebook/wav2vec2-base-960h
    cached_embeddings: Arc<Mutex<HashMap<u64, Vec<f32>>>>,  // timestamp → 768-D
}

impl<B: Backend> Wav2Vec2Model<B> {
    /// Load model from HuggingFace (first run downloads 360MB)
    pub async fn load(device: &Device) -> Result<Self, Box<dyn Error>> {
        // Download + cache facebook/wav2vec2-base-960h ONNX
        // Initialize Burn-wgpu backend
        // Return frozen model (no gradient computation)
    }

    /// Inference: 16kHz audio → 768-D embeddings
    /// Input: &[f32] audio samples (16 kHz, mono, 1 second = 16k samples)
    /// Output: Vec<f32> shape [49, 768] → mean-pooled to [768]
    pub fn embed(&self, audio_16khz: &[f32]) -> Result<Vec<f32>, Box<dyn Error>> {
        // Resample to 16kHz if needed (audio.rs utilities)
        // Inference on GPU
        // Output 768-D vector
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_load_model() { /* loads in < 5s, 360MB cache */ }

    #[tokio::test]
    async fn test_embed_1s_audio() { /* 16k samples → 768-D in < 50ms */ }

    #[tokio::test]
    async fn test_deterministic() { /* same audio → same embedding */ }

    #[tokio::test]
    async fn test_batch_inference() { /* 4 samples parallel */ }

    #[tokio::test]
    async fn test_device_transfer() { /* CPU → GPU → CPU */ }

    #[tokio::test]
    async fn test_memory_footprint() { /* < 2GB VRAM */ }
}
```

**Generation protection**:
- ❌ DON'T quantize intermediate layers (frozen encoder must be float32)
- ✅ DO cache embeddings (prevent redundant GPU passes)
- ✅ DO use Burn-wgpu (GPU acceleration non-negotiable)

---

### J.2: Multimodal Fusion (35 min)

**File**: `src/ml/multimodal_fusion.rs`

```rust
pub struct MultimodalFeature {
    pub audio_features: [f32; 196],       // From Track C (spectral extraction)
    pub ray_features: [f32; 128],         // From Track D (TDOA/beamforming)
    pub wav2vec2_embedding: [f32; 768],   // From wav2vec2 (J.1)
    pub fused: [f32; 1092],               // Concatenated + normalized
}

impl MultimodalFeature {
    /// Concatenate [196D audio + 128D ray + 768D wav2vec2] → 1092-D
    /// Per-modality L2 normalization prevents one modality from drowning others
    pub fn fuse(
        audio: &[f32; 196],
        ray: &[f32; 128],
        wav2vec2: &[f32; 768],
    ) -> Self {
        let audio_norm = Self::l2_normalize(audio);
        let ray_norm = Self::l2_normalize(ray);
        let wav2vec2_norm = Self::l2_normalize(wav2vec2);

        let mut fused = [0.0; 1092];
        fused[0..196].copy_from_slice(&audio_norm);
        fused[196..324].copy_from_slice(&ray_norm);
        fused[324..1092].copy_from_slice(&wav2vec2_norm);

        Self { audio_features: *audio, ray_features: *ray, wav2vec2_embedding: *wav2vec2, fused }
    }

    fn l2_normalize(v: &[f32]) -> Vec<f32> {
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        v.iter().map(|x| x / norm.max(1e-9)).collect()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_concatenation() { /* shape [196+128+768=1092] */ }

    #[test]
    fn test_normalization() { /* each modality ≤ 1.0 */ }

    #[test]
    fn test_no_nan_inf() { /* robust to edge cases */ }

    #[test]
    fn test_modality_balance() { /* no modality dominates */ }
}
```

**Generation protection**:
- ✅ Per-modality normalization (prevents audio from drowning ray/embedding)
- ✅ Deterministic (same inputs → same fusion)
- ❌ DON'T skip normalization (breaks neural network input distribution)

---

### J.3: Event Corpus Generation (15 min)

**File**: `src/ml/event_corpus.rs`

```rust
pub struct EventCorpus {
    pub total_events: usize,
    pub time_range_days: f32,
    pub output_path: String,  // HDF5 file path
}

impl EventCorpus {
    /// Load forensic_log.jsonl → Extract audio samples → wav2vec2 inference → HDF5
    pub async fn prepare(
        jsonl_path: &str,
        h5_out_path: &str,
        sample_rate_hz: u32,
    ) -> Result<EventCorpus, Box<dyn Error>> {
        let mut h5_file = hdf5::File::create(h5_out_path)?;
        let mut wav2vec2 = Wav2Vec2Model::load(&Device::cuda()).await?;

        let events = load_forensic_events(jsonl_path)?;
        let mut multimodal_features = Vec::new();
        let mut timestamps = Vec::new();
        let mut tags = Vec::new();

        for event in events {
            // Extract 250ms audio window
            let audio_samples = extract_audio_window(&event, 250, sample_rate_hz)?;

            // Inference: audio → 768-D embedding
            let embedding = wav2vec2.embed(&audio_samples)?;

            // Fuse with audio + ray features from event
            let audio_features = extract_audio_features(&event)?;  // 196-D from C.2
            let ray_features = extract_ray_features(&event)?;     // 128-D from D.1
            let fused = MultimodalFeature::fuse(&audio_features, &ray_features, &embedding).fused;

            multimodal_features.push(fused);
            timestamps.push(event.timestamp_micros);
            tags.push(event.tag.clone());
        }

        // Write HDF5
        h5_file.create_dataset("multimodal_features", &multimodal_features)?;
        h5_file.create_dataset("timestamps", &timestamps)?;
        h5_file.create_dataset("tags", &tags)?;

        Ok(EventCorpus {
            total_events: events.len(),
            time_range_days: (timestamps.iter().max().unwrap() - timestamps.iter().min().unwrap()) as f32 / 86_400_000_000.0,
            output_path: h5_out_path.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_load_100_events() { /* corpus.h5 with 100 events */ }

    #[tokio::test]
    async fn test_shape_validation() { /* multimodal_features [100, 1092] */ }

    #[tokio::test]
    async fn test_timestamp_precision() { /* microseconds, not floats */ }
}
```

---

## Interface Contract (For Track K)

**Export from J**:
```rust
pub struct CorpusMetadata {
    pub total_events: usize,
    pub time_range_days: f32,
    pub feature_dimension: usize,  // 1092
}

pub fn load_event_corpus(h5_path: &str) -> Result<(Vec<[f32; 1092]>, Vec<u64>), Box<dyn Error>> {
    // K imports this; no modifications by K
}
```

Track K imports and uses this interface without modification.

---

## Local Validation (Pre-Commit Hook)

**File**: `.git/hooks/pre-commit`

```bash
#!/bin/bash
# Check: wav2vec2 model never quantized below float32
if grep -r "float16\|fp16" src/ml/wav2vec2_loader.rs 2>/dev/null; then
    echo "❌ ERROR: wav2vec2 intermediate layers must be float32 (generation-critical)"
    exit 1
fi

# Check: Multimodal fusion includes L2 normalization
if ! grep -q "l2_normalize" src/ml/multimodal_fusion.rs; then
    echo "❌ ERROR: Multimodal fusion missing per-modality normalization"
    exit 1
fi

# Run tests
cargo test wav2vec2_integration --lib -- --nocapture
if [ $? -ne 0 ]; then
    echo "❌ Tests failed"
    exit 1
fi

echo "✅ Track J validation passed"
exit 0
```

---

## Success Criteria

- [ ] Wav2Vec2 model loads and runs on RX 6700 XT
- [ ] Inference: < 50ms per 1-second audio sample
- [ ] 768-D embeddings deterministic (same audio → same output)
- [ ] Multimodal fusion: [196D + 128D + 768D] → 1092-D with per-modality normalization
- [ ] Event corpus (10,000 events) generates in < 20 minutes
- [ ] HDF5 corpus shape verified: [N, 1092] for features, [N] for timestamps
- [ ] All 10 tests passing (wav2vec2_integration)
- [ ] Memory footprint < 2GB (model + buffers)
- [ ] Zero NaN/Inf in output
- [ ] Interface contract stable (K can import without modifications)

---

## Notes

**Parallelism**: J is independent. Works in parallel with A-E, I, Particle System, VI.

**Next step**: Track K (TimeGNN) imports CorpusMetadata and trained embeddings. No blocking.

**Generation protection**: wav2vec2 frozen encoder is acoustic grounding. Never quantize intermediate layers. L2 normalization in fusion is non-negotiable (prevents modality imbalance).
