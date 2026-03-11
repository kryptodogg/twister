use burn::tensor::backend::Backend;
use burn::tensor::{Tensor, TensorData};
use crate::anc_calibration::FullRangeCalibration;
use crate::vbuffer::GpuVBuffer;
use crate::ml::impulse_coherence::{ImpulseCoherenceAnalyzer, CpuVBufferWindow};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::Mutex;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub use_audio: bool,
    pub use_anc_phase: bool,
    pub use_vbuffer_coherence: bool,
    pub use_tdoa_confidence: bool,
    pub use_multi_device_corr: bool,
    pub use_harmonic_analysis: bool,
    pub use_impulse_detection: bool,
    pub use_impulse_phase_lock: bool,
    // Visual microphone (C925e webcam)
    pub use_visual_microphone: bool,
    pub visual_num_frequency_bins: usize,
    pub visual_preserve_rgb_separation: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            use_audio: true,
            use_anc_phase: false,
            use_vbuffer_coherence: false,
            use_tdoa_confidence: false,
            use_multi_device_corr: false,
            use_harmonic_analysis: false,
            use_impulse_detection: false,
            use_impulse_phase_lock: false,
            use_visual_microphone: false,
            visual_num_frequency_bins: 3,
            visual_preserve_rgb_separation: true,
        }
    }
}

impl FeatureFlags {
    pub fn total_audio_dim(&self) -> usize {
        let mut dim = 196;
        if self.use_anc_phase { dim += 64; }
        if self.use_vbuffer_coherence { dim += 64; }
        if self.use_tdoa_confidence { dim += 1; }
        if self.use_multi_device_corr { dim += 4; }
        if self.use_harmonic_analysis { dim += 32; }
        if self.use_impulse_detection { dim += 4; }
        dim
    }

    pub fn total_visual_dim(&self) -> usize {
        if !self.use_visual_microphone {
            return 0;
        }
        if self.visual_preserve_rgb_separation {
            (3 * self.visual_num_frequency_bins) + 12 + 3 + 4 + 4
        } else {
            self.visual_num_frequency_bins + 4 + 4
        }
    }

    pub fn total_dim(&self) -> usize {
        self.total_audio_dim() + self.total_visual_dim()
    }
}

#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub timestamp_us: u64,
}

#[derive(Debug, Clone)]
pub struct SignalFeaturePayload {
    pub audio_samples: Vec<f32>,
    pub freq_hz: f32,
    pub tdoa_confidence: Option<f32>,
    pub device_corr: Option<[f32; 4]>,
    pub vbuffer_coherence: Option<[f32; 64]>,
    pub anc_phase: Option<Vec<f32>>,
    pub harmonic_energy: Option<Vec<f32>>,
    pub impulse_times: Option<Vec<usize>>,
    // Visual microphone fields
    pub video_frame: Option<VideoFrame>,
    pub video_frame_timestamp_us: u64,
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
            impulse_times: None,
            video_frame: None,
            video_frame_timestamp_us: 0,
            visual_features: None,
        }
    }
}

pub struct ModularFeatureExtractor<B: Backend> {
    pub device: B::Device,
    pub impulse_coherence: ImpulseCoherenceAnalyzer,
    pub vbuffer_window: CpuVBufferWindow,
    pub gpu_vbuffer: Arc<Mutex<GpuVBuffer>>,
}

impl<B: Backend> ModularFeatureExtractor<B> {
    pub fn new(
        device: &B::Device,
        window_frames: usize,
        vbuffer_window: CpuVBufferWindow,
        gpu_vbuffer: Arc<Mutex<GpuVBuffer>>
    ) -> Self {
        Self {
            device: device.clone(),
            impulse_coherence: ImpulseCoherenceAnalyzer::new(window_frames),
            vbuffer_window,
            gpu_vbuffer,
        }
    }

