//! SNR (Signal-to-Noise Ratio) estimation

use ndarray::Array1;

/// SNR estimation result
#[derive(Debug, Clone)]
pub struct SNREstimate {
    /// Estimated SNR in dB
    pub snr_db: f32,
    /// Signal power in dB
    pub signal_power_db: f32,
    /// Noise power in dB
    pub noise_power_db: f32,
    /// Confidence (0-1)
    pub confidence: f32,
    /// Number of samples used
    pub num_samples: usize,
}

/// SNR estimator
pub struct SNREstimator {
    /// Noise floor estimate (dB)
    noise_floor_db: f32,
    /// Smoothing factor
    smoothing: f32,
    /// Minimum signal duration (samples)
    min_duration: usize,
}

impl SNREstimator {
    /// Create a new SNR estimator
    pub fn new(noise_floor_db: f32, smoothing: f32) -> Self {
        Self {
            noise_floor_db,
            smoothing,
            min_duration: 256,
        }
    }

    /// Estimate SNR from audio samples
    pub fn estimate(&self, samples: &[f32]) -> SNREstimate {
        if samples.len() < self.min_duration {
            return SNREstimate {
                snr_db: 0.0,
                signal_power_db: 0.0,
                noise_power_db: self.noise_floor_db,
                confidence: 0.0,
                num_samples: samples.len(),
            };
        }

        // Compute signal power
        let signal_power: f32 = samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32;
        let signal_power_db = 10.0 * signal_power.log10();

        // Estimate noise power (using percentile method)
        let mut sorted = samples.iter().map(|&s| s * s).collect::<Vec<f32>>();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        // Use lower 10th percentile as noise estimate
        let noise_idx = sorted.len() / 10;
        let noise_power = sorted.get(noise_idx).copied().unwrap_or(signal_power);
        let noise_power_db = 10.0 * noise_power.max(1e-10).log10();

        // Compute SNR
        let snr_db = signal_power_db - noise_power_db.max(self.noise_floor_db);

        // Confidence based on sample count and SNR stability
        let confidence = (samples.len() as f32 / 10000.0).min(1.0);

        SNREstimate {
            snr_db: snr_db.clamp(-20.0, 120.0),
            signal_power_db,
            noise_power_db: noise_power_db.max(self.noise_floor_db),
            confidence,
            num_samples: samples.len(),
        }
    }

    /// Estimate SNR from PSD
    pub fn estimate_from_psd(&self, psd: &Array1<f32>, signal_bins: &[usize]) -> SNREstimate {
        if psd.is_empty() || signal_bins.is_empty() {
            return SNREstimate {
                snr_db: 0.0,
                signal_power_db: 0.0,
                noise_power_db: self.noise_floor_db,
                confidence: 0.0,
                num_samples: 0,
            };
        }

        // Signal power from specified bins
        let signal_power: f32 = signal_bins
            .iter()
            .filter_map(|&i| psd.get(i))
            .sum();
        let signal_power_db = 10.0 * signal_power.max(1e-10).log10();

        // Noise power from remaining bins
        let noise_bins: Vec<usize> = (0..psd.len())
            .filter(|i| !signal_bins.contains(i))
            .collect();
        
        let noise_power: f32 = noise_bins
            .iter()
            .filter_map(|&i| psd.get(i))
            .sum::<f32>()
            / noise_bins.len() as f32;
        let noise_power_db = 10.0 * noise_power.max(1e-10).log10();

        // Compute SNR
        let snr_db = signal_power_db - noise_power_db.max(self.noise_floor_db);

        // Confidence based on number of signal bins
        let confidence = (signal_bins.len() as f32 / psd.len() as f32).min(1.0);

        SNREstimate {
            snr_db: snr_db.clamp(-20.0, 120.0),
            signal_power_db,
            noise_power_db: noise_power_db.max(self.noise_floor_db),
            confidence,
            num_samples: psd.len(),
        }
    }

    /// Update noise floor estimate
    pub fn update_noise_floor(&mut self, samples: &[f32]) {
        if samples.len() < self.min_duration {
            return;
        }

        // Use lower percentile as noise estimate
        let mut sorted = samples.iter().map(|&s| s * s).collect::<Vec<f32>>();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let noise_idx = sorted.len() / 20; // 5th percentile
        let noise_power = sorted.get(noise_idx).copied().unwrap_or(1e-10);
        let noise_db = 10.0 * noise_power.log10();

        // Smooth update
        self.noise_floor_db = self.smoothing * self.noise_floor_db
            + (1.0 - self.smoothing) * noise_db;
    }

    /// Get current noise floor
    pub fn noise_floor(&self) -> f32 {
        self.noise_floor_db
    }

    /// Set noise floor
    pub fn set_noise_floor(&mut self, noise_floor_db: f32) {
        self.noise_floor_db = noise_floor_db;
    }
}

impl Default for SNREstimator {
    fn default() -> Self {
        Self::new(-60.0, 0.95)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snr_estimator_sine() {
        let estimator = SNREstimator::new(-60.0, 0.95);
        
        // Generate sine wave (high SNR)
        let samples: Vec<f32> = (0..10000)
            .map(|i| (i as f32 * 0.01).sin() * 0.5)
            .collect();
        
        let estimate = estimator.estimate(&samples);
        
        assert!(estimate.snr_db > 20.0, "SNR should be high for sine wave");
        assert!(estimate.confidence > 0.5);
        assert_eq!(estimate.num_samples, 10000);
    }

    #[test]
    fn test_snr_estimator_noise() {
        let estimator = SNREstimator::new(-60.0, 0.95);
        
        // Generate white noise (low SNR)
        use rand_distr::{Distribution, Normal};
        let normal = Normal::new(0.0, 0.1).unwrap();
        let mut rng = rand::thread_rng();
        let samples: Vec<f32> = (0..10000)
            .map(|_| normal.sample(&mut rng))
            .collect();
        
        let estimate = estimator.estimate(&samples);
        
        // SNR should be low for pure noise
        assert!(estimate.snr_db < 20.0, "SNR should be low for noise");
    }

    #[test]
    fn test_snr_from_psd() {
        use ndarray::Array1;
        
        let estimator = SNREstimator::new(-60.0, 0.95);
        
        // Create PSD with peak at bin 10
        let mut psd = Array1::zeros(256);
        psd[10] = 1.0;
        psd[11] = 0.8;
        psd[9] = 0.5;
        // Rest is noise floor
        for i in 0..256 {
            if i != 9 && i != 10 && i != 11 {
                psd[i] = 0.001;
            }
        }
        
        let estimate = estimator.estimate_from_psd(&psd, &[9, 10, 11]);
        
        assert!(estimate.snr_db > 10.0, "SNR should detect peak");
        assert!(estimate.confidence > 0.0);
    }

    #[test]
    fn test_noise_floor_update() {
        let mut estimator = SNREstimator::new(-60.0, 0.95);
        
        // Update with quiet noise
        let noise: Vec<f32> = (0..10000).map(|_| 0.001).collect();
        estimator.update_noise_floor(&noise);
        
        // Noise floor should be updated
        assert!(estimator.noise_floor() < -50.0);
    }
}
