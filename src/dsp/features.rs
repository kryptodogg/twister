//! Feature extraction for RF-Audio fusion ML

use ndarray::{Array1, Array2};
use crate::dsp::{WelchPSD, PSDConfig, TDOAEstimator, TDOAConfig};
#[cfg(feature = "rtlsdr")]
use crate::hardware::rtlsdr::IQSample;

/// RF feature vector
#[derive(Debug, Clone)]
pub struct RFFeatures {
    /// Power spectral density (256 bins)
    pub psd: Array1<f32>,
    /// Spectral kurtosis (RFI indicator)
    pub spectral_kurtosis: f32,
    /// Total band power
    pub total_power: f32,
    /// Peak frequency bin
    pub peak_bin: usize,
    /// Band power ratios (low/mid/high)
    pub band_ratios: [f32; 3],
    /// RFI detection flag
    pub rfi_detected: bool,
}

/// Audio feature vector
#[derive(Debug, Clone)]
pub struct AudioFeatures {
    /// Power spectral density (128 bins)
    pub psd: Array1<f32>,
    /// TDOA features (16 values)
    pub tdoa: Array1<f32>,
    /// Cross-correlation peak
    pub correlation_peak: f32,
    /// Residual noise energy
    pub residual_energy: f32,
    /// Channel energies
    pub channel_energies: [f32; 3],
    /// Spectral centroid
    pub spectral_centroid: f32,
    /// Spectral rolloff
    pub spectral_rolloff: f32,
    /// Zero crossing rate
    pub zcr: f32,
}

/// Combined feature vector for Mamba input
#[derive(Debug, Clone)]
pub struct FeatureVector {
    /// RF PSD (256)
    pub rf_psd: Array1<f32>,
    /// Audio PSD (128)
    pub audio_psd: Array1<f32>,
    /// TDOA features (16)
    pub tdoa: Array1<f32>,
    /// ANC state (32)
    pub anc_state: Array1<f32>,
}

impl FeatureVector {
    /// Total dimension
    pub const DIM: usize = 432; // 256 + 128 + 16 + 32

    /// Concatenate all features into single array
    pub fn to_array(&self) -> Array1<f32> {
        let mut features = Vec::with_capacity(Self::DIM);
        features.extend_from_slice(self.rf_psd.as_slice().expect("PSD array should be contiguous"));
        features.extend_from_slice(self.audio_psd.as_slice().expect("PSD array should be contiguous"));
        features.extend_from_slice(self.tdoa.as_slice().expect("TDOA array should be contiguous"));
        features.extend_from_slice(self.anc_state.as_slice().expect("ANC array should be contiguous"));
        Array1::from_vec(features)
    }

    /// Create from component arrays
    pub fn new(
        rf_psd: Array1<f32>,
        audio_psd: Array1<f32>,
        tdoa: Array1<f32>,
        anc_state: Array1<f32>,
    ) -> Self {
        Self {
            rf_psd,
            audio_psd,
            tdoa,
            anc_state,
        }
    }

    /// Create zero-initialized feature vector
    pub fn zeros() -> Self {
        Self {
            rf_psd: Array1::zeros(256),
            audio_psd: Array1::zeros(128),
            tdoa: Array1::zeros(16),
            anc_state: Array1::zeros(32),
        }
    }
}

/// Feature extractor for RF-Audio fusion
pub struct FeatureExtractor {
    /// RF PSD estimator
    rf_psd: WelchPSD,
    /// Audio PSD estimator
    audio_psd: WelchPSD,
    /// TDOA estimator
    tdoa: TDOAEstimator,
    /// RF sample rate
    rf_sample_rate: u32,
    /// Audio sample rate
    audio_sample_rate: u32,
}