    pub fn extract(&self, payload: &SignalFeaturePayload, flags: &FeatureFlags) -> (Tensor<B, 1>, Tensor<B, 1>) {
        let mut features = vec![0.0f32; 196];
        if !payload.audio_samples.is_empty() {
            for i in 0..196.min(payload.audio_samples.len()) {
                features[i] = payload.audio_samples[i];
            }
        }
        let mut mask = vec![if flags.use_audio { 1.0f32 } else { 0.0f32 }; 196];

        if let Some(anc) = &payload.anc_phase {
            features.extend(anc.iter().take(64));
        } else {
            features.extend(vec![0.0f32; 64]);
        }
        mask.extend(vec![if flags.use_anc_phase { 1.0f32 } else { 0.0f32 }; 64]);

        if flags.use_vbuffer_coherence {
            let gpu_channels_mock = vec![0.5f32; 64];
            features.extend_from_slice(&gpu_channels_mock);
            mask.extend(vec![1.0f32; 64]);
        } else {
            features.extend(vec![0.0f32; 64]);
            mask.extend(vec![0.0f32; 64]);
        }

        if let Some(tdoa) = payload.tdoa_confidence {
            features.push(tdoa);
        } else {
            features.push(0.0);
        }
        mask.push(if flags.use_tdoa_confidence { 1.0f32 } else { 0.0f32 });

        if let Some(corr) = &payload.device_corr {
            features.extend(corr.iter());
        } else {
            features.extend(vec![0.0f32; 4]);
        }
        mask.extend(vec![if flags.use_multi_device_corr { 1.0f32 } else { 0.0f32 }; 4]);

        if let Some(harm) = &payload.harmonic_energy {
            features.extend(harm.iter().take(32));
        } else {
            features.extend(vec![0.0f32; 32]);
        }
        mask.extend(vec![if flags.use_harmonic_analysis { 1.0f32 } else { 0.0f32 }; 32]);

        if flags.use_impulse_detection {
            if let Some(times) = &payload.impulse_times {
                let coherence = self.impulse_coherence.extract(&self.vbuffer_window, times);
                features.push(coherence.phase_lock_strength);
                features.push(coherence.timing_jitter);
                features.push(coherence.cross_frame_coherence);
                features.push(coherence.is_controlled_synthesis);
                mask.extend(vec![1.0f32; 4]);
            } else {
                features.extend(vec![0.0f32; 4]);
                mask.extend(vec![1.0f32; 4]);
            }
        }

        let feature_tensor = Tensor::<B, 1>::from_data(TensorData::from(features.as_slice()), &self.device);
        let mask_tensor = Tensor::<B, 1>::from_data(TensorData::from(mask.as_slice()), &self.device);
        let masked_features = feature_tensor.mul(mask_tensor.clone());

        (masked_features, mask_tensor)
    }
}

use burn::module::Module;
use burn::nn::{Linear, LinearConfig};

#[derive(Debug, Clone)]
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

    fc_audio1: Linear<B>,
    fc_audio2: Linear<B>,

    fc_visual1: Option<Linear<B>>,
    fc_visual2: Option<Linear<B>>,

    fc_fusion1: Linear<B>,
    fc_fusion2: Linear<B>,

    fc_latent: Linear<B>,

    fc_dec_audio1: Linear<B>,
    fc_dec_audio2: Linear<B>,

    fc_dec_visual1: Option<Linear<B>>,
    fc_dec_visual2: Option<Linear<B>>,
}

