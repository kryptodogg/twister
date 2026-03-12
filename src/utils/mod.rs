//! Utilities — V3 Core Infrastructure
//! 
//! # V3 Architecture Notes
//! - `atomic::AtomicF32` — ONLY atomic f32 wrapper in codebase
//! - `latency::QpcTimer` — Windows QPC / Linux CLOCK_MONOTONIC_RAW timestamps
//! - All other utilities are secondary to Track 0-A foundation

pub mod atomic;
pub mod config;
pub mod error;
pub mod latency;
pub mod logging;
pub mod metrics;

pub use atomic::AtomicF32;
pub use config::{SystemConfig, AudioConfig, RtlSdrConfig};
pub use error::{Error, Result};
pub use latency::QpcTimer;
pub use logging::init_logging;
