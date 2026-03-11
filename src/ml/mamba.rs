//! Mamba State Space Model for Track A - Mamba Inference Loop
//! 
//! This module implements a simplified Mamba model for processing FieldParticle data.
//! In a production implementation, this would use the Burn framework with proper
//! state space model architecture.

use crate::ml::field_particle::FieldParticle;

/// Configuration for the Mamba model
#[derive(Clone)]
pub struct MambaConfig {
    pub input_dim: usize,
    pub hidden_dim: usize,
    pub latent_dim: usize,
    pub num_layers: usize,
    pub state_dim: usize,
}

impl Default for MambaConfig {
    fn default() -> Self {
        Self {
            input_dim: 8,        // Position (3) + Phase (2) + Energy (1) + Material (1) + Padding (1)
            hidden_dim: 128,     // Hidden state dimension
            latent_dim: 64,      // Output latent dimension
            num_layers: 4,       // Number of Mamba layers
            state_dim: 32,       // State space dimension
        }
    }
}

/// Simplified Mamba State Space Model
/// 
/// This is a placeholder implementation that will be replaced with a proper
/// Burn-based Mamba model in production. For now, it provides the interface
/// and basic functionality needed for the dispatch loop.
pub struct MambaModel {
    config: MambaConfig,
    // In production, these would be Burn tensors
    weights: Vec<f32>,
    biases: Vec<f32>,
}

impl MambaModel {
    pub fn new(config: MambaConfig) -> Self {
        // Initialize with random weights (in production, this would be loaded from checkpoint)
        let num_params = config.input_dim * config.hidden_dim 
                        + config.hidden_dim * config.latent_dim
                        + config.latent_dim * config.latent_dim;
        
        let weights = vec![0.1; num_params];
        let biases = vec![0.0; config.latent_dim];
        
        Self { config, weights, biases }
    }
    
    /// Forward pass through the Mamba model
    /// 
    /// Processes a batch of FieldParticles and produces latent embeddings
    pub fn forward(&self, particles: &[FieldParticle]) -> Vec<f32> {
        if particles.is_empty() {
            return vec![0.0; self.config.latent_dim];
        }
        
        // Convert particles to input vectors
        let input_vectors = self.particles_to_input(particles);
        
        // Process through simplified Mamba layers
        let mut hidden_state = self.process_input(&input_vectors);
        
        // Apply final projection to latent space
        let mut embeddings = vec![0.0; self.config.latent_dim];
        
        for i in 0..self.config.latent_dim {
            let mut sum = 0.0;
            for j in 0..self.config.hidden_dim {
                let weight_idx = i * self.config.hidden_dim + j;
                sum += hidden_state[j] * self.weights[weight_idx];
            }
            embeddings[i] = sum + self.biases[i];
        }
        
        // Apply tanh activation
        for val in embeddings.iter_mut() {
            *val = val.tanh();
        }
        
        embeddings
    }
    
    /// Convert FieldParticles to input vectors for the Mamba model
    fn particles_to_input(&self, particles: &[FieldParticle]) -> Vec<f32> {
        let mut inputs = Vec::with_capacity(particles.len() * self.config.input_dim);
        
        for particle in particles {
            // Extract features from FieldParticle
            let position = particle.position;
            let phase_i = particle.phase_i;
            let phase_q = particle.phase_q;
            let energy = particle.energy;
            let material_id = particle.material_id as f32 / 65535.0; // Normalize to 0-1
            
            // Build input vector: [x, y, z, phase_i, phase_q, energy, material_id, padding]
            inputs.extend_from_slice(&[
                position[0], position[1], position[2],
                phase_i, phase_q, energy, material_id, 0.0
            ]);
        }
        
        inputs
    }
    
    /// Simplified processing of input vectors
    /// In production, this would implement the actual Mamba state space recurrence
    fn process_input(&self, inputs: &[f32]) -> Vec<f32> {
        let batch_size = inputs.len() / self.config.input_dim;
        let mut hidden = vec![0.0; self.config.hidden_dim];
        
        // Simple feedforward processing (placeholder for actual Mamba recurrence)
        for batch_idx in 0..batch_size {
            let start_idx = batch_idx * self.config.input_dim;
            let input_slice = &inputs[start_idx..start_idx + self.config.input_dim];
            
            // Apply input-to-hidden transformation
            for i in 0..self.config.hidden_dim {
                let mut sum = 0.0;
                for j in 0..self.config.input_dim {
                    let weight_idx = self.config.input_dim * i + j;
                    sum += input_slice[j] * self.weights[weight_idx];
                }
                hidden[i] = (sum + 0.1).tanh(); // Simple activation
            }
        }
        
        hidden
    }
    
    /// Get model configuration
    pub fn config(&self) -> &MambaConfig {
        &self.config
    }
}

/// Factory for creating Mamba models
pub struct MambaFactory;

impl MambaFactory {
    /// Create a default Mamba model for FieldParticle processing
    pub fn create_default() -> MambaModel {
        let config = MambaConfig {
            input_dim: 8,
            hidden_dim: 128,
            latent_dim: 64,
            num_layers: 4,
            state_dim: 32,
        };
        
        MambaModel::new(config)
    }
    
    /// Create a lightweight Mamba model for edge devices
    pub fn create_lightweight() -> MambaModel {
        let config = MambaConfig {
            input_dim: 8,
            hidden_dim: 64,
            latent_dim: 32,
            num_layers: 2,
            state_dim: 16,
        };
        
        MambaModel::new(config)
    }
    
    /// Create a high-capacity Mamba model for server processing
    pub fn create_high_capacity() -> MambaModel {
        let config = MambaConfig {
            input_dim: 8,
            hidden_dim: 256,
            latent_dim: 128,
            num_layers: 8,
            state_dim: 64,
        };
        
        MambaModel::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml::field_particle::FieldParticle;
    
    #[test]
    fn test_mamba_model_creation() {
        let model = MambaFactory::create_default();
        assert_eq!(model.config().input_dim, 8);
        assert_eq!(model.config().hidden_dim, 128);
        assert_eq!(model.config().latent_dim, 64);
    }
    
    #[test]
    fn test_mamba_forward_pass() {
        let model = MambaFactory::create_default();
        
        // Create test particles
        let particles = vec![
            FieldParticle {
                position: [0.5, 0.3, 0.1],
                phase_i: 0.7,
                phase_q: 0.2,
                energy: 0.8,
                material_id: 0x0010, // Audio
                _padding: [0; 3],
            },
            FieldParticle {
                position: [0.2, 0.6, 0.4],
                phase_i: 0.3,
                phase_q: 0.9,
                energy: 0.6,
                material_id: 0x0100, // RF
                _padding: [0; 3],
            },
        ];
        
        let embeddings = model.forward(&particles);
        assert_eq!(embeddings.len(), 64);
        
        // Check that embeddings are in reasonable range
        for embedding in &embeddings {
            assert!(*embedding >= -1.0 && *embedding <= 1.0);
        }
    }
    
    #[test]
    fn test_empty_particles() {
        let model = MambaFactory::create_default();
        let embeddings = model.forward(&[]);
        assert_eq!(embeddings.len(), 64);
        assert!(embeddings.iter().all(|&x| x == 0.0));
    }
}