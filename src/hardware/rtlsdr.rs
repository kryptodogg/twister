#[cfg(feature = "rtlsdr")]
use anyhow::{Result, anyhow};
#[cfg(feature = "rtlsdr")]
use num_complex::Complex32;

#[cfg(feature = "rtlsdr")]
pub struct RtlSdrEngine {
    device: Option<rtlsdr::RTLSDRDevice>,
}

#[cfg(feature = "rtlsdr")]
impl RtlSdrEngine {
    pub fn with_device(index: u32) -> Result<Self> {
        // SAFETY: RTL-SDR device index is always a small non-negative value (0-7)
        // Conversion from u32 to i32 is safe here.
        match rtlsdr::open(index as i32) {
            Ok(device) => Ok(Self { device: Some(device) }),
            Err(e) => Err(anyhow!("Failed to open RTL-SDR device {}: {:?}", index, e)),
        }
    }

    pub fn set_sample_rate(&mut self, rate: u32) -> Result<()> {
        self.device.as_mut().ok_or(anyhow!("Device not open"))?.set_sample_rate(rate).map_err(|e| anyhow!("{:?}", e))
    }

    pub fn tune(&mut self, freq_hz: u32) -> Result<()> {
        self.device.as_mut().ok_or(anyhow!("Device not open"))?.set_center_freq(freq_hz).map_err(|e| anyhow!("{:?}", e))
    }

    pub fn read_iq(&mut self) -> Result<Vec<Complex32>> {
        let dev = self.device.as_mut().ok_or(anyhow!("Device not open"))?;
        // Raw IQ Read (BSS Policy: Unfiltered)
        let buffer = dev.read_sync(16384).map_err(|e| anyhow!("{:?}", e))?;
        Ok(buffer.chunks(2)
            .map(|c| {
                let i = (c[0] as f32 - 127.5) / 128.0;
                let q = (c[1] as f32 - 127.5) / 128.0;
                Complex32::new(i, q)
            }).collect())
    }

    pub fn is_open(&self) -> bool { self.device.is_some() }
}
