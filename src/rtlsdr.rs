// src/rtlsdr.rs — Safe Rust Wrapper for RTL-SDR Hardware
//
// Provides memory-safe abstractions over the rtlsdr_ffi FFI bindings.
// Supports I/Q capture from 10 kHz to 300 MHz at up to 2.4 MS/s.

#[cfg(feature = "rtlsdr")]
use crate::rtlsdr_ffi;
use num_complex::Complex;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

/// Default sample buffer size (256 KB = 128K I/Q samples at 2 bytes each)
pub const DEFAULT_BUFFER_SIZE: usize = 256 * 1024;

/// Minimum sample rate (250 KS/s)
pub const MIN_SAMPLE_RATE: u32 = 250_000;

/// Maximum sample rate (2.4 MS/s)
pub const MAX_SAMPLE_RATE: u32 = 2_400_000;

/// Minimum frequency (1 Hz)
pub const MIN_FREQ_HZ: u32 = 1;

/// Maximum frequency (300 MHz)
pub const MAX_FREQ_HZ: u32 = 300_000_000;

/// RTL-SDR device handle with safe abstractions
#[cfg(feature = "rtlsdr")]
pub struct RtlSdrDevice {
    raw: *mut rtlsdr_ffi::rtlsdr_dev_t,
    sample_rate: u32,
    center_freq: u32,
    gain_db: f32,
    agc_enabled: bool,
    is_open: AtomicBool,
}

unsafe impl Send for RtlSdrDevice {}
unsafe impl Sync for RtlSdrDevice {}

#[cfg(feature = "rtlsdr")]
impl RtlSdrDevice {
    /// Enumerate available RTL-SDR devices
    pub fn enumerate() -> Vec<DeviceInfo> {
        let count = unsafe { rtlsdr_ffi::rtlsdr_get_device_count() };
        let mut devices = Vec::with_capacity(count as usize);

        for i in 0..count {
            devices.push(DeviceInfo {
                index: i,
                name: unsafe {
                    let ptr = rtlsdr_ffi::rtlsdr_get_device_name(i);
                    if ptr.is_null() {
                        "Unknown".to_string()
                    } else {
                        format!("Device {}", i)
                    }
                },
            });
        }

        devices
    }

    /// Open RTL-SDR device by index (0 = first device)
    pub fn open(index: u32) -> anyhow::Result<Self> {
        let mut raw: *mut rtlsdr_ffi::rtlsdr_dev_t = ptr::null_mut();

        let result = unsafe { rtlsdr_ffi::rtlsdr_open(&mut raw, index) };
        if !rtlsdr_ffi::is_rtl_success(result) {
            anyhow::bail!(
                "Failed to open RTL-SDR device {}: {}",
                index,
                rtlsdr_ffi::rtl_error_to_string(result)
            );
        }

        println!("[RTL-SDR] Opened device {}", index);

        let mut device = Self {
            raw,
            sample_rate: MIN_SAMPLE_RATE,
            center_freq: 100_000_000, // Default 100 MHz
            gain_db: 0.0,
            agc_enabled: true,
            is_open: AtomicBool::new(true),
        };

        // Initialize with defaults
        device.reset_buffer()?;
        device.set_sample_rate(MIN_SAMPLE_RATE)?;
        device.set_center_freq(100_000_000)?;
        device.set_agc_mode(true)?;

        Ok(device)
    }

    /// Set center frequency in Hz (10 kHz - 300 MHz)
    pub fn set_center_freq(&mut self, freq_hz: u32) -> anyhow::Result<()> {
        if freq_hz < MIN_FREQ_HZ || freq_hz > MAX_FREQ_HZ {
            anyhow::bail!(
                "Frequency {} Hz out of range ({} - {} Hz)",
                freq_hz,
                MIN_FREQ_HZ,
                MAX_FREQ_HZ
            );
        }

        let result = unsafe { rtlsdr_ffi::rtlsdr_set_center_freq(self.raw, freq_hz) };
        if !rtlsdr_ffi::is_rtl_success(result) {
            anyhow::bail!(
                "Failed to set center frequency: {}",
                rtlsdr_ffi::rtl_error_to_string(result)
            );
        }

        self.center_freq = freq_hz;
        println!(
            "[RTL-SDR] Center frequency: {:.3} MHz",
            freq_hz as f64 / 1e6
        );
        Ok(())
    }

    /// Get current center frequency in Hz
    pub fn center_freq(&self) -> u32 {
        self.center_freq
    }

    /// Set sample rate in Hz (250 KS/s - 2.4 MS/s)
    pub fn set_sample_rate(&mut self, rate_hz: u32) -> anyhow::Result<()> {
        if rate_hz < MIN_SAMPLE_RATE || rate_hz > MAX_SAMPLE_RATE {
            anyhow::bail!(
                "Sample rate {} Hz out of range ({} - {} Hz)",
                rate_hz,
                MIN_SAMPLE_RATE,
                MAX_SAMPLE_RATE
            );
        }

        let result = unsafe { rtlsdr_ffi::rtlsdr_set_sample_rate(self.raw, rate_hz) };
        if !rtlsdr_ffi::is_rtl_success(result) {
            anyhow::bail!(
                "Failed to set sample rate: {}",
                rtlsdr_ffi::rtl_error_to_string(result)
            );
        }

        self.sample_rate = rate_hz;
        println!("[RTL-SDR] Sample rate: {:.2} MS/s", rate_hz as f64 / 1e6);
        Ok(())
    }

