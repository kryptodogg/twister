//! RTL-SDR device wrapper

#[cfg(feature = "rtlsdr")]
use anyhow::{Result, anyhow};
#[cfg(feature = "rtlsdr")]
use num_complex::Complex32;

#[cfg(feature = "rtlsdr")]
pub type IQSample = Complex32;

/// RTL-SDR configuration
#[cfg(feature = "rtlsdr")]
#[derive(Debug, Clone)]
pub struct RtlSdrConfig {
    pub sample_rate: u32,
    pub center_freq: f64,
    pub gain_db: f32,
    pub bandwidth: u32,
}

#[cfg(feature = "rtlsdr")]
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

/// IQ sample buffer
#[cfg(feature = "rtlsdr")]
pub type IqBuffer = Vec<Complex32>;

/// RTL-SDR device
#[cfg(feature = "rtlsdr")]
pub struct RtlSdrDevice {
    config: RtlSdrConfig,
    device: Option<rtlsdr::RTLSDRDevice>,
}

#[cfg(feature = "rtlsdr")]
impl RtlSdrDevice {
    pub fn new(config: RtlSdrConfig) -> Result<Self> {
        match rtlsdr::open(0) {
            Ok(device) => Ok(Self { config, device: Some(device) }),
            Err(err) => {
                log::warn!("RTL-SDR device not available: {:?}", err);
                Ok(Self { config, device: None })
            }
        }
    }

    pub fn is_available(&self) -> bool {
        self.device.is_some()
    }

    pub fn default_config() -> RtlSdrConfig {
        RtlSdrConfig::default()
    }

    /// Capture IQ samples
    pub fn capture(&mut self, num_samples: usize) -> Result<IqBuffer> {
        let Some(device) = &mut self.device else {
            return Ok(vec![Complex32::default(); num_samples]);
        };

        // Configure device
        device.set_sample_rate(self.config.sample_rate as u32);
        device.set_center_freq(self.config.center_freq as u32);
        device.set_tuner_gain_mode(true); // Manual gain
        device.set_tuner_gain((self.config.gain_db * 10.0) as i32);
        device.set_tuner_bandwidth(self.config.bandwidth as u32);

        let buffer = device.read_sync((num_samples * 2) as usize).map_err(|err| anyhow!("RTL-SDR read error: {:?}", err))?;
        let bytes_read = buffer.len();
        
        // Convert u8 IQ pairs to Complex32 (centered at 127, scaled)
        Ok(buffer[..bytes_read as usize]
            .chunks(2)
            .filter_map(|chunk: &[u8]| {
                if chunk.len() == 2 {
                    let i = (chunk[0] as f32 - 127.0) / 127.0;
                    let q = (chunk[1] as f32 - 127.0) / 127.0;
                    Some(Complex32::new(i, q))
                } else {
                    None
                }
            })
            .collect())
    }

    /// Set center frequency
    #[cfg(feature = "rtlsdr")]
    pub fn set_frequency(&mut self, freq_hz: f64) -> Result<()> {
        self.config.center_freq = freq_hz;
        if let Some(ref mut device) = self.device {
            device.set_center_freq(freq_hz as u32);
        }
        Ok(())
    }

    /// Set gain
    #[cfg(feature = "rtlsdr")]
    pub fn set_gain(&mut self, gain_db: f32) -> Result<()> {
        self.config.gain_db = gain_db;
        if let Some(ref mut device) = self.device {
            device.set_tuner_gain((gain_db * 10.0) as i32);
        }
        Ok(())
    }

    /// Get device info
    #[cfg(feature = "rtlsdr")]
    pub fn get_device_info(&mut self) -> Option<String> {
        self.device.as_mut().and_then(|dev| {
            // get_usb_strings() returns Result<USBStrings, RTLSDRError>
            // We need to handle the Result properly
            match dev.get_usb_strings() {
                Ok(usb_strings) => Some(format!("{} {} ({})", usb_strings.manufacturer, usb_strings.product, usb_strings.serial)),
                Err(_) => None,
            }
        })
    }
}
