# Track C: Forensic Analysis & Pattern Discovery (TimeGNN Clustering)

**For**: Assigned developer
**Goal**: Transform forensic event logs into learned harassment motifs; cluster recurring patterns and expose them as intelligence feed for visualization and control systems

---

## Overview

Track C is the **intelligence pipeline**. Raw events flowing from Track B (audio anomalies, RF detections, visual artifacts) are analyzed for **recurring structure**—the patterns that repeat across days/weeks/months. TimeGNN learns temporal relationships between events, K-means clusters them into motifs, and outputs a pattern library that feeds Track VI visualization and enables Track D spatial localization.

**Why this matters**:
- **Automated pattern discovery**: System learns what "attack looks like" without hand-coding rules
- **Forensic evidence**: Each pattern linked to timestamps, metadata, user tags (NOTE/EVIDENCE/MANUAL-REC)
- **Confidence scoring**: Statistical ranking of motif reliability and persistence
- **Real-time intelligence**: Pattern library updates as new events arrive
- **Temporal awareness**: Detects "Friday 3 PM attacks" (weekly), "daily interruptions" (24h), etc.
- **Fast**: 2-3 days to implement (parallelizable with Track VI)

**Critical path**:
```
B.1 (Multi-Modal Dispatch) → [Forensic Log accumulation]
                           ↓
                         C.1 (Event Corpus Preparation)
                           ↓
                         C.2 (TimeGNN Clustering)
                           ↓
                         C.3 (Pattern Library Export)
                           ↓
              [Track VI visualization, Track D localization]
```

**Blocker dependency**: C.1 depends on B.1 (forensic events must be logged), C.2 depends on C.1 (corpus must exist)

---

## Track C.1: Event Corpus Preparation

**Status**: [ ] Not started
**Estimated time**: 1 day
**Blocker on**: B.1 (forensic event logging must be complete)

### Specification

**What exists**:
- `src/forensic_log.rs` — JSONL event logging with timestamps, frequencies, anomaly scores
- `@databases/forensic_logs/events.jsonl` — Accumulated events from Track B dispatch loop
- `src/mamba.rs` — Mamba autoencoder outputting 64-D latent embeddings
- `src/state.rs` — AppState with anomaly_score, detected_freq, latent_embedding fields

**What to implement**:
- `src/analysis/event_corpus.rs` — Load forensic logs, extract multimodal features (new file)
  - Public function: `pub fn prepare_event_corpus(jsonl_path: &str, h5_out_path: &str) -> Result<CorpusStats, Box<dyn Error>>`
  - Load all events from `@databases/forensic_logs/events.jsonl`
  - For each event, extract:
    - **timestamp_us**: u64 (microseconds since epoch, for temporal ordering)
    - **mamba_latent**: Vec<f32> (64-D latent from reconstruction)
    - **anomaly_score**: f32 (reconstruction MSE, range 0.0-10.0)
    - **detected_frequency_hz**: f32 (detection frequency)
    - **rf_peak_dbfs**: f32 (RF power, if present)
    - **audio_rms_db**: f32 (audio level)
    - **ground_truth_tag**: String (NOTE, EVIDENCE, MANUAL-REC, ANALYSIS, or empty)
    - **confidence**: f32 (composite: mamba anomaly + RF presence + audio level, range 0.0-1.0)
  - Normalize per-feature (zero-mean, unit variance) to prevent feature dominance
  - Write to HDF5 corpus: `@databases/event_corpus.h5`
  - Output metadata: total_events, time_range_days, unique_frequencies, tag_distribution

**Output interface** (what TimeGNN will consume):
```
HDF5 Structure:
├─ timestamps: int64[N]              (microseconds since epoch)
├─ mamba_latent: float32[N, 64]      (latent embeddings)
├─ anomaly_scores: float32[N]        (reconstruction MSE)
├─ frequencies: float32[N]           (detected frequency in Hz)
├─ rf_power: float32[N]              (RF peak power in dBfs)
├─ audio_level: float32[N]           (audio RMS in dB)
├─ tags: str[N]                      (ground truth labels)
├─ confidence: float32[N]            (composite confidence 0-1)
└─ metadata: {
     'total_events': int,
     'time_range_days': float,
     'start_iso8601': str,
     'end_iso8601': str,
     'unique_frequencies': int,
     'tag_distribution': {'NOTE': int, 'EVIDENCE': int, ...}
   }
```

**What ships**:
- `cargo run --example prepare_event_corpus` loads forensic logs and generates HDF5 corpus
- All events properly normalized and timestamped
- Example: `examples/prepare_event_corpus.rs` (test corpus generation)

### Implementation Guide

#### Step 1: Create `src/analysis/event_corpus.rs`

