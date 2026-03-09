// src/ml/anomaly_gate.rs — Fast path threshold logic (Track C.4)
//
// Determines whether an incoming spectral frame contains enough anomalous
// structure to warrant a full training/forensic log loop.

use super::spectral_frame::SpectralFrame;

#[derive(Debug, Clone)]
pub struct AnomalyGateDecision {
    pub forward_to_trainer: bool,
    pub confidence: f32,
    pub reason: String,
}

/// Evaluates a spectral frame against the Mamba anomaly score threshold.
/// MUST remain non-blocking and < 1ms execution time.
pub fn evaluate_gate(
    frame: &SpectralFrame,
    mamba_score: f32,
    threshold: f32,
) -> AnomalyGateDecision {
    // Heuristic 1: If Mamba says it's heavily anomalous, always forward.
    if mamba_score >= threshold {
        return AnomalyGateDecision {
            forward_to_trainer: true,
            confidence: (mamba_score / 10.0).min(1.0), // Assuming 10.0 is near-max expected score
            reason: format!("Mamba anomaly {:.2} >= threshold {:.2}", mamba_score, threshold),
        };
    }

    // Heuristic 2: Fast spatial anomaly detection.
    // If the interaural level difference is extreme but signal is present.
    let max_ild = frame.itd_ild[2].max(frame.itd_ild[3]).abs();
    if max_ild > 15.0 {
        return AnomalyGateDecision {
            forward_to_trainer: true,
            confidence: 0.6,
            reason: "Extreme ILD spatial anomaly detected".to_string(),
        };
    }

    // Heuristic 3: Fast coherent peak detection in mel spectrum.
    // Check if one bin has extreme power relative to mean (coherent tone).
    let sum: f32 = frame.fft_magnitude.iter().sum();
    let mean = sum / frame.fft_magnitude.len() as f32;
    let mut max_val = 0.0f32;
    for &val in frame.fft_magnitude.iter() {
        if val > max_val {
            max_val = val;
        }
    }

    if mean > 1e-6 && max_val > mean * 15.0 {
         return AnomalyGateDecision {
            forward_to_trainer: true,
            confidence: 0.7,
            reason: "Coherent spectral peak detected".to_string(),
        };
    }

    AnomalyGateDecision {
        forward_to_trainer: false,
        confidence: 0.0,
        reason: "Below threshold, background noise".to_string(),
    }
}
