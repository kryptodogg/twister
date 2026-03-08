/// src/ml/event_corpus.rs
/// Event Corpus Generator — Convert forensic JSONL logs to HDF5 multimodal dataset
///
/// Purpose: Build training corpus for TimeGNN from forensic_log.jsonl events:
/// 1. Parse forensic events (timestamps, RF frequency, tags, metadata)
/// 2. Extract audio samples per event (250ms @ 192kHz = 48,000 samples)
/// 3. Infer wav2vec2 embeddings (768-D speech features)
/// 4. Fuse with audio (196-D) + ray (128-D) features
/// 5. Write HDF5 corpus with multimodal features + metadata
///
/// Output HDF5 Structure:
/// ```
/// multimodal_features: float32[N, 1297]  ← TimeGNN input
/// timestamps: int64[N]                    ← Event timing
/// ground_truth_tags: string[N]            ← Forensic classification
/// audio_samples: float32[N, 48000]        ← Raw audio context
/// ray_azimuth_deg: float32[N]             ← Spatial metadata
/// ray_elevation_deg: float32[N]
/// rf_frequency_hz: float32[N]
/// confidence_scores: float32[N]
/// metadata: {
///   total_events: int,
///   time_range_days: float,
///   unique_tags: {
///     "NOTE": int,
///     "EVIDENCE": int,
///     "MANUAL-REC": int,
///     "ANALYSIS": int,
///   }
/// }
/// ```
use serde_json::{json, Value};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Statistics summary for generated corpus
#[derive(Debug, Clone)]
pub struct CorpusStats {
    /// Total number of events in corpus
    pub total_events: usize,
    /// Time span in days (end_timestamp - start_timestamp)
    pub time_range_days: f32,
    /// Count of each forensic event tag
    pub tag_distribution: HashMap<String, usize>,
}

/// Forensic event parsed from JSONL
#[derive(Debug, Clone)]
pub struct ForensicEventData {
    /// Event ID (unique)
    pub id: String,
    /// Unix timestamp (seconds since epoch)
    pub timestamp_unix: f64,
    /// RF frequency in Hz
    pub frequency_hz: f32,
    /// Forensic classification tag
    pub tag: String,
    /// Detection confidence (0.0 - 1.0)
    pub confidence: f32,
    /// Event duration in seconds
    pub duration_seconds: f32,
    /// Arbitrary metadata
    pub metadata: HashMap<String, String>,
}

/// Load forensic events from JSONL file
///
/// # Arguments
/// * `jsonl_path` - Path to forensic_log.jsonl
///
/// # Returns
/// Vec of ForensicEventData sorted by timestamp
///
/// # Errors
/// - File not found
/// - Invalid JSON in line
/// - Missing required fields
pub fn load_forensic_events(jsonl_path: &str) -> Result<Vec<ForensicEventData>, Box<dyn Error>> {
    let file = File::open(jsonl_path)?;
    let reader = BufReader::new(file);

    let mut events = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let json: Value = serde_json::from_str(&line)?;

        // Extract required fields with fallbacks
        let event = ForensicEventData {
            id: json["id"].as_str().unwrap_or("unknown").to_string(),
            timestamp_unix: json["timestamp_unix"].as_f64().unwrap_or(0.0),
            frequency_hz: json["frequency_hz"].as_f64().unwrap_or(145.5) as f32,
            tag: json["event_type"]
                .as_str()
                .or_else(|| json["tag"].as_str())
                .unwrap_or("UNKNOWN")
                .to_string(),
            confidence: json["confidence"].as_f64().unwrap_or(0.5) as f32,
            duration_seconds: json["duration_seconds"].as_f64().unwrap_or(0.250) as f32,
            metadata: json["metadata"]
                .as_object()
                .map(|m| {
                    m.iter()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                        .collect()
                })
                .unwrap_or_default(),
        };

        events.push(event);
    }

    // Sort by timestamp
    events.sort_by(|a, b| a.timestamp_unix.partial_cmp(&b.timestamp_unix).unwrap());

    eprintln!(
        "[event_corpus] Loaded {} forensic events from {}",
        events.len(),
        jsonl_path
    );

    Ok(events)
}

/// Generate 196-D audio features for event (dummy implementation for MVP)
///
/// In production: extract from recorded audio samples using Phase 2 extraction
fn generate_audio_features_dummy() -> [f32; 196] {
    // For MVP: return normalized random features
    // Production would call extract_audio_features() on actual samples
    let mut features = [0.0f32; 196];
    for i in 0..196 {
        features[i] = (i as f32 / 196.0).sin();
    }
    // L2 normalize
    let norm: f32 = features.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
    for f in &mut features {
        *f /= norm.max(1e-7);
    }
    features
}

