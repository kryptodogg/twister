// src/ml/impulse_coherence.rs
// Impulse coherence analyzer to detect phase-locked pulse trains (speech synthesis).

pub struct ImpulseCoherenceAnalyzer {
    pub window_frames: usize,
}

pub struct ImpulseCoherenceMetrics {
    pub phase_lock_strength: f32,
    pub timing_jitter: f32,
    pub cross_frame_coherence: f32,
    pub is_controlled_synthesis: f32,
}

// We need a dummy VBuffer struct for the CPU to hold frames (magnitudes) for impulse detection.
// Wait, the new V-buffer is entirely on GPU now!
// But impulse detection operates on "frames". The prompt says:
// `vbuffer.get_frame(-(frame_offset as i32))`
// Since all data is on GPU now, reading it back to CPU for impulse coherence would stall.
// However, the prompt specifically requested this CPU implementation: "src/ml/impulse_coherence.rs (~120 lines)"
// So let's provide a CPU mock/stub structure that takes CPU frames, or define a simple CPU buffer.
// "Do not implement CPU-side ring buffers. Execute the following architecture exactly: Everything happens on the GPU... Create src/ml/modular_features.rs"
// BUT then ADDENDUM said: "// src/ml/impulse_coherence.rs (~120 lines)" with a `VBuffer` type.
// If VBuffer is on GPU, we either fetch N frames or pass a CPU window. Let's create a minimal `CpuVBuffer` just for this if needed, or assume we fetch `Vec<f32>` from GPU.
// Let's define a trait or struct for it.

pub struct CpuVBufferWindow {
    pub frames: Vec<Vec<f32>>,
}

impl CpuVBufferWindow {
    pub fn num_frames(&self) -> usize {
        self.frames.len()
    }

    pub fn get_frame(&self, offset: i32) -> Option<&[f32]> {
        let idx = -offset as usize;
        if idx < self.frames.len() {
            Some(&self.frames[idx])
        } else {
            None
        }
    }
}

impl ImpulseCoherenceAnalyzer {
    pub fn new(window_frames: usize) -> Self {
        Self { window_frames }
    }

    pub fn phase_lock_strength(&self, vbuffer: &CpuVBufferWindow) -> f32 {
        let mut frame_impulse_patterns = Vec::new();

        for frame_offset in 0..self.window_frames.min(vbuffer.num_frames()) {
            if let Some(frame) = vbuffer.get_frame(-(frame_offset as i32)) {
                let impulses = self.detect_frame_impulses(frame);
                frame_impulse_patterns.push(impulses);
            }
        }

        if frame_impulse_patterns.len() < 2 {
            return 0.0;
        }

        let mut correlation_sum = 0.0f32;
        for i in 0..frame_impulse_patterns.len() - 1 {
            let corr =
                self.correlate_patterns(&frame_impulse_patterns[i], &frame_impulse_patterns[i + 1]);
            correlation_sum += corr;
        }

        correlation_sum / (frame_impulse_patterns.len() - 1) as f32
    }

    fn correlate_patterns(&self, pattern1: &[bool], pattern2: &[bool]) -> f32 {
        if pattern1.len() != pattern2.len() || pattern1.is_empty() {
            return 0.0;
        }

        let matches = pattern1
            .iter()
            .zip(pattern2)
            .filter(|(a, b)| a == b)
            .count();
        matches as f32 / pattern1.len() as f32
    }

    pub fn timing_jitter(&self, impulse_times: &[usize]) -> f32 {
        if impulse_times.len() < 2 {
            return 1.0;
        }

        let mut spacings = Vec::new();
        for i in 1..impulse_times.len() {
            spacings.push(impulse_times[i] - impulse_times[i - 1]);
        }

        let mean: f32 = spacings.iter().sum::<usize>() as f32 / spacings.len() as f32;
        let variance: f32 = spacings
            .iter()
            .map(|&s| (s as f32 - mean).powi(2))
            .sum::<f32>()
            / spacings.len() as f32;

        (variance.sqrt() / mean).clamp(0.0, 1.0)
    }

    pub fn cross_frame_coherence(&self, vbuffer: &CpuVBufferWindow) -> f32 {
        let mut patterns = Vec::new();
        for frame_offset in 0..self.window_frames.min(vbuffer.num_frames()) {
            if let Some(frame) = vbuffer.get_frame(-(frame_offset as i32)) {
                patterns.push(self.detect_frame_impulses(frame));
            }
        }

        if patterns.len() < 2 {
            return 0.0;
        }

        let mut coherence = 0.0f32;
        for i in 0..patterns.len() - 1 {
            coherence += self.correlate_patterns(&patterns[i], &patterns[i + 1]);
        }

        coherence / (patterns.len() - 1) as f32
    }

    pub fn is_controlled_synthesis(&self, metrics: &ImpulseCoherenceMetrics) -> f32 {
        let lock_weight = metrics.phase_lock_strength;
        let jitter_weight = 1.0 - metrics.timing_jitter;
        let coherence_weight = metrics.cross_frame_coherence;

        (lock_weight + jitter_weight + coherence_weight) / 3.0
    }

    fn detect_frame_impulses(&self, frame: &[f32]) -> Vec<bool> {
        let threshold = frame.iter().fold(f32::MIN, |a, &b| a.max(b)) * 0.7;
        frame.iter().map(|&s| s.abs() > threshold).collect()
    }

    pub fn extract(
        &self,
        vbuffer: &CpuVBufferWindow,
        impulse_times: &[usize],
    ) -> ImpulseCoherenceMetrics {
        let phase_lock = self.phase_lock_strength(vbuffer);
        let jitter = self.timing_jitter(impulse_times);
        let coherence = self.cross_frame_coherence(vbuffer);

        let mut metrics = ImpulseCoherenceMetrics {
            phase_lock_strength: phase_lock,
            timing_jitter: jitter,
            cross_frame_coherence: coherence,
            is_controlled_synthesis: 0.0,
        };

        metrics.is_controlled_synthesis = self.is_controlled_synthesis(&metrics);
        metrics
    }
}
