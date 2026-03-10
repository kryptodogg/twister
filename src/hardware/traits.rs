//! Hardware traits

use anyhow::Result;

/// Capture device trait
pub trait CaptureDevice {
    fn start(&mut self) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn is_running(&self) -> bool;
}

/// Playback device trait
pub trait PlaybackDevice {
    fn start(&mut self) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn is_running(&self) -> bool;
}

/// Calibration trait
pub trait Calibratable {
    fn calibrate(&mut self) -> Result<CalibrationResult>;
}

/// Calibration result
#[derive(Debug, Clone)]
pub struct CalibrationResult {
    pub offset: f32,
    pub scale: f32,
    pub snr_db: f32,
}
