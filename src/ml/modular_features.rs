use burn::tensor::backend::Backend;
use burn::tensor::{Tensor, TensorData};
use burn::nn::{Linear, LinearConfig};
use burn::module::Module;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct FeatureFlags {
    pub use_audio: bool,              // Always true (baseline 196-D)
    pub use_anc_phase: bool,          // +64-D
    pub use_vbuffer_coherence: bool,  // +64-D
    pub use_tdoa_confidence: bool,    // +1-D
    pub use_multi_device_corr: bool,  // +4-D
    pub use_harmonic_analysis: bool,  // +32-D
    pub use_impulse_detection: bool,  // +20-D

    // NEW: Visual microphone (color-preserving)
    pub use_visual_microphone: bool,   // FALSE → optional +32-64D
    pub visual_num_frequency_bins: usize,  // 3 (low/mid/high) or 16 (detailed)
    pub visual_preserve_rgb_separation: bool, // TRUE = keep R,G,B separate; FALSE = luminance only
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            use_audio: true,
            use_anc_phase: false,       // Learned during training
            use_vbuffer_coherence: false,
            use_tdoa_confidence: false,
            use_multi_device_corr: false,
            use_harmonic_analysis: false,
            use_impulse_detection: false,
            use_visual_microphone: false,
            visual_num_frequency_bins: 3,
            visual_preserve_rgb_separation: true,
        }
    }
}

impl FeatureFlags {
    /// Total dimension based on active flags
    pub fn total_audio_dim(&self) -> usize {
        let mut dim = 196; // audio (always on)
        if self.use_anc_phase { dim += 64; }
        if self.use_vbuffer_coherence { dim += 64; }
        if self.use_tdoa_confidence { dim += 1; }
        if self.use_multi_device_corr { dim += 4; }
        if self.use_harmonic_analysis { dim += 32; }
        if self.use_impulse_detection { dim += 20; }
        dim
    }

    pub fn total_visual_dim(&self) -> usize {
        if !self.use_visual_microphone {
            return 0;
        }

        if self.visual_preserve_rgb_separation {
            (3 * self.visual_num_frequency_bins) + 12 + 3 + 4 + 4  // RGB separate: energy(3*bins) + flow(12) + coherence(3) + global(4) + color(4)
        } else {
            self.visual_num_frequency_bins + 4 + 4  // Luminance only: energy(bins) + flow(4) + global(4)
        }
    }

    pub fn total_dim(&self) -> usize {
        self.total_audio_dim() + self.total_visual_dim()
    }

    /// Binary mask tensor: 1.0 if feature active, 0.0 if masked (for audio)
    pub fn to_audio_mask(&self) -> Vec<f32> {
        let mut mask = vec![1.0; 196]; // audio always active

        if !self.use_anc_phase { mask.extend(vec![0.0; 64]); }
        else { mask.extend(vec![1.0; 64]); }

        if !self.use_vbuffer_coherence { mask.extend(vec![0.0; 64]); }
        else { mask.extend(vec![1.0; 64]); }

        if !self.use_tdoa_confidence { mask.push(0.0); }
        else { mask.push(1.0); }

        if !self.use_multi_device_corr { mask.extend(vec![0.0; 4]); }
        else { mask.extend(vec![1.0; 4]); }

        if !self.use_harmonic_analysis { mask.extend(vec![0.0; 32]); }
        else { mask.extend(vec![1.0; 32]); }

        if !self.use_impulse_detection { mask.extend(vec![0.0; 20]); }
        else { mask.extend(vec![1.0; 20]); }

        mask
    }
}

#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,     // RGB (3 bytes per pixel)
    pub timestamp_us: u64,
}

/// Ephemeral, high-throughput payload for real-time ML inference
#[derive(Debug, Clone)]
pub struct SignalFeaturePayload {
    pub audio_samples: Vec<f32>,        // Time-domain audio (196-D)
    pub freq_hz: f32,                   // Detected frequency
    pub tdoa_confidence: Option<f32>,   // 0-1 confidence
    pub device_corr: Option<[f32; 4]>,  // Cross-device correlation
    pub vbuffer_coherence: Option<[f32; 64]>, // Spectral stability
    pub anc_phase: Option<Vec<f32>>,    // Phase from ANC LUT
    pub harmonic_energy: Option<Vec<f32>>, // Log-frequency harmonics
    pub impulse_detection: Option<[f32; 20]>, // Peak detection

    // NEW: Video frame
    pub video_frame: Option<VideoFrame>,   // RGB frame from C925e
    pub video_frame_timestamp_us: u64,     // Timestamp synchronization

    // Pre-computed visual features if any (allows passing pre-extracted features instead of raw frames)
    pub visual_features: Option<Vec<f32>>,
}

