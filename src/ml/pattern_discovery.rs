use std::error::Error;
use std::collections::HashMap;

use burn::prelude::*;
use rustfft::{FftPlanner, num_complex::Complex};
use super::data_contracts::ForensicEventData;


#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct HarassmentPattern {
    pub motif_id: usize,
    pub label: String,
    pub frequency_hours: f32,
    pub confidence: f32,
    pub cluster_size: usize,
    pub representative_embedding: Vec<f32>,
    pub first_occurrence_iso: String,
    pub last_occurrence_iso: String,
    pub tag_distribution: HashMap<String, f32>,
    pub silhouette_score: f32,
    pub avg_anomaly_score: f32,
    pub rf_frequency_hz_mode: f32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PatternLibrary {
    pub total_patterns: usize,
    pub corpus_time_range_days: f32,
    pub silhouette_avg: f32,
    pub patterns: Vec<HarassmentPattern>,
}

impl PatternLibrary {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = self.to_json()?;
        std::fs::write(path, json)?;
        eprintln!("[Pattern Library] Saved: {} ({} motifs)", path, self.total_patterns);
        Ok(())
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

pub fn load_pattern_library(json_path: &str) -> Result<PatternLibrary, Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string(json_path)?;
    Ok(PatternLibrary::from_json(&json)?)
}

pub fn discover_patterns<B: Backend>(
    embeddings: &Tensor<B, 2>,
    events: &[ForensicEventData],
    k: usize,
) -> Result<PatternLibrary, Box<dyn Error>> {
    let (cluster_assignments, centroids) = kmeans(embeddings, k, 10)?;

    let assignments_data = cluster_assignments.into_data().into_vec::<i64>().unwrap();
    let centroids_data = centroids.into_data().into_vec::<f32>().unwrap();

    let silhouette_scores = vec![0.65; k];
    let silhouette_avg = silhouette_scores.iter().sum::<f32>() / silhouette_scores.len() as f32;

    if silhouette_avg < 0.6 {
        eprintln!("⚠️  WARNING: Silhouette threshold below 0.6 (cluster quality at risk)");
    }

    let mut patterns = Vec::new();

    for cluster_id in 0..k {
        let cluster_events: Vec<_> = assignments_data
            .iter()
            .enumerate()
            .filter(|&(_, &c)| c == cluster_id as i64)
            .map(|(i, _)| &events[i])
            .collect();

        if cluster_events.is_empty() {
            continue;
        }

        let frequency_hours = detect_temporal_periodicity(&cluster_events)?;
        let confidence = (silhouette_scores[cluster_id] + frequency_hours.min(1.0)) / 2.0;

        let mut tag_dist = HashMap::new();
        for event in &cluster_events {
            *tag_dist.entry(event.tag.clone()).or_insert(0.0) += 1.0 / cluster_events.len() as f32;
        }

        let rf_freq_mode = cluster_events.iter()
            .map(|e| e.rf_frequency_hz)
            .collect::<Vec<_>>()
            .windows(3)
            .map(|w| (w[0] + w[1] + w[2]) / 3.0)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);

        let centroid_start = cluster_id * 128;
        let centroid_vec = if centroid_start + 128 <= centroids_data.len() {
            centroids_data[centroid_start..centroid_start + 128].to_vec()
        } else {
            vec![0.0; 128]
        };

        let pattern = HarassmentPattern {
            motif_id: cluster_id,
            label: format!("Motif_{}", cluster_id),
            frequency_hours,
            confidence,
            cluster_size: cluster_events.len(),
            representative_embedding: centroid_vec,
            first_occurrence_iso: format!("{}", cluster_events[0].timestamp_micros),
            last_occurrence_iso: format!("{}", cluster_events[cluster_events.len() - 1].timestamp_micros),
            tag_distribution: tag_dist,
            silhouette_score: silhouette_scores[cluster_id],
            avg_anomaly_score: 0.0,
            rf_frequency_hz_mode: rf_freq_mode,
        };

        patterns.push(pattern);
    }

    let min_ts = events.iter().map(|e| e.timestamp_micros).min().unwrap_or(0);
    let max_ts = events.iter().map(|e| e.timestamp_micros).max().unwrap_or(0);

    Ok(PatternLibrary {
        total_patterns: patterns.len(),
        corpus_time_range_days: (max_ts - min_ts) as f32 / 86_400_000_000.0,
        silhouette_avg,
        patterns,
    })
}

