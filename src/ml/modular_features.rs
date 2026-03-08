use crate::features::AudioFeatures;
use crate::ml::fold_frequency_harmonics::{FoldFrequencyAnalyzer, FoldFrequencyFeatures};
use crate::ml::impulse_modulation::{ImpulseModulationAnalyzer, ModulationFeatures};
use crate::ml::wideband_harmonic_analysis::{WidebandHarmonicAnalyzer, WidebandHarmonicFeatures};

pub struct ForensicFeatures {
    pub base_audio_features: Option<AudioFeatures>, // Example 196-D
    pub wideband_harmonics: Option<WidebandHarmonicFeatures>, // 110-D
    pub fold_features: Option<FoldFrequencyFeatures>, // 28-D
    pub modulation_features: Option<ModulationFeatures>, // 67-D
}

#[derive(Debug, Clone)]
pub struct FeatureFlags {
    pub use_harmonic_analysis: bool,
    pub fold_correction: bool,
    pub use_impulse_detection: bool,
}

pub struct ModularFeatureExtractor {
    pub audio_features: Vec<f32>, // Simulated 196-D
    pub wideband_analyzer: WidebandHarmonicAnalyzer,
    pub fold_analyzer: FoldFrequencyAnalyzer,
    pub modulation_analyzer: ImpulseModulationAnalyzer,
    pub audio_fft_mag: Vec<f32>,
    pub sample_rate: u32,
}

impl ModularFeatureExtractor {
    pub fn new(sample_rate: u32, audio_features: Vec<f32>, audio_fft_mag: Vec<f32>) -> Self {
        Self {
            audio_features,
            wideband_analyzer: WidebandHarmonicAnalyzer::new(),
            fold_analyzer: FoldFrequencyAnalyzer::new(sample_rate),
            modulation_analyzer: ImpulseModulationAnalyzer::new(sample_rate),
            audio_fft_mag,
            sample_rate,
        }
    }

    pub fn extract(&self, time_domain_samples: &[f32], flags: &FeatureFlags) -> Vec<f32> {
        let mut features = self.audio_features.clone();

        if flags.use_harmonic_analysis {
            let mag = self.audio_fft_mag.clone();

            // Wideband harmonics
            let harmonics = self.wideband_analyzer.extract(&mag, self.sample_rate);

            // Add log-frequency representation
            features.extend_from_slice(&harmonics.log_spectrogram); // +96-D
            features.extend_from_slice(&harmonics.octave_pattern); // +12-D
            features.push(harmonics.fundamental_confidence); // +1-D
            features.push(harmonics.harmonic_coherence); // +1-D
                                                         // Total: +110-D

            if flags.fold_correction {
                let aliased_energy = 0.5; // Placeholder for actual aliased energy computation
                let fold_features = self
                    .fold_analyzer
                    .extract(&harmonics.baseband_harmonics, aliased_energy);

                features.extend_from_slice(&fold_features.fold_frequency_map); // +10-D
                features.push(fold_features.aliased_energy); // +1-D
                features.push(fold_features.fold_coherence); // +1-D
                features.extend_from_slice(&fold_features.pulse_train_signature);
            // +16-D
            // Total: +28-D
            } else {
                features.extend_from_slice(&[0.0; 28]);
            }
        }

        if flags.use_impulse_detection {
            let mod_features = self.modulation_analyzer.extract(time_domain_samples);

            features.extend_from_slice(&mod_features.modulation_envelope); // +64-D
            features.push(mod_features.modulation_frequency / 10000.0); // +1-D (normalized)
            features.push(mod_features.modulation_entropy); // +1-D
            features.push(mod_features.modulation_periodicity); // +1-D
                                                                // Total: +67-D
        } else {
            features.extend_from_slice(&[0.0; 67]);
        }

        features
    }
}
