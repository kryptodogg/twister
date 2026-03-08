// src/detection.rs — Shared Detection Event Types  (v0.4)

use std::time::SystemTime;

// ── Interaction Products ──────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ProductType {
    Sum,
    Difference,
    Harmonic,
    Intermodulation,
}

impl ProductType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sum => "Sum",
            Self::Difference => "Difference",
            Self::Harmonic => "Harmonic",
            Self::Intermodulation => "Intermodulation",
        }
    }
    pub fn as_u32(&self) -> u32 {
        match self {
            Self::Sum => 0,
            Self::Difference => 1,
            Self::Harmonic => 2,
            Self::Intermodulation => 3,
        }
    }
}

// ── Detection Events ──────────────────────────────────────────────────────────
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DetectionEvent {
    pub id: String,
    #[serde(with = "timestamp_serde")]
    pub timestamp: SystemTime,
    pub f1_hz: f32,
    pub f2_hz: f32,
    pub product_hz: f32,
    pub product_type: ProductType,
    pub magnitude: f32,
    pub phase_angle: f32,
    pub coherence_frames: u32,
    pub spl_db: f32,
    pub session_id: String,
    pub hardware: HardwareLayer,
    pub embedding: Vec<f32>,
    pub frequency_band: crate::bispectrum::FrequencyBand,

    // ── Forensic Analysis Fields (v0.5) ────────────────────────────────────
    /// DC bias in audio signal (Volts)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_dc_bias_v: Option<f32>,
    /// DC bias in SDR signal (Volts)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdr_dc_bias_v: Option<f32>,
    /// Mamba anomaly score in dB
    #[serde(default)]
    pub mamba_anomaly_db: f32,
    /// Timestamp synchronization between RF and audio detections (milliseconds)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_sync_ms: Option<i64>,
    /// Whether this detection is part of a coordinated multi-vector attack
    #[serde(default)]
    pub is_coordinated: bool,
    /// Detection method (e.g., "bispectrum", "harmonic", "anomaly")
    #[serde(default)]
    pub detection_method: String,
}

// ── Hardware Layer ────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum HardwareLayer {
    Microphone,
    RtlSdr,     // RTL-SDR dongle  ← replaces PlutoSDR in v0.4
    NRF24Array, // TODO: Implement NRF24 packet sniffing layer.
    MmWave,     // TODO: Implement MmWave radar ingestion.
}

impl HardwareLayer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Microphone => "Microphone",
            Self::RtlSdr => "RtlSdr",
            Self::NRF24Array => "NRF24Array",
            Self::MmWave => "MmWave",
        }
    }
}

// ── Synthesis & Denial ────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DenialTarget {
    pub freq_hz: f32,
    pub gain: f32,
}

pub const MIN_COHERENCE_FRAMES: u32 = 10;

mod timestamp_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S: Serializer>(t: &SystemTime, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_f64(
            t.duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
        )
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<SystemTime, D::Error> {
        let secs = f64::deserialize(d)?;
        Ok(UNIX_EPOCH + std::time::Duration::from_secs_f64(secs))
    }
}
