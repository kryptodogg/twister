//! System configuration

/// Audio configuration
#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_size: usize,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 192_000,
            channels: 3,
            buffer_size: 1024,
        }
    }
}

/// RTL-SDR configuration
#[derive(Debug, Clone)]
pub struct RtlSdrConfig {
    pub sample_rate: u32,
    pub center_freq: f64,
    pub gain_db: f32,
    pub bandwidth: u32,
}

impl Default for RtlSdrConfig {
    fn default() -> Self {
        Self {
            sample_rate: 2_048_000,
            center_freq: 144_500_000.0,
            gain_db: 30.0,
            bandwidth: 2_400_000,
        }
    }
}

/// System-wide configuration
#[derive(Debug, Clone)]
pub struct SystemConfig {
    pub hardware: HardwareConfig,
    pub pipeline: PipelineConfig,
    pub control: ControlConfig,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            hardware: HardwareConfig::default(),
            pipeline: PipelineConfig::default(),
            control: ControlConfig::default(),
        }
    }
}

/// Hardware configuration
#[derive(Debug, Clone)]
pub struct HardwareConfig {
    pub audio_capture: AudioConfig,
    pub rtlsdr: RtlSdrConfig,
}

impl Default for HardwareConfig {
    fn default() -> Self {
        Self {
            audio_capture: AudioConfig::default(),
            rtlsdr: RtlSdrConfig::default(),
        }
    }
}

/// Pipeline configuration
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub target_latency_ms: f32,
    pub channel_buffer_size: usize,
    pub latency_monitoring: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            target_latency_ms: 35.0,
            channel_buffer_size: 1024,
            latency_monitoring: true,
        }
    }
}

/// Control configuration
#[derive(Debug, Clone)]
pub struct ControlConfig {
    pub mode: ControlMode,
    pub target_snr_db: f32,
}

impl Default for ControlConfig {
    fn default() -> Self {
        Self {
            mode: ControlMode::ANC,
            target_snr_db: 108.0,
        }
    }
}

/// Control mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlMode {
    ANC,
    Silence,
    Music,
}
