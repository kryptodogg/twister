// src/ml/modular_features.rs
// The exclusive home for the new 1124-D high-performance ML tensor pipeline.
//
// Extracts features directly using GPU data + optional CPU coherence masks.
// Option B: Passing mask as explicit feature channels.

use crate::vbuffer::{GpuVBuffer, V_FREQ_BINS};
use crate::ml::impulse_coherence::{ImpulseCoherenceAnalyzer, ImpulseCoherenceMetrics, CpuVBufferWindow};
use std::sync::Arc;
use parking_lot::Mutex;

pub struct FeatureFlags {
    pub use_vbuffer_coherence: bool,
    pub use_impulse_detection: bool,
}

pub struct ForensicEvent {
    // Basic forensic metadata
    pub impulse_times: Vec<usize>,
}

pub struct ForensicFeatures {
    pub vbuffer_coherence_mask: Option<[f32; 64]>,
}

pub struct ModularFeatureExtractor {
    pub impulse_coherence: ImpulseCoherenceAnalyzer,
    pub vbuffer_window: CpuVBufferWindow,
    pub gpu_vbuffer: Arc<Mutex<GpuVBuffer>>,
}

impl ModularFeatureExtractor {
    pub fn new(window_frames: usize, vbuffer_window: CpuVBufferWindow, gpu_vbuffer: Arc<Mutex<GpuVBuffer>>) -> Self {
        Self {
            impulse_coherence: ImpulseCoherenceAnalyzer::new(window_frames),
            vbuffer_window,
            gpu_vbuffer,
        }
    }

    pub fn extract(&self, event: &ForensicEvent, flags: &FeatureFlags) -> Vec<f32> {
        // Base features
        let mut features = Vec::new();

        if flags.use_vbuffer_coherence {
            // Option B: Explicit channels (do not pre-multiply).
            // We use the raw values written to the GPU VBuffer: [Magnitude, Phase, TemporalFlux, CoherenceMask].
            // Usually, these would be mapped back to CPU or read directly by the Burn ML backend without leaving GPU.
            // For now, if we are passing this to Burn via CPU tensors, we return dummy channels.
            // But structurally, the vector gets extended by the explicit channels.

            // Mock output simulating the 1024-D explicit channel data (Magnitude + Phase + Coherence...)
            let gpu_channels_mock = vec![0.5f32; V_FREQ_BINS * 2]; // 512 Mag + 512 Coherence
            features.extend_from_slice(&gpu_channels_mock);
        }

        if flags.use_impulse_detection {
            let coherence = self.impulse_coherence.extract(&self.vbuffer_window, &event.impulse_times);

            features.push(coherence.phase_lock_strength);        // +1-D
            features.push(coherence.timing_jitter);              // +1-D
            features.push(coherence.cross_frame_coherence);      // +1-D
            features.push(coherence.is_controlled_synthesis);    // +1-D (key metric)
            // Total: +4-D
        }

        features
    }

/// Additional Impulse Train Feature Extraction Logic
pub struct ImpulseTrainFeatures {
    pub impulse_detection: [f32; 512],
    pub impulse_spacing: f32,
    pub impulse_spacing_jitter: f32,
    pub amplitude_envelope: [f32; 64],
    pub impulse_phase_lock: f32,
    pub pulse_train_confidence: f32,
}

impl ImpulseTrainFeatures {
    pub fn to_vec(&self) -> Vec<f32> {
        let mut vec = Vec::with_capacity(512 + 1 + 1 + 64 + 1 + 1); // 580 total
        vec.extend_from_slice(&self.impulse_detection);
        vec.push(self.impulse_spacing);
        vec.push(self.impulse_spacing_jitter);
        vec.extend_from_slice(&self.amplitude_envelope);
        vec.push(self.impulse_phase_lock);
        vec.push(self.pulse_train_confidence);
        vec
    }
}

pub fn detect_impulses(stft_magnitude: &[f32]) -> [f32; 512] {
    let mut impulses = [0.0f32; 512];
    let len = stft_magnitude.len().min(512);
    for i in 0..len {
        let mag = stft_magnitude[i];
        let prev = if i > 0 { stft_magnitude[i - 1] } else { 0.0 };
        let next = if i < len - 1 { stft_magnitude[i + 1] } else { 0.0 };

        // Peak detection: local maximum
        if mag > prev && mag > next && mag > next.max(prev) * 1.5 {
            impulses[i] = mag;  // Mark as impulse
        }
    }
    impulses
}

pub fn measure_pulse_train_coherence(
    time_domain: &[f32],
    sample_rate: u32,
) -> (f32, f32, f32) {
    let mut impulse_times = Vec::new();
    let threshold = time_domain.iter().fold(f32::MIN, |a, &b| a.max(b)) * 0.8;

    for (i, &sample) in time_domain.iter().enumerate() {
        if sample.abs() > threshold {
            impulse_times.push(i);
        }
    }

    if impulse_times.len() < 2 {
        return (0.0, 0.0, 0.0);
    }

    let mut spacings = Vec::new();
    for i in 1..impulse_times.len() {
        spacings.push(impulse_times[i] - impulse_times[i - 1]);
    }

    let mean_spacing = spacings.iter().sum::<usize>() as f32 / spacings.len() as f32;
    let variance: f32 = spacings.iter()
        .map(|&s| (s as f32 - mean_spacing).powi(2))
        .sum::<f32>() / spacings.len() as f32;

    let jitter = (variance.sqrt() / mean_spacing).clamp(0.0, 1.0);
    let spacing_hz = sample_rate as f32 / mean_spacing;
    let confidence = 1.0 - jitter;

    (spacing_hz, jitter, confidence)
}