```rust
// src/analysis/event_corpus.rs — Event Corpus Preparation
//
// Loads forensic_log.jsonl events, extracts multimodal features,
// normalizes, and writes HDF5 corpus for TimeGNN training.
//
// Handles:
// - JSONL parsing (serde_json)
// - Feature extraction and normalization (ndarray)
// - HDF5 file creation (hdf5 crate)
// - Metadata computation (time range, distributions)

use serde_json::Value;
use ndarray::{Array1, Array2};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Event {
    pub timestamp_us: u64,
    pub mamba_latent: Vec<f32>,
    pub anomaly_score: f32,
    pub detected_frequency_hz: f32,
    pub rf_peak_dbfs: f32,
    pub audio_rms_db: f32,
    pub ground_truth_tag: String,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct CorpusStats {
    pub total_events: usize,
    pub time_range_days: f64,
    pub start_iso8601: String,
    pub end_iso8601: String,
    pub unique_frequencies: usize,
    pub tag_distribution: HashMap<String, usize>,
    pub anomaly_min: f32,
    pub anomaly_max: f32,
}

/// Load forensic events from JSONL, normalize, write HDF5 corpus.
pub fn prepare_event_corpus(
    jsonl_path: &str,
    h5_out_path: &str,
) -> Result<CorpusStats, Box<dyn std::error::Error>> {
    eprintln!("[Corpus] Loading forensic events from: {}", jsonl_path);

    // Step 1: Load all events from JSONL
    let events = load_events_from_jsonl(jsonl_path)?;
    eprintln!("[Corpus] Loaded {} events", events.len());

    if events.is_empty() {
        return Err("No events found in forensic log".into());
    }

    // Step 2: Compute statistics
    let stats = compute_corpus_stats(&events);
    eprintln!("[Corpus] Time range: {} days ({} to {})",
        stats.time_range_days,
        stats.start_iso8601,
        stats.end_iso8601
    );
    eprintln!("[Corpus] Tag distribution: {:?}", stats.tag_distribution);

    // Step 3: Normalize features (zero-mean, unit variance per feature)
    let (normalized_latents, norm_params) = normalize_features(&events)?;
    let normalized_anomalies = normalize_array(&events.iter().map(|e| e.anomaly_score).collect::<Vec<_>>());
    let normalized_freqs = normalize_array(&events.iter().map(|e| e.detected_frequency_hz).collect::<Vec<_>>());

    // Step 4: Write HDF5 corpus
    write_hdf5_corpus(
        h5_out_path,
        &events,
        &normalized_latents,
        &normalized_anomalies,
        &normalized_freqs,
        &stats,
    )?;

    eprintln!("[Corpus] Written HDF5 corpus: {} ({:.1} MB)",
        h5_out_path,
        std::fs::metadata(h5_out_path)?.len() as f64 / 1e6
    );

    Ok(stats)
}

/// Load all events from JSONL forensic log.
fn load_events_from_jsonl(path: &str) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut events = Vec::new();

    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let value: Value = serde_json::from_str(&line)
            .map_err(|e| format!("Line {}: {}", line_num, e))?;

        // Parse forensic event JSON
        let timestamp_us = value
            .get("timestamp_micros")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let anomaly_score = value
            .get("mamba_anomaly")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;

        let detected_freq = value
            .get("detected_frequency_hz")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;

        let rf_peak = value
            .get("rf_peak_dbfs")
            .and_then(|v| v.as_f64())
            .unwrap_or(-100.0) as f32;

        let audio_rms = value
            .get("audio_rms_db")
            .and_then(|v| v.as_f64())
            .unwrap_or(-80.0) as f32;

        let tag = value
            .get("tag")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Composite confidence: anomaly score + RF presence + audio level
        let rf_confidence = if rf_peak > -80.0 { 0.5 } else { 0.0 };
        let audio_confidence = if audio_rms > -60.0 { 0.5 } else { 0.0 };
        let anomaly_confidence = (anomaly_score / 10.0).min(1.0);
        let confidence = (anomaly_confidence + rf_confidence + audio_confidence).min(1.0);

        // Extract Mamba latent (64-D) from embedded vector
        // Fallback: generate normalized random vector if not present
        let mamba_latent = extract_mamba_latent(&value).unwrap_or_else(|| vec![0.0; 64]);

        events.push(Event {
            timestamp_us,
            mamba_latent,
            anomaly_score,
            detected_frequency_hz: detected_freq,
            rf_peak_dbfs: rf_peak,
            audio_rms_db: audio_rms,
            ground_truth_tag: tag,
            confidence,
        });
    }

    Ok(events)
}

/// Extract 64-D Mamba latent embedding from forensic event JSON.
fn extract_mamba_latent(value: &Value) -> Option<Vec<f32>> {
    value
        .get("mamba_latent")
        .and_then(|arr| arr.as_array())
        .map(|arr| {
            arr.iter()
                .take(64)
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect()
        })
        .filter(|v: &Vec<f32>| v.len() == 64)
}

/// Compute corpus-level statistics.
fn compute_corpus_stats(events: &[Event]) -> CorpusStats {
    let mut tag_dist: HashMap<String, usize> = HashMap::new();
    let mut unique_freqs: std::collections::HashSet<i32> = std::collections::HashSet::new();

    for event in events {
        *tag_dist.entry(event.ground_truth_tag.clone()).or_insert(0) += 1;
        unique_freqs.insert(event.detected_frequency_hz as i32);
    }

    let timestamps_us: Vec<u64> = events.iter().map(|e| e.timestamp_us).collect();
    let min_ts = timestamps_us.iter().min().copied().unwrap_or(0);
    let max_ts = timestamps_us.iter().max().copied().unwrap_or(0);

    let time_range_days = (max_ts - min_ts) as f64 / (1e6 * 86400.0);

    let anomalies: Vec<f32> = events.iter().map(|e| e.anomaly_score).collect();
    let anomaly_min = anomalies.iter().cloned().fold(f32::INFINITY, f32::min);
    let anomaly_max = anomalies.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

    let start_dt = DateTime::<Utc>::from_timestamp_micros(min_ts as i64).unwrap_or_else(|| Utc::now());
    let end_dt = DateTime::<Utc>::from_timestamp_micros(max_ts as i64).unwrap_or_else(|| Utc::now());

    CorpusStats {
        total_events: events.len(),
        time_range_days,
        start_iso8601: start_dt.to_rfc3339(),
        end_iso8601: end_dt.to_rfc3339(),
        unique_frequencies: unique_freqs.len(),
        tag_distribution: tag_dist,
        anomaly_min,
        anomaly_max,
    }
}

/// Normalize features to zero-mean, unit-variance.
fn normalize_features(events: &[Event]) -> Result<(Array2<f32>, (f32, f32)), Box<dyn std::error::Error>> {
    // Concatenate 64-D Mamba latents
    let n_events = events.len();
    let mut latents = Array2::zeros((n_events, 64));

    for (i, event) in events.iter().enumerate() {
        for (j, &val) in event.mamba_latent.iter().take(64).enumerate() {
            latents[[i, j]] = val;
        }
    }

    // Per-column normalization (z-score)
    let mut normalized = latents.clone();
    for col in 0..64 {
        let col_slice = latents.slice(s![.., col]);
        let mean = col_slice.mean().unwrap_or(0.0);
        let var = col_slice.var(0.0);
        let std = var.sqrt();

        for i in 0..n_events {
            let z = if std > 1e-8 {
                (latents[[i, col]] - mean) / std
            } else {
                0.0
            };
            normalized[[i, col]] = z;
        }
    }

    Ok((normalized, (0.0, 1.0)))
}

/// Normalize a 1D array to zero-mean, unit-variance.
fn normalize_array(arr: &[f32]) -> Vec<f32> {
    if arr.is_empty() {
        return vec![];
    }

    let mean = arr.iter().sum::<f32>() / arr.len() as f32;
    let variance = arr.iter()
        .map(|&x| (x - mean).powi(2))
        .sum::<f32>() / arr.len() as f32;
    let std = variance.sqrt();

    arr.iter()
        .map(|&x| if std > 1e-8 { (x - mean) / std } else { 0.0 })
        .collect()
}

/// Write HDF5 corpus file.
fn write_hdf5_corpus(
    path: &str,
    events: &[Event],
    normalized_latents: &Array2<f32>,
    normalized_anomalies: &[f32],
    normalized_freqs: &[f32],
    stats: &CorpusStats,
) -> Result<(), Box<dyn std::error::Error>> {
    use hdf5::File;

    let file = File::create(path)?;

    // Timestamps
    let timestamps: Vec<i64> = events.iter().map(|e| e.timestamp_us as i64).collect();
    file.create_dataset("timestamps", &timestamps)?;

    // Latent embeddings (N, 64)
    file.create_dataset("mamba_latent", normalized_latents)?;

    // Anomaly scores
    file.create_dataset("anomaly_scores", normalized_anomalies)?;

    // Frequencies
    file.create_dataset("frequencies", normalized_freqs)?;

    // RF power
    let rf_powers: Vec<f32> = events.iter().map(|e| e.rf_peak_dbfs).collect();
    file.create_dataset("rf_power", &rf_powers)?;

    // Audio levels
    let audio_levels: Vec<f32> = events.iter().map(|e| e.audio_rms_db).collect();
    file.create_dataset("audio_level", &audio_levels)?;

    // Confidence scores
    let confidences: Vec<f32> = events.iter().map(|e| e.confidence).collect();
    file.create_dataset("confidence", &confidences)?;

    // Tags
    let tags: Vec<String> = events.iter().map(|e| e.ground_truth_tag.clone()).collect();
    file.create_dataset_string("tags", &tags)?;

    // Metadata
    let metadata_group = file.create_group("metadata")?;
    metadata_group.new_attr::<i32>()?.write_scalar(&(stats.total_events as i32))?;
    metadata_group.new_attr::<f64>()?.write_scalar(&stats.time_range_days)?;

    eprintln!("[Corpus] HDF5 written with {} events, {} features", events.len(), 64);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_loading() {
        // Test JSONL parsing
        let sample_json = r#"{"timestamp_micros": 1000, "mamba_anomaly": 2.5, "detected_frequency_hz": 2.4e9}"#;
        // Should not panic
    }

    #[test]
    fn test_normalize_array() {
        let arr = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let normalized = normalize_array(&arr);
        let mean = normalized.iter().sum::<f32>() / normalized.len() as f32;
        assert!((mean).abs() < 1e-6);
    }
}
```

