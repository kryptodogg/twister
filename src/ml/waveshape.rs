//! Waveshape Projector for Track A - Mamba Inference Loop
//! 
//! This module projects Mamba latent embeddings to Drive/Fold/Asym parameters
//! for real-time waveshaping synthesis. The projector implements the mapping
//! from high-dimensional latent space to the three key synthesis parameters.

use crate::ml::mamba::MambaModel;

/// Configuration for the waveshape projector
#[derive(Clone)]
pub struct WaveshapeConfig {
    pub drive_range: (f32, f32),    // Min/max drive values
    pub fold_range: (f32, f32),     // Min/max fold values  
    pub asym_range: (f32, f32),     // Min/max asymmetry values
    pub projection_dim: usize,      // Dimension of projection layer
}

impl Default for WaveshapeConfig {
    fn default() -> Self {
        Self {
            drive_range: (0.0, 2.0),     // Drive: 0-2x gain
            fold_range: (0.0, 1.0),      // Fold: 0-100% folding
            asym_range: (0.0, 1.0),      // Asym: 0-100% asymmetry
            projection_dim: 64,          // Matches Mamba latent dimension
        }
    }
}

/// Waveshape projector that maps Mamba embeddings to synthesis parameters
pub struct WaveshapeProjector {
    config: WaveshapeConfig,
    // In production, these would be Burn tensors
    projection_weights: Vec<f32>,
    bias: Vec<f32>,
}

impl WaveshapeProjector {
    pub fn new() -> Self {
        Self::with_config(WaveshapeConfig::default())
    }
    
    pub fn with_config(config: WaveshapeConfig) -> Self {
        // Initialize projection weights (in production, this would be learned)
        let num_params = config.projection_dim * 3; // 3 outputs: drive, fold, asym
        let projection_weights = vec![0.1; num_params];
        let bias = vec![0.0; 3];
        
        Self {
            config,
            projection_weights,
            bias,
        }
    }
    
    /// Project Mamba embeddings to Drive/Fold/Asym parameters
    pub fn project(&self, embeddings: &[f32]) -> Waveshape {
        if embeddings.is_empty() {
            return Waveshape::default();
        }
        
        // Apply linear projection
        let mut projected = vec![0.0; 3];
        
        for i in 0..3 {
            let mut sum = 0.0;
            for j in 0..embeddings.len() {
                let weight_idx = i * embeddings.len() + j;
                sum += embeddings[j] * self.projection_weights[weight_idx];
            }
            projected[i] = sum + self.bias[i];
        }
        
        // Apply activation functions and range mapping
        let drive = self.map_to_range(projected[0], self.config.drive_range, true);
        let fold = self.map_to_range(projected[1], self.config.fold_range, false);
        let asym = self.map_to_range(projected[2], self.config.asym_range, false);
        
        Waveshape { drive, fold, asym }
    }
    
    /// Map a value to a specific range with optional sigmoid activation
    fn map_to_range(&self, value: f32, range: (f32, f32), use_sigmoid: bool) -> f32 {
        let normalized = if use_sigmoid {
            // Sigmoid activation for drive (smooth saturation)
            1.0 / (1.0 + (-value).exp())
        } else {
            // Tanh activation for fold/asym (bipolar)
            value.tanh()
        };
        
        // Map to target range
        let (min_val, max_val) = range;
        min_val + (normalized * (max_val - min_val))
    }
    
    /// Get projector configuration
    pub fn config(&self) -> &WaveshapeConfig {
        &self.config
    }
}

/// Waveshape parameters for synthesis
#[derive(Debug, Clone, Copy)]
pub struct Waveshape {
    pub drive: f32,   // Gain/overdrive amount (0.0-2.0)
    pub fold: f32,    // Wavefolding intensity (0.0-1.0)  
    pub asym: f32,    // Asymmetry/wave shaping (0.0-1.0)
}

impl Default for Waveshape {
    fn default() -> Self {
        Self {
            drive: 1.0,  // Unity gain
            fold: 0.0,   // No folding
            asym: 0.5,   // Symmetric
        }
    }
}

/// Factory for creating waveshape projectors
pub struct WaveshapeFactory;

impl WaveshapeFactory {
    /// Create a default waveshape projector
    pub fn create_default() -> WaveshapeProjector {
        WaveshapeProjector::new()
    }
    
