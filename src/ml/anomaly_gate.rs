use crate::ml::spectral_frame::SpectralFrame;

#[derive(Debug, Clone)]
pub struct AnomalyGateConfig {
    pub anomaly_score_threshold: f32,
    pub confidence_threshold: f32,
    pub force_forward: bool,
}

impl Default for AnomalyGateConfig {
    fn default() -> Self {
        Self {
            anomaly_score_threshold: 1.0,
            confidence_threshold: 0.5,
            force_forward: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnomalyGateDecision {
    pub forward_to_trainer: bool,
    pub confidence: f32,
    pub reason: String,
    pub anomaly_score_value: f32,
}

pub fn evaluate_anomaly_gate(
    frame: &SpectralFrame,
    config: &AnomalyGateConfig,
) -> AnomalyGateDecision {
    if config.force_forward {
        return AnomalyGateDecision {
            forward_to_trainer: true,
            confidence: frame.confidence,
            reason: "Forced forward".to_string(),
            anomaly_score_value: frame.mamba_anomaly_score,
        };
    }

    if frame.mamba_anomaly_score > config.anomaly_score_threshold && frame.confidence > config.confidence_threshold {
        AnomalyGateDecision {
            forward_to_trainer: true,
            confidence: frame.confidence,
            reason: "High anomaly score and confidence".to_string(),
            anomaly_score_value: frame.mamba_anomaly_score,
        }
    } else {
        AnomalyGateDecision {
            forward_to_trainer: false,
            confidence: frame.confidence,
            reason: "Below threshold".to_string(),
            anomaly_score_value: frame.mamba_anomaly_score,
        }
    }
}
