use std::collections::HashMap;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ForensicEvent {
    AudioFrameProcessed {
        timestamp_micros: u64,
        device_idx: u32,
        sample_rate_hz: u32,
        frame_size_samples: u32,
        rms_db: f32,
        peak_db: f32,
        clipping_detected: bool,
    },
    AnomalyGateDecision {
        timestamp_micros: u64,
        anomaly_score: f32,
        confidence: f32,
        threshold_used: f32,
        forward_to_trainer: bool,
        reason: String,
    },
}