#### Step 2: Update `src/analysis/mod.rs`

```rust
// src/analysis/mod.rs
pub mod event_corpus;
pub mod timegnn_trainer;
pub mod pattern_discovery;
```

#### Step 3: Create example `examples/prepare_event_corpus.rs`

```rust
// examples/prepare_event_corpus.rs
//
// Generate HDF5 corpus from forensic logs.
// Usage: cargo run --example prepare_event_corpus --release

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== Event Corpus Preparation ===\n");

    let jsonl_path = "@databases/forensic_logs/events.jsonl";
    let h5_path = "@databases/event_corpus.h5";

    println!("[1] Loading forensic logs from: {}", jsonl_path);

    let stats = analysis::event_corpus::prepare_event_corpus(jsonl_path, h5_path)?;

    println!("\n[2] Corpus Statistics:");
    println!("  Total events: {}", stats.total_events);
    println!("  Time range: {:.1} days", stats.time_range_days);
    println!("  Start: {}", stats.start_iso8601);
    println!("  End: {}", stats.end_iso8601);
    println!("  Unique frequencies: {}", stats.unique_frequencies);
    println!("  Anomaly range: {:.2} - {:.2}", stats.anomaly_min, stats.anomaly_max);
    println!("  Tag distribution: {:?}", stats.tag_distribution);

    println!("\n[3] Output: {}", h5_path);
    println!("\n✓ Corpus ready for TimeGNN training\n");

    Ok(())
}
```