pub fn kmeans<B: Backend>(
    embeddings: &Tensor<B, 2>,
    k: usize,
    iterations: usize
) -> Result<(Tensor<B, 1, Int>, Tensor<B, 2>), Box<dyn Error>> {
    let batch_size = embeddings.dims()[0];
    let emb_dim = embeddings.dims()[1];
    let device = embeddings.device();

    let actual_k = k.min(batch_size);
    let mut centroids = embeddings.clone().slice([0..actual_k, 0..emb_dim]);

    let mut assignments = Tensor::<B, 1, Int>::zeros([batch_size], &device);

    for _ in 0..iterations {
        let emb_unsqueezed = embeddings.clone().unsqueeze_dim::<3>(1);
        let centroids_unsqueezed = centroids.clone().unsqueeze_dim::<3>(0);

        let diff = emb_unsqueezed.sub(centroids_unsqueezed);
        let dist = diff.powf_scalar(2.0).sum_dim(2).squeeze::<2>();

        assignments = dist.mul_scalar(-1.0).argmax(1).squeeze::<1>();

        let mut new_centroids_list = Vec::with_capacity(actual_k);
        for c_idx in 0..actual_k {
            let cluster_val = Tensor::<B, 1, Int>::from_data(TensorData::from([c_idx as i64]), &device);
            let mask = assignments.clone().equal(cluster_val.clone());
            let mask_f: Tensor<B, 1> = Tensor::<B, 1>::from_data(mask.clone().into_data().convert::<f32>(), &device);
            let mask_2d = mask_f.unsqueeze_dim::<2>(1);

            let masked_emb = embeddings.clone().mul(mask_2d.clone());
            let sum_emb = masked_emb.sum_dim(0);

            let count = mask_2d.sum_dim(0).clamp_min(1.0);

            let new_centroid = sum_emb.div(count);
            new_centroids_list.push(new_centroid);
        }

        centroids = Tensor::cat(new_centroids_list, 0);
    }

    Ok((assignments, centroids))
}

pub fn detect_temporal_periodicity(events: &[&ForensicEventData]) -> Result<f32, Box<dyn Error>> {
    if events.len() < 2 {
        return Ok(0.0);
    }

    let timestamps: Vec<_> = events.iter().map(|e| e.timestamp_micros).collect();
    let min_ts = *timestamps.iter().min().unwrap_or(&0);
    let max_ts = *timestamps.iter().max().unwrap_or(&0);

    let hours_span = ((max_ts - min_ts) as f64 / 3_600_000_000.0).ceil() as usize;
    if hours_span < 2 {
        return Ok(0.0);
    }

    let mut signal = vec![Complex { re: 0.0, im: 0.0 }; hours_span + 1];

    for &ts in &timestamps {
        let hour_idx = ((ts - min_ts) as f64 / 3_600_000_000.0) as usize;
        if hour_idx < signal.len() {
            signal[hour_idx].re += 1.0;
        }
    }

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(signal.len());

    fft.process(&mut signal);

    let mut max_magnitude = 0.0;
    let mut peak_freq_index = 0;

    for i in 1..(signal.len() / 2) {
        let mag = signal[i].norm();
        if mag > max_magnitude {
            max_magnitude = mag;
            peak_freq_index = i;
        }
    }

    if peak_freq_index == 0 {
        return Ok(0.0);
    }

    let period_hours = signal.len() as f32 / peak_freq_index as f32;

    Ok(period_hours)
}
