use crate::hardware::{SignalBackend, BackendError};

pub struct AudioDevice {
    pub name: String,
}

impl AudioDevice {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }
}

impl SignalBackend for AudioDevice {
    fn write_iq(&mut self, _samples: &[f32]) -> Result<(), BackendError> {
        Err(BackendError::InvalidData("Audio device does not support IQ".to_string()))
    }

    fn write_pcm(&mut self, _samples: &[f32]) -> Result<(), BackendError> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), BackendError> { Ok(()) }

    fn describe(&self) -> &str { &self.name }
}