    /// Get current sample rate in Hz
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Set tuner gain in dB (0-50 dB typical)
    pub fn set_gain(&mut self, gain_db: f32) -> anyhow::Result<()> {
        // Gain is in tenths of dB
        let gain_tenths = (gain_db * 10.0) as i32;

        let result = unsafe { rtlsdr_ffi::rtlsdr_set_tuner_gain(self.raw, gain_tenths) };
        if !rtlsdr_ffi::is_rtl_success(result) {
            anyhow::bail!(
                "Failed to set tuner gain: {}",
                rtlsdr_ffi::rtl_error_to_string(result)
            );
        }

        self.gain_db = gain_db;
        self.agc_enabled = false;
        println!("[RTL-SDR] Tuner gain: {:.1} dB (manual)", gain_db);
        Ok(())
    }

    /// Get current tuner gain in dB
    pub fn gain(&self) -> f32 {
        self.gain_db
    }

    /// Enable/disable AGC mode
    pub fn set_agc_mode(&mut self, enabled: bool) -> anyhow::Result<()> {
        let result =
            unsafe { rtlsdr_ffi::rtlsdr_set_agc_mode(self.raw, if enabled { 1 } else { 0 }) };
        if !rtlsdr_ffi::is_rtl_success(result) {
            anyhow::bail!(
                "Failed to set AGC mode: {}",
                rtlsdr_ffi::rtl_error_to_string(result)
            );
        }

        self.agc_enabled = enabled;
        println!(
            "[RTL-SDR] AGC: {}",
            if enabled { "enabled" } else { "disabled" }
        );
        Ok(())
    }

    /// Check if AGC is enabled
    pub fn agc_enabled(&self) -> bool {
        self.agc_enabled
    }

    /// Set direct sampling mode: 0 = off, 1 = I-branch, 2 = Q-branch
    pub fn set_direct_sampling(&mut self, mode: u32) -> anyhow::Result<()> {
        let result = unsafe { rtlsdr_ffi::rtlsdr_set_direct_sampling(self.raw, mode as i32) };
        if !rtlsdr_ffi::is_rtl_success(result) {
            anyhow::bail!(
                "Failed to set direct sampling (mode {}): {}",
                mode,
                rtlsdr_ffi::rtl_error_to_string(result)
            );
        }

        println!("[RTL-SDR] Direct sampling mode set to {}", mode);
        Ok(())
    }

    /// Reset sample buffer
    pub fn reset_buffer(&mut self) -> anyhow::Result<()> {
        let result = unsafe { rtlsdr_ffi::rtlsdr_reset_buffer(self.raw) };
        if !rtlsdr_ffi::is_rtl_success(result) {
            anyhow::bail!(
                "Failed to reset buffer: {}",
                rtlsdr_ffi::rtl_error_to_string(result)
            );
        }
        Ok(())
    }

    /// Read I/Q samples (blocking, synchronous)
    ///
    /// Returns interleaved I/Q as u8 samples (0-255, 127 = zero amplitude)
    pub fn read_iq_u8(&self, buf: &mut [u8]) -> anyhow::Result<usize> {
        if !self.is_open.load(Ordering::Relaxed) {
            anyhow::bail!("Device is not open");
        }

        let mut n_read: i32 = 0;
        let result = unsafe {
            rtlsdr_ffi::rtlsdr_read_sync(self.raw, buf.as_mut_ptr(), buf.len() as i32, &mut n_read)
        };

        if !rtlsdr_ffi::is_rtl_success(result) {
            anyhow::bail!("Read error: {}", rtlsdr_ffi::rtl_error_to_string(result));
        }

        Ok(n_read as usize)
    }

    /// Read I/Q samples and convert to complex f32 (normalized to [-1, 1])
    ///
    /// Input:  u8 I/Q interleaved (0-255, 127 = zero)
    /// Output: Complex<f32> with I and Q in [-1, 1]
    pub fn read_iq_f32(&self, buf: &mut [u8]) -> anyhow::Result<Vec<Complex<f32>>> {
        let n_read = self.read_iq_u8(buf)?;

        // Convert u8 I/Q to complex f32
        // u8 0-255 → f32 -1.0 to 1.0 (127 = 0)
        let samples = buf[..n_read]
            .chunks_exact(2)
            .map(|chunk| {
                let i = (chunk[0] as f32 - 127.0) / 127.0;
                let q = (chunk[1] as f32 - 127.0) / 127.0;
                Complex::new(i, q)
            })
            .collect();

        Ok(samples)
    }

    /// Convert u8 I/Q buffer to complex f32 (for external use)
    pub fn iq_u8_to_f32(buf: &[u8]) -> Vec<Complex<f32>> {
        buf.chunks_exact(2)
            .map(|chunk| {
                let i = (chunk[0] as f32 - 127.0) / 127.0;
                let q = (chunk[1] as f32 - 127.0) / 127.0;
                Complex::new(i, q)
            })
            .collect()
    }

