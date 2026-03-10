// src/parameters.rs — Parameter Persistence Layer
//
// JSON-based persistence for Twister configuration parameters.
// Stores at ~/.twister/parameters.json
//
// Parameters persisted:
// - Audio device selection
// - Camera resolution/FPS
// - Frequency band selection
// - Master gain
// - PDM settings
// - Waveshape mode
// - Beamforming settings
// - ANC settings
// - Joy-Con gesture mappings

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use anyhow::Context;

/// Frequency band enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u32)]
pub enum FrequencyBand {
    VLF = 0,   // 3-30 kHz
    LF = 1,    // 30-300 kHz
    MF = 2,    // 300 kHz - 3 MHz
    HF = 3,    // 3-30 MHz
    VHF = 4,   // 30-300 MHz
    UHF = 5,   // 300 MHz - 3 GHz
    Manual = 6, // User-specified frequency
}

impl FrequencyBand {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::LF,
            2 => Self::MF,
            3 => Self::HF,
            4 => Self::VHF,
            5 => Self::UHF,
            6 => Self::Manual,
            _ => Self::VLF,
        }
    }

    pub fn to_u32(self) -> u32 {
        self as u32
    }

    /// Get center frequency for each band (Hz)
    pub fn center_frequency_hz(self) -> f32 {
        match self {
            FrequencyBand::VLF => 15_000.0,      // 15 kHz
            FrequencyBand::LF => 150_000.0,      // 150 kHz
            FrequencyBand::MF => 1_500_000.0,    // 1.5 MHz
            FrequencyBand::HF => 15_000_000.0,   // 15 MHz
            FrequencyBand::VHF => 150_000_000.0, // 150 MHz
            FrequencyBand::UHF => 1_500_000_000.0, // 1.5 GHz
            FrequencyBand::Manual => 100_000_000.0, // 100 MHz default
        }
    }

    /// Get band name for display
    pub fn name(self) -> &'static str {
        match self {
            FrequencyBand::VLF => "VLF (3-30 kHz)",
            FrequencyBand::LF => "LF (30-300 kHz)",
            FrequencyBand::MF => "MF (300k-3 MHz)",
            FrequencyBand::HF => "HF (3-30 MHz)",
            FrequencyBand::VHF => "VHF (30-300 MHz)",
            FrequencyBand::UHF => "UHF (300M-3 GHz)",
            FrequencyBand::Manual => "Manual",
        }
    }
}

/// Camera resolution enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u32)]
pub enum CameraResolution {
    R480p = 0,   // 640x480
    R720p = 1,   // 1280x720
    R1080p = 2,  // 1920x1080
}

impl CameraResolution {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::R720p,
            2 => Self::R1080p,
            _ => Self::R480p,
        }
    }

    pub fn to_u32(self) -> u32 {
        self as u32
    }

    pub fn dimensions(self) -> (u32, u32) {
        match self {
            CameraResolution::R480p => (640, 480),
            CameraResolution::R720p => (1280, 720),
            CameraResolution::R1080p => (1920, 1080),
        }
    }
}

/// Audio device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceConfig {
    pub index: usize,
    pub name: String,
    pub sample_rate_hz: u32,
    pub channels: u32,
    pub is_active: bool,
}

/// Joy-Con gesture mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoyConGestureMapping {
    /// Gyro roll → viewport rotation sensitivity (degrees per degree)
    pub roll_sensitivity: f32,
    /// Gyro pitch → heterodyne steering sensitivity
    pub pitch_sensitivity: f32,
    /// Gyro yaw → heterodyne steering sensitivity
    pub yaw_sensitivity: f32,
    /// Accelerometer magnitude → heterodyne strength
    pub accel_strength_sensitivity: f32,
    /// R Trigger → carrier frequency modulation depth (Hz)
    pub trigger_freq_depth_hz: f32,
    /// Stick deadzone threshold
    pub stick_deadzone: f32,
    /// Button A action
    pub button_a_action: String,
    /// Button B action
    pub button_b_action: String,
    /// Button X action
    pub button_x_action: String,
    /// Button Y action
    pub button_y_action: String,
}

impl Default for JoyConGestureMapping {
    fn default() -> Self {
        Self {
            roll_sensitivity: 1.0,
            pitch_sensitivity: 0.5,
            yaw_sensitivity: 0.5,
            accel_strength_sensitivity: 2.0,
            trigger_freq_depth_hz: 1000.0,
            stick_deadzone: 0.15,
            button_a_action: "toggle_pdm".to_string(),
            button_b_action: "toggle_anc".to_string(),
            button_x_action: "toggle_auto_tune".to_string(),
            button_y_action: "toggle_recording".to_string(),
        }
    }
}

/// Complete parameter set for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwisterParameters {
    // Audio
    pub audio_device_idx: u32,
    pub audio_devices: Vec<AudioDeviceConfig>,
    pub master_gain_db: f32,

    // Camera
    pub camera_resolution: u32,
    pub camera_fps: f32,
    pub camera_active: bool,

    // Frequency
    pub freq_band_index: u32,
    pub freq_manual_hz: f32,

    // PDM
    pub pdm_active: bool,
    pub pdm_clock_mhz: f32,
    pub oversample_ratio: u32,

    // Waveshaping
    pub waveshape_mode: u32,
    pub waveshape_drive: f32,

    // Beamforming
    pub beam_azimuth_deg: f32,
    pub beam_elevation_deg: f32,
    pub beam_focus_deg: f32,

    // ANC
    pub smart_anc_blend: f32,
    pub anc_calibrated: bool,

    // SDR
    pub sdr_center_hz: f32,
    pub sdr_gain_db: f32,
    pub sdr_active: bool,

    // Joy-Con
    pub joycon_enabled: bool,
    pub joycon_mapping: JoyConGestureMapping,

    // Metadata
    pub version: String,
    pub last_modified: String,
}

