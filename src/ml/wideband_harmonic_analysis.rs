pub struct WidebandHarmonicAnalyzer {
    min_freq: f32,         // 1.0 Hz
    max_freq: f32,         // 12_288_000.0 Hz
    _bins_per_octave: u32, // 8 (96 total bins = 12 octaves × 8)
    num_bins: u32,         // 96
}

#[derive(Debug, Clone)]
pub struct WidebandHarmonicFeatures {
    pub log_spectrogram: [f32; 96],   // Energy per log-frequency bin
    pub fundamental_octave: u32,      // Which octave (0-11) is fundamental in
    pub fundamental_confidence: f32,  // 0.0-1.0
    pub harmonic_coherence: f32,      // Phase lock across octaves
    pub octave_pattern: [f32; 12],    // Energy distribution across 12 octaves
    pub baseband_harmonics: Vec<f32>, // Detected peak frequencies for fold analysis
}

impl WidebandHarmonicAnalyzer {
    pub fn new() -> Self {
        Self {
            min_freq: 1.0,
            max_freq: 12_288_000.0,
            _bins_per_octave: 8,
            num_bins: 96, // 12 octaves × 8 bins
        }
    }

    /// Convert frequency to log-space bin
    fn freq_to_bin(&self, freq: f32) -> f32 {
        let log_min = self.min_freq.log2();
        let log_max = self.max_freq.log2();
        let log_freq = freq.log2();

        ((log_freq - log_min) / (log_max - log_min)) * self.num_bins as f32
    }

    /// Convert bin to frequency
    fn _bin_to_freq(&self, bin: f32) -> f32 {
        let log_min = self.min_freq.log2();
        let log_max = self.max_freq.log2();
        let log_freq = log_min + (bin / self.num_bins as f32) * (log_max - log_min);
        2.0_f32.powf(log_freq)
    }

    /// Map STFT (linear frequency) to log-frequency bins
    pub fn linear_to_log_spectrogram(&self, stft_magnitude: &[f32], sample_rate: u32) -> [f32; 96] {
        let mut log_spec = [0.0f32; 96];
        let freq_resolution = sample_rate as f32 / stft_magnitude.len() as f32;

        // For each STFT bin, add its energy to the log-frequency bin
        for (stft_bin, &mag) in stft_magnitude.iter().enumerate() {
            let freq = stft_bin as f32 * freq_resolution;

            if freq > self.min_freq && freq < self.max_freq {
                let log_bin = self.freq_to_bin(freq) as usize;
                if log_bin < 96 {
                    log_spec[log_bin] += mag;
                }
            }
        }

        // Normalize
        let sum: f32 = log_spec.iter().sum();
        if sum > 0.0 {
            for bin in &mut log_spec {
                *bin /= sum;
            }
        }

        log_spec
    }

    /// Detect fundamental in any octave (octave-invariant)
    pub fn detect_fundamental_octave(&self, log_spec: &[f32; 96]) -> (u32, f32) {
        let mut best_octave = 0u32;
        let mut best_confidence = 0.0f32;

        // Each octave = 8 bins
        for octave in 0..12 {
            let start = (octave * 8) as usize;
            let end = ((octave + 1) * 8) as usize;

            let octave_energy: f32 = log_spec[start..end].iter().sum();

            if octave_energy > best_confidence {
                best_confidence = octave_energy;
                best_octave = octave;
            }
        }

        (best_octave, best_confidence)
    }

    /// Extract octave pattern: which octaves have energy?
    /// Harassment often concentrated in 1-2 octaves (coherent signal)
    /// Noise spreads across many octaves
    pub fn octave_pattern(&self, log_spec: &[f32; 96]) -> [f32; 12] {
        let mut pattern = [0.0f32; 12];

        for octave in 0..12 {
            let start = (octave * 8) as usize;
            let end = ((octave + 1) * 8) as usize;
            pattern[octave] = log_spec[start..end].iter().sum();
        }

        // Normalize
        let sum: f32 = pattern.iter().sum();
        if sum > 0.0 {
            for p in &mut pattern {
                *p /= sum;
            }
        }

        pattern
    }

    /// Harmonic coherence across octaves
    /// Real signal: harmonics appear at regular octave intervals
    /// Noise: random distribution
    pub fn harmonic_coherence(&self, log_spec: &[f32; 96]) -> f32 {
        // Check if there's energy at octave spacings (bin_offset = 8)
        let mut octave_correlation = 0.0f32;
        let mut count = 0;

        for base_bin in 0..88 {
            let octave_bin = base_bin + 8;
            if octave_bin < 96 {
                octave_correlation += log_spec[base_bin] * log_spec[octave_bin];
                count += 1;
            }
        }

        if count > 0 {
            octave_correlation / count as f32
        } else {
            0.0
        }
    }

    pub fn extract(&self, stft_magnitude: &[f32], sample_rate: u32) -> WidebandHarmonicFeatures {
        // Extract baseband peaks (top 10)
        let mut peaks = Vec::new();
        let freq_resolution = sample_rate as f32 / stft_magnitude.len() as f32;
        for (i, &mag) in stft_magnitude.iter().enumerate() {
            if mag > 0.1 {
                // Threshold
                peaks.push((i as f32 * freq_resolution, mag));
            }
        }
        peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let baseband_harmonics: Vec<f32> = peaks.iter().take(10).map(|x| x.0).collect();
        let log_spec = self.linear_to_log_spectrogram(stft_magnitude, sample_rate);
        let (fund_octave, fund_conf) = self.detect_fundamental_octave(&log_spec);
        let pattern = self.octave_pattern(&log_spec);
        let coherence = self.harmonic_coherence(&log_spec);

        WidebandHarmonicFeatures {
            log_spectrogram: log_spec,
            fundamental_octave: fund_octave,
            fundamental_confidence: fund_conf,
            harmonic_coherence: coherence,
            octave_pattern: pattern,
            baseband_harmonics,
        }
    }
}
