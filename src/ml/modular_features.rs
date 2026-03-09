use burn::tensor::backend::Backend;
use burn::tensor::{Tensor, TensorData};
use crate::forensic::ForensicEvent;
use crate::anc_calibration::FullRangeCalibration;
use serde::{Deserialize, Serialize};

/// Modular feature toggles for ML analysis and active denial
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub use_audio: bool,              // Always true (baseline 196-D)
    pub use_anc_phase: bool,          // +64-D
    pub use_vbuffer_coherence: bool,  // +64-D
    pub use_tdoa_confidence: bool,    // +1-D
    pub use_multi_device_corr: bool,  // +4-D
    pub use_harmonic_analysis: bool,
    pub use_impulse_detection: bool,
    pub use_impulse_phase_lock: bool,  // +32-D
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
        }
    }
}

/// Ephemeral, high-throughput payload for real-time ML inference
#[derive(Debug, Clone)]
pub struct SignalFeaturePayload {
    pub audio_samples: Vec<f32>,
    pub freq_hz: f32,
    // Add other raw inputs here as needed, but keep it lean
    pub tdoa_confidence: Option<f32>,
    pub device_corr: Option<[f32; 4]>,
    pub vbuffer_coherence: Option<[f32; 64]>,
    pub anc_phase: Option<Vec<f32>>,
    pub harmonic_energy: Option<Vec<f32>>,
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
        }
    }
}

pub struct ModularFeatureExtractor<B: Backend> {
    device: B::Device,
}

impl<B: Backend> ModularFeatureExtractor<B> {
    pub fn new(device: &B::Device) -> Self {
        Self {
            device: device.clone(),
        }
    }

    /// Extract features and apply binary masking based on flags
    /// Input: SignalFeaturePayload
    /// Output: (361-D Feature Tensor, Binary Mask Tensor)
    pub fn extract(&self, payload: &SignalFeaturePayload, flags: &FeatureFlags) -> (Tensor<B, 1>, Tensor<B, 1>) {
        // 1. Audio Baseline (196-D)
        // Here we'd perform STFT or just load dummy features for MVP
        // Since we need to keep everything on VRAM, we construct a tensor directly.
        // For MVP, we'll generate a random 196-D tensor or extract from audio.
        let mut audio_features = vec![0.0f32; 196];
        if !payload.audio_samples.is_empty() {
             for i in 0..196.min(payload.audio_samples.len()) {
                 audio_features[i] = payload.audio_samples[i];
             }
        }

        let mut features = audio_features;
        let mut mask = vec![if flags.use_audio { 1.0f32 } else { 0.0f32 }; 196];

        // 2. ANC Phase (64-D)
        if let Some(anc) = &payload.anc_phase {
            features.extend(anc.iter().take(64));
        } else {
            features.extend(vec![0.0f32; 64]);
        }
        mask.extend(vec![if flags.use_anc_phase { 1.0f32 } else { 0.0f32 }; 64]);

        // 3. V-buffer Coherence (64-D)
        if let Some(vbuf) = &payload.vbuffer_coherence {
            features.extend(vbuf.iter());
        } else {
            features.extend(vec![0.0f32; 64]);
        }
        mask.extend(vec![if flags.use_vbuffer_coherence { 1.0f32 } else { 0.0f32 }; 64]);

        // 4. TDOA Confidence (1-D)
        if let Some(tdoa) = payload.tdoa_confidence {
            features.push(tdoa);
        } else {
            features.push(0.0);
        }
        mask.push(if flags.use_tdoa_confidence { 1.0f32 } else { 0.0f32 });

        // 5. Multi-device Correlation (4-D)
        if let Some(corr) = &payload.device_corr {
            features.extend(corr.iter());
        } else {
            features.extend(vec![0.0f32; 4]);
        }
        mask.extend(vec![if flags.use_multi_device_corr { 1.0f32 } else { 0.0f32 }; 4]);

        // 6. Harmonic Energy (32-D)
        if let Some(harm) = &payload.harmonic_energy {
            features.extend(harm.iter().take(32));
        } else {
            features.extend(vec![0.0f32; 32]);
        }
        mask.extend(vec![if flags.use_harmonic_analysis { 1.0f32 } else { 0.0f32 }; 32]);

        // Pad to exactly 361-D if needed (196 + 64 + 64 + 1 + 4 + 32 = 361)
        assert_eq!(features.len(), 361);
        assert_eq!(mask.len(), 361);

        let feature_tensor = Tensor::<B, 1>::from_data(TensorData::from(features.as_slice()), &self.device);
        let mask_tensor = Tensor::<B, 1>::from_data(TensorData::from(mask.as_slice()), &self.device);

        // Apply binary masking to zero-out inactive features
        let masked_features = feature_tensor.mul(mask_tensor.clone());

        (masked_features, mask_tensor)
    }
}

// Active-denial toggles (runtime)
pub struct ActiveDenialToggles {
    pub anc_cancellation: bool,
    pub harmonic_cancellation: bool,
    pub frequency_sweep: bool,
}

pub fn apply_active_denial_toggle(
    signal: &mut [f32],
    flags: &FeatureFlags,
    anc_lut: &FullRangeCalibration,
    // harmonics_synth: &HarmonicSynthesizer, // Add back when synthesizer exists
) {
    if flags.use_anc_phase {
        // Note: apply_correction doesn't exist on FullRangeCalibration yet, so this is a stub
        // signal = anc_lut.apply_correction(signal);
    }
    if flags.use_harmonic_analysis {
        // signal = harmonics_synth.cancel_chords(signal);
    }
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
    pub fn new() -> Self {
        Self
    }

    pub fn extract_pattern(&self, _event: &ImpulseTrainEvent) -> Vec<f32> {
        // Stub implementation
        vec![0.0; 10]
    }

    pub fn score_anomaly(&self, _pattern: &[f32]) -> f32 {
        // Stub implementation
        0.0
    }
}

pub fn detect_impulse_times(samples: &[f32], threshold: f32) -> Vec<f32> {
    // Stub implementation: Find peaks
    let mut times = Vec::new();
    for (i, &s) in samples.iter().enumerate() {
        if s > threshold {
            times.push(i as f32);
        }
    }
    times
}

pub fn measure_pulse_train_coherence(_samples: &[f32], _sample_rate: u32) -> (f32, f32, f32) {
    // Stub implementation: Returns (spacing_hz, jitter, confidence)
    (10.0, 0.01, 0.9)
}
