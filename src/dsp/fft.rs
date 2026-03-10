//! FFT processing

use rustfft::{Fft, FftPlanner};
use num_complex::Complex32;

/// FFT processor
pub struct FFTProcessor {
    fft: std::sync::Arc<dyn Fft<f32>>,
    ifft: std::sync::Arc<dyn Fft<f32>>,
    size: usize,
}

impl FFTProcessor {
    pub fn new(size: usize) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(size);
        let ifft = planner.plan_fft_inverse(size);
        Self { fft, ifft, size }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn process(&self, input: &mut [Complex32]) {
        self.fft.process_with_scratch(input, &mut vec![Complex32::default(); self.size]);
    }

    pub fn process_inverse(&self, input: &mut [Complex32]) {
        self.ifft.process_with_scratch(input, &mut vec![Complex32::default(); self.size]);
    }
}
