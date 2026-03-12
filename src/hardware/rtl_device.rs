use crate::hardware::{SignalBackend, BackendError};
#[cfg(feature = "rtlsdr")]
use crate::hardware::rtlsdr::RtlSdrEngine;

/// RTL-SDR Backend (librtlsdr wrapper)
pub struct RtlDevice {
    pub index: u32,
    #[cfg(feature = "rtlsdr")]
    engine: Option<RtlSdrEngine>,
}

impl RtlDevice {
    pub fn new(index: u32) -> Self {
        Self {
            index,
            #[cfg(feature = "rtlsdr")]
            engine: None
        }
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