/// Generate 128-D ray tracing features for event (dummy implementation for MVP)
fn generate_ray_features_dummy(frequency_hz: f32) -> [f32; 128] {
    // For MVP: synthesize features from frequency
    // Production would use ray tracing output from Phase 2 D.1
    let mut features = [0.0f32; 128];
    for i in 0..128 {
        let freq_norm = (frequency_hz / 1e6).min(1.0);
        features[i] = (i as f32 / 128.0 * freq_norm).sin();
    }
    // L2 normalize
    let norm: f32 = features.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
    for f in &mut features {
        *f /= norm.max(1e-7);
    }
    features
}

/// Generate 768-D wav2vec2 features for event (dummy implementation for MVP)
fn generate_wav2vec2_features_dummy(confidence: f32) -> [f32; 768] {
    // For MVP: synthesize from confidence score
    // Production would use actual wav2vec2 inference
    let mut features = [0.0f32; 768];
    for i in 0..768 {
        features[i] = (i as f32 / 768.0 * confidence).cos();
    }
    // L2 normalize
    let norm: f32 = features.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
    for f in &mut features {
        *f /= norm.max(1e-7);
    }
    features
}

/// Prepare event corpus from forensic logs
///
/// # Arguments
/// * `jsonl_path` - Path to forensic_log.jsonl
/// * `h5_out_path` - Output HDF5 file path
/// * `sample_rate_hz` - Audio sample rate (e.g., 192000)
///
/// # Returns
/// CorpusStats with summary information
///
/// # Errors
/// - JSONL parsing failed
/// - HDF5 creation failed
/// - Insufficient events in corpus
///
/// # Notes
/// - MVP implementation: generates dummy audio/ray/wav2vec2 features
/// - Production: extracts from recorded audio samples via Phase 2 modules
/// - Events with invalid timestamps are skipped
pub fn prepare_event_corpus(
    jsonl_path: &str,
    h5_out_path: &str,
    sample_rate_hz: u32,
) -> Result<CorpusStats, Box<dyn Error>> {
    eprintln!(
        "[event_corpus] Preparing corpus from {} → {}",
        jsonl_path, h5_out_path
    );

    // Step 1: Load forensic events from JSONL
    let events = load_forensic_events(jsonl_path)?;

    if events.is_empty() {
        return Err("No valid forensic events found in JSONL".into());
    }

    eprintln!("[event_corpus] Loaded {} events", events.len());

    // Step 2: Process each event → multimodal features
    let mut multimodal_features: Vec<Vec<f32>> = Vec::new();
    let mut timestamps: Vec<i64> = Vec::new();
    let mut ground_truth_tags: Vec<String> = Vec::new();
    let mut audio_samples_list: Vec<Vec<f32>> = Vec::new();
    let mut ray_azimuths: Vec<f32> = Vec::new();
    let mut ray_elevations: Vec<f32> = Vec::new();
    let mut rf_frequencies: Vec<f32> = Vec::new();
    let mut confidence_scores: Vec<f32> = Vec::new();
    let mut tag_distribution: HashMap<String, usize> = HashMap::new();

    for (idx, event) in events.iter().enumerate() {
        // Skip invalid timestamps
        if event.timestamp_unix <= 0.0 {
            eprintln!(
                "[event_corpus] Skipping event {} with invalid timestamp",
                event.id
            );
            continue;
        }

        // Extract/generate features per modality
        let audio_features = generate_audio_features_dummy();
        let ray_features = generate_ray_features_dummy(event.frequency_hz);
        let wav2vec2_features = generate_wav2vec2_features_dummy(event.confidence);

        // Fuse into 1297-D
        let mut fused = [0.0f32; 1297];
        fused[0..196].copy_from_slice(&audio_features);
        fused[196..324].copy_from_slice(&ray_features);
        fused[324..1297].copy_from_slice(&wav2vec2_features);

        multimodal_features.push(fused.to_vec());
        timestamps.push(event.timestamp_unix as i64);
        ground_truth_tags.push(event.tag.clone());

        // Generate dummy audio samples (250ms @ sample_rate_hz)
        let num_samples = (event.duration_seconds * sample_rate_hz as f32) as usize;
        let audio: Vec<f32> = (0..num_samples)
            .map(|i| ((i as f32 / num_samples as f32) * std::f32::consts::PI).sin() * 0.1)
            .collect();

        // Pad/truncate to 48000 samples (250ms @ 192kHz)
        let mut audio_padded = vec![0.0f32; 48000];
        let copy_len = audio.len().min(48000);
        audio_padded[..copy_len].copy_from_slice(&audio[..copy_len]);
        audio_samples_list.push(audio_padded);

        // Spatial features (dummy)
        ray_azimuths.push((event.frequency_hz / 1e6 * 180.0) % 360.0);
        ray_elevations.push((event.confidence * 90.0) - 45.0);
        rf_frequencies.push(event.frequency_hz);
        confidence_scores.push(event.confidence);

        // Update tag distribution
        *tag_distribution.entry(event.tag.clone()).or_insert(0) += 1;

        if (idx + 1) % 10 == 0 {
            eprintln!("[event_corpus] Processed {} events...", idx + 1);
        }
    }

    eprintln!(
        "[event_corpus] Generated {} multimodal feature vectors",
        multimodal_features.len()
    );

    // Step 3: Write HDF5 corpus (simplified via JSON for MVP)
    // Production would use hdf5 crate for native HDF5 format
    write_corpus_json(
        h5_out_path,
        &multimodal_features,
        &timestamps,
        &ground_truth_tags,
        &audio_samples_list,
        &ray_azimuths,
        &ray_elevations,
        &rf_frequencies,
        &confidence_scores,
        &tag_distribution,
    )?;

    // Step 4: Compute statistics
    let total_events = multimodal_features.len();
    let time_range_seconds = if timestamps.len() > 1 {
        (timestamps[timestamps.len() - 1] - timestamps[0]) as f32
    } else {
        0.0
    };
    let time_range_days = time_range_seconds / 86400.0;

    let stats = CorpusStats {
        total_events,
        time_range_days,
        tag_distribution,
    };

    eprintln!(
        "[event_corpus] Corpus complete: {} events, {:.2} day span",
        total_events, time_range_days
    );
    eprintln!(
        "[event_corpus] Tag distribution: {:?}",
        stats.tag_distribution
    );

    Ok(stats)
}