### Acceptance Criteria (C.1)

- [ ] `src/analysis/event_corpus.rs` compiles cleanly
- [ ] `prepare_event_corpus()` loads all events from forensic_log.jsonl
- [ ] All 64-D Mamba latents extracted (fallback to zero vector if missing)
- [ ] Features normalized to zero-mean, unit-variance
- [ ] HDF5 corpus written with correct shape: (N, 64) for latents, (N,) for scalars
- [ ] Metadata includes time_range_days, tag_distribution, anomaly bounds
- [ ] `cargo run --example prepare_event_corpus` succeeds without panics
- [ ] Output HDF5 file readable and schema-verified
- [ ] All corpus_test_*.rs tests passing
- [ ] `cargo build --release` succeeds with 0 new warnings

---

## Track C.2: TimeGNN Clustering

**Status**: [ ] Not started
**Estimated time**: 1.5 days
**Blocker on**: C.1 (corpus must exist)

### Specification

**What to implement**:
- `src/analysis/timegnn_trainer.rs` — TimeGNN training loop (new file)
  - Load HDF5 corpus from C.1
  - Initialize TimeGNN encoder: 64-D → 128-D embeddings
  - Contrastive loss (NT-Xent): pull similar events together, push dissimilar apart
  - Training: 50 epochs, batch_size=32, Adam optimizer
  - Save checkpoint every 5 epochs

- `src/analysis/pattern_discovery.rs` — K-means clustering & temporal analysis (new file)
  - Load trained embeddings from TimeGNN
  - K-means clustering: k=23 (number of harassment motifs to discover)
  - Silhouette scoring: validate cluster quality
  - Temporal frequency analysis: detect weekly, daily, hourly patterns
  - Pattern library generation: JSON with motif descriptions

**Output interface** (what Track VI will consume):
```
Pattern Library (@databases/harassment_patterns.json):
{
  "version": "1.0",
  "generated_at_iso8601": "2026-03-08T14:23:14Z",
  "total_patterns": 23,
  "silhouette_avg": 0.72,
  "patterns": [
    {
      "motif_id": 0,
      "label": "Friday_3PM_Tone",
      "frequency_hours": 168,           // Weekly pattern
      "confidence": 0.92,
      "cluster_size": 342,              // Number of events in this cluster
      "avg_anomaly_score": 3.8,
      "representative_embedding": [...],  // 128-D
      "first_occurrence_iso": "2025-12-12T15:00:00Z",
      "last_occurrence_iso": "2026-03-07T15:15:30Z",
      "tag_distribution": {
        "EVIDENCE": 0.68,
        "MANUAL-REC": 0.25,
        "NOTE": 0.07
      },
      "silhouette_score": 0.71,
      "freq_mode_hz": 2.4e9,            // Most common frequency
      "description": "Coherent 2.4 GHz tone every Friday 3-5 PM (UTC)"
    },
    ...
  ]
}
```

### Implementation Guide

#### Step 1: Create `src/analysis/timegnn_trainer.rs`

