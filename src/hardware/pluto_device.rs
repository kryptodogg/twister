use crate::hardware::{SignalBackend, BackendError};

pub struct PlutoDevice;

impl SignalBackend for PlutoDevice {
    fn write_iq(&mut self, _samples: &[f32]) -> Result<(), BackendError> {
        Ok(())
    }

    fn write_pcm(&mut self, _samples: &[f32]) -> Result<(), BackendError> {
        Err(BackendError::InvalidData("Pluto+ expects IQ samples".to_string()))
    }

    fn flush(&mut self) -> Result<(), BackendError> { Ok(()) }

    fn describe(&self) -> &str { "Pluto+" }
}
