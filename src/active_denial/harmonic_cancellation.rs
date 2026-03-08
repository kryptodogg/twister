pub struct HarmonicCancellationSynthesizer {
    sample_rate: u32,
}

impl HarmonicCancellationSynthesizer {
    pub fn new(sample_rate: u32) -> Self {
        Self { sample_rate }
    }

    pub fn synthesize_anti_phase(
        &self,
        detected_f0: f32,
        harmonic_magnitudes: &[f32],
        harmonic_phases: &[f32],
    ) -> Vec<f32> {
        let duration_secs = 1.0;
        let num_samples = (self.sample_rate as f32 * duration_secs) as usize;
        let mut output = vec![0.0f32; num_samples];

        // Synthesize anti-phase harmonics (180° phase shift)
        for (h, &mag) in harmonic_magnitudes.iter().enumerate() {
            let harmonic_freq = detected_f0 * (h as f32 + 1.0);
            let anti_phase = harmonic_phases[h] + std::f32::consts::PI; // 180° opposite

            for t in 0..num_samples {
                let phase = 2.0
                    * std::f32::consts::PI
                    * harmonic_freq
                    * (t as f32 / self.sample_rate as f32)
                    + anti_phase;
                output[t] += mag * phase.sin();
            }
        }

        output
    }
}
