use std::collections::VecDeque;

/// A 3D coordinate vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

/// Estimates elevation from 4-device energy ratios.
pub struct ElevationEstimator {
    /// Known physical positions of the 4 microphones.
    device_positions: [Vec3; 4],
    /// Per-device amplitudes history (RMS energy)
    energy_history: VecDeque<[f32; 4]>,
    /// Number of frames to keep for elevation smoothing.
    elevation_smoothing_window: usize,
    /// Previously computed elevation for exponential smoothing.
    smoothed_elevation: f32,
}

impl ElevationEstimator {
    /// Creates a new ElevationEstimator with default linear geometry positions.
    pub fn new() -> Self {
        Self {
            device_positions: [
                Vec3::new(0.0, 0.0, 0.2),  // Device 0 (C925e, top)
                Vec3::new(0.5, 0.0, 0.0),  // Device 1 (Rear Pink, middle)
                Vec3::new(1.0, 0.0, -0.2), // Device 2 (Rear Blue, bottom)
                Vec3::new(0.25, 0.0, 0.5), // Device 3 (RTL-SDR, external/elevated)
            ],
            energy_history: VecDeque::with_capacity(10),
            elevation_smoothing_window: 10,
            smoothed_elevation: 0.0,
        }
    }

    /// Estimate elevation from 4-device energy ratios
    ///
    /// # Arguments
    /// * `amplitudes` - [f32; 4] per-device RMS energy
    /// * `azimuth_rad` - Known azimuth from TDOA (for validation)
    ///
    /// # Returns
    /// (elevation_rad, confidence) where elevation ∈ [-π/2, π/2]
    pub fn estimate_elevation(&mut self, amplitudes: &[f32; 4], _azimuth_rad: f32) -> (f32, f32) {
        // Store in history
        if self.energy_history.len() == self.elevation_smoothing_window {
            self.energy_history.pop_front();
        }
        self.energy_history.push_back(*amplitudes);

        // Smooth amplitudes using a simple average over history
        let mut smooth_amps = [0.0; 4];
        let n = self.energy_history.len() as f32;
        for hist in &self.energy_history {
            for i in 0..4 {
                smooth_amps[i] += hist[i];
            }
        }
        for i in 0..4 {
            smooth_amps[i] /= n;
        }

        // Avoid division by zero
        let e1 = smooth_amps[1].max(1e-6);
        let e2 = smooth_amps[2].max(1e-6);

        // Vertical energy ratio: E_top / E_bottom
        let ratio = (smooth_amps[0] * smooth_amps[3]) / (e1 * e2);

        // Map ratio to elevation angle.
        // 1.0 -> 0.0 rad
        // > 1.0 -> > 0 rad (positive elevation)
        // < 1.0 -> < 0 rad (negative elevation)
        // Use a logarithmic mapping or sigmoid for bounded output [-π/2, π/2]

        // Simple mapping:
        // We know ratio ~ 1.0 -> 0.0
        // e.g., mapping ratio using natural log: ln(ratio) is 0 at 1, positive for >1, negative for <1.
        let raw_elevation = ratio.ln() * 0.5; // Scale factor 0.5 to keep it reasonable

        // Clamp to [-π/2, π/2]
        let clamped_elevation =
            raw_elevation.clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);

        self.smoothed_elevation = self.smooth_elevation(clamped_elevation);

        // Compute confidence based on total energy. Low energy = noise = low confidence.
        let total_energy: f32 = smooth_amps.iter().sum();
        let confidence = (total_energy * 10.0).clamp(0.0, 1.0); // Arbitrary scaling for confidence

        (self.smoothed_elevation, confidence)
    }

    /// Compute per-device attenuation via path loss
    /// Free-space path loss: L_dB = 20*log10(distance) + 20*log10(frequency)
    /// Normalized: L_norm = (L_dB - L_min) / (L_max - L_min)
    #[allow(dead_code)]
    fn compute_path_loss(&self, _azimuth: f32, _elevation: f32, freq_hz: f32) -> [f32; 4] {
        // Mock computation. In a real scenario, this would compute the expected
        // path loss from a hypothetical source to each microphone given az/el.
        let mut losses = [0.0; 4];
        for i in 0..4 {
            losses[i] =
                20.0 * (self.device_positions[i].x.max(0.1)).log10() + 20.0 * freq_hz.log10();
        }
        losses
    }

    /// Smooth elevation using Kalman-like filter (exponential moving average)
    fn smooth_elevation(&self, raw_elevation: f32) -> f32 {
        let alpha = 0.2; // EMA smoothing factor
        alpha * raw_elevation + (1.0 - alpha) * self.smoothed_elevation
    }
}
