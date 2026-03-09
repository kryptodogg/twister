pub struct FoldFrequencyAnalyzer {
    sample_rate: u32,
    nyquist: f32,
}

#[derive(Debug, Clone)]
pub struct FoldFrequencyFeatures {
    pub baseband_harmonics: Vec<f32>,     // Detected in [0, Nyquist]
    pub fold_frequency_map: [f32; 10], // What could these be if aliased from above? (Fixed to 10-D)
    pub aliased_energy: f32,           // Total energy in aliased band
    pub fold_coherence: f32,           // Do aliased harmonics align with real ones?
    pub pulse_train_signature: [f32; 16], // Harmonic spacing pattern
}

impl FoldFrequencyAnalyzer {
    pub fn new(sample_rate: u32) -> Self {
        let nyquist = sample_rate as f32 / 2.0;
        Self {
            sample_rate,
            nyquist,
        }
    }

    pub fn compute_fold_maps(&self, detected_frequencies: &[f32]) -> Vec<Vec<f32>> {
        let mut fold_maps = Vec::new();

        for &freq in detected_frequencies {
            let mut aliases = vec![freq]; // Real frequency

            // Compute aliases in higher Nyquist zones
            let mut zone = 1;
            loop {
                let zone_offset = zone as f32 * self.nyquist;

                // Alias in upper zone: zone_offset - freq
                let alias_upper = zone_offset - freq;
                if alias_upper > 0.0 && alias_upper < zone_offset {
                    aliases.push(alias_upper);
                }

                // Alias in next zone: zone_offset + freq
                let alias_next = zone_offset + freq;
                if alias_next < 100_000_000.0 {
                    // Stop at 100 MHz (practical limit)
                    aliases.push(alias_next);
                }

                zone += 1;
                if zone > 10 {
                    break;
                } // Limit to 10 zones
            }

            fold_maps.push(aliases);
        }

        fold_maps
    }

    pub fn pulse_train_signature(&self, baseband_harmonics: &[f32]) -> [f32; 16] {
        let mut signature = [0.0f32; 16];

        if baseband_harmonics.is_empty() {
            return signature;
        }

        // Assume first harmonic is fundamental
        let f0 = baseband_harmonics[0];

        // Check for harmonics at 2f0, 3f0, 4f0... up to 16f0
        for h in 1..=16 {
            let harmonic_freq = f0 * h as f32;

            // Find if this harmonic is present (within tolerance)
            let tolerance = f0 * 0.05; // 5% tolerance
            for &detected in baseband_harmonics {
                if (detected - harmonic_freq).abs() < tolerance {
                    signature[h - 1] = detected; // Mark as detected
                    break;
                }
            }
        }

        signature
    }

    pub fn fold_coherence(&self, baseband: &[f32], fold_maps: &[Vec<f32>]) -> f32 {
        let mut coherence_sum = 0.0f32;
        let mut count = 0;

        for (base_freq, aliases) in baseband.iter().zip(fold_maps) {
            for alias_freq in aliases.iter().skip(1) {
                for other_base in baseband {
                    if (other_base - alias_freq).abs() < base_freq * 0.1 {
                        coherence_sum += 1.0;
                        count += 1;
                    }
                }
            }
        }

        if count > 0 {
            coherence_sum / count as f32
        } else {
            0.0
        }
    }

    pub fn extract(
        &self,
        baseband_harmonics: &[f32],
        aliased_energy: f32,
    ) -> FoldFrequencyFeatures {
        let fold_maps = self.compute_fold_maps(baseband_harmonics);
        let pulse_sig = self.pulse_train_signature(baseband_harmonics);
        let coherence = self.fold_coherence(baseband_harmonics, &fold_maps);

        let mut fold_map_fixed = [0.0f32; 10];
        for (i, v) in fold_maps.iter().take(10).enumerate() {
            fold_map_fixed[i] = v.len() as f32;
        }

        FoldFrequencyFeatures {
            baseband_harmonics: baseband_harmonics.to_vec(),
            fold_frequency_map: fold_map_fixed,
            aliased_energy,
            fold_coherence: coherence,
            pulse_train_signature: pulse_sig,
        }
    }
}
