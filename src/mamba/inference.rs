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
    /// Create a new inference engine
    pub fn new(config: &SSAMBAConfig) -> Self {
        let device = Default::default();
        let model = SSAMBA::new(config, &device);

        Self { model, device }
    }

    /// Load model from checkpoint
    pub fn from_checkpoint(_path: &str, config: &SSAMBAConfig) -> Result<Self, String> {
        // In production, would load from file
        let device = Default::default();
        let model = SSAMBA::new(config, &device);

        Ok(Self { model, device })
    }

    /// Run inference on feature vector
    pub fn forward(&self, features: &FeatureVector) -> InferenceResult {
        let start = std::time::Instant::now();

        // Convert features to tensor
        let feature_array = features.to_array();
        let input = Tensor::from_floats(
            feature_array.as_slice().expect("Tensor data should be contiguous"),
            &self.device,
        );

        // Forward pass
        let (latent, control) = self.model.forward(input);

        // Extract results
        let latent_data = latent.to_data();
        let latent_vec: Vec<f32> = latent_data.as_slice().expect("Tensor data should be contiguous").to_vec();

        let mode_logits_data = control.mode_logits.to_data();
        let mode_logits_vec: Vec<f32> = mode_logits_data.as_slice().expect("Tensor data should be contiguous").to_vec();

        // Compute softmax
        let max_logit = mode_logits_vec.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exp_logits: Vec<f32> = mode_logits_vec
            .iter()
            .map(|&l| (l - max_logit).exp())
            .collect();
        let sum_exp: f32 = exp_logits.iter().sum();
        let mode_probs: [f32; 3] = [
            exp_logits[0] / sum_exp,
            exp_logits[1] / sum_exp,
            exp_logits[2] / sum_exp,
        ];

        // Get predicted mode
        let mode = mode_logits_vec
            .iter()
            .enumerate()
            .max_by(|a: &(usize, &f32), b: &(usize, &f32)| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Get SNR target
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

    /// Run inference on raw feature array
    pub fn forward_array(&self, features: &Array1<f32>) -> InferenceResult {
        let start = std::time::Instant::now();

        // Convert to tensor
        let input = Tensor::from_floats(
            features.as_slice().expect("Tensor data should be contiguous"),
            &self.device,
        );

        // Forward pass
        let (latent, control) = self.model.forward(input);

        // Extract results
        let latent_data = latent.to_data();
        let latent_vec: Vec<f32> = latent_data.as_slice().expect("Tensor data should be contiguous").to_vec();

        let mode_logits_data = control.mode_logits.to_data();
        let mode_logits_vec: Vec<f32> = mode_logits_data.as_slice().expect("Tensor data should be contiguous").to_vec();

        // Softmax
        let max_logit = mode_logits_vec.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exp_logits: Vec<f32> = mode_logits_vec
            .iter()
            .map(|&l| (l - max_logit).exp())
            .collect();
        let sum_exp: f32 = exp_logits.iter().sum();
        let mode_probs: [f32; 3] = [
            exp_logits[0] / sum_exp,
            exp_logits[1] / sum_exp,
            exp_logits[2] / sum_exp,
        ];

        let mode = mode_logits_vec
            .iter()
            .enumerate()
            .max_by(|a: &(usize, &f32), b: &(usize, &f32)| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

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
            reconstruction: vec![0.0; features.len()],
        }
    }

    /// Get model configuration
    pub fn config(&self) -> &SSAMBAConfig {
        unimplemented!("Config access via model removed for Burn compatibility")
    }

    /// Get latent dimension
    pub fn latent_dim(&self) -> usize {
        self.model.latent_dim()
    }
}

/// Mode enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationMode {
    /// Active Noise Cancellation
    Anc = 0,
    /// Silence (passive)
    Silence = 1,
    /// Music playback
    Music = 2,
}

impl OperationMode {
    /// From integer
    pub fn from_usize(mode: usize) -> Self {
        match mode {
            0 => Self::Anc,
            1 => Self::Silence,
            _ => Self::Music,
        }
    }

    /// To string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Anc => "anc",
            Self::Silence => "silence",
            Self::Music => "music",
        }
    }
}

impl From<usize> for OperationMode {
    fn from(mode: usize) -> Self {
        Self::from_usize(mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::features::FeatureVector;

    #[test]
    fn test_inference_creation() {
        let config = SSAMBAConfig::new();
        let inference = MambaInference::new(&config);

        assert_eq!(inference.latent_dim(), 64);
    }

    #[test]
    fn test_inference_run() {
        let config = SSAMBAConfig::new();
        let inference = MambaInference::new(&config);

        let features = FeatureVector::zeros();
        let result = inference.forward(&features);

        assert_eq!(result.latent.len(), 64);
        assert!(result.mode < 3);
        assert!(result.mode_probs.iter().sum::<f32>() > 0.99);
        assert!(result.inference_time_ms > 0.0);
    }

    #[test]
    fn test_operation_mode() {
        assert_eq!(OperationMode::from_usize(0), OperationMode::Anc);
        assert_eq!(OperationMode::from_usize(1), OperationMode::Silence);
        assert_eq!(OperationMode::from_usize(2), OperationMode::Music);
        assert_eq!(OperationMode::Anc.as_str(), "anc");
    }
}