impl FeatureExtractor {
    /// Create a new feature extractor
    pub fn new(rf_sample_rate: u32, audio_sample_rate: u32) -> Self {
        let rf_psd_config = PSDConfig {
            fft_size: 512,
            overlap: 0.5,
            num_averages: 4,
            window: crate::dsp::psd::WindowType::Hann,
        };

        let audio_psd_config = PSDConfig {
            fft_size: 256,
            overlap: 0.5,
            num_averages: 4,
            window: crate::dsp::psd::WindowType::Hann,
        };

        let tdoa_config = TDOAConfig {
            max_lag: 64,
            sample_rate: audio_sample_rate,
            gcc_phat: true,
            smoothing: 0.1,
        };

        Self {
            rf_psd: WelchPSD::new(rf_psd_config),
            audio_psd: WelchPSD::new(audio_psd_config),
            tdoa: TDOAEstimator::new(tdoa_config),
            rf_sample_rate,
            audio_sample_rate,
        }
    }

    /// Extract RF features from IQ samples
    #[cfg(feature = "rtlsdr")]
    pub fn extract_rf_features(&self, iq: &[IQSample]) -> RFFeatures {
        // Compute PSD
        let psd = self.rf_psd.compute_iq(iq);
        
        // Resize to 256 bins if needed
        let psd_256 = if psd.len() >= 256 {
            psd.slice(s![..256]).to_owned()
        } else {
            let mut padded = Array1::zeros(256);
            padded.slice_mut(s![..psd.len()]).assign(&psd);
            padded
        };

        // Compute spectral kurtosis
        let mean = psd.mean().unwrap_or(0.0);
        let std = psd.std(1.0);
        let kurtosis = if std > 1e-10 {
            psd.mapv(|x| (x - mean).powi(4)).mean().unwrap_or(0.0) / (std.powi(4) + 1e-10) - 3.0
        } else {
            0.0
        };

        // Total power
        let total_power = psd.sum();

        // Peak bin
        let mut peak_bin = 0;
        let mut max_val = f32::NEG_INFINITY;
        for (i, &v) in psd.iter().enumerate() {
            if v > max_val {
                max_val = v;
                peak_bin = i;
            }
        }

        // Band power ratios (low/mid/high)
        let n = psd.len();
        let low_power = psd.slice(s![..n/3]).sum();
        let mid_power = psd.slice(s![n/3..2*n/3]).sum();
        let high_power = psd.slice(s![2*n/3..]).sum();
        let total = low_power + mid_power + high_power + 1e-10;
        let band_ratios = [
            low_power / total,
            mid_power / total,
            high_power / total,
        ];

        // RFI detection (high kurtosis indicates narrowband interference)
        let rfi_detected = kurtosis > 2.0;

        RFFeatures {
            psd: psd_256,
            spectral_kurtosis: kurtosis,
            total_power,
            peak_bin,
            band_ratios,
            rfi_detected,
        }
    }

    /// Extract audio features from multi-channel audio
    pub fn extract_audio_features(
        &self,
        channels: &[Vec<f32>],
    ) -> AudioFeatures {
        // Compute PSD for first channel (or average)
        let psd = if !channels.is_empty() {
            self.audio_psd.compute(&channels[0])
        } else {
            Array1::zeros(128)
        };

        // Resize to 128 bins if needed
        let psd_128 = if psd.len() >= 128 {
            psd.slice(s![..128]).to_owned()
        } else {
            let mut padded = Array1::zeros(128);
            padded.slice_mut(s![..psd.len()]).assign(&psd);
            padded
        };

        // TDOA features (between channel 0 and 1)
        let tdoa_features = if channels.len() >= 2 {
            self.tdoa.get_features(&channels[0], &channels[1], 14)
        } else {
            Array1::zeros(16)
        };

        // Cross-correlation peak
        let correlation_peak = if channels.len() >= 2 {
            let corr = crate::dsp::tdoa::CrossCorrelation::compute(
                &channels[0],
                &channels[1],
                64,
            );
            corr.peak_value
        } else {
            0.0
        };

        // Channel energies
        let channel_energies = [
            channels.get(0).map(|c| c.iter().map(|&s| s * s).sum::<f32>()).unwrap_or(0.0),
            channels.get(1).map(|c| c.iter().map(|&s| s * s).sum::<f32>()).unwrap_or(0.0),
            channels.get(2).map(|c| c.iter().map(|&s| s * s).sum::<f32>()).unwrap_or(0.0),
        ];

        // Residual energy (difference between channels)
        let residual_energy = if channels.len() >= 2 {
            channels[0]
                .iter()
                .zip(channels[1].iter())
                .map(|(&a, &b)| (a - b).powi(2))
                .sum::<f32>()
                / channels[0].len() as f32
        } else {
            0.0
        };

        // Spectral centroid
        let freqs: Vec<f32> = (0..psd_128.len()).map(|i| i as f32).collect();
        let total_weight = psd_128.sum() + 1e-10;
        let spectral_centroid = freqs
            .iter()
            .zip(psd_128.iter())
            .map(|(&f, &p)| f * p)
            .sum::<f32>()
            / total_weight;

        // Spectral rolloff (frequency below which 85% of energy is contained)
        let cumulative: Vec<f32> = psd_128
            .iter()
            .scan(0.0, |acc, &p| {
                *acc += p;
                Some(*acc)
            })
            .collect();
        let threshold = cumulative.last().copied().unwrap_or(1.0) * 0.85;
        let spectral_rolloff = cumulative
            .iter()
            .position(|&c| c >= threshold)
            .unwrap_or(psd_128.len() - 1) as f32;

        // Zero crossing rate
        let zcr = if !channels.is_empty() {
            let signal = &channels[0];
            let crossings = signal
                .windows(2)
                .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
                .count();
            crossings as f32 / signal.len() as f32
        } else {
            0.0
        };

        AudioFeatures {
            psd: psd_128,
            tdoa: tdoa_features,
            correlation_peak,
            residual_energy,
            channel_energies,
            spectral_centroid,
            spectral_rolloff,
            zcr,
        }
    }