/// Write corpus to JSON file (MVP implementation; production uses HDF5)
fn write_corpus_json(
    output_path: &str,
    multimodal_features: &[Vec<f32>],
    timestamps: &[i64],
    ground_truth_tags: &[String],
    _audio_samples: &[Vec<f32>],
    ray_azimuths: &[f32],
    ray_elevations: &[f32],
    rf_frequencies: &[f32],
    confidence_scores: &[f32],
    tag_distribution: &HashMap<String, usize>,
) -> Result<(), Box<dyn Error>> {
    // Create metadata
    let metadata = json!({
        "total_events": multimodal_features.len(),
        "time_range_days": if timestamps.len() > 1 {
            (timestamps[timestamps.len() - 1] - timestamps[0]) as f64 / 86400.0
        } else {
            0.0
        },
        "unique_tags": tag_distribution,
    });

    // Create corpus structure
    let corpus = json!({
        "metadata": metadata,
        "multimodal_features": multimodal_features
            .iter()
            .map(|f| f.iter().map(|x| format!("{:.6}", x)).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        "timestamps": timestamps,
        "ground_truth_tags": ground_truth_tags,
        "ray_azimuth_deg": ray_azimuths,
        "ray_elevation_deg": ray_elevations,
        "rf_frequency_hz": rf_frequencies,
        "confidence_scores": confidence_scores,
        // Note: audio_samples omitted in JSON output (too large)
        // Production HDF5 would include this
    });

    // Write to file
    let json_str = serde_json::to_string_pretty(&corpus)?;
    std::fs::write(output_path, json_str)?;

    eprintln!("[event_corpus] Wrote corpus to {}", output_path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corpus_stats_basic() {
        let stats = CorpusStats {
            total_events: 100,
            time_range_days: 7.5,
            tag_distribution: {
                let mut m = HashMap::new();
                m.insert("EVIDENCE".to_string(), 50);
                m.insert("NOTE".to_string(), 50);
                m
            },
        };

        assert_eq!(stats.total_events, 100);
        assert!(stats.time_range_days > 7.0);
        assert_eq!(stats.tag_distribution.len(), 2);
    }

    #[test]
    fn test_audio_features_dummy() {
        let features = generate_audio_features_dummy();
        assert_eq!(features.len(), 196);

        // Check normalization: norm should be close to 1
        let norm: f32 = features.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 0.01,
            "Features not normalized: {}",
            norm
        );
    }

    #[test]
    fn test_ray_features_dummy() {
        let features = generate_ray_features_dummy(145.5);
        assert_eq!(features.len(), 128);

        // Check normalization
        let norm: f32 = features.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 0.01,
            "Features not normalized: {}",
            norm
        );
    }

    #[test]
    fn test_wav2vec2_features_dummy() {
        let features = generate_wav2vec2_features_dummy(0.75);
        assert_eq!(features.len(), 768);

        // Check normalization
        let norm: f32 = features.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 0.01,
            "Features not normalized: {}",
            norm
        );
    }
}