    /// Create a projector optimized for audio synthesis
    pub fn create_audio_optimized() -> WaveshapeProjector {
        let config = WaveshapeConfig {
            drive_range: (0.0, 3.0),     // Higher drive range for audio
            fold_range: (0.0, 1.0),
            asym_range: (0.0, 1.0),
            projection_dim: 64,
        };
        
        WaveshapeProjector::with_config(config)
    }
    
    /// Create a projector optimized for RF signal processing
    pub fn create_rf_optimized() -> WaveshapeProjector {
        let config = WaveshapeConfig {
            drive_range: (0.0, 1.0),     // Lower drive for RF
            fold_range: (0.0, 0.5),      // Limited folding for RF
            asym_range: (0.2, 0.8),      // Centered asymmetry for RF
            projection_dim: 64,
        };
        
        WaveshapeProjector::with_config(config)
    }
}

/// Real-time waveshape parameter interpolator
/// 
/// Smoothly interpolates between parameter sets to avoid clicks and pops
pub struct WaveshapeInterpolator {
    current: Waveshape,
    target: Waveshape,
    interpolation_time: f32,  // Time in seconds
    elapsed_time: f32,
}

impl WaveshapeInterpolator {
    pub fn new() -> Self {
        Self {
            current: Waveshape::default(),
            target: Waveshape::default(),
            interpolation_time: 0.05,  // 50ms interpolation
            elapsed_time: 0.0,
        }
    }
    
    /// Set target parameters with smooth interpolation
    pub fn set_target(&mut self, target: Waveshape, interpolation_time: f32) {
        self.target = target;
        self.interpolation_time = interpolation_time.max(0.001); // Minimum 1ms
        self.elapsed_time = 0.0;
    }
    
    /// Update interpolation state
    pub fn update(&mut self, dt: f32) -> Waveshape {
        self.elapsed_time += dt;
        
        let t = (self.elapsed_time / self.interpolation_time).min(1.0);
        
        // Linear interpolation
        let drive = self.current.drive + (self.target.drive - self.current.drive) * t;
        let fold = self.current.fold + (self.target.fold - self.current.fold) * t;
        let asym = self.current.asym + (self.target.asym - self.current.asym) * t;
        
        self.current = Waveshape { drive, fold, asym };
        
        self.current
    }
    
    /// Get current parameters without interpolation
    pub fn current(&self) -> Waveshape {
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_waveshape_projector_creation() {
        let projector = WaveshapeFactory::create_default();
        assert_eq!(projector.config().drive_range, (0.0, 2.0));
        assert_eq!(projector.config().fold_range, (0.0, 1.0));
        assert_eq!(projector.config().asym_range, (0.0, 1.0));
    }
    
    #[test]
    fn test_waveshape_projection() {
        let projector = WaveshapeFactory::create_default();
        
        // Test with random embeddings
        let embeddings = vec![0.5; 64];
        let waveshape = projector.project(&embeddings);
        
        assert!(waveshape.drive >= 0.0 && waveshape.drive <= 2.0);
        assert!(waveshape.fold >= 0.0 && waveshape.fold <= 1.0);
        assert!(waveshape.asym >= 0.0 && waveshape.asym <= 1.0);
    }
    
    #[test]
    fn test_waveshape_interpolator() {
        let mut interpolator = WaveshapeInterpolator::new();
        
        let target = Waveshape {
            drive: 2.0,
            fold: 1.0,
            asym: 0.0,
        };
        
        interpolator.set_target(target, 0.1); // 100ms interpolation
        
        // Update over time
        for _ in 0..10 {
            let current = interpolator.update(0.01); // 10ms steps
            assert!(current.drive >= 1.0 && current.drive <= 2.0);
            assert!(current.fold >= 0.0 && current.fold <= 1.0);
        }
        
        // After sufficient time, should reach target
        let final_params = interpolator.update(0.1);
        assert!((final_params.drive - 2.0).abs() < 0.01);
        assert!((final_params.fold - 1.0).abs() < 0.01);
        assert!((final_params.asym - 0.0).abs() < 0.01);
    }
    
    #[test]
    fn test_empty_embeddings() {
        let projector = WaveshapeFactory::create_default();
        let waveshape = projector.project(&[]);
        
        assert_eq!(waveshape.drive, 1.0);
        assert_eq!(waveshape.fold, 0.0);
        assert_eq!(waveshape.asym, 0.5);
    }
}