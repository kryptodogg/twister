use crate::hardware::{SignalBackend, BackendError};
#[cfg(feature = "rtlsdr")]
use crate::hardware::rtlsdr::RtlSdrEngine;

/// RTL-SDR Backend (librtlsdr wrapper)
/// 
/// # Send Safety
/// The underlying RTLSDRDevice does not implement Send due to its C FFI handle.
/// We wrap it in a Send-safe wrapper since our usage pattern ensures sequential access.
pub struct RtlDevice {
    pub index: u32,
    #[cfg(feature = "rtlsdr")]
    engine: Option<RtlSdrEngineWrapper>,
}

#[cfg(feature = "rtlsdr")]
struct RtlSdrEngineWrapper(RtlSdrEngine);

#[cfg(feature = "rtlsdr")]
// SAFETY: RTLSDRDevice is backed by a C library handle that is safe to send across threads
// when accessed sequentially through our ring buffer. Concurrent access is prevented by
// the dispatch loop's ownership model.
unsafe impl Send for RtlSdrEngineWrapper {}

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
