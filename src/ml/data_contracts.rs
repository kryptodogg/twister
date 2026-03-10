use std::collections::HashMap;

/// Event data structure for corpus
#[derive(Debug, Clone)]
pub struct ForensicEventData {
    /// Unique event identifier
    pub id: String,
    /// Unix timestamp (microseconds)
    pub timestamp_micros: i64,
    /// 1297-D multimodal feature vector
    pub features: Vec<f32>,
    /// Forensic classification tag
    pub tag: String,
    /// Detection confidence [0, 1]
    pub confidence: f32,
    /// RF frequency in Hz
    pub rf_frequency_hz: f32,
    /// Duration
    pub duration_seconds: f32,
    pub timestamp_unix: f64,
    pub frequency_hz: f32,
    pub metadata: HashMap<String, String>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PointMambaEncoderOutput {
    pub embedding: [f32; 256],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RFDetection {
    pub azimuth: f32,
    pub elevation: f32,
    pub frequency: f32,
    pub intensity: f32,
    pub timestamp: u64,
    pub confidence: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IMUSample {
    pub accel: [f32; 3],
    pub gyro: [f32; 3],
    pub timestamp: u64,
}
