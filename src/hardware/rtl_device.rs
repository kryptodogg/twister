use crate::hardware::{SignalBackend, BackendError};

/// RTL-SDR Backend (librtlsdr wrapper)
pub struct RtlDevice {
    pub device_index: u32,
}

impl RtlDevice {
    pub fn new(index: u32) -> Self {
        Self { device_index: index }
    }
}

impl SignalBackend for RtlDevice {
    fn write_iq(&mut self, _samples: &[f32]) -> Result<(), BackendError> {
        Err(BackendError::InvalidData("RTL-SDR is RX-only".to_string()))
    }

    fn write_pcm(&mut self, _samples: &[f32]) -> Result<(), BackendError> {
        Err(BackendError::InvalidData("RTL-SDR expects IQ samples".to_string()))
    }

    fn flush(&mut self) -> Result<(), BackendError> { Ok(()) }
    fn describe(&self) -> &str { "RTL-SDR" }
}
