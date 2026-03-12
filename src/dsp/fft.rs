use rustfft::{FftPlanner, num_complex::Complex};

/// FFTProcessor: High-resolution spectral analysis for forensic discovery.
pub struct FFTProcessor {
    pub size: usize,
    planner: FftPlanner<f32>,
}

impl FFTProcessor {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            planner: FftPlanner::new(),
        }
    }

    /// Computes the magnitude spectrum of a real-valued signal.
    pub fn compute_magnitudes(&mut self, samples: &[f32]) -> Vec<f32> {
        let n = samples.len().min(self.size);
        let fft = self.planner.plan_fft_forward(self.size);

        let mut buffer: Vec<Complex<f32>> = samples.iter().take(n)
            .enumerate()
            .map(|(i, &s)| {
                // Hann Window
                let w = 0.5 * (1.0 - (std::f32::consts::TAU * i as f32 / (n - 1) as f32).cos());
                Complex::new(s * w, 0.0)
            })
            .collect();

        if buffer.len() < self.size {
            buffer.resize(self.size, Complex::new(0.0, 0.0));
        }

        fft.process(&mut buffer);
        buffer.iter().take(self.size / 2).map(|c| c.norm()).collect()
    }
}
