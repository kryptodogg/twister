//! Mamba autoencoder module for RF-Audio fusion
//!
//! Implements SSAMBA (State Space Audio Mamba) architecture:
//! - Input: [RF_PSD(256) | Audio_PSD(128) | TDOA(16) | ANC_state(32)] = 432
//! - Latent: 64-D representation
//! - Control heads: mode logits (3) + desired_snr_db (1)

pub mod model;
pub mod inference;
pub mod training;

pub use model::{SSAMBA as MambaAutoencoder, SSAMBAConfig, MambaBlock, ControlHead};
pub use inference::{MambaInference, InferenceResult};
pub use training::{MambaTrainer as OnlineTrainer, TrainingConfig};

/// Mamba context length (432 bins)
pub const MAMBA_CONTEXT_LEN: usize = 432;
/// Mamba input bins (432 bins)
pub const MAMBA_INPUT_BINS: usize = 432;

pub fn compute_rms_db(x: &[f32]) -> f32 {
    let sum_sq: f32 = x.iter().map(|&v| v * v).sum();
    if sum_sq < 1e-10 { return -100.0; }
    let rms = (sum_sq / x.len() as f32).sqrt();
    20.0 * rms.log10()
}

/// Training metrics for UI telemetry
#[derive(Debug, Clone, Default)]
pub struct TrainingMetrics {
    pub epoch: u32,
    pub batch_count: u32,
    pub avg_loss: f32,
    pub total_events: usize,
    pub avg_confidence: f32,
}

/// Training pair for Mamba autoencoder
#[derive(Debug, Clone)]
pub struct TrainingPair {
    pub tx_spectrum: Vec<f32>,
    pub rx_spectrum: Vec<f32>,
    pub label: Vec<f32>,
}
