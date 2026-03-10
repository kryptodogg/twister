//! Error handling

use anyhow::Error as AnyhowError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Hardware(String),
    DSP(String),
    Pipeline(String),
    Unknown(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Hardware(msg) => write!(f, "Hardware error: {}", msg),
            Error::DSP(msg) => write!(f, "DSP error: {}", msg),
            Error::Pipeline(msg) => write!(f, "Pipeline error: {}", msg),
            Error::Unknown(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<AnyhowError> for Error {
    fn from(err: AnyhowError) -> Self {
        Error::Unknown(err.to_string())
    }
}
