//! Mamba autoencoder module for RF-Audio fusion
//!
//! # V3 Architecture Note
//! SSAMBA (State Space Audio Mamba) is being replaced by UnifiedFieldMamba with SAST token ordering.
//! This module contains only the base model. Inference and training are being rewritten.
//!
//! Track B reference: UnifiedFieldMamba implementation supersedes this architecture.

pub mod model;
// inference and training deleted — V3 UnifiedFieldMamba replaces

pub use model::SSAMBA as MambaAutoencoder;
// SSAMBAConfig, ControlHead deleted — V3 uses different config structure

/// Mamba context length (432 bins)
pub const MAMBA_CONTEXT_LEN: usize = 432;
/// Mamba input bins (432 bins)
pub const MAMBA_INPUT_BINS: usize = 432;

/// Compute RMS dB from spectrum (utility for DSP)
pub fn compute_rms_db(x: &[f32]) -> f32 {
    let sum_sq: f32 = x.iter().map(|&v| v * v).sum();
    if sum_sq < 1e-10 { return -100.0; }
    let rms = (sum_sq / x.len() as f32).sqrt();
    20.0 * rms.log10()
}

/// Training pair for Mamba autoencoder (V2 format — being replaced)
#[derive(Debug, Clone)]
pub struct TrainingPair {
    pub tx_spectrum: Vec<f32>,
    pub rx_spectrum: Vec<f32>,
    pub label: Vec<f32>,
}