impl SignalFeaturePayload {
    pub fn new(audio_samples: Vec<f32>, freq_hz: f32) -> Self {
        Self {
            audio_samples,
            freq_hz,
            tdoa_confidence: None,
            device_corr: None,
            vbuffer_coherence: None,
            anc_phase: None,
            harmonic_energy: None,
            impulse_detection: None,
            video_frame: None,
            video_frame_timestamp_us: 0,
            visual_features: None,
        }
    }
}

/// Stub for a future visual feature extraction pipeline.
/// Returns a flat vector of visual features.
pub fn extract_visual_microphone_features(
    _frame_current: &VideoFrame,
    _frame_history: &std::collections::VecDeque<VideoFrame>,
    flags: &FeatureFlags,
) -> Vec<f32> {
    // Return empty vector or zeros of appropriate size based on flags
    vec![0.0f32; flags.total_visual_dim()]
}

pub struct FeatureImportance {
    pub audio_importance: f32,
    pub visual_importance: f32,
    pub flags_audio_dim: usize,
    pub flags_visual_dim: usize,
}

#[derive(Module, Debug)]
pub struct ModularFeatureEncoder<B: Backend> {
    pub audio_dim: usize,
    pub visual_dim: usize,
    pub hidden_dim: usize,
    pub latent_dim: usize,

    // Audio path
    fc_audio1: Linear<B>,
    fc_audio2: Linear<B>,

    // Visual path
    fc_visual1: Option<Linear<B>>,
    fc_visual2: Option<Linear<B>>,

    // Fusion path
    fc_fusion1: Linear<B>,
    fc_fusion2: Linear<B>,

    // Latent bottleneck
    fc_latent: Linear<B>,

    // Audio decoder
    fc_dec_audio1: Linear<B>,
    fc_dec_audio2: Linear<B>,

    // Visual decoder
    fc_dec_visual1: Option<Linear<B>>,
    fc_dec_visual2: Option<Linear<B>>,
}

impl<B: Backend> ModularFeatureEncoder<B> {
    pub fn new(flags: FeatureFlags, device: &B::Device) -> Self {
        let audio_dim = flags.total_audio_dim();
        let visual_dim = flags.total_visual_dim();
        let hidden_dim = 256;
        let latent_dim = 128;

        // Visual layers are only instantiated if visual features are enabled
        let (fc_visual1, fc_visual2, fc_dec_visual1, fc_dec_visual2) = if visual_dim > 0 {
            (
                Some(LinearConfig::new(visual_dim, hidden_dim).init(device)),
                Some(LinearConfig::new(hidden_dim, hidden_dim).init(device)),
                Some(LinearConfig::new(latent_dim, hidden_dim).init(device)),
                Some(LinearConfig::new(hidden_dim, visual_dim).init(device)),
            )
        } else {
            (None, None, None, None)
        };

        let fusion_input_dim = if visual_dim > 0 { hidden_dim * 2 } else { hidden_dim };

        Self {
            audio_dim,
            visual_dim,
            hidden_dim,
            latent_dim,

            fc_audio1: LinearConfig::new(audio_dim, hidden_dim).init(device),
            fc_audio2: LinearConfig::new(hidden_dim, hidden_dim).init(device),

            fc_visual1,
            fc_visual2,

            fc_fusion1: LinearConfig::new(fusion_input_dim, hidden_dim).init(device),
            fc_fusion2: LinearConfig::new(hidden_dim, hidden_dim).init(device),

            fc_latent: LinearConfig::new(hidden_dim, latent_dim).init(device),

            fc_dec_audio1: LinearConfig::new(latent_dim, hidden_dim).init(device),
            fc_dec_audio2: LinearConfig::new(hidden_dim, audio_dim).init(device),

            fc_dec_visual1,
            fc_dec_visual2,
        }
    }

