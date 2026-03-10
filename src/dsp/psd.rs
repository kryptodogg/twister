//! Welch Power Spectral Density estimation

use ndarray::Array1;
use rustfft::{FftPlanner, Fft};
use num_complex::Complex;
use std::sync::Arc;

/// PSD configuration
#[derive(Debug, Clone)]
pub struct PSDConfig {
    /// FFT size
    pub fft_size: usize,
    /// Overlap ratio (0.0-1.0)
    pub overlap: f32,
    /// Number of averages
    pub num_averages: usize,
    /// Window type
    pub window: WindowType,
}

impl Default for PSDConfig {
    fn default() -> Self {
        Self {
            fft_size: 512,
            overlap: 0.5,
            num_averages: 4,
            window: WindowType::Hann,
        }
    }
}

/// Window type for PSD estimation
#[derive(Debug, Clone, Copy)]
pub enum WindowType {
    Rectangular,
    Hann,
    Hamming,
    Blackman,
}

impl WindowType {
    /// Generate window coefficients
    pub fn generate(&self, size: usize) -> Vec<f32> {
        match self {
            WindowType::Rectangular => vec![1.0f32; size],
            WindowType::Hann => (0..size)
                .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / size as f32).cos()))
                .collect(),
            WindowType::Hamming => (0..size)
                .map(|i| 0.54 - 0.46 * (2.0 * std::f32::consts::PI * i as f32 / size as f32).cos())
                .collect(),
            WindowType::Blackman => (0..size)
                .map(|i| {
                    0.42 
                    - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / size as f32).cos()
                    + 0.08 * (4.0 * std::f32::consts::PI * i as f32 / size as f32).cos()
                })
                .collect(),
        }
    }

    /// Get window normalization factor
    pub fn normalization(&self, size: usize) -> f32 {
        let window = self.generate(size);
        let sum_sq: f32 = window.iter().map(|&w| w * w).sum();
        sum_sq
    }
}

/// Welch PSD estimator
pub struct WelchPSD {
    config: PSDConfig,
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    norm_factor: f32,
}

impl WelchPSD {
    /// Create a new Welch PSD estimator
    pub fn new(config: PSDConfig) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(config.fft_size);
        
        let window = config.window.generate(config.fft_size);
        let norm_factor = config.window.normalization(config.fft_size);

        Self {
            config,
            fft,
            window,
            norm_factor,
        }
    }

    /// Compute PSD from samples
    pub fn compute(&self, samples: &[f32]) -> Array1<f32> {
        let hop_size = ((1.0 - self.config.overlap) * self.config.fft_size as f32) as usize;
        let num_segments = if hop_size > 0 {
            (samples.len().saturating_sub(self.config.fft_size)) / hop_size + 1
        } else {
            1
        };

        let num_averages = num_segments.min(self.config.num_averages);
        let mut psd = vec![0.0f32; self.config.fft_size / 2];

        for i in 0..num_averages {
            let start = i * hop_size;
            let segment = &samples[start..start + self.config.fft_size.min(samples.len() - start)];
            
            if segment.len() < self.config.fft_size {
                break;
            }

            // Apply window and compute FFT
            let mut buffered: Vec<Complex<f32>> = segment
                .iter()
                .zip(self.window.iter())
                .map(|(&s, &w)| Complex::new(s * w, 0.0))
                .collect();

            self.fft.process(&mut buffered);

            // Compute power spectrum
            for (j, bin) in psd.iter_mut().enumerate() {
                *bin += buffered[j].norm_sqr();
            }
        }

        // Average and normalize
        let scale = 1.0 / (num_averages as f32 * self.norm_factor * self.config.fft_size as f32);
        Array1::from_vec(psd.iter().map(|&p| p * scale).collect())
    }

    /// Compute PSD from complex IQ samples
    pub fn compute_iq(&self, iq: &[Complex<f32>]) -> Array1<f32> {
        let hop_size = ((1.0 - self.config.overlap) * self.config.fft_size as f32) as usize;
        let num_segments = if hop_size > 0 {
            (iq.len().saturating_sub(self.config.fft_size)) / hop_size + 1
        } else {
            1
        };

        let num_averages = num_segments.min(self.config.num_averages);
        let mut psd = vec![0.0f32; self.config.fft_size / 2];

        for i in 0..num_averages {
            let start = i * hop_size;
            let segment = &iq[start..start + self.config.fft_size.min(iq.len() - start)];
            
            if segment.len() < self.config.fft_size {
                break;
            }

            // Apply window
            let mut buffered: Vec<Complex<f32>> = segment
                .iter()
                .zip(self.window.iter())
                .map(|(&s, &w)| s * w)
                .collect();

            self.fft.process(&mut buffered);

            // Compute power spectrum
            for (j, bin) in psd.iter_mut().enumerate() {
                *bin += buffered[j].norm_sqr();
            }
        }

        // Average and normalize
        let scale = 1.0 / (num_averages as f32 * self.norm_factor * self.config.fft_size as f32);
        Array1::from_vec(psd.iter().map(|&p| p * scale).collect())
    }

    /// Get frequency bins in Hz
    pub fn frequency_bins(&self, sample_rate: u32) -> Vec<f32> {
        (0..self.config.fft_size / 2)
            .map(|i| (i as f32 * sample_rate as f32) / self.config.fft_size as f32)
            .collect()
    }

    /// Get configuration
    pub fn config(&self) -> &PSDConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_welch_psd_sine() {
        let config = PSDConfig::default();
        let psd_estimator = WelchPSD::new(config);

        // Generate sine wave
        let n_samples = 2048;
        let freq = 0.1; // Normalized frequency
        let samples: Vec<f32> = (0..n_samples)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32).sin())
            .collect();

        let psd = psd_estimator.compute(&samples);

        // Should have a peak somewhere
        let max_psd = psd.iter().cloned().fold(0.0f32, f32::max);
        assert!(max_psd > 0.0);

        // PSD should be non-negative
        assert!(psd.iter().all(|&p| p >= 0.0));
    }

    #[test]
    fn test_window_generation() {
        let window = WindowType::Hann.generate(64);
        assert_eq!(window.len(), 64);
        assert!((window[0] - 0.0).abs() < 0.001);
        assert!((window[63] - 0.0).abs() < 0.001);
        assert!(window[32] > 0.9);
    }

    #[test]
    fn test_frequency_bins() {
        let config = PSDConfig::default();
        let psd_estimator = WelchPSD::new(config);
        let bins = psd_estimator.frequency_bins(192_000);

        assert_eq!(bins.len(), 256);
        assert!((bins[0] - 0.0).abs() < 0.001);
        assert!(bins[1] > 0.0);
    }
}
