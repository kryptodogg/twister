//! Hardware Abstraction Layer — V3 Track 0-B
//!
//! # V3 Architecture Notes
//! - All devices implement SignalBackend trait
//! - QpcTimer provides forensic timestamps (Windows QPC / Linux CLOCK_MONOTONIC_RAW)
//! - rtlsdr is optional (feature-gated)
//! - calibration.rs deleted — being rewritten for V3

pub mod audio_device;
pub mod audio;
// pub mod calibration; — deleted, V3 rewrite
pub mod gpu;
pub mod pluto_device;
pub mod rtl_device;
#[cfg(feature = "rtlsdr")]
pub mod rtlsdr;
pub mod sync;
pub mod traits;

pub use audio_device::AudioDevice;
pub use pluto_device::PlutoDevice;
pub use rtl_device::RtlDevice;

use crate::utils::latency::QpcTimer;
use std::sync::Arc;
use rustfft::{FftPlanner, num_complex::Complex};

#[derive(Debug)]
pub enum BackendError {
    DeviceNotFound(String),
    ConfigurationError(String),
    IoError(String),
    InvalidData(String),
}

impl From<std::io::Error> for BackendError {
    fn from(err: std::io::Error) -> Self {
        BackendError::IoError(err.to_string())
    }
}

pub trait SignalBackend: Send {
    fn write_iq(&mut self, samples: &[f32]) -> Result<(), BackendError>;
    fn write_pcm(&mut self, samples: &[f32]) -> Result<(), BackendError>;
    fn flush(&mut self) -> Result<(), BackendError>;
    fn describe(&self) -> &str;
}

pub struct HardwareRegistry {
    pub timer: Arc<QpcTimer>,
    pub audio_status: String,
    pub rtl_status: String,
    pub pluto_status: String,
}

impl HardwareRegistry {
    pub fn new(timer: Arc<QpcTimer>) -> Self {
        Self {
            timer,
            audio_status: "RED".to_string(),
            rtl_status: "RED".to_string(),
            pluto_status: "RED".to_string(),
        }
    }

    pub fn scan(&mut self) {
        self.audio_status = "GREEN".to_string();
        self.rtl_status = "RED".to_string();
        self.pluto_status = "RED".to_string();
    }
}

// ── Physical Baseline Synthesis (Step 5) ─────────────────────────────────────

pub fn generate_cw_iq(num_samples: usize) -> Vec<f32> {
    let mut buf = Vec::with_capacity(num_samples * 2);
    for _ in 0..num_samples {
        buf.push(1.0); // I
        buf.push(0.0); // Q
    }
    buf
}

pub fn generate_sinc_tone_iq(freq_hz: f32, sample_rate: f32, num_samples: usize, kernel_width: usize) -> Vec<f32> {
    let mut iq = Vec::with_capacity(num_samples * 2);
    let dt = 1.0 / sample_rate;
    for i in 0..num_samples {
        let t = i as f32 * dt;
        let val = (2.0 * std::f32::consts::PI * freq_hz * t).sin();

        let mut filtered = 0.0;
        let half_width = kernel_width as f32 / 2.0;
        for k in 0..kernel_width {
            let offset = k as f32 - half_width;
            let sinc_val = if offset == 0.0 { 1.0 } else { (std::f32::consts::PI * offset).sin() / (std::f32::consts::PI * offset) };
            filtered += val * sinc_val;
        }

        iq.push(filtered / kernel_width as f32); // I
        iq.push(0.0); // Q
    }
    iq
}

pub fn generate_wofdm_baseline_iq(num_symbols: usize) -> Vec<f32> {
    let fft_size = 512;
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_inverse(fft_size);

    let mut result = Vec::with_capacity(num_symbols * fft_size * 2);
    for _ in 0..num_symbols {
        let mut spectrum: Vec<Complex<f32>> = (0..fft_size)
            .map(|_| Complex::new(1.0, 0.0))
            .collect();

        fft.process(&mut spectrum);

        for s in spectrum {
            result.push(s.re / (fft_size as f32).sqrt());
            result.push(s.im / (fft_size as f32).sqrt());
        }
    }
    result
}
