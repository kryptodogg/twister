pub struct PSDConfig {
    pub fft_size: usize,
    pub overlap: usize,
}

/// WelchPSD: Power Spectral Density estimation using Welch's Method.
/// Provides low-variance frequency estimates for the BSS interface.
pub struct WelchPSD {
    pub config: PSDConfig,
    fft: crate::dsp::fft::FFTProcessor,
}

impl WelchPSD {
    pub fn new(config: PSDConfig) -> Self {
        Self {
            fft: crate::dsp::fft::FFTProcessor::new(config.fft_size),
            config,
        }
    }

    /// Computes the average power spectrum.
    pub fn compute_psd(&mut self, samples: &[f32]) -> Vec<f32> {
        if samples.len() < self.config.fft_size {
            return vec![0.0; self.config.fft_size / 2];
        }

        let mut psd_sum = vec![0.0; self.config.fft_size / 2];
        let step = self.config.fft_size - self.config.overlap;
        let mut count = 0;

        for window in samples.windows(self.config.fft_size).step_by(step) {
            let mags = self.fft.compute_magnitudes(window);
            for (i, &m) in mags.iter().enumerate() {
                psd_sum[i] += m * m;
            }
            count += 1;
        }

        if count > 0 {
            for val in psd_sum.iter_mut() {
                *val /= count as f32;
            }
        }

        psd_sum
    }
}