impl<B: Backend> ModularFeatureEncoder<B> {
    pub fn new(flags: FeatureFlags, device: &B::Device) -> Self {
        let audio_dim = flags.total_audio_dim();
        let visual_dim = flags.total_visual_dim();
        let hidden_dim = 256;
        let latent_dim = 128;

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

        let fusion_input_dim = if visual_dim > 0 {
            hidden_dim * 2
        } else {
            hidden_dim
        };

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

    pub fn forward(
        &self,
        features: Tensor<B, 2>,
    ) -> (Tensor<B, 2>, Tensor<B, 1>, FeatureImportance) {
        let batch_size = features.dims()[0];
        let audio_features = features.clone().slice([0..batch_size, 0..self.audio_dim]);

        let audio_h1 =
            burn::tensor::activation::relu(self.fc_audio1.forward(audio_features.clone()));
        let audio_h2 = burn::tensor::activation::relu(self.fc_audio2.forward(audio_h1));

        let fused_h1_input = if self.visual_dim > 0 {
            let visual_features = features.clone().slice([
                0..batch_size,
                self.audio_dim..self.audio_dim + self.visual_dim,
            ]);
            let v1 = self.fc_visual1.as_ref().unwrap();
            let v2 = self.fc_visual2.as_ref().unwrap();
            let visual_h1 = burn::tensor::activation::relu(v1.forward(visual_features.clone()));
            let visual_h2 = burn::tensor::activation::relu(v2.forward(visual_h1));

            Tensor::cat(vec![audio_h2, visual_h2], 1)
        } else {
            audio_h2
        };

        let fused_h1 = burn::tensor::activation::relu(self.fc_fusion1.forward(fused_h1_input));
        let fused_h2 = burn::tensor::activation::relu(self.fc_fusion2.forward(fused_h1));

        let latent = self.fc_latent.forward(fused_h2);

        let dec_audio_h1 =
            burn::tensor::activation::relu(self.fc_dec_audio1.forward(latent.clone()));
        let audio_reconstructed = self.fc_dec_audio2.forward(dec_audio_h1);
        let audio_diff = audio_reconstructed.sub(audio_features);
        let audio_mse = audio_diff.powf_scalar(2.0).mean_dim(1).squeeze();

        let combined_mse = if self.visual_dim > 0 {
            let visual_features = features.clone().slice([
                0..batch_size,
                self.audio_dim..self.audio_dim + self.visual_dim,
            ]);
            let d1 = self.fc_dec_visual1.as_ref().unwrap();
            let d2 = self.fc_dec_visual2.as_ref().unwrap();

            let dec_visual_h1 = burn::tensor::activation::relu(d1.forward(latent.clone()));
            let visual_reconstructed = d2.forward(dec_visual_h1);
            let visual_diff = visual_reconstructed.sub(visual_features);
            let visual_mse = visual_diff.powf_scalar(2.0).mean_dim(1).squeeze();

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
}

pub struct ActiveDenialToggles {
    pub anc_cancellation: bool,
    pub harmonic_cancellation: bool,
    pub frequency_sweep: bool,
}

pub fn apply_active_denial_toggle(
    _signal: &mut [f32],
    _flags: &FeatureFlags,
    _anc_lut: &FullRangeCalibration,
) {
    // Stub for active denial correction logic
}

#[derive(Debug, Clone)]
pub struct ImpulseTrainEvent {
    pub timestamp: f64,
    pub impulse_times: Vec<f32>,
    pub spacing_hz: f32,
    pub jitter: f32,
    pub confidence: f32,
    pub source_device: u32,
}

pub struct ImpulsePatternModel;

impl ImpulsePatternModel {
    pub fn new() -> Self { Self }
    pub fn extract_pattern(&self, _event: &ImpulseTrainEvent) -> Vec<f32> { vec![0.0; 10] }
    pub fn score_anomaly(&self, _pattern: &[f32]) -> f32 { 0.0 }
}

pub fn detect_impulse_times(samples: &[f32], threshold: f32) -> Vec<f32> {
    let mut times = Vec::new();
    for (i, &s) in samples.iter().enumerate() {
        if s > threshold { times.push(i as f32); }
    }
    times
}

pub fn measure_pulse_train_coherence(_samples: &[f32], _sample_rate: u32) -> (f32, f32, f32) {
    (10.0, 0.01, 0.9)
}
