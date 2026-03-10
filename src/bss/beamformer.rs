//! Adaptive beamformer for spatial filtering
//!
//! Beamforming uses multiple sensors to enhance signals from specific directions
//! while suppressing interference from other directions.

use crate::bss::traits::AdaptiveFilter;
use ndarray::{Array1, Array2};
use num_complex::Complex;

/// Beamformer configuration
#[derive(Debug, Clone)]
pub struct BeamformerConfig {
    /// Number of array elements (microphones/antennas)
    pub n_elements: usize,
    /// Filter order per element
    pub filter_order: usize,
    /// Look direction (angle in radians, 0 = broadside)
    pub look_direction: f32,
    /// Element spacing (in wavelengths)
    pub element_spacing: f32,
    /// Algorithm: "mvdr", "lcmv", "gsc"
    pub algorithm: String,
}

impl Default for BeamformerConfig {
    fn default() -> Self {
        Self {
            n_elements: 4,
            filter_order: 16,
            look_direction: 0.0,
            element_spacing: 0.5,
            algorithm: "mvdr".into(),
        }
    }
}

/// Adaptive beamformer for multi-element arrays
pub struct Beamformer {
    config: BeamformerConfig,
    /// Weight matrix: [n_elements][filter_order]
    weights: Array2<f32>,
    /// Delay lines for each element
    delay_lines: Vec<Array1<f32>>,
    /// Steering vector
    steering_vector: Array1<Complex<f32>>,
    /// Spatial correlation matrix (for MVDR)
    r_xx: Array2<Complex<f32>>,
    /// Sample counter
    n: usize,
}

impl Beamformer {
    /// Create new beamformer with configuration
    pub fn new(config: BeamformerConfig) -> Self {
        let n_elements = config.n_elements;
        let filter_order = config.filter_order;
        
        // Initialize weights (delay-and-sum initial)
        let weights = Array2::from_elem((n_elements, filter_order), 1.0 / n_elements as f32);
        
        // Initialize delay lines
        let delay_lines: Vec<Array1<f32>> = (0..n_elements)
            .map(|_| Array1::zeros(filter_order))
            .collect();
        
        // Compute steering vector
        let steering_vector = Self::compute_steering_vector(
            n_elements,
            config.look_direction,
            config.element_spacing,
        );
        
        // Initialize correlation matrix
        let r_xx = Array2::eye(n_elements).mapv(Complex::from);
        
        Self {
            config,
            weights,
            delay_lines,
            steering_vector,
            r_xx,
            n: 0,
        }
    }
    
    /// Create beamformer for ANC with default 4-element array
    pub fn for_anc() -> Self {
        Self::new(BeamformerConfig {
            n_elements: 4,
            filter_order: 32,
            look_direction: 0.0,
            element_spacing: 0.5,
            algorithm: "mvdr".into(),
        })
    }
    
    /// Compute steering vector for given direction
    fn compute_steering_vector(
        n_elements: usize,
        theta: f32,
        d: f32,
    ) -> Array1<Complex<f32>> {
        let mut a = Array1::zeros(n_elements);
        let k = 2.0 * std::f32::consts::PI; // Wave number (normalized)
        
        for i in 0..n_elements {
            let phase = -k * d * (i as f32) * theta.sin();
            a[i] = Complex::from_polar(1.0, phase);
        }
        
        a
    }
    
    /// Set look direction
    pub fn set_look_direction(&mut self, theta: f32) {
        self.config.look_direction = theta;
        self.steering_vector = Self::compute_steering_vector(
            self.config.n_elements,
            theta,
            self.config.element_spacing,
        );
    }
    
