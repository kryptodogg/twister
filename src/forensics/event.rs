//! Forensic event definition

use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use ndarray::Array1;

/// Forensic event capturing RF-Audio state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForensicEvent {
    /// Unique event ID
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Event metadata
    pub metadata: EventMetadata,
    /// RF context
    pub rf_context: RFContext,
    /// Audio context
    pub audio_context: AudioContext,
    /// Mamba latent representation (64-D)
    pub latent: Vec<f32>,
    /// Control output
    pub control: ControlState,
    /// System state
    pub system_state: SystemState,
}

/// Event metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Event type
    pub event_type: EventType,
    /// Location identifier
    pub location_id: Option<String>,
    /// Session ID
    pub session_id: String,
    /// Sequence number
    pub sequence: u64,
    /// Tags
    pub tags: Vec<String>,
}

/// Event type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    /// Mode transition
    ModeTransition,
    /// RF interference detected
    RFInterference,
    /// Audio anomaly
    AudioAnomaly,
    /// Calibration event
    Calibration,
    /// User interaction
    UserInteraction,
    /// Periodic snapshot
    Snapshot,
    /// Error event
    Error,
}

/// RF context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RFContext {
    /// Center frequency (Hz)
    pub center_frequency_hz: u32,
    /// Sample rate (Hz)
    pub sample_rate_hz: u32,
    /// PSD values (256 bins)
    pub psd: Vec<f32>,
    /// Total power (dB)
    pub total_power_db: f32,
    /// Spectral kurtosis
    pub spectral_kurtosis: f32,
    /// Peak frequency bin
    pub peak_bin: usize,
    /// Band ratios [low, mid, high]
    pub band_ratios: [f32; 3],
    /// RFI detected
    pub rfi_detected: bool,
    /// SNR estimate (dB)
    pub snr_db: f32,
}

/// Audio context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioContext {
    /// Sample rate (Hz)
    pub sample_rate_hz: u32,
    /// Number of channels
    pub num_channels: usize,
    /// PSD values (128 bins)
    pub psd: Vec<f32>,
    /// TDOA features (16 values)
    pub tdoa: Vec<f32>,
    /// TDOA estimate (samples)
    pub tdoa_estimate: f32,
    /// Correlation peak
    pub correlation_peak: f32,
    /// Residual energy
    pub residual_energy: f32,
    /// Channel energies
    pub channel_energies: [f32; 3],
    /// Spectral centroid
    pub spectral_centroid: f32,
    /// Zero crossing rate
    pub zcr: f32,
    /// Ambient noise level (dB)
    pub ambient_noise_db: f32,
}

/// Control state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlState {
    /// Current mode
    pub mode: ControlMode,
    /// Mode probabilities
    pub mode_probs: [f32; 3],
    /// Target SNR (dB)
    pub target_snr_db: f32,
    /// ANC weights version
    pub anc_weights_version: u32,
    /// Fade state (0-1)
    pub fade_state: f32,
}

/// Control mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlMode {
    /// Active Noise Cancellation
    Anc,
    /// Silence (passive)
    Silence,
    /// Music playback
    Music,
}

/// System state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    /// CPU usage (%)
    pub cpu_usage: f32,
    /// Memory usage (MB)
    pub memory_mb: u32,
    /// Pipeline latency (ms)
    pub pipeline_latency_ms: f32,
    /// GPU utilization (%)
    pub gpu_utilization: f32,
    /// Temperature (C)
    pub temperature_c: Option<f32>,
    /// Uptime (seconds)
    pub uptime_secs: u64,
}

