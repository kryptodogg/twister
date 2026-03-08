
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
