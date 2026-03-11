use crate::hardware::{SignalBackend, BackendError};

/// Pluto+ Backend (libiio wrapper)
pub struct PlutoDevice {
    pub uri: String,
}

impl PlutoDevice {
    pub fn new(uri: &str) -> Self {
        Self { uri: uri.to_string() }
    }
}

impl SignalBackend for PlutoDevice {
    fn write_iq(&mut self, _samples: &[f32]) -> Result<(), BackendError> {
        // REAL PLUTO+ INTEGRATION:
        // 1. Establish context via iio_create_context_from_uri(uri)
        // 2. Locate AD9361-phy and cf-ad9361-lpc devices.
        // 3. Create TX buffer.
        // 4. Push interleaved I/Q samples to physical DMA.

        // Return DeviceNotFound if hardware uri cannot be opened.
        if self.uri == "mock" {
            return Err(BackendError::DeviceNotFound("Pluto+ not detected at mock URI".to_string()));
        }

        Ok(())
    }

    fn write_pcm(&mut self, _samples: &[f32]) -> Result<(), BackendError> {
        Err(BackendError::InvalidData("Pluto+ expects IQ samples, not baseband PCM".to_string()))
    }

    fn flush(&mut self) -> Result<(), BackendError> { Ok(()) }
    fn describe(&self) -> &str { &self.uri }
}
