// src/ml/anomaly_gate.rs — Anomaly Detection Gate (Interface Contract & Heuristics)
//
// Decides whether to forward SpectralFrame to training pipeline based on
// Mamba anomaly score and fast spectral heuristics.

use crate::ml::spectral_frame::SpectralFrame;
use serde::{Deserialize, Serialize};

/// **AnomalyGateDecision**: Non-blocking decision to enqueue training pair
///
/// This struct gates entry into the training pipeline. Must execute in < 1ms
/// (generation-critical: cannot block dispatch loop).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnomalyGateDecision {
    /// Whether to enqueue this spectral frame as a training pair
    pub forward_to_trainer: bool,

    /// Confidence in the gate decision (0.0-1.0)
    /// Used for weighting training contribution if needed
    pub confidence: f32,

    /// Human-readable reason for the decision
    pub reason: String,

    /// The anomaly score that triggered this decision
    pub anomaly_score_value: f32,
}

impl AnomalyGateDecision {
    /// Create a gate decision (forward = true)
    pub fn forward(anomaly_score: f32, confidence: f32, reason: &str) -> Self {
        Self {
            forward_to_trainer: true,
            confidence: confidence.clamp(0.0, 1.0),
            reason: reason.to_string(),
            anomaly_score_value: anomaly_score,
        }
    }

    /// Create a gate decision (forward = false, low score)
    pub fn reject_low_anomaly(anomaly_score: f32) -> Self {
        Self {
            forward_to_trainer: false,
            confidence: (1.0 - anomaly_score.min(1.0)).clamp(0.0, 1.0),
            reason: "anomaly_score_below_threshold".to_string(),
            anomaly_score_value: anomaly_score,
        }
    }

    /// Create a gate decision (forward = false, low confidence)
    pub fn reject_low_confidence(confidence: f32) -> Self {
        Self {
            forward_to_trainer: false,
            confidence: (1.0 - confidence).clamp(0.0, 1.0),
            reason: "detection_confidence_too_low".to_string(),
            anomaly_score_value: 0.0,
        }
    }

    /// Stub for testing
    pub fn stub_forward() -> Self {
        Self::forward(2.5, 0.9, "anomaly_detected")
    }

    /// Stub for testing
    pub fn stub_reject() -> Self {
        Self::reject_low_anomaly(0.3)
    }
}

/// **Anomaly Gate Configuration**
/// Tunable thresholds for the gate decision logic
#[derive(Clone, Debug)]
pub struct AnomalyGateConfig {
    /// Threshold above which anomaly is forwarded to training
    pub anomaly_score_threshold: f32,

    /// Minimum confidence required to forward
    pub min_confidence: f32,

    /// Whether to always forward (for debugging)
    pub force_forward: bool,
}

impl Default for AnomalyGateConfig {
    fn default() -> Self {
        Self {
            anomaly_score_threshold: 1.0,
            min_confidence: 0.5,
            force_forward: false,
        }
    }
}

/// **Evaluate the anomaly gate**
/// Non-blocking decision (< 1ms) whether to forward spectral frame to training
pub fn evaluate_anomaly_gate(
    frame: &SpectralFrame,
    config: &AnomalyGateConfig,
) -> AnomalyGateDecision {
    // 0. Force mode (for testing/debugging)
    if config.force_forward {
        return AnomalyGateDecision::forward(
            frame.mamba_anomaly_score,
            frame.confidence,
            "force_forward",
        );
    }

    // 1. Confidence gate (primary safety check)
    if frame.confidence < config.min_confidence {
        return AnomalyGateDecision::reject_low_confidence(frame.confidence);
    }

    // 2. Mamba anomaly score gate (primary decision criterion)
    if frame.mamba_anomaly_score >= config.anomaly_score_threshold {
        return AnomalyGateDecision::forward(
            frame.mamba_anomaly_score,
            (frame.mamba_anomaly_score / 10.0).min(1.0),
            &format!(
                "Mamba anomaly {:.2} >= threshold {:.2}",
                frame.mamba_anomaly_score, config.anomaly_score_threshold
            ),
        );
    }

    // 3. Heuristic: Fast spatial anomaly detection (Extreme ILD)
    let max_ild = frame.itd_ild[2].max(frame.itd_ild[3]).abs();
    if max_ild > 15.0 {
        return AnomalyGateDecision::forward(
            0.6, // Synthetic score for heuristic
            0.6,
            "Extreme ILD spatial anomaly detected",
        );
    }

    // 4. Heuristic: Fast coherent peak detection in mel spectrum.
    let sum: f32 = frame.fft_magnitude.iter().sum();
    let mean = sum / frame.fft_magnitude.len() as f32;
    let mut max_val = 0.0f32;
    for &val in frame.fft_magnitude.iter() {
        if val > max_val {
            max_val = val;
        }
    }

    if mean > 1e-6 && max_val > mean * 15.0 {
        return AnomalyGateDecision::forward(
            0.7, // Synthetic score
            0.7,
            "Coherent spectral peak detected",
        );
    }

    AnomalyGateDecision::reject_low_anomaly(frame.mamba_anomaly_score)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gate_forward_high_anomaly() {
        let mut frame = SpectralFrame::stub();
        frame.mamba_anomaly_score = 1.5;
        let config = AnomalyGateConfig::default();

        let decision = evaluate_anomaly_gate(&frame, &config);
        assert!(decision.forward_to_trainer);
    }

    #[test]
    fn test_gate_reject_low_confidence() {
        let mut frame = SpectralFrame::stub();
        frame.confidence = 0.2;

        let config = AnomalyGateConfig::default();
        let decision = evaluate_anomaly_gate(&frame, &config);
        assert!(!decision.forward_to_trainer);
        assert!(decision.reason.contains("confidence"));
    }

    #[test]
    fn test_gate_heuristic_ild() {
        let mut frame = SpectralFrame::stub();
        frame.mamba_anomaly_score = 0.1;
        frame.itd_ild[2] = 20.0; // Extreme ILD

        let config = AnomalyGateConfig::default();
        let decision = evaluate_anomaly_gate(&frame, &config);
        assert!(decision.forward_to_trainer);
        assert!(decision.reason.contains("ILD"));
    }

    #[test]
    fn test_gate_serialization() {
        let decision = AnomalyGateDecision::stub_forward();
        let json = serde_json::to_string(&decision).unwrap();
        let decision2: AnomalyGateDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(decision.forward_to_trainer, decision2.forward_to_trainer);
    }

    #[test]
    fn test_gate_timing_nonblocking() {
        let frame = SpectralFrame::stub();
        let config = AnomalyGateConfig::default();

        let start = std::time::Instant::now();
        let _ = evaluate_anomaly_gate(&frame, &config);
        let elapsed = start.elapsed();

        assert!(elapsed.as_millis() < 1);
    }
}
