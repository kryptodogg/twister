//! Hardware calibration utilities

use crate::hardware::{AudioCapture, AudioPlayback, RtlSdrDevice};
use crate::utils::error::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

/// Calibration results for all devices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationResult {
    pub timestamp: String,
    pub rtlsdr: Option<RtlSdrCalibration>,
    pub audio_capture: Option<AudioCaptureCalibration>,
    pub audio_playback: Option<AudioPlaybackCalibration>,
    pub sync_offset_ms: f64,
    pub latency_budget: LatencyBudget,
}

/// RTL-SDR calibration data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtlSdrCalibration {
    pub actual_sample_rate: u32,
    pub frequency_error_ppm: f32,
    pub gain_calibration_db: f32,
    pub noise_floor_db: f32,
}

/// Audio capture calibration data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioCaptureCalibration {
    pub actual_sample_rate: u32,
    pub channel_balance: Vec<f32>,
    pub frequency_response: Vec<f32>,
    pub noise_floor_db: f32,
}

/// Audio playback calibration data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioPlaybackCalibration {
    pub actual_sample_rate: u32,
    pub pdm_calibration: Vec<f32>,
    pub output_level_db: f32,
}

/// Latency budget breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyBudget {
    pub rf_capture_ms: f64,
    pub audio_capture_ms: f64,
    pub dsp_processing_ms: f64,
    pub ml_inference_ms: f64,
    pub output_ms: f64,
    pub total_ms: f64,
    pub target_ms: f64,
    pub margin_ms: f64,
}

/// Hardware calibration manager
pub struct Calibration {
    results: HashMap<String, CalibrationResult>,
}

/// Per-device calibration
pub struct DeviceCalibration {
    pub device_name: String,
    pub calibration_data: Vec<f32>,
}

impl Calibration {
    /// Create a new calibration instance
    pub fn new() -> Self {
        Self {
            results: HashMap::new(),
        }
    }

    /// Calibrate all hardware
    pub fn calibrate_all(&mut self) -> Result<CalibrationResult> {
        let mut result = CalibrationResult {
            timestamp: chrono::Utc::now().to_rfc3339(),
            rtlsdr: None,
            audio_capture: None,
            audio_playback: None,
            sync_offset_ms: 0.0,
            latency_budget: LatencyBudget::default(),
        };

        // Calibrate RTL-SDR
        if let Ok(rtlsdr_cal) = self.calibrate_rtlsdr() {
            result.rtlsdr = Some(rtlsdr_cal);
        }

        // Calibrate audio capture
        if let Ok(audio_cal) = self.calibrate_audio_capture() {
            result.audio_capture = Some(audio_cal);
        }

        // Calibrate audio playback
        if let Ok(playback_cal) = self.calibrate_audio_playback() {
            result.audio_playback = Some(playback_cal);
        }

        // Measure sync offset
        result.sync_offset_ms = self.measure_sync_offset();

        // Calculate latency budget
        result.latency_budget = self.calculate_latency_budget();

        self.results.insert(result.timestamp.clone(), result.clone());

        Ok(result)
    }

    /// Calibrate RTL-SDR device
    fn calibrate_rtlsdr(&self) -> Result<RtlSdrCalibration> {
        use crate::hardware::RtlSdrConfig;

        let config = RtlSdrConfig::default();
        let mut device = RtlSdrDevice::new_mock(config.clone());

        // Measure actual sample rate
        let start = Instant::now();
        let mut buffer = vec![num_complex::Complex::new(0.0f32, 0.0f32); 10240];
        let _ = device.read_iq(&mut buffer);
        let elapsed = start.elapsed();

        let actual_sample_rate = (buffer.len() as f64 / elapsed.as_secs_f64()) as u32;
        let frequency_error_ppm = ((actual_sample_rate as f32 - config.sample_rate as f32)
            / config.sample_rate as f32) * 1e6;

        // Measure noise floor
        let noise_floor_db = self.measure_noise_floor_rtlsdr(&mut device);

        Ok(RtlSdrCalibration {
            actual_sample_rate,
            frequency_error_ppm,
            gain_calibration_db: config.gain_db,
            noise_floor_db,
        })
    }

    /// Calibrate audio capture
    fn calibrate_audio_capture(&self) -> Result<AudioCaptureCalibration> {
        use crate::hardware::AudioConfig;

        let config = AudioConfig::default();
        let mut device = AudioCapture::new(config.clone())?;

        // Measure actual sample rate
        let start = Instant::now();
        let mut buffer = vec![0.0f32; 19200]; // 100ms at 192kHz
        let _ = device.read(&mut buffer);
        let elapsed = start.elapsed();

        let actual_sample_rate = (buffer.len() as f64 / elapsed.as_secs_f64()) as u32;

        // Measure channel balance (for multi-channel)
        let channel_balance = vec![1.0f32; config.channels as usize];

        // Measure noise floor
        let noise_floor_db = self.measure_noise_floor_audio(&mut device);

        Ok(AudioCaptureCalibration {
            actual_sample_rate,
            channel_balance,
            frequency_response: vec![1.0f32; 256], // Placeholder
            noise_floor_db,
        })
    }

    /// Calibrate audio playback
    fn calibrate_audio_playback(&self) -> Result<AudioPlaybackCalibration> {
        use crate::hardware::AudioConfig;

        let config = AudioConfig::default();
        let _device = AudioPlayback::new(config.clone())?;

        Ok(AudioPlaybackCalibration {
            actual_sample_rate: config.sample_rate,
            pdm_calibration: vec![1.0f32; 256], // Placeholder
            output_level_db: -3.0, // Default headroom
        })
    }