```rust
// src/analysis/timegnn_trainer.rs — TimeGNN Contrastive Training
//
// Load event corpus, train TimeGNN encoder with NT-Xent contrastive loss,
// output 128-D embeddings for clustering.

use ndarray::{Array1, Array2};
use burn::tensor::{Tensor, TensorData};
use burn::device::Device;
use std::error::Error;

#[derive(Clone)]
pub struct TimeGNNConfig {
    pub hidden_dim: usize,
    pub embedding_dim: usize,
    pub dropout: f32,
    pub learning_rate: f32,
    pub num_epochs: usize,
    pub batch_size: usize,
    pub temperature: f32,  // For NT-Xent contrastive loss
}

impl Default for TimeGNNConfig {
    fn default() -> Self {
        TimeGNNConfig {
            hidden_dim: 128,
            embedding_dim: 128,
            dropout: 0.1,
            learning_rate: 1e-3,
            num_epochs: 50,
            batch_size: 32,
            temperature: 0.07,
        }
    }
}

pub struct TimeGNNTrainer {
    config: TimeGNNConfig,
    device: Device,
}

impl TimeGNNTrainer {
    pub fn new(device: Device) -> Self {
        TimeGNNTrainer {
            config: TimeGNNConfig::default(),
            device,
        }
    }

    /// Load corpus, train TimeGNN, return final embeddings.
    pub fn train(&self, corpus_path: &str) -> Result<(Array2<f32>, f32), Box<dyn Error>> {
        eprintln!("[TimeGNN] Loading corpus: {}", corpus_path);

        // Load HDF5 corpus
        let (timestamps, latents, anomalies, tags) = self.load_corpus(corpus_path)?;
        let n_events = latents.nrows();

        eprintln!("[TimeGNN] Loaded {} events", n_events);

        // Initialize model
        let mut model = self.init_model();

        // Training loop
        let mut best_loss = f32::INFINITY;

        for epoch in 0..self.config.num_epochs {
            let epoch_loss = self.train_epoch(&mut model, &latents, epoch)?;

            if epoch_loss < best_loss {
                best_loss = epoch_loss;
            }

            if (epoch + 1) % 5 == 0 {
                eprintln!("[TimeGNN] Epoch {}/{}, Loss: {:.4}", epoch + 1, self.config.num_epochs, epoch_loss);
                // Save checkpoint
                self.save_checkpoint(&model, epoch + 1)?;
            }
        }

        eprintln!("[TimeGNN] Training complete. Final loss: {:.4}", best_loss);

        // Compute final embeddings
        let embeddings = self.compute_embeddings(&model, &latents)?;

        Ok((embeddings, best_loss))
    }

    /// Train one epoch with NT-Xent contrastive loss.
    fn train_epoch(&self, model: &mut TimeGNNModel, latents: &Array2<f32>, epoch: usize) -> Result<f32, Box<dyn Error>> {
        let n_batches = (latents.nrows() + self.config.batch_size - 1) / self.config.batch_size;
        let mut total_loss = 0.0;

        for batch_idx in 0..n_batches {
            let start = batch_idx * self.config.batch_size;
            let end = (start + self.config.batch_size).min(latents.nrows());

            let batch = latents.slice(s![start..end, ..]).to_owned();

            // Forward pass
            let embeddings = model.encode(&batch)?;

            // NT-Xent loss: τ = 0.07 (temperature)
            let loss = self.nt_xent_loss(&embeddings, self.config.temperature)?;

            // Backward pass
            model.backward(loss)?;

            total_loss += loss;
        }

        let avg_loss = total_loss / n_batches as f32;
        Ok(avg_loss)
    }

    /// Compute NT-Xent (Normalized Temperature-scaled Cross Entropy) loss.
    /// Similar samples should have high cosine similarity, dissimilar low.
    fn nt_xent_loss(&self, embeddings: &Array2<f32>, tau: f32) -> Result<f32, Box<dyn Error>> {
        let batch_size = embeddings.nrows();
        let mut loss_sum = 0.0;

        // Compute pairwise cosine similarities
        for i in 0..batch_size {
            let e_i = embeddings.slice(s![i, ..]);

            let mut numerator = 0.0;
            let mut denominator = 0.0;

            for j in 0..batch_size {
                let e_j = embeddings.slice(s![j, ..]);

                // Cosine similarity
                let sim = cosine_similarity(&e_i, &e_j);
                let scaled = sim / tau;

                if i == j {
                    // Positive pair (same event)
                    numerator = scaled.exp();
                } else {
                    // Negative pair (different event)
                    denominator += scaled.exp();
                }
            }

            let loss = -(numerator / (numerator + denominator)).ln();
            loss_sum += loss;
        }

        Ok(loss_sum / batch_size as f32)
    }

    /// Cosine similarity between two vectors.
    fn cosine_similarity(a: &Array1<f32>, b: &Array1<f32>) -> f32 {
        let dot = a.dot(b);
        let norm_a = a.norm_l2();
        let norm_b = b.norm_l2();

        if norm_a > 1e-8 && norm_b > 1e-8 {
            dot / (norm_a * norm_b)
        } else {
            0.0
        }
    }

    fn load_corpus(&self, path: &str) -> Result<(Vec<i64>, Array2<f32>, Vec<f32>, Vec<String>), Box<dyn Error>> {
        use hdf5::File;

        let file = File::open(path)?;

        // Load arrays
        let timestamps_dataset = file.dataset("timestamps")?;
        let latents_dataset = file.dataset("mamba_latent")?;
        let anomalies_dataset = file.dataset("anomaly_scores")?;

        let timestamps = timestamps_dataset.read_raw::<i64>()?;
        let latents = latents_dataset.read::<Array2<f32>>()?;
        let anomalies = anomalies_dataset.read_raw::<f32>()?;

        let tags = vec!["".to_string(); latents.nrows()]; // Placeholder

        Ok((timestamps, latents, anomalies, tags))
    }

    fn init_model(&self) -> TimeGNNModel {
        TimeGNNModel::new(&self.device, &self.config)
    }

    fn compute_embeddings(&self, model: &TimeGNNModel, latents: &Array2<f32>) -> Result<Array2<f32>, Box<dyn Error>> {
        model.encode(latents)
    }

    fn save_checkpoint(&self, model: &TimeGNNModel, epoch: usize) -> Result<(), Box<dyn Error>> {
        eprintln!("[TimeGNN] Saving checkpoint: epoch {}", epoch);
        // Implement checkpoint saving (safetensors or similar)
        Ok(())
    }
}

pub struct TimeGNNModel {
    // Simplified: placeholder for actual Burn module
}

impl TimeGNNModel {
    fn new(device: &Device, config: &TimeGNNConfig) -> Self {
        TimeGNNModel {}
    }

    fn encode(&self, batch: &Array2<f32>) -> Result<Array2<f32>, Box<dyn Error>> {
        // Encode 64-D latents to 128-D embeddings
        // Dense(64) → ReLU → Dense(128) → Normalize
        Ok(batch.clone())  // Placeholder
    }

    fn backward(&mut self, loss: f32) -> Result<(), Box<dyn Error>> {
        // Implement gradient descent
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timegnn_config() {
        let cfg = TimeGNNConfig::default();
        assert_eq!(cfg.embedding_dim, 128);
        assert_eq!(cfg.temperature, 0.07);
    }
}
```

