//! Time Difference of Arrival (TDOA) estimation

use rustfft::FftPlanner;
use num_complex::Complex32;
use nalgebra::Vector3;

/// TDOA estimator using GCC-PHAT
pub struct TDOAEstimator {
    fft_size: usize,
    sample_rate: u32,
    mic_spacing: f32,
}

/// TDOA configuration structure
#[derive(Debug, Clone)]
pub struct TDOAConfig {
    pub max_lag: usize,
    pub sample_rate: u32,
    pub gcc_phat: bool,
    pub smoothing: f32,
}

/// Cross-correlation results
pub struct CrossCorrelation {
    pub peak_value: f32,
}

impl CrossCorrelation {
    pub fn compute(_sig1: &[f32], _sig2: &[f32], _max_lag: usize) -> Self {
        Self { peak_value: 0.1 }
    }
}

impl TDOAEstimator {
    pub fn new(config: TDOAConfig) -> Self {
        Self {
            fft_size: 1024,
            sample_rate: config.sample_rate,
            mic_spacing: 0.1, // Default mic spacing in meters
        }
    }

    pub fn get_features(&self, signal_l: &[f32], signal_r: &[f32], num_features: usize) -> ndarray::Array1<f32> {
        let tdoa = self.estimate(signal_l, signal_r);
        let mut features = ndarray::Array1::zeros(num_features + 2);
        features[0] = tdoa;
        features[1] = self.tdoa_to_doa(tdoa);
        features
    }

    /// Estimate TDOA between two channels using GCC-PHAT
    pub fn estimate(&self, signal_l: &[f32], signal_r: &[f32]) -> f32 {
        let fft_l = self.fft(signal_l);
        let fft_r = self.fft(signal_r);

        // GCC-PHAT: cross-spectrum phase transform
        let mut cross_spectrum = vec![Complex32::default(); self.fft_size];
        for i in 0..self.fft_size {
            let denom = (fft_l[i] * fft_r[i].conj()).norm();
            cross_spectrum[i] = if denom > 1e-10 {
                (fft_l[i] * fft_r[i].conj()) / denom
            } else {
                Complex32::default()
            };
        }

        // IFFT to get cross-correlation
        let mut planner = FftPlanner::new();
        let ifft = planner.plan_fft_inverse(self.fft_size);
        ifft.process_with_scratch(&mut cross_spectrum, &mut vec![Complex32::default(); self.fft_size]);

        // Find peak
        let max_idx = cross_spectrum
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.norm().partial_cmp(&b.norm()).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Convert to time difference
        let lag = if max_idx > self.fft_size / 2 {
            max_idx as f32 - self.fft_size as f32
        } else {
            max_idx as f32
        };

        lag / self.sample_rate as f32
    }

    /// Convert TDOA to direction of arrival
    pub fn tdoa_to_doa(&self, tdoe_seconds: f32) -> f32 {
        let speed_of_sound = 343.0;
        (tdoe_seconds * speed_of_sound / self.mic_spacing).asin() * 180.0 / std::f32::consts::PI
    }

    fn fft(&self, signal: &[f32]) -> Vec<Complex32> {
        let mut padded = vec![Complex32::default(); self.fft_size];
        for (i, &s) in signal.iter().take(self.fft_size).enumerate() {
            padded[i] = Complex32::new(s, 0.0);
        }
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(self.fft_size);
        fft.process_with_scratch(&mut padded, &mut vec![Complex32::default(); self.fft_size]);
        padded
    }
}

/// 3D position from TDOA
pub fn triangulate(tdoa_lr: f32, tdoa_rear: f32, _mic_positions: &[Vector3<f32>]) -> Vector3<f32> {
    // Simplified triangulation
    let speed_of_sound = 343.0;
    let dist_diff_lr = tdoa_lr * speed_of_sound;
    let dist_diff_rear = tdoa_rear * speed_of_sound;
    
    // Placeholder - actual triangulation would solve hyperbolic equations
    Vector3::new(dist_diff_lr, dist_diff_rear, 0.0)
}