    /// Extract combined feature vector
    #[cfg(feature = "rtlsdr")]
    pub fn extract_features(
        &self,
        iq: &[IQSample],
        audio_channels: &[Vec<f32>],
        anc_state: &[f32; 32],
    ) -> FeatureVector {
        let rf_features = self.extract_rf_features(iq);
        let audio_features = self.extract_audio_features(audio_channels);

        FeatureVector {
            rf_psd: rf_features.psd,
            audio_psd: audio_features.psd,
            tdoa: audio_features.tdoa,
            anc_state: Array1::from_vec(anc_state.to_vec()),
        }
    }

    /// Extract features with default ANC state
    #[cfg(feature = "rtlsdr")]
    pub fn extract_features_default(&self, iq: &[IQSample], audio_channels: &[Vec<f32>]) -> FeatureVector {
        self.extract_features(iq, audio_channels, &[0.0; 32])
    }
}

use ndarray::s;

#[cfg(test)]
mod tests {
    use super::*;
    use num_complex::Complex;

    #[cfg(feature = "rtlsdr")]
    #[test]
    fn test_rf_features() {
        let extractor = FeatureExtractor::new(2_048_000, 192_000);
        
        // Generate test IQ data (sine wave)
        let iq: Vec<IQSample> = (0..1024)
            .map(|i| Complex::new((i as f32 * 0.1).sin(), (i as f32 * 0.1).cos()))
            .collect();
        
        let features = extractor.extract_rf_features(&iq);
        
        assert_eq!(features.psd.len(), 256);
        assert!(features.total_power > 0.0);
        assert!(features.peak_bin < 256);
    }

    #[test]
    fn test_audio_features() {
        let extractor = FeatureExtractor::new(2_048_000, 192_000);
        
        // Generate test audio (3 channels)
        let channels: Vec<Vec<f32>> = (0..3)
            .map(|ch| (0..1024).map(|i| ((i + ch * 100) as f32 * 0.01).sin()).collect())
            .collect();
        
        let features = extractor.extract_audio_features(&channels);
        
        assert_eq!(features.psd.len(), 128);
        assert_eq!(features.tdoa.len(), 16);
        assert!(features.channel_energies.iter().any(|&e| e > 0.0));
    }

    #[test]
    fn test_feature_vector_concatenation() {
        let fv = FeatureVector::zeros();
        let array = fv.to_array();
        
        assert_eq!(array.len(), 432);
    }

    #[test]
    fn test_feature_vector_dimensions() {
        assert_eq!(FeatureVector::DIM, 432);
    }
}