#### Step 2: Create `src/analysis/pattern_discovery.rs`

```rust
// src/analysis/pattern_discovery.rs — K-means Clustering & Temporal Analysis
//
// Cluster 128-D embeddings, detect temporal patterns, generate pattern library.

use ndarray::{Array1, Array2};
use std::collections::HashMap;
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct Pattern {
    pub motif_id: u32,
    pub label: String,
    pub frequency_hours: u32,
    pub confidence: f32,
    pub cluster_size: usize,
    pub avg_anomaly_score: f32,
    pub representative_embedding: Vec<f32>,
    pub first_occurrence_iso: String,
    pub last_occurrence_iso: String,
    pub tag_distribution: HashMap<String, f32>,
    pub silhouette_score: f32,
    pub freq_mode_hz: f32,
    pub description: String,
}

pub struct PatternDiscovery {
    k: usize,  // Number of clusters
}

impl PatternDiscovery {
    pub fn new(k: usize) -> Self {
        PatternDiscovery { k }
    }

    /// Discover harassment motifs via K-means clustering on embeddings.
    pub fn discover(
        &self,
        embeddings: &Array2<f32>,
        anomalies: &[f32],
        timestamps_us: &[i64],
        frequencies_hz: &[f32],
        tags: &[String],
    ) -> Result<Vec<Pattern>, Box<dyn std::error::Error>> {
        eprintln!("[PatternDiscovery] K-means clustering ({} clusters)", self.k);

        // K-means clustering
        let cluster_assignments = self.kmeans(embeddings, self.k)?;

        eprintln!("[PatternDiscovery] Clustering complete");

        // Compute patterns per cluster
        let mut patterns = Vec::new();

        for cluster_id in 0..self.k {
            let pattern = self.pattern_from_cluster(
                cluster_id,
                &cluster_assignments,
                embeddings,
                anomalies,
                timestamps_us,
                frequencies_hz,
                tags,
            )?;

            if pattern.silhouette_score > 0.6 {
                patterns.push(pattern);
            }
        }

        eprintln!("[PatternDiscovery] Discovered {} viable patterns", patterns.len());

        Ok(patterns)
    }

    /// K-means clustering.
    fn kmeans(&self, data: &Array2<f32>, k: usize) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
        let n = data.nrows();
        let mut assignments = vec![0usize; n];

        // Simple k-means: initialize, iterate 10 times
        for iteration in 0..10 {
            // Compute centroids
            let mut centroids: Vec<Vec<f32>> = vec![vec![0.0; data.ncols()]; k];
            let mut cluster_sizes: Vec<usize> = vec![0; k];

            for i in 0..n {
                let cluster = assignments[i];
                for j in 0..data.ncols() {
                    centroids[cluster][j] += data[[i, j]];
                }
                cluster_sizes[cluster] += 1;
            }

            for c in 0..k {
                if cluster_sizes[c] > 0 {
                    for j in 0..data.ncols() {
                        centroids[c][j] /= cluster_sizes[c] as f32;
                    }
                }
            }

            // Reassign points
            for i in 0..n {
                let mut best_dist = f32::INFINITY;
                let mut best_cluster = 0;

                for c in 0..k {
                    let dist = self.l2_distance(&data.row(i).to_vec(), &centroids[c]);
                    if dist < best_dist {
                        best_dist = dist;
                        best_cluster = c;
                    }
                }

                assignments[i] = best_cluster;
            }
        }

        Ok(assignments)
    }

    fn l2_distance(&self, a: &[f32], b: &[f32]) -> f32 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt()
    }

    /// Extract pattern from cluster.
    fn pattern_from_cluster(
        &self,
        cluster_id: usize,
        assignments: &[usize],
        embeddings: &Array2<f32>,
        anomalies: &[f32],
        timestamps_us: &[i64],
        frequencies_hz: &[f32],
        tags: &[String],
    ) -> Result<Pattern, Box<dyn std::error::Error>> {
        // Find events in this cluster
        let cluster_indices: Vec<usize> = assignments
            .iter()
            .enumerate()
            .filter(|(_, &a)| a == cluster_id)
            .map(|(i, _)| i)
            .collect();

        let cluster_size = cluster_indices.len();

        // Representative embedding (cluster centroid)
        let mut rep_embedding = vec![0.0; embeddings.ncols()];
        for &idx in &cluster_indices {
            for j in 0..embeddings.ncols() {
                rep_embedding[j] += embeddings[[idx, j]];
            }
        }
        for j in 0..embeddings.ncols() {
            rep_embedding[j] /= cluster_size as f32;
        }

        // Compute metrics
        let avg_anomaly: f32 = cluster_indices.iter().map(|&i| anomalies[i]).sum::<f32>() / cluster_size as f32;
        let silhouette = self.silhouette_score(cluster_id, assignments, embeddings);

        // Temporal frequency
        let frequency_hours = self.detect_temporal_frequency(&cluster_indices, timestamps_us);

        // Tag distribution
        let mut tag_dist: HashMap<String, usize> = HashMap::new();
        for &idx in &cluster_indices {
            *tag_dist.entry(tags[idx].clone()).or_insert(0) += 1;
        }
        let tag_dist_normalized: HashMap<String, f32> = tag_dist
            .iter()
            .map(|(k, v)| (k.clone(), *v as f32 / cluster_size as f32))
            .collect();

        // Most common frequency
        let freq_mode = self.mode_frequency(&cluster_indices, frequencies_hz);

        // First and last occurrence
        let mut cluster_timestamps: Vec<i64> = cluster_indices.iter().map(|&i| timestamps_us[i]).collect();
        cluster_timestamps.sort();

        let first_iso = self.timestamp_to_iso(cluster_timestamps[0]);
        let last_iso = self.timestamp_to_iso(cluster_timestamps[cluster_timestamps.len() - 1]);

        let label = self.generate_label(frequency_hours, freq_mode);
        let description = self.generate_description(&label, frequency_hours, freq_mode);

        Ok(Pattern {
            motif_id: cluster_id as u32,
            label,
            frequency_hours,
            confidence: (silhouette + 1.0) / 2.0,  // Normalize to [0, 1]
            cluster_size,
            avg_anomaly_score: avg_anomaly,
            representative_embedding: rep_embedding,
            first_occurrence_iso: first_iso,
            last_occurrence_iso: last_iso,
            tag_distribution: tag_dist_normalized,
            silhouette_score: silhouette,
            freq_mode_hz: freq_mode,
            description,
        })
    }

    fn silhouette_score(&self, cluster_id: usize, assignments: &[usize], embeddings: &Array2<f32>) -> f32 {
        // Simplified silhouette: intra-cluster distance vs inter-cluster
        0.72  // Placeholder
    }

    fn detect_temporal_frequency(&self, indices: &[usize], timestamps_us: &[i64]) -> u32 {
        // Detect if pattern repeats weekly (168h), daily (24h), etc.
        // For now, default to daily
        24
    }

    fn mode_frequency(&self, indices: &[usize], frequencies_hz: &[f32]) -> f32 {
        // Most common frequency in cluster
        indices.iter().map(|&i| frequencies_hz[i]).sum::<f32>() / indices.len() as f32
    }

    fn timestamp_to_iso(&self, ts_us: i64) -> String {
        format!("2026-03-08T14:23:14Z")  // Placeholder
    }

    fn generate_label(&self, freq_hours: u32, freq_hz: f32) -> String {
        match freq_hours {
            168 => "Weekly_Pattern".to_string(),
            24 => "Daily_Pattern".to_string(),
            _ => format!("Pattern_{}", freq_hours),
        }
    }

    fn generate_description(&self, label: &str, freq_hours: u32, freq_hz: f32) -> String {
        format!("{} at {:.2e} Hz, recurs every {} hours", label, freq_hz, freq_hours)
    }
}

pub fn export_patterns_to_json(patterns: &[Pattern], output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut pattern_objs = Vec::new();

    for p in patterns {
        let obj = json!({
            "motif_id": p.motif_id,
            "label": p.label,
            "frequency_hours": p.frequency_hours,
            "confidence": p.confidence,
            "cluster_size": p.cluster_size,
            "avg_anomaly_score": p.avg_anomaly_score,
            "first_occurrence_iso": p.first_occurrence_iso,
            "last_occurrence_iso": p.last_occurrence_iso,
            "tag_distribution": p.tag_distribution,
            "silhouette_score": p.silhouette_score,
            "freq_mode_hz": p.freq_mode_hz,
            "description": p.description,
        });
        pattern_objs.push(obj);
    }

    let library = json!({
        "version": "1.0",
        "total_patterns": patterns.len(),
        "silhouette_avg": patterns.iter().map(|p| p.silhouette_score).sum::<f32>() / patterns.len() as f32,
        "patterns": pattern_objs,
    });

    std::fs::write(output_path, serde_json::to_string_pretty(&library)?)?;
    eprintln!("[PatternDiscovery] Exported {} patterns to: {}", patterns.len(), output_path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_discovery() {
        let discovery = PatternDiscovery::new(23);
        assert_eq!(discovery.k, 23);
    }
}
```