    /// Forward pass through the network with dual pathways.
    /// Input shape: [batch_size, total_dim]
    /// Output:
    /// - latent embedding: [batch_size, 128]
    /// - combined anomaly score (MSE loss scalar): [batch_size]
    /// - FeatureImportance
    pub fn forward(&self, features: Tensor<B, 2>) -> (Tensor<B, 2>, Tensor<B, 1>, FeatureImportance) {
        // AUDIO PATH
        // We know features is at least audio_dim wide.
        // Create an index array to slice dim=1 for audio features
        let batch_size = features.dims()[0];
        // burn::tensor::Tensor::slice is a method but it takes ranges... Wait. slice is like this in burn:
        // slice([0..batch_size, 0..audio_dim])
        let audio_features = features.clone().slice([0..batch_size, 0..self.audio_dim]);

        let audio_h1 = burn::tensor::activation::relu(self.fc_audio1.forward(audio_features.clone()));
        let audio_h2 = burn::tensor::activation::relu(self.fc_audio2.forward(audio_h1));

        let fused_h1_input = if self.visual_dim > 0 {
            // VISUAL PATH
            let visual_features = features.clone().slice([0..batch_size, self.audio_dim..self.audio_dim + self.visual_dim]);
            let v1 = self.fc_visual1.as_ref().unwrap();
            let v2 = self.fc_visual2.as_ref().unwrap();
            let visual_h1 = burn::tensor::activation::relu(v1.forward(visual_features.clone()));
            let visual_h2 = burn::tensor::activation::relu(v2.forward(visual_h1));

            // Concatenate along dim 1
            Tensor::cat(vec![audio_h2, visual_h2], 1)
        } else {
            audio_h2
        };

        // FUSION
        let fused_h1 = burn::tensor::activation::relu(self.fc_fusion1.forward(fused_h1_input));
        let fused_h2 = burn::tensor::activation::relu(self.fc_fusion2.forward(fused_h1));

        // LATENT
        let latent = self.fc_latent.forward(fused_h2);  // [batch, 128]

        // RECONSTRUCTION
        let dec_audio_h1 = burn::tensor::activation::relu(self.fc_dec_audio1.forward(latent.clone()));
        let audio_reconstructed = self.fc_dec_audio2.forward(dec_audio_h1);
        let audio_diff = audio_reconstructed.sub(audio_features);
        let audio_mse = audio_diff.powf_scalar(2.0).mean_dim(1).squeeze();

        let combined_mse = if self.visual_dim > 0 {
            let visual_features = features.clone().slice([0..batch_size, self.audio_dim..self.audio_dim + self.visual_dim]);
            let d1 = self.fc_dec_visual1.as_ref().unwrap();
            let d2 = self.fc_dec_visual2.as_ref().unwrap();

            let dec_visual_h1 = burn::tensor::activation::relu(d1.forward(latent.clone()));
            let visual_reconstructed = d2.forward(dec_visual_h1);
            let visual_diff = visual_reconstructed.sub(visual_features);
            let visual_mse = visual_diff.powf_scalar(2.0).mean_dim(1).squeeze();

            // Average them
            audio_mse.add(visual_mse).div_scalar(2.0)
        } else {
            audio_mse
        };

        let importance = FeatureImportance {
            audio_importance: 0.5,
            visual_importance: if self.visual_dim > 0 { 0.5 } else { 0.0 },
            flags_audio_dim: self.audio_dim,
            flags_visual_dim: self.visual_dim,
        };

        (latent, combined_mse, importance)
    }

    /// Extract features from a payload and build a continuous 1D tensor [total_dim].
    pub fn extract_tensor(
        &self,
        payload: &SignalFeaturePayload,
        flags: &FeatureFlags,
        device: &B::Device,
    ) -> Tensor<B, 1> {
        let mut features = vec![0.0f32; 196]; // Audio always active
        if !payload.audio_samples.is_empty() {
            for i in 0..196.min(payload.audio_samples.len()) {
                features[i] = payload.audio_samples[i];
            }
        }

        if flags.use_anc_phase {
            if let Some(anc) = &payload.anc_phase {
                features.extend(anc.iter().take(64));
                if anc.len() < 64 {
                    features.extend(vec![0.0f32; 64 - anc.len()]);
                }
            } else {
                features.extend(vec![0.0f32; 64]);
            }
        }

        if flags.use_vbuffer_coherence {
            if let Some(vbuf) = &payload.vbuffer_coherence {
                features.extend(vbuf.iter());
            } else {
                features.extend(vec![0.0f32; 64]);
            }
        }

        if flags.use_tdoa_confidence {
            if let Some(tdoa) = payload.tdoa_confidence {
                features.push(tdoa);
            } else {
                features.push(0.0);
            }
        }

        if flags.use_multi_device_corr {
            if let Some(corr) = &payload.device_corr {
                features.extend(corr.iter());
            } else {
                features.extend(vec![0.0f32; 4]);
            }
        }

        if flags.use_harmonic_analysis {
            if let Some(harm) = &payload.harmonic_energy {
                features.extend(harm.iter().take(32));
                if harm.len() < 32 {
                    features.extend(vec![0.0f32; 32 - harm.len()]);
                }
            } else {
                features.extend(vec![0.0f32; 32]);
            }
        }

        if flags.use_impulse_detection {
            if let Some(impulse) = &payload.impulse_detection {
                features.extend(impulse.iter().take(20));
            } else {
                features.extend(vec![0.0f32; 20]);
            }
        }

        if flags.use_visual_microphone {
            if let Some(visual) = &payload.visual_features {
                features.extend(visual.iter().take(flags.total_visual_dim()));
                if visual.len() < flags.total_visual_dim() {
                    features.extend(vec![0.0f32; flags.total_visual_dim() - visual.len()]);
                }
            } else {
                features.extend(vec![0.0f32; flags.total_visual_dim()]);
            }
        }

        assert_eq!(features.len(), flags.total_dim(), "Extracted features length does not match total_dim");

        Tensor::<B, 1>::from_data(
            TensorData::new(features, [flags.total_dim()]),
            device,
        )
    }
}
