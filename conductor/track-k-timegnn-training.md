# Track K: TimeGNN Contrastive Training

**Domain**: ML Pattern Discovery
**Ownership**: ML Engineer (depends on J interface, isolated from others)
**Duration**: 3 hours
**Blocker on**: Track J (CorpusMetadata interface)
**Unblocks**: Track VI (Pattern library for visualization)

---

## Overview

Train TimeGNN on multimodal features (from Track J) to discover **23 recurring harassment motifs**. Use contrastive learning (NT-Xent loss) to cluster similar attacks, detect temporal patterns (weekly cycles, daily bursts), and output a pattern library as JSON for live ANALYSIS tab visualization.

**Generation protection**: Contrastive loss temperature τ must be 0.07 (too high → random clustering, too low → numerical instability). Silhouette score threshold ≥ 0.6 (validates cluster quality). Never skip temporal frequency analysis—it reveals attack periodicity invisible in static clustering.

---

## File Ownership

**K.1 — Exclusive to this track**:
- `src/ml/timegnn_trainer.rs` (400 lines) — Training loop, checkpointing
- `src/analysis/pattern_discovery.rs` (300 lines) — K-means, temporal analysis, confidence scoring
- `src/analysis/pattern_library.rs` (150 lines) — JSON serialization, pattern struct
- `tests/timegnn_training.rs` (350 lines, 12 tests)

**Read-only imports**:
- `src/ml/event_corpus.rs` (Track J interface: load_event_corpus)
- Burn tensor ops, burn-wgpu backend
- hdf5 crate (read corpus)

**No modifications to**:
- `src/main.rs` (no dispatch loop integration)
- `src/state.rs` (pattern library loaded at startup, read-only in dispatch)

---

## Deliverables

### K.1: TimeGNN Contrastive Training (90 min)

**File**: `src/ml/timegnn_trainer.rs`

```rust
pub struct TimeGNNTrainer<B: Backend> {
    device: Device,
    model: TimeGNNEncoder,     // [1092-D] → [128-D] embeddings
    optimizer: Adam<B>,
    checkpoint_dir: String,
}

impl<B: Backend> TimeGNNTrainer<B> {
    pub async fn train(
        corpus_path: &str,
        output_checkpoint: &str,
        epochs: usize,
    ) -> Result<TrainingMetrics, Box<dyn Error>> {
        // Load corpus (Track J interface)
        let (features, timestamps) = load_event_corpus(corpus_path)?;

        let mut trainer = Self::new(Device::cuda(), "checkpoints/".to_string())?;
        let mut metrics = TrainingMetrics::default();

        for epoch in 0..epochs {
            let mut epoch_loss = 0.0;

            // Batch loop (batch_size = 32)
            for batch in features.chunks(32) {
                let embeddings = trainer.model.forward(batch);  // [32, 128]

                // Contrastive loss (NT-Xent, τ = 0.07)
                let loss = Self::nt_xent_loss(&embeddings, 0.07)?;

                // Backprop + update
                trainer.optimizer.step(&loss)?;
                epoch_loss += loss.to_scalar();
            }

            epoch_loss /= (features.len() / 32) as f32;
            metrics.epoch_losses.push(epoch_loss);
            eprintln!("[TimeGNN] Epoch {}/{}, Loss: {:.3}", epoch + 1, epochs, epoch_loss);

            // Checkpoint every 5 epochs
            if (epoch + 1) % 5 == 0 {
                trainer.save_checkpoint(&format!("{}/timegnn_epoch_{}.pt", output_checkpoint, epoch + 1))?;
            }
        }

        trainer.save_checkpoint(&format!("{}/timegnn_final.pt", output_checkpoint))?;
        Ok(metrics)
    }

    /// NT-Xent loss (Normalized Temperature-scaled Cross-Entropy)
    /// Groups similar attacks together; separates different attacks
    fn nt_xent_loss(embeddings: &Tensor<B, 2>, temperature: f32) -> Result<Tensor<B, 1>, Box<dyn Error>> {
        let batch_size = embeddings.dims()[0];

        // Cosine similarity: [batch_size, batch_size]
        let sim_matrix = Self::cosine_similarity(embeddings)?;

        // Scale by temperature τ
        let scaled = sim_matrix / temperature;

        // Contrastive loss: log-softmax trick
        // L = -log( exp(sim_pos/τ) / (exp(sim_pos/τ) + Σ exp(sim_neg/τ)) )
        let loss = Self::contrastive_loss_from_sim(&scaled, batch_size)?;

        Ok(loss)
    }

    fn cosine_similarity(embeddings: &Tensor<B, 2>) -> Result<Tensor<B, 2>, Box<dyn Error>> {
        // Normalized dot product: ||e_i|| * ||e_j|| / (||e_i|| * ||e_j||)
        // Output: [batch_size, batch_size]
    }

    fn contrastive_loss_from_sim(sim: &Tensor<B, 2>, batch_size: usize) -> Result<Tensor<B, 1>, Box<dyn Error>> {
        // Positive: diagonal elements (sim[i,i])
        // Negative: off-diagonal elements
        // Loss = mean(-log(softmax(sim)))
    }
}

pub struct TrainingMetrics {
    pub epoch_losses: Vec<f32>,
    pub convergence_epoch: usize,
    pub final_loss: f32,
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_load_corpus() { /* 100-event test corpus */ }

    #[tokio::test]
    async fn test_nt_xent_loss() { /* contrastive loss computation */ }

    #[tokio::test]
    async fn test_gradient_flow() { /* backprop works */ }

    #[tokio::test]
    async fn test_convergence() { /* loss decreases: 2.1 → 0.34 in 50 epochs */ }

    #[tokio::test]
    async fn test_checkpoint_save_load() { /* state persists */ }

    #[tokio::test]
    async fn test_training_on_10k_events() { /* ~60 min wall-clock time */ }
}
```