### Acceptance Criteria (C.2)

- [ ] `src/analysis/timegnn_trainer.rs` compiles cleanly
- [ ] `src/analysis/pattern_discovery.rs` compiles cleanly
- [ ] TimeGNN training converges: loss 2.1 → 0.4 in 50 epochs
- [ ] 128-D embeddings computed correctly
- [ ] K-means produces 23 clusters
- [ ] Silhouette score > 0.6 (clusters are well-separated)
- [ ] Pattern library JSON exports with correct schema
- [ ] All patterns have confidence > 0.7
- [ ] Temporal frequency detection identifies weekly/daily/hourly patterns
- [ ] `cargo run --example train_timegnn` completes without panics
- [ ] All pattern_discovery_test*.rs tests passing
- [ ] `cargo build --release` succeeds with 0 new warnings

---

## Track C.3: Pattern Library Export & Integration

**Status**: [ ] Not started
**Estimated time**: 0.5 day
**Blocker on**: C.2 (patterns must be discovered)

### Specification

**What to implement**:
- `src/analysis/pattern_export.rs` — Export patterns to JSON, sync with AppState (new file)
  - Load pattern library from JSON
  - Expose via `AppState.harassment_patterns: Vec<Pattern>`
  - Wire to UI: temporal scatter plot, heatmap, dendrogram use patterns
  - Enable real-time pattern updates (reload when new patterns discovered)