impl ForensicEvent {
    /// Create a new forensic event
    pub fn new(
        metadata: EventMetadata,
        rf_context: RFContext,
        audio_context: AudioContext,
        latent: Array1<f32>,
        control: ControlState,
        system_state: SystemState,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            metadata,
            rf_context,
            audio_context,
            latent: latent.to_vec(),
            control,
            system_state,
        }
    }

    /// Create a snapshot event
    pub fn snapshot(
        rf_context: RFContext,
        audio_context: AudioContext,
        latent: Array1<f32>,
        control: ControlState,
    ) -> Self {
        Self::new(
            EventMetadata {
                event_type: EventType::Snapshot,
                location_id: None,
                session_id: Uuid::new_v4().to_string(),
                sequence: 0,
                tags: vec!["snapshot".into()],
            },
            rf_context,
            audio_context,
            latent,
            control,
            SystemState::default(),
        )
    }

    /// Create a mode transition event
    pub fn mode_transition(
        from_mode: ControlMode,
        to_mode: ControlMode,
        rf_context: RFContext,
        audio_context: AudioContext,
        latent: Array1<f32>,
    ) -> Self {
        Self::new(
            EventMetadata {
                event_type: EventType::ModeTransition,
                location_id: None,
                session_id: Uuid::new_v4().to_string(),
                sequence: 0,
                tags: vec![
                    "transition".into(),
                    format!("from_{:?}", from_mode),
                    format!("to_{:?}", to_mode),
                ],
            },
            rf_context,
            audio_context,
            latent,
            ControlState {
                mode: to_mode,
                mode_probs: [0.0, 0.0, 1.0],
                target_snr_db: 108.0,
                anc_weights_version: 0,
                fade_state: 0.0,
            },
            SystemState::default(),
        )
    }

    /// Get latent as slice
    pub fn latent_slice(&self) -> &[f32] {
        &self.latent
    }

    /// Get event dimension (for Qdrant)
    pub fn vector_dim() -> usize {
        64 // Latent dimension
    }
}

impl Default for SystemState {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_mb: 0,
            pipeline_latency_ms: 0.0,
            gpu_utilization: 0.0,
            temperature_c: None,
            uptime_secs: 0,
        }
    }
}

impl Default for ControlMode {
    fn default() -> Self {
        Self::Anc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forensic_event_creation() {
        let event = ForensicEvent::snapshot(
            RFContext {
                center_frequency_hz: 100_000_000,
                sample_rate_hz: 2_048_000,
                psd: vec![0.0; 256],
                total_power_db: -50.0,
                spectral_kurtosis: 0.0,
                peak_bin: 0,
                band_ratios: [0.33, 0.33, 0.34],
                rfi_detected: false,
                snr_db: 50.0,
            },
            AudioContext {
                sample_rate_hz: 192_000,
                num_channels: 3,
                psd: vec![0.0; 128],
                tdoa: vec![0.0; 16],
                tdoa_estimate: 0.0,
                correlation_peak: 0.0,
                residual_energy: 0.0,
                channel_energies: [0.0, 0.0, 0.0],
                spectral_centroid: 0.0,
                zcr: 0.0,
                ambient_noise_db: 40.0,
            },
            Array1::zeros(64),
            ControlState {
                mode: ControlMode::Anc,
                mode_probs: [1.0, 0.0, 0.0],
                target_snr_db: 108.0,
                anc_weights_version: 0,
                fade_state: 1.0,
            },
        );

        assert!(!event.id.is_empty());
        assert_eq!(event.metadata.event_type, EventType::Snapshot);
        assert_eq!(event.latent.len(), 64);
    }

    #[test]
    fn test_mode_transition_event() {
        let event = ForensicEvent::mode_transition(
            ControlMode::Silence,
            ControlMode::Anc,
            RFContext {
                center_frequency_hz: 100_000_000,
                sample_rate_hz: 2_048_000,
                psd: vec![0.0; 256],
                total_power_db: -50.0,
                spectral_kurtosis: 0.0,
                peak_bin: 0,
                band_ratios: [0.33, 0.33, 0.34],
                rfi_detected: false,
                snr_db: 50.0,
            },
            AudioContext {
                sample_rate_hz: 192_000,
                num_channels: 3,
                psd: vec![0.0; 128],
                tdoa: vec![0.0; 16],
                tdoa_estimate: 0.0,
                correlation_peak: 0.0,
                residual_energy: 0.0,
                channel_energies: [0.0, 0.0, 0.0],
                spectral_centroid: 0.0,
                zcr: 0.0,
                ambient_noise_db: 40.0,
            },
            Array1::zeros(64),
        );

        assert_eq!(event.metadata.event_type, EventType::ModeTransition);
        assert!(event.metadata.tags.contains(&"from_Silence".to_string()));
        assert!(event.metadata.tags.contains(&"to_Anc".to_string()));
    }

    #[test]
    fn test_vector_dim() {
        assert_eq!(ForensicEvent::vector_dim(), 64);
    }
}