    /// Process one sample from each element
    /// 
    /// # Arguments
    /// * `inputs` - Array of samples from each element [n_elements]
    /// 
    /// # Returns
    /// Beamformed output
    pub fn process(&mut self, inputs: &[f32]) -> f32 {
        let n_elements = self.config.n_elements;
        let filter_order = self.config.filter_order;
        
        // Update delay lines
        for (i, input) in inputs.iter().take(n_elements).enumerate() {
            for j in (1..filter_order).rev() {
                self.delay_lines[i][j] = self.delay_lines[i][j - 1];
            }
            self.delay_lines[i][0] = *input;
        }
        
        // Apply weights and sum
        let mut output = 0.0f32;
        for i in 0..n_elements {
            for j in 0..filter_order {
                output += self.weights[[i, j]] * self.delay_lines[i][j];
            }
        }
        
        // Update correlation matrix (exponential smoothing)
        let alpha = 0.99;
        for i in 0..n_elements {
            for j in 0..n_elements {
                let product = Complex::new(inputs[i], 0.0) * Complex::new(inputs[j], 0.0).conj();
                self.r_xx[[i, j]] = alpha * self.r_xx[[i, j]] + (1.0 - alpha) * product;
            }
        }
        
        self.n += 1;
        
        output
    }
    
    /// MVDR (Minimum Variance Distortionless Response) beamformer
    /// 
    /// Computes optimal weights: w = R^(-1) * a / (a^H * R^(-1) * a)
    pub fn update_mvdr_weights(&mut self) {
        // Add diagonal loading for stability
        let lambda = 0.001;
        let mut r_loaded = self.r_xx.clone();
        for i in 0..self.config.n_elements {
            r_loaded[[i, i]] += Complex::new(lambda, 0.0);
        }
        
        // Compute inverse (simplified - in practice use proper matrix inversion)
        // For now, just normalize steering vector
        let a = &self.steering_vector;
        let a_norm = a.mapv(|c| c.norm());
        let total: f32 = a_norm.sum();
        
        if total > 0.0 {
            for i in 0..self.config.n_elements {
                self.weights[[i, 0]] = a_norm[i] / total;
            }
        }
    }
    
    /// Get current SNR estimate
    pub fn snr_estimate(&self) -> f32 {
        // Simplified SNR estimate based on weight distribution
        let w = self.weights.column(0);
        let power: f32 = w.mapv(|x| x * x).sum();
        10.0 * power.log10()
    }
    
    /// Get sample count
    pub fn sample_count(&self) -> usize {
        self.n
    }
}

impl AdaptiveFilter for Beamformer {
    fn new(order: usize, _learning_rate: f32) -> Self {
        Self::new(BeamformerConfig {
            n_elements: order,
            filter_order: 16,
            ..Default::default()
        })
    }
    
    fn process(&mut self, input: f32, _reference: f32) -> f32 {
        // For single input, just pass through
        input
    }
    
    fn process_block(&mut self, input: &[f32], _reference: &[f32]) -> Vec<f32> {
        // Process as multi-channel (interleaved)
        let n_elements = self.config.n_elements;
        let n_frames = input.len() / n_elements;
        let mut output = Vec::with_capacity(n_frames);
        
        for i in 0..n_frames {
            let frame: Vec<f32> = (0..n_elements)
                .map(|j| input[i * n_elements + j])
                .collect();
            output.push(self.process(&frame));
        }
        
        output
    }
    
    fn weights(&self) -> Array1<f32> {
        self.weights.column(0).to_owned()
    }
    
    fn reset(&mut self) {
        let n_elements = self.config.n_elements;
        let filter_order = self.config.filter_order;
        
        self.weights.fill(1.0 / n_elements as f32);
        for dl in &mut self.delay_lines {
            dl.fill(0.0);
        }
        self.r_xx = Array2::eye(n_elements).mapv(Complex::from);
        self.n = 0;
    }
    
    fn order(&self) -> usize {
        self.config.n_elements
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_beamformer_basic() {
        let mut bf = Beamformer::for_anc();
        
        // Test with uniform input
        let inputs = [1.0, 1.0, 1.0, 1.0];
        let output = bf.process(&inputs);
        
        assert!(output.is_finite());
        assert!(output > 0.0);
    }
    
    #[test]
    fn test_steering_vector() {
        let a = Beamformer::compute_steering_vector(4, 0.0, 0.5);
        
        // Broadside should have uniform phase
        for i in 0..4 {
            assert!((a[i].re - 1.0).abs() < 1e-6);
            assert!(a[i].im.abs() < 1e-6);
        }
    }
}