**Generation protection**:
- ✅ τ = 0.07 (scientifically validated for contrastive learning)
- ❌ DON'T use τ > 0.5 (destroys clustering quality)
- ❌ DON'T skip backprop (gradient flow is critical)
- ✅ DO checkpoint every 5 epochs (resume training if interrupted)

---

### K.2: Pattern Discovery & Clustering (60 min)

**File**: `src/analysis/pattern_discovery.rs`

```rust
pub struct HarassmentPattern {
    pub motif_id: usize,
    pub label: String,                    // "Friday_3PM_Tone", etc.
    pub frequency_hours: f32,              // Periodicity (24h daily, 168h weekly)
    pub confidence: f32,                   // 0.0-1.0
    pub cluster_size: usize,
    pub representative_embedding: Vec<f32>, // 128-D
    pub first_occurrence_iso: String,
    pub last_occurrence_iso: String,
    pub tag_distribution: HashMap<String, f32>,
    pub silhouette_score: f32,
    pub avg_anomaly_score: f32,
    pub rf_frequency_hz_mode: f32,
}

pub struct PatternLibrary {
    pub total_patterns: usize,
    pub corpus_time_range_days: f32,
    pub silhouette_avg: f32,
    pub patterns: Vec<HarassmentPattern>,
}

pub fn discover_patterns(
    embeddings: &[[f32; 128]],
    events: &[ForensicEvent],
    k: usize,  // k=23 clusters
) -> Result<PatternLibrary, Box<dyn Error>> {
    // K-means clustering on 128-D embeddings
    let (cluster_assignments, centroids) = kmeans(embeddings, k)?;

    // Silhouette scoring
    let silhouette_scores = compute_silhouette_scores(embeddings, &cluster_assignments, &centroids)?;
    let silhouette_avg = silhouette_scores.iter().sum::<f32>() / silhouette_scores.len() as f32;

    // Per-cluster analysis
    let mut patterns = Vec::new();

    for cluster_id in 0..k {
        let cluster_events: Vec<_> = cluster_assignments
            .iter()
            .enumerate()
            .filter(|(_, &c)| c == cluster_id)
            .map(|(i, _)| &events[i])
            .collect();

        if cluster_events.is_empty() {
            continue;
        }

        // Temporal frequency analysis: FFT on cluster event timestamps
        let frequency_hours = detect_temporal_periodicity(&cluster_events)?;

        // Confidence: silhouette + frequency stability
        let confidence = (silhouette_scores[cluster_id] + frequency_stability_score(&cluster_events)?) / 2.0;

        // Tag distribution
        let mut tag_dist = HashMap::new();
        for event in &cluster_events {
            *tag_dist.entry(event.tag.clone()).or_insert(0.0) += 1.0 / cluster_events.len() as f32;
        }

        // RF frequency mode
        let rf_freq_mode = cluster_events.iter()
            .map(|e| e.rf_frequency_hz)
            .collect::<Vec<_>>()
            .windows(3)
            .map(|w| (w[0] + w[1] + w[2]) / 3.0)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);

        let pattern = HarassmentPattern {
            motif_id: cluster_id,
            label: format!("Motif_{}", cluster_id),  // Better labeling in post-processing
            frequency_hours,
            confidence,
            cluster_size: cluster_events.len(),
            representative_embedding: centroids[cluster_id].to_vec(),
            first_occurrence_iso: iso8601_from_micros(cluster_events[0].timestamp_micros),
            last_occurrence_iso: iso8601_from_micros(cluster_events[cluster_events.len() - 1].timestamp_micros),
            tag_distribution: tag_dist,
            silhouette_score: silhouette_scores[cluster_id],
            avg_anomaly_score: cluster_events.iter().map(|e| e.anomaly_score).sum::<f32>() / cluster_events.len() as f32,
            rf_frequency_hz_mode: rf_freq_mode,
        };

        patterns.push(pattern);
    }

    Ok(PatternLibrary {
        total_patterns: patterns.len(),
        corpus_time_range_days: (events.iter().map(|e| e.timestamp_micros).max().unwrap() - events.iter().map(|e| e.timestamp_micros).min().unwrap()) as f32 / 86_400_000_000.0,
        silhouette_avg,
        patterns,
    })
}

fn detect_temporal_periodicity(events: &[&ForensicEvent]) -> Result<f32, Box<dyn Error>> {
    // FFT on event timestamp differences
    // Detect peaks: daily (24h), weekly (168h), etc.
    let timestamps: Vec<_> = events.iter().map(|e| e.timestamp_micros).collect();
    let diffs: Vec<_> = timestamps.windows(2).map(|w| (w[1] - w[0]) / 3_600_000_000.0).collect();  // Hours

    // Simple peak detection in histogram
    let mut histogram = vec![0; 168];  // 0-167 hours
    for &diff in &diffs {
        if diff as usize < 168 {
            histogram[diff as usize] += 1;
        }
    }

    let peak_idx = histogram.iter().position(|&h| h == *histogram.iter().max().unwrap()).unwrap_or(0);
    Ok(peak_idx as f32)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_kmeans_clustering() { /* k=23 clusters */ }

    #[test]
    fn test_silhouette_scoring() { /* avg > 0.6 */ }

    #[test]
    fn test_temporal_periodicity() { /* detects weekly (168h) */ }

    #[test]
    fn test_pattern_discovery_10k_events() { /* 23 motifs discovered */ }
}
```

