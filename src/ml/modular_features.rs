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
}
