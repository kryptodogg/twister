//! Control policy for RF-Audio fusion ANC
//!
//! Provides:
//! - SNR estimation at rear mic
//! - Mode decision: ANC (SNR<108dB + RF stress) vs Music (SNR good + RF calm) vs Neutral
//! - Fade logic for smooth transitions

pub mod snr;
pub mod mode;
pub mod fade;
pub mod policy;

pub use snr::{SNREstimator, SNREstimate};
pub use mode::{ModeDecision, ModeConfig};
pub use fade::{FadeController, FadeState};
pub use policy::{ControlPolicy, PolicyConfig, ControlOutput};