**Generation protection**:
- ✅ Silhouette score ≥ 0.6 (validates cluster separation)
- ✅ Temporal frequency detection (reveals periodicity)
- ❌ DON'T skip temporal analysis (patterns invisible without frequency domain)
- ✅ DO compute confidence as (silhouette + temporal_stability) / 2

---

### K.3: Pattern Library Export (30 min)

**File**: `src/analysis/pattern_library.rs`

```rust
impl PatternLibrary {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn save(&self, path: &str) -> Result<(), Box<dyn Error>> {
        let json = self.to_json()?;
        std::fs::write(path, json)?;
        eprintln!("[Pattern Library] Saved: {} ({} motifs)", path, self.total_patterns);
        Ok(())
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// JSON output format (example):
// {
//   "total_patterns": 23,
//   "corpus_time_range_days": 97.0,
//   "silhouette_avg": 0.72,
//   "patterns": [
//     {
//       "motif_id": 0,
//       "label": "Friday_3PM_Tone",
//       "frequency_hours": 168.0,
//       "confidence": 0.92,
//       "cluster_size": 342,
//       ...
//     },
//     ...
//   ]
// }
```

---

## Interface Contract (For Track VI)

**Export from K**:
```rust
pub fn load_pattern_library(json_path: &str) -> Result<PatternLibrary, Box<dyn Error>> {
    let json = std::fs::read_to_string(json_path)?;
    PatternLibrary::from_json(&json)
}

// VI.1-VI.3 import this interface (read-only)
// No modifications by VI
```

---

## Local Validation (Pre-Commit Hook)

**File**: `.git/hooks/pre-commit`

```bash
#!/bin/bash
# Check: Contrastive loss temperature τ = 0.07
if ! grep -q "temperature.*0.07\|0\.07" src/ml/timegnn_trainer.rs; then
    echo "❌ ERROR: Contrastive loss temperature must be 0.07 (generation-critical)"
    exit 1
fi

# Check: Silhouette threshold ≥ 0.6
if grep -q "silhouette.*<.*0.6\|< 0\.6" src/analysis/pattern_discovery.rs; then
    echo "⚠️  WARNING: Silhouette threshold below 0.6 (cluster quality at risk)"
fi

# Check: Temporal frequency detection included
if ! grep -q "detect_temporal_periodicity\|temporal" src/analysis/pattern_discovery.rs; then
    echo "❌ ERROR: Temporal frequency analysis missing (patterns invisible without it)"
    exit 1
fi

# Run tests
cargo test timegnn_training --lib -- --nocapture
if [ $? -ne 0 ]; then
    echo "❌ Tests failed"
    exit 1
fi

echo "✅ Track K validation passed"
exit 0
```

---

## Success Criteria

- [ ] TimeGNN loads 10,000-event corpus from Track J
- [ ] Contrastive loss converges: 2.1 → 0.34 dB in ~50 epochs
- [ ] 23 harassment motifs discovered
- [ ] Silhouette score average > 0.6 (validates clustering)
- [ ] Temporal periodicity detected (daily 24h, weekly 168h, etc.)
- [ ] Pattern library JSON exports with all metadata
- [ ] Training completes in < 90 minutes on RX 6700 XT
- [ ] All 12 tests passing (timegnn_training)
- [ ] Pattern library stable (can be loaded/saved repeatedly)
- [ ] Interface contract ready (VI imports without modification)

---

## Notes

**Parallelism**: K depends on J (CorpusMetadata interface) but is otherwise independent. J and K can be developed in parallel once J's interface is stable.

**Next step**: Track VI.1-VI.3 import pattern library for visualization. No blocking.

**Generation protection**: τ=0.07 is scientifically tuned (too high → random, too low → numerical instability). Silhouette ≥ 0.6 is non-negotiable quality gate. Temporal frequency analysis reveals attack periodicity—skipping it loses critical insights.