    /// Get device information
    pub fn device_info(&self) -> DeviceInfo {
        DeviceInfo {
            index: 0, // Would need to store this during open
            name: "RTL-SDR".to_string(),
        }
    }

    /// Check if device is still open
    pub fn is_open(&self) -> bool {
        self.is_open.load(Ordering::Relaxed)
    }
}

impl Drop for RtlSdrDevice {
    fn drop(&mut self) {
        if self.is_open.load(Ordering::Relaxed) {
            unsafe {
                let result = rtlsdr_ffi::rtlsdr_close(self.raw);
                if rtlsdr_ffi::is_rtl_success(result) {
                    println!("[RTL-SDR] Device closed");
                } else {
                    eprintln!(
                        "[RTL-SDR] Close error: {}",
                        rtlsdr_ffi::rtl_error_to_string(result)
                    );
                }
            }
            self.is_open.store(false, Ordering::Relaxed);
        }
    }
}

/// Device information structure
#[cfg(feature = "rtlsdr")]
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub index: u32,
    pub name: String,
}

/// RTL-SDR capture engine for continuous streaming
#[cfg(feature = "rtlsdr")]
pub struct RtlSdrEngine {
    device: RtlSdrDevice,
    buffer: Vec<u8>,
    running: AtomicBool,
}

#[cfg(feature = "rtlsdr")]
impl RtlSdrEngine {
    /// Create new RTL-SDR engine with default device
    pub fn new() -> anyhow::Result<Self> {
        let device = RtlSdrDevice::open(0)?;
        let buffer = vec![0u8; DEFAULT_BUFFER_SIZE];

        Ok(Self {
            device,
            buffer,
            running: AtomicBool::new(false),
        })
    }

    /// Create RTL-SDR engine with specific device index
    pub fn with_device(index: u32) -> anyhow::Result<Self> {
        let device = RtlSdrDevice::open(index)?;
        let buffer = vec![0u8; DEFAULT_BUFFER_SIZE];

        Ok(Self {
            device,
            buffer,
            running: AtomicBool::new(false),
        })
    }

    /// Configure for HF reception (10 kHz - 24 MHz)
    pub fn configure_hf(&mut self) -> anyhow::Result<()> {
        self.device.set_direct_sampling(2)?; // Q-branch (mode 2) is best for V3 direct sampling
        self.device.set_agc_mode(true)?;
        Ok(())
    }

    /// Configure for VHF/UHF reception (24 MHz - 300 MHz)
    pub fn configure_vhf_uhf(&mut self) -> anyhow::Result<()> {
        self.device.set_direct_sampling(0)?; // Disable direct sampling
        self.device.set_agc_mode(true)?;
        Ok(())
    }

    /// Tune to frequency
    pub fn tune(&mut self, freq_hz: u32) -> anyhow::Result<()> {
        self.device.set_center_freq(freq_hz)
    }

    /// Set sample rate
    pub fn set_sample_rate(&mut self, rate_hz: u32) -> anyhow::Result<()> {
        self.device.set_sample_rate(rate_hz)
    }

    /// Set tuner gain in dB (0-50 dB typical). Disables AGC.
    pub fn set_gain(&mut self, gain_db: f32) -> anyhow::Result<()> {
        self.device.set_gain(gain_db)
    }

    /// Enable or disable AGC mode.
    pub fn set_agc_mode(&mut self, enabled: bool) -> anyhow::Result<()> {
        self.device.set_agc_mode(enabled)
    }

    /// Read I/Q samples as complex f32
    pub fn read_iq(&mut self) -> anyhow::Result<Vec<Complex<f32>>> {
        self.device.read_iq_f32(&mut self.buffer)
    }

    /// Get reference to underlying device
    pub fn device(&self) -> &RtlSdrDevice {
        &self.device
    }

    /// Start/stop capture (for async operation)
    pub fn set_running(&self, running: bool) {
        self.running.store(running, Ordering::Relaxed);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn is_open(&self) -> bool {
        self.device.is_open()
    }
}

impl Default for RtlSdrEngine {
    fn default() -> Self {
        Self::new().expect("Failed to initialize RTL-SDR")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_devices() {
        let devices = RtlSdrDevice::enumerate();
        // May be empty if no hardware connected, but shouldn't panic
        println!("Found {} RTL-SDR devices", devices.len());
    }

    #[test]
    fn test_iq_conversion() {
        // Test u8 → f32 conversion
        let test_data = [127u8, 127, 255, 0, 0, 0]; // Zero, max I, max Q
        let complex = RtlSdrDevice::iq_u8_to_f32(&test_data);

        assert_eq!(complex.len(), 3);
        assert!((complex[0].re - 0.0).abs() < 0.01);
        assert!((complex[0].im - 0.0).abs() < 0.01);
        assert!((complex[1].re - 1.0).abs() < 0.01);
        assert!((complex[2].im - (-1.0)).abs() < 0.01);
    }
}
