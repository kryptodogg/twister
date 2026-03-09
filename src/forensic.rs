
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ForensicEvent {
    SessionStart { timestamp_micros: u64 },
    SessionEnd { timestamp_micros: u64 },
    AudioFrameProcessed { timestamp_micros: u64 },
    RFDetection { timestamp_micros: u64 },
    MambaInference { timestamp_micros: u64 },
    Bispectrum { timestamp_micros: u64 },
    AnomalyGateDecision { timestamp_micros: u64 },
}

pub struct ForensicLogger {}
impl ForensicLogger {
    pub async fn new(_: &str) -> Result<Self, String> { Ok(Self {}) }
    pub fn log_detection(&self, _: &crate::detection::DetectionEvent) -> Result<(), String> { Ok(()) }
}

pub async fn verify_log_integrity(_: &str) -> Result<(), String> { Ok(()) }

impl Clone for ForensicLogger {
    fn clone(&self) -> Self { Self {} }
}
