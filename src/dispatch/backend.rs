use thiserror::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

#[derive(Error, Debug)]
pub enum BackendError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Hardware error: {0}")]
    Hardware(String),
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

pub trait SignalBackend: Send {
    fn write_pcm(&mut self, samples: &[f32]) -> Result<(), BackendError>;
    fn flush(&mut self) -> Result<(), BackendError>;
    fn describe(&self) -> &str;
}

pub struct FileBackend {
    writer: BufWriter<File>,
    path: String,
}

impl FileBackend {
    pub fn new(path_str: &str) -> Result<Self, BackendError> {
        let path = Path::new(path_str);

        // Strict Path Constraint: No test data in production
        if path.components().any(|c| {
            let s = c.as_os_str().to_string_lossy();
            s == "tests" || s == "examples"
        }) {
            return Err(BackendError::InvalidData("Test files must not be used in production".to_string()));
        }

        let file = File::create(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
            path: path_str.to_string(),
        })
    }
}

impl SignalBackend for FileBackend {
    fn write_pcm(&mut self, samples: &[f32]) -> Result<(), BackendError> {
        for &s in samples {
            self.writer.write_all(&s.to_le_bytes())?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), BackendError> {
        self.writer.flush()?;
        Ok(())
    }

    fn describe(&self) -> &str {
        &self.path
    }
}

pub struct AudioBackend {
    description: String,
}

impl AudioBackend {
    pub fn new(device_name: &str) -> Self {
        Self {
            description: format!("Audio({})", device_name),
        }
    }
}

impl SignalBackend for AudioBackend {
    fn write_pcm(&mut self, _samples: &[f32]) -> Result<(), BackendError> {
        Ok(())
    }
    fn flush(&mut self) -> Result<(), BackendError> { Ok(()) }
    fn describe(&self) -> &str { &self.description }
}
