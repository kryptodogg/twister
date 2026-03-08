pub struct ImpulseModulationAnalyzer {
    sample_rate: u32,
}

#[derive(Debug, Clone)]
pub struct ModulationFeatures {
    pub impulse_amplitudes: Vec<f32>,   // Sequence of impulse heights
    pub modulation_envelope: [f32; 64], // Envelope of amplitude variation
    pub modulation_frequency: f32,      // How fast does amplitude change? (Hz)
    pub modulation_entropy: f32,        // 0.0=predictable, 1.0=random
    pub modulation_periodicity: f32,    // Does envelope repeat?
}

impl ImpulseModulationAnalyzer {
    pub fn new(sample_rate: u32) -> Self {
        Self { sample_rate }
    }

    pub fn extract_impulse_amplitudes(&self, time_domain: &[f32]) -> Vec<f32> {
        let mut amplitudes = Vec::new();
        let max_val = time_domain.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
        let threshold = max_val * 0.7;

        for &sample in time_domain {
            if sample.abs() > threshold {
                amplitudes.push(sample.abs());
            }
        }

        amplitudes
    }

    pub fn modulation_envelope(&self, amplitudes: &[f32]) -> [f32; 64] {
        let mut envelope = [0.0f32; 64];

        if amplitudes.is_empty() {
            return envelope;
        }

        let bin_size = amplitudes.len() / 64;
        if bin_size == 0 {
            // Not enough points for 64 bins, just pad the beginning
            for (i, &amp) in amplitudes.iter().take(64).enumerate() {
                envelope[i] = amp;
            }
            return envelope;
        }

        for (bin, envelope_bin) in envelope.iter_mut().enumerate() {
            let start = bin * bin_size;
            let end = ((bin + 1) * bin_size).min(amplitudes.len());
            *envelope_bin =
                amplitudes[start..end].iter().sum::<f32>() / (end - start).max(1) as f32;
        }

        envelope
    }

    pub fn modulation_frequency(&self, amplitudes: &[f32], sample_rate: u32) -> f32 {
        if amplitudes.len() < 2 {
            return 0.0;
        }

        let mut diffs = Vec::new();
        for i in 1..amplitudes.len() {
            diffs.push((amplitudes[i] - amplitudes[i - 1]).abs());
        }

        let mean_diff: f32 = diffs.iter().sum::<f32>() / diffs.len() as f32;

        (mean_diff * sample_rate as f32 / amplitudes.len() as f32).clamp(0.0, 10000.0)
    }

    pub fn modulation_entropy(&self, amplitudes: &[f32]) -> f32 {
        if amplitudes.is_empty() {
            return 0.0;
        }

        let mut histogram = [0u32; 10];

        for &amp in amplitudes {
            let bin = (amp.clamp(0.0, 1.0) * 10.0) as usize;
            histogram[bin.min(9)] += 1;
        }

        let total = amplitudes.len() as f32;
        let entropy: f32 = histogram
            .iter()
            .filter(|&&count| count > 0)
            .map(|&count| {
                let p = count as f32 / total;
                -p * p.log2()
            })
            .sum::<f32>();

        entropy / 10.0f32.log2()
    }

    pub fn modulation_periodicity(&self, amplitudes: &[f32]) -> f32 {
        if amplitudes.len() < 20 {
            return 0.0;
        }

        let half = amplitudes.len() / 2;
        let correlation: f32 = amplitudes[..half]
            .iter()
            .zip(&amplitudes[half..half * 2])
            .map(|(a, b)| a * b)
            .sum();

        let norm: f32 = (amplitudes[..half].iter().map(|x| x * x).sum::<f32>()
            * amplitudes[half..half * 2]
                .iter()
                .map(|x| x * x)
                .sum::<f32>())
        .sqrt();

        if norm > 0.0 {
            (correlation / norm).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    pub fn extract(&self, time_domain: &[f32]) -> ModulationFeatures {
        let amplitudes = self.extract_impulse_amplitudes(time_domain);
        let envelope = self.modulation_envelope(&amplitudes);
        let modfreq = self.modulation_frequency(&amplitudes, self.sample_rate);
        let entropy = self.modulation_entropy(&amplitudes);
        let periodicity = self.modulation_periodicity(&amplitudes);

        ModulationFeatures {
            impulse_amplitudes: amplitudes,
            modulation_envelope: envelope,
            modulation_frequency: modfreq,
            modulation_entropy: entropy,
            modulation_periodicity: periodicity,
        }
    }
}