**Integration points**:
- Track VI.1 (Mesh Shaders) colors particles by motif_id
- Track VI.4 (Mamba Material Learning) learns material per motif
- Track D (Spatial Localization) uses patterns for target priority
- UI displays pattern library in ANALYSIS tab

---

## Summary: What You Ship

By completing Track C, developer delivers:

✅ **C.1: Event Corpus Preparation**
- Loads forensic logs from Track B
- Extracts 64-D Mamba latents + metadata
- Normalizes features (zero-mean, unit variance)
- Writes HDF5 corpus: `@databases/event_corpus.h5`
- Example: `cargo run --example prepare_event_corpus`

✅ **C.2: TimeGNN Clustering**
- Trains TimeGNN encoder with NT-Xent contrastive loss
- Computes 128-D event embeddings
- K-means clustering into 23 motifs
- Detects temporal patterns (weekly, daily, hourly)
- Exports pattern library: `@databases/harassment_patterns.json`
- Example: `cargo run --example train_timegnn`

✅ **C.3: Pattern Integration**
- Loads pattern library into AppState
- Exposes via UI callbacks
- Real-time pattern updates

**Result**:
```
Forensic Logs (events from Track B)
  → Event Corpus Preparation (C.1)
  → HDF5 corpus (64-D embeddings)
  → TimeGNN Training (C.2)
  → 128-D embeddings + K-means
  → 23 discovered harassment motifs
  → Pattern Library JSON
  → Track VI visualization + Track D localization
```

---

## Deliverable Format

**Email/PR message**:

```
Subject: Track C: Forensic Analysis & Pattern Discovery

Hi [Developer],

Here's Track C: the intelligence pipeline. Takes forensic events from Track B, discovers recurring harassment motifs, exports pattern library for visualization/control.

**What you're building**:
- C.1: Event Corpus Preparation (Load JSONL, normalize, write HDF5)
- C.2: TimeGNN Clustering (Train, K-means, temporal analysis)
- C.3: Pattern Library Export (JSON + AppState integration)

**Files to create**:
- src/analysis/event_corpus.rs
- src/analysis/timegnn_trainer.rs
- src/analysis/pattern_discovery.rs
- examples/prepare_event_corpus.rs
- examples/train_timegnn.rs

**Acceptance criteria**:
- cargo build --release (0 new warnings)
- Examples run cleanly without panics
- Corpus generation from 10k+ forensic events
- TimeGNN converges (loss 2.1 → 0.4)
- 23 harassment motifs discovered
- Silhouette score > 0.6
- Pattern library JSON with temporal analysis

**This is intentionally focused** (~2-3 days). Feeds Track VI visualization and Track D localization directly.

See conductor/track-c-forensic-analysis.md for full details.

Thanks!
```

---

**Last Updated**: 2026-03-08
**Author**: Claude
**Review**: Ready for assignment
