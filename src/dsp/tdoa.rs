use rustfft::{FftPlanner, num_complex::Complex};

#[derive(Clone, Debug)]
pub struct TDOAConfig {
    pub sample_rate: f32,
    pub max_delay_s: f32,
}

/// TDOAEstimator: Time Difference of Arrival estimation via Cross-Correlation.
/// Essential for 3D spatial localization in the hologram.
pub struct TDOAEstimator {
    pub config: TDOAConfig,
    planner: FftPlanner<f32>,
}

impl TDOAEstimator {
    pub fn new(config: TDOAConfig) -> Self {
        Self {
            config,
            planner: FftPlanner::new(),
        }
    }

    /// Computes the time delay (in seconds) between two signals.
    pub fn estimate_delay(&mut self, signal_a: &[f32], signal_b: &[f32]) -> f32 {
        let n = signal_a.len().min(signal_b.len());
        if n == 0 { return 0.0; }

        let fft_size = n.next_power_of_two();
        let fft = self.planner.plan_fft_forward(fft_size);
        let ifft = self.planner.plan_fft_inverse(fft_size);

        let mut spec_a: Vec<Complex<f32>> = signal_a.iter().take(n).map(|&s| Complex::new(s, 0.0)).collect();
        spec_a.resize(fft_size, Complex::new(0.0, 0.0));
        let mut spec_b: Vec<Complex<f32>> = signal_b.iter().take(n).map(|&s| Complex::new(s, 0.0)).collect();
        spec_b.resize(fft_size, Complex::new(0.0, 0.0));

        fft.process(&mut spec_a);
        fft.process(&mut spec_b);

        // Cross-correlation in frequency domain: S_a * conj(S_b)
        for i in 0..fft_size {
            spec_a[i] = spec_a[i] * spec_b[i].conj();
        }

        ifft.process(&mut spec_a);

        // Find peak index
        let (peak_idx, _) = spec_a.iter().enumerate()
            .max_by(|(_, a), (_, b)| a.re.partial_cmp(&b.re).unwrap())
            .unwrap_or((0, &Complex::new(0.0, 0.0)));

        // Shift to signed lag
        let lag = if peak_idx > fft_size / 2 {
            peak_idx as i32 - fft_size as i32
        } else {
            peak_idx as i32
        };

        lag as f32 / self.config.sample_rate
    }
}
