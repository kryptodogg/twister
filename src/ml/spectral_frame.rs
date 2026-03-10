// src/ml/spectral_frame.rs — Real-time audio spectral features (Track C.2)
//
// Extracts instantaneous spectral information from the audio dispatch loop.
// Used for rapid anomaly evaluation before expensive Mamba/TimeGNN passes.

use serde::{Deserialize, Serialize};

/// **SpectralFrame**: Audio-derived features ready for anomaly detection gate and training
///
/// This struct bridges Track C (audio processing) and the training pipeline.
/// Produced every ~100ms in the dispatch loop from FFT, TDOA, and Mamba inference results.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpectralFrame {
    /// Microseconds since Unix epoch
    pub timestamp_micros: u64,

    /// 128 mel-scale bins from FFT (512 bins -> 128 via mel-scale binning)
    /// Range: [0.0, 1.0] normalized magnitude
    pub fft_magnitude: Vec<f32>,

    /// Bispectrum top 64 components by energy (phase coupling)
    /// Detects non-linear phase relationships (RF modulation signature)
    pub bispectrum: Vec<f32>,

    /// Interaural time difference (ITD) and level difference (ILD)
    /// From mic pair cross-correlation
    /// [itd_left_right, itd_front_back, ild_left_right, ild_front_back]
    pub itd_ild: [f32; 4],

    /// Beamformer outputs at 3 fixed azimuths: -45°, 0°, +45°
    /// Energy-weighted by angle to source (from TDOA estimation)
    pub beamformer_outputs: [f32; 3],

    /// Mamba autoencoder reconstruction MSE (anomaly score)
    /// Range: [0.0, ∞) where 0 = normal, >1.0 = anomalous
    pub mamba_anomaly_score: f32,

    /// Detection confidence (0.0-1.0)
    /// Derived from SNR, correlation quality, etc.
    pub confidence: f32,
}

impl Default for SpectralFrame {
    fn default() -> Self {
        SpectralFrame {
            timestamp_micros: 0,
            fft_magnitude: vec![0.0; 128],
            bispectrum: vec![0.0; 64],
            itd_ild: [0.0; 4],
            beamformer_outputs: [0.0; 3],
            mamba_anomaly_score: 0.0,
            confidence: 0.0,
        }
    }
}

impl SpectralFrame {
    /// Create a new spectral frame from computed features
    pub fn new(
        timestamp_micros: u64,
        fft_magnitude: Vec<f32>,
        bispectrum: Vec<f32>,
        itd_ild: [f32; 4],
        beamformer_outputs: [f32; 3],
        mamba_anomaly_score: f32,
        confidence: f32,
    ) -> Self {
        Self {
            timestamp_micros,
            fft_magnitude,
            bispectrum,
            itd_ild,
            beamformer_outputs,
            mamba_anomaly_score,
            confidence,
        }
    }

    /// Validate frame integrity (generation protection check)
    pub fn is_valid(&self) -> bool {
        // All finite values
        self.fft_magnitude.iter().all(|v| v.is_finite()) &&
        self.bispectrum.iter().all(|v| v.is_finite()) &&
        self.itd_ild.iter().all(|v| v.is_finite()) &&
        self.beamformer_outputs.iter().all(|v| v.is_finite()) &&
        self.mamba_anomaly_score.is_finite() &&
        self.confidence.is_finite() &&
        // Confidence in valid range
        self.confidence >= 0.0 && self.confidence <= 1.0
    }

    /// Stub for testing (realistic mock data)
    pub fn stub() -> Self {
        Self {
            timestamp_micros: 1_000_000_000,
            fft_magnitude: vec![0.5; 128],
            bispectrum: vec![0.3; 64],
            itd_ild: [0.001, 0.002, 0.1, 0.15],
            beamformer_outputs: [0.7, 0.8, 0.6],
            mamba_anomaly_score: 0.5,
            confidence: 0.9,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_frame_creation() {
        let frame = SpectralFrame::stub();
        assert!(frame.is_valid());
    }

    #[test]
    fn test_spectral_frame_serialization() {
        let frame = SpectralFrame::stub();
        let json = serde_json::to_string(&frame).unwrap();
        let frame2: SpectralFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(frame.timestamp_micros, frame2.timestamp_micros);
    }

    #[test]
    fn test_spectral_frame_invalid_nan() {
        let mut frame = SpectralFrame::stub();
        frame.mamba_anomaly_score = f32::NAN;
        assert!(!frame.is_valid());
    }

    #[test]
    fn test_spectral_frame_invalid_confidence() {
        let mut frame = SpectralFrame::stub();
        frame.confidence = 1.5; // Out of range
        assert!(!frame.is_valid());
    }
}
