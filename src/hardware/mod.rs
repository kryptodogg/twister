//! Hardware abstraction layer

pub mod audio;
#[cfg(feature = "rtlsdr")]
pub mod rtlsdr;
pub mod traits;

pub use audio::{AudioCapture, AudioPlayback};
#[cfg(feature = "rtlsdr")]
pub use rtlsdr::RtlSdrDevice;
pub use traits::*;
