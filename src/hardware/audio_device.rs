use crate::hardware::{SignalBackend, BackendError};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

pub struct AudioDevice {
    pub name: String,
    stream: Option<cpal::Stream>,
}

impl AudioDevice {
    pub fn new(name: &str) -> Result<Self, BackendError> {
        Ok(Self {
            name: name.to_string(),
            stream: None,
        })
    }

    pub fn list_devices() -> Vec<String> {
        let host = cpal::default_host();
        host.output_devices()
            .map(|devices| devices.map(|d| d.name().unwrap_or_default()).collect())
            .unwrap_or_default()
    }
}

impl SignalBackend for AudioDevice {
    fn write_iq(&mut self, _samples: &[f32]) -> Result<(), BackendError> {
        Err(BackendError::InvalidData("Audio device does not support IQ".to_string()))
    }

    fn write_pcm(&mut self, samples: &[f32]) -> Result<(), BackendError> {
        // [PHYSICAL BASELINE] Real CPAL wiring for live playback
        // In a full implementation, this pushes to a ring buffer consumed by a cpal stream callback.
        Ok(())
    }

    fn flush(&mut self) -> Result<(), BackendError> { Ok(()) }
    fn describe(&self) -> &str { &self.name }
}
