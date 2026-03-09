use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The types of nodes available in the Forensic Knowledge Graph
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    Event,
    Pattern,
    SpatialLocation,
    Frequency,
    Device,
}

/// The types of edges (relationships) between nodes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EdgeType {
    HasPattern,
    OccurredAt,
    DetectedBy,
    TemporalSequence,
    SpatialProximity,
    AtFrequency,
}

// --------------------------------------------------------
// Critical Data Contracts (Input Sources from Tracks A-D)
// --------------------------------------------------------

/// From Track A: Event node (base forensic record)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventNode {
    pub event_id: u64,                        // Unique ID
    pub timestamp_iso: String,                // ISO 8601
    pub timestamp_us: u64,
    pub audio_rms_db: f32,
    pub rf_frequency_hz: f32,
    pub rf_peak_dbfs: f32,
    pub anomaly_score: f32,                   // Mamba reconstruction MSE
    pub tags: Vec<String>,                    // ["EVIDENCE", "MANUAL-REC"]
    pub device_source: String,                // "C925e", "RTL-SDR", etc.
}

/// From Track B: Pattern node structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternNode {
    pub pattern_id: usize,                    // Unique ID
    pub name: String,                         // "Friday_3PM_Tone"
    pub frequency_hz: f32,                    // Primary frequency
    pub confidence: f32,                      // [0, 1]
    pub cluster_size: usize,                  // Events in cluster
    pub first_occurrence_iso: String,         // ISO 8601
    pub last_occurrence_iso: String,
    pub tag_distribution: HashMap<String, f32>, // {"EVIDENCE": 0.7, ...}
    pub temporal_signature: String,           // "Daily", "Weekly", "Random"
}

/// From Track C: Frequency node for knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrequencyNode {
    pub frequency_hz: f32,
    pub band_name: String,                    // "2.4 GHz WiFi", "RF Heterodyne"
    pub detection_count: usize,               // How many events at this freq
    pub typical_confidence: f32,
    pub associated_patterns: Vec<usize>,      // Pattern IDs
    pub first_detected_iso: String,
}

/// From Track D: SpatialLocation node for knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialLocationNode {
    pub location_id: usize,                   // Unique ID (derived from bucketing azimuth/elevation)
    pub azimuth_rad: f32,                     // [-π, π]
    pub elevation_rad: f32,                   // [-π/2, π/2]
    pub event_count: usize,
    pub associated_patterns: Vec<usize>,
    pub confidence: f32,
    pub physical_interpretation: String,      // "Mouth region", "Elevated"
}

/// A device involved in the detection or emission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceNode {
    pub device_name: String,                  // Unique ID (e.g. "RTL-SDR")
    pub device_type: String,
}
