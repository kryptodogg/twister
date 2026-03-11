//! Mamba inference engine

use crate::mamba::model::{SSAMBA, SSAMBAConfig};
use crate::dsp::features::FeatureVector;
use ndarray::Array1;
use burn::tensor::Tensor;
use burn_ndarray::NdArray;

type Backend = NdArray<f32>;

/// Inference result
#[derive(Debug, Clone)]
pub struct InferenceResult {
    /// Latent representation (64-D)
    pub latent: Vec<f32>,
    /// Predicted mode (0=ANC, 1=Silence, 2=Music)
    pub mode: usize,
    /// Mode probabilities
    pub mode_probs: [f32; 3],
    /// SNR target (dB)
    pub snr_target_db: f32,
    /// Inference time (ms)
    pub inference_time_ms: f32,
    /// Anomaly score
    pub anomaly_score: f32,
    /// Signal reconstruction
    pub reconstruction: Vec<f32>,
}

/// Mamba inference engine
pub struct MambaInference {
    model: SSAMBA<Backend>,
    device: burn::tensor::Device<Backend>,
}

impl MambaInference {
    pub fn new(config: &SSAMBAConfig) -> Self {
        let device = Default::default();
        let model = SSAMBA::new(config, &device);
        Self { model, device }
    }

    pub fn from_checkpoint(_path: &str, config: &SSAMBAConfig) -> Result<Self, String> {
        let device = Default::default();
        let model = SSAMBA::new(config, &device);
        Ok(Self { model, device })
    }

    pub fn forward(&self, features: &FeatureVector) -> InferenceResult {
        let start = std::time::Instant::now();
        let feature_array = features.to_array();
        let input = Tensor::from_floats(
            feature_array.as_slice().expect("Tensor data should be contiguous"),
            &self.device,
        );
        let (latent, control) = self.model.forward(input);
        let latent_data = latent.to_data();
        let latent_vec: Vec<f32> = latent_data.as_slice().expect("Tensor data should be contiguous").to_vec();
        let mode_logits_data = control.mode_logits.to_data();
        let mode_logits_vec: Vec<f32> = mode_logits_data.as_slice().expect("Tensor data should be contiguous").to_vec();
        let max_logit = mode_logits_vec.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exp_logits: Vec<f32> = mode_logits_vec.iter().map(|&l| (l - max_logit).exp()).collect();
        let sum_exp: f32 = exp_logits.iter().sum();
        let mode_probs: [f32; 3] = [exp_logits[0] / sum_exp, exp_logits[1] / sum_exp, exp_logits[2] / sum_exp];
        let mode = mode_logits_vec.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|(i, _)| i).unwrap_or(0);
        let snr_data = control.snr_target.to_data();
        let snr_vec: Vec<f32> = snr_data.as_slice().expect("Tensor data should be contiguous").to_vec();
        let snr_target_db = snr_vec[0];
        let inference_time_ms = start.elapsed().as_secs_f32() * 1000.0;

        InferenceResult {
            latent: latent_vec,
            mode,
            mode_probs,
            snr_target_db,
            inference_time_ms,
            anomaly_score: 0.0,
            reconstruction: vec![0.0; features.rf_psd.len()],
        }
    }
}
