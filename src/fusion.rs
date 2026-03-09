//! Bispectrum + Mamba latent + TDOA fusion (crystal-ball-reconstructor)
//!
//! Fuses multi-sensor data for enhanced signal reconstruction and source localization:
//! - Bispectrum: Heterodyne/cross-coupling detection
//! - Mamba: Anomaly scoring + latent embeddings
//! - TDOA: Beam azimuth + confidence
//! - PDM: Wideband (20kHz-6MHz) enhancement

use crate::detection::DetectionEvent;

#[derive(Clone, Debug)]
pub struct FusionResult {
    /// Fused frequency estimate (Hz)
    pub freq_hz: f32,
    /// Combined confidence [0.0, 1.0]
    pub confidence: f32,
    /// Beam azimuth (degrees) for UI/Log (Standardized from radians)
    pub azimuth_deg: f32,
    /// Mamba anomaly score (dB)
    pub anomaly_db: f32,
    /// Latent embedding distance (L2 norm)
    pub embedding_distance: f32,
    /// Source type: 0=Natural, 1=Synthetic, 2=Jammer, 3=Unknown
    pub source_type: u32,
}

impl Default for FusionResult {
    fn default() -> Self {
        Self {
            freq_hz: 0.0,
            confidence: 0.0,
            azimuth_deg: 0.0,
            anomaly_db: 0.0,
            embedding_distance: 0.0,
            source_type: 3,
        }
    }
}

/// Multi-sensor fusion engine for enhanced detection and localization
#[derive(Clone, Debug)]
pub struct FusionEngine {
    /// Reference baseline latent (0-vector = "normal" audio)
    baseline_latent: Vec<f32>,
    /// Last observed latent for dynamic waveshape anomaly detection
    last_latent: Vec<f32>,
}

impl FusionEngine {
    pub fn new() -> Self {
        FusionEngine {
            baseline_latent: vec![0.0; 64], // MAMBA_LATENT_DIM = 64
            last_latent: Vec::new(),
        }
    }

    /// Fuse multi-sensor detection data
    pub fn fuse(
        &mut self,
        bispec_event: Option<&DetectionEvent>,
        mamba_anomaly: f32,
        mamba_latent: &[f32],
        beam_azimuth_rad: f32,
        beam_confidence: f32,
    ) -> FusionResult {
        let mut result = FusionResult::default();

        // ── Temporal Derivative (Waveshape Anomaly) ─────────────────────────
        let mut latent_derivative = 0.0;
        if !self.last_latent.is_empty() && mamba_latent.len() == self.last_latent.len() {
            let diff_sq: f32 = mamba_latent
                .iter()
                .zip(&self.last_latent)
                .map(|(a, b)| (a - b).powi(2))
                .sum();
            latent_derivative = diff_sq.sqrt();
        }
        self.last_latent = mamba_latent.to_vec();

        // ── Bispectrum Channel ────────────────────────────────────
        if let Some(event) = bispec_event {
            result.freq_hz = event.f1_hz; // Primary heterodyne frequency
            result.anomaly_db = event.magnitude; // Bispectrum magnitude (dB)
        }

        // ── Mamba Anomaly Channel ────────────────────────────────
        result.anomaly_db = mamba_anomaly.max(result.anomaly_db); // Take stronger signal

        // ── Latent Embedding Distance ────────────────────────────
        // Compute L2 distance between current and baseline
        if mamba_latent.len() == self.baseline_latent.len() {
            let dist_sq: f32 = mamba_latent
                .iter()
                .zip(&self.baseline_latent)
                .map(|(a, b)| (a - b).powi(2))
                .sum();
            result.embedding_distance = dist_sq.sqrt();
        }

        // ── TDOA Beam Channel ────────────────────────────────────
        result.azimuth_deg = beam_azimuth_rad.to_degrees();
        result.confidence = beam_confidence;

        // ── Source Classification ────────────────────────────────
        // Mamba-learned decision boundary (Centroid-based latent clustering)
        // Instead of a hardcoded scalar threshold, we measure distance to known attack centroids
        // in the Mamba latent space.

        let min_dist_to_normal = result.embedding_distance;
        let mut min_dist_to_jammer = 100.0f32;
        let mut min_dist_to_synth = 50.0f32;

        if mamba_latent.len() == self.baseline_latent.len() {
            // Placeholder learned centroids (these will be loaded from DB / weights in the future)
            // For now, we simulate the learned boundary checking
            let jammer_centroid = vec![1.5; mamba_latent.len()];
            let synth_centroid = vec![0.5; mamba_latent.len()];

            let dist_j: f32 = mamba_latent
                .iter()
                .zip(&jammer_centroid)
                .map(|(a, b)| (a - b).powi(2))
                .sum();
            min_dist_to_jammer = dist_j.sqrt();

            let dist_s: f32 = mamba_latent
                .iter()
                .zip(&synth_centroid)
                .map(|(a, b)| (a - b).powi(2))
                .sum();
            min_dist_to_synth = dist_s.sqrt();
        }

        // Distance-based classification
        if min_dist_to_jammer < min_dist_to_synth
            && min_dist_to_jammer < min_dist_to_normal
            && result.anomaly_db > 15.0
        {
            result.source_type = 2; // Jammer
        } else if min_dist_to_synth < min_dist_to_normal && result.anomaly_db > 5.0 {
            result.source_type = 1; // Synthetic
        } else if latent_derivative > 2.0 {
            // Rapid waveshape shifts = anomalous environmental coupling (e.g. holding controller)
            result.source_type = 2;
            result.anomaly_db += 10.0 * latent_derivative.log10().max(0.0);
        } else {
            result.source_type = 0; // Natural
        }

        // ── Final Confidence ─────────────────────────────────────
        // Weight by TDOA beam confidence and anomaly magnitude
        result.confidence =
            (beam_confidence * 0.5 + (result.anomaly_db / 30.0).min(1.0) * 0.5).clamp(0.0, 1.0);

        result
    }
}

impl Default for FusionEngine {
    fn default() -> Self {
        Self::new()
    }
}

pub mod imu_pose_fusion;