    /// Measure sync offset between devices
    fn measure_sync_offset(&self) -> f64 {
        // In a real implementation, this would:
        // 1. Send a known test signal through all devices
        // 2. Measure arrival times
        // 3. Calculate relative offsets
        // For now, return estimated value
        0.5 // 0.5ms estimated sync offset
    }

    /// Calculate latency budget
    fn calculate_latency_budget(&self) -> LatencyBudget {
        let budget = LatencyBudget {
            rf_capture_ms: 5.0,
            audio_capture_ms: 5.0,
            dsp_processing_ms: 10.0,
            ml_inference_ms: 8.0,
            output_ms: 5.0,
            total_ms: 33.0,
            target_ms: 35.0,
            margin_ms: 2.0,
        };
        
        budget
    }

    /// Measure noise floor for RTL-SDR
    fn measure_noise_floor_rtlsdr(&self, device: &mut RtlSdrDevice) -> f32 {
        let mut buffer = vec![num_complex::Complex::new(0.0f32, 0.0f32); 4096];
        let _ = device.read_iq(&mut buffer);
        
        let power: f32 = buffer.iter().map(|s| s.norm_sqr()).sum::<f32>() / buffer.len() as f32;
        10.0 * power.log10()
    }

    /// Measure noise floor for audio
    fn measure_noise_floor_audio(&self, device: &mut AudioCapture) -> f32 {
        let mut buffer = vec![0.0f32; 4096];
        let _ = device.read(&mut buffer);
        
        let power: f32 = buffer.iter().map(|&s| s * s).sum::<f32>() / buffer.len() as f32;
        20.0 * power.sqrt().log10()
    }

    /// Get calibration report
    pub fn report(&self) -> String {
        if let Some((_, result)) = self.results.iter().last() {
            let mut report = String::from("=== Calibration Report ===\n\n");
            
            report.push_str(&format!("Timestamp: {}\n\n", result.timestamp));
            
            if let Some(ref rtl) = result.rtlsdr {
                report.push_str("RTL-SDR:\n");
                report.push_str(&format!("  Sample Rate: {} Hz (error: {:.2} ppm)\n", 
                    rtl.actual_sample_rate, rtl.frequency_error_ppm));
                report.push_str(&format!("  Noise Floor: {:.2} dB\n\n", rtl.noise_floor_db));
            }
            
            if let Some(ref audio) = result.audio_capture {
                report.push_str("Audio Capture:\n");
                report.push_str(&format!("  Sample Rate: {} Hz\n", audio.actual_sample_rate));
                report.push_str(&format!("  Noise Floor: {:.2} dB\n\n", audio.noise_floor_db));
            }
            
            report.push_str("Latency Budget:\n");
            report.push_str(&format!("  RF Capture: {:.1} ms\n", result.latency_budget.rf_capture_ms));
            report.push_str(&format!("  Audio Capture: {:.1} ms\n", result.latency_budget.audio_capture_ms));
            report.push_str(&format!("  DSP Processing: {:.1} ms\n", result.latency_budget.dsp_processing_ms));
            report.push_str(&format!("  ML Inference: {:.1} ms\n", result.latency_budget.ml_inference_ms));
            report.push_str(&format!("  Output: {:.1} ms\n", result.latency_budget.output_ms));
            report.push_str(&format!("  Total: {:.1} ms (target: {:.1} ms, margin: {:.1} ms)\n",
                result.latency_budget.total_ms,
                result.latency_budget.target_ms,
                result.latency_budget.margin_ms));
            
            report
        } else {
            "No calibration results available".into()
        }
    }

    /// Get latest calibration result
    pub fn latest(&self) -> Option<&CalibrationResult> {
        self.results.values().last()
    }
}

impl Default for Calibration {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for LatencyBudget {
    fn default() -> Self {
        Self {
            rf_capture_ms: 5.0,
            audio_capture_ms: 5.0,
            dsp_processing_ms: 10.0,
            ml_inference_ms: 8.0,
            output_ms: 5.0,
            total_ms: 33.0,
            target_ms: 35.0,
            margin_ms: 2.0,
        }
    }
}

impl DeviceCalibration {
    /// Create a new device calibration
    pub fn new(device_name: String) -> Self {
        Self {
            device_name,
            calibration_data: Vec::new(),
        }
    }

    /// Add calibration data point
    pub fn add_data(&mut self, value: f32) {
        self.calibration_data.push(value);
    }

    /// Get average calibration value
    pub fn average(&self) -> f32 {
        if self.calibration_data.is_empty() {
            0.0
        } else {
            self.calibration_data.iter().sum::<f32>() / self.calibration_data.len() as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calibration_creation() {
        let cal = Calibration::new();
        assert!(cal.results.is_empty());
    }

    #[test]
    fn test_device_calibration() {
        let mut dev_cal = DeviceCalibration::new("test".into());
        dev_cal.add_data(1.0);
        dev_cal.add_data(2.0);
        dev_cal.add_data(3.0);
        
        assert!((dev_cal.average() - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_latency_budget_default() {
        let budget = LatencyBudget::default();
        assert!(budget.total_ms <= budget.target_ms);
        assert!(budget.margin_ms > 0.0);
    }
}