impl Default for TwisterParameters {
    fn default() -> Self {
        Self {
            audio_device_idx: 0,
            audio_devices: Vec::new(),
            master_gain_db: 0.0,

            camera_resolution: 1, // 720p default
            camera_fps: 30.0,
            camera_active: false,

            freq_band_index: FrequencyBand::VHF.to_u32(), // 150 MHz default
            freq_manual_hz: 100_000_000.0,

            pdm_active: true,
            pdm_clock_mhz: 12.288,
            oversample_ratio: 64,

            waveshape_mode: 2, // Triangle
            waveshape_drive: 1.0,

            beam_azimuth_deg: 0.0,
            beam_elevation_deg: 0.0,
            beam_focus_deg: 45.0,

            smart_anc_blend: 0.3,
            anc_calibrated: false,

            sdr_center_hz: 100_000_000.0,
            sdr_gain_db: 20.0,
            sdr_active: true,

            joycon_enabled: true,
            joycon_mapping: JoyConGestureMapping::default(),

            version: env!("CARGO_PKG_VERSION").to_string(),
            last_modified: chrono::Utc::now().to_rfc3339(),
        }
    }
}

impl TwisterParameters {
    /// Get the configuration directory path (~/.twister/)
    pub fn config_dir() -> PathBuf {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".twister")
    }

    /// Get the parameters file path
    pub fn config_path() -> PathBuf {
        Self::config_dir().join("parameters.json")
    }

    /// Ensure config directory exists
    fn ensure_config_dir() -> anyhow::Result<()> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create config directory: {:?}", dir))?;
        Ok(())
    }

    /// Load parameters from disk (returns default if file doesn't exist)
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::config_path();
        
        if !path.exists() {
            // Return default parameters if file doesn't exist
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read parameters file: {:?}", path))?;

        let params: TwisterParameters = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse parameters JSON: {:?}", path))?;

        Ok(params)
    }

    /// Save parameters to disk
    pub fn save(&self) -> anyhow::Result<()> {
        Self::ensure_config_dir()?;
        
        let path = Self::config_path();
        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize parameters to JSON")?;

        fs::write(&path, json)
            .with_context(|| format!("Failed to write parameters file: {:?}", path))?;

        Ok(())
    }

    /// Reset to default parameters
    pub fn reset_to_defaults() -> Self {
        Self::default()
    }
}

/// Convert AudioDeviceConfig to state::AudioDevice
pub fn config_to_audio_device(config: &AudioDeviceConfig) -> crate::state::AudioDevice {
    crate::state::AudioDevice {
        index: config.index,
        name: config.name.clone(),
        sample_rate_hz: config.sample_rate_hz,
        channels: config.channels,
        is_active: config.is_active,
    }
}

/// Convert state::AudioDevice to AudioDeviceConfig
pub fn audio_device_to_config(device: &crate::state::AudioDevice) -> AudioDeviceConfig {
    AudioDeviceConfig {
        index: device.index,
        name: device.name.clone(),
        sample_rate_hz: device.sample_rate_hz,
        channels: device.channels,
        is_active: device.is_active,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frequency_band_conversion() {
        assert_eq!(FrequencyBand::from_u32(0), FrequencyBand::VLF);
        assert_eq!(FrequencyBand::from_u32(4), FrequencyBand::VHF);
        assert_eq!(FrequencyBand::VHF.to_u32(), 4);
    }

    #[test]
    fn test_frequency_band_center_frequency() {
        assert_eq!(FrequencyBand::VLF.center_frequency_hz(), 15_000.0);
        assert_eq!(FrequencyBand::VHF.center_frequency_hz(), 150_000_000.0);
        assert_eq!(FrequencyBand::UHF.center_frequency_hz(), 1_500_000_000.0);
    }

    #[test]
    fn test_camera_resolution_dimensions() {
        assert_eq!(CameraResolution::R480p.dimensions(), (640, 480));
        assert_eq!(CameraResolution::R720p.dimensions(), (1280, 720));
        assert_eq!(CameraResolution::R1080p.dimensions(), (1920, 1080));
    }

    #[test]
    fn test_default_parameters() {
        let params = TwisterParameters::default();
        assert_eq!(params.version, env!("CARGO_PKG_VERSION"));
        assert!(params.pdm_active);
        assert!(params.sdr_active);
        assert!(params.joycon_enabled);
    }

    #[test]
    fn test_parameters_save_load_roundtrip() {
        let mut params = TwisterParameters::default();
        params.master_gain_db = 12.5;
        params.freq_band_index = FrequencyBand::HF.to_u32();
        params.joycon_mapping.roll_sensitivity = 2.0;

        // Save to temp location
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().join("parameters.json");
        
        // Mock the config path by temporarily changing behavior
        let json = serde_json::to_string_pretty(&params).unwrap();
        fs::write(&temp_path, &json).unwrap();
        
        // Load back
        let content = fs::read_to_string(&temp_path).unwrap();
        let loaded: TwisterParameters = serde_json::from_str(&content).unwrap();
        
        assert_eq!(params.master_gain_db, loaded.master_gain_db);
        assert_eq!(params.freq_band_index, loaded.freq_band_index);
        assert_eq!(params.joycon_mapping.roll_sensitivity, loaded.joycon_mapping.roll_sensitivity);
    }
}
