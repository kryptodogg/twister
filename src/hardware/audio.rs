//! Audio capture and playback

use crate::hardware::traits::{CaptureDevice, PlaybackDevice};
use crate::utils::AudioConfig;
use anyhow::Result;

/// Audio capture device (3-channel: rear mic + C925e stereo)
pub struct AudioCapture {
    config: AudioConfig,
    running: bool,
}

impl AudioCapture {
    pub fn new(config: AudioConfig) -> Result<Self> {
        Ok(Self {
            config,
            running: false,
        })
    }

    pub fn default_config() -> AudioConfig {
        AudioConfig::default()
    }

    pub fn config(&self) -> &AudioConfig {
        &self.config
    }

    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.config.channels
    }
}

impl CaptureDevice for AudioCapture {
    fn start(&mut self) -> Result<()> {
        log::info!("Starting audio capture with config: {:?}", self.config);
        self.running = true;
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        log::info!("Stopping audio capture");
        self.running = false;
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running
    }
}

/// Audio playback device
pub struct AudioPlayback {
    config: AudioConfig,
    running: bool,
}

impl AudioPlayback {
    pub fn new(config: AudioConfig) -> Result<Self> {
        Ok(Self {
            config,
            running: false,
        })
    }

    pub fn default_config() -> AudioConfig {
        AudioConfig::default()
    }

    pub fn config(&self) -> &AudioConfig {
        &self.config
    }

    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.config.channels
    }
}

impl PlaybackDevice for AudioPlayback {
    fn start(&mut self) -> Result<()> {
        log::info!("Starting audio playback with config: {:?}", self.config);
        self.running = true;
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        log::info!("Stopping audio playback");
        self.running = false;
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running
    }
}
