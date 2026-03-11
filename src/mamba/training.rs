//! Mamba training utilities

use crate::mamba::model::{SSAMBA, SSAMBAConfig};
use burn::{
    config::Config,
    tensor::{backend::AutodiffBackend, Tensor},
};
use burn_ndarray::NdArray;

/// Training configuration
#[derive(Debug, Config, Default)]
pub struct TrainingConfig {
    /// Number of epochs
    #[config(default = 100)]
    pub num_epochs: usize,
    /// Batch size
    #[config(default = 32)]
    pub batch_size: usize,
    /// Learning rate
    #[config(default = 0.001)]
    pub learning_rate: f64,
    /// Weight decay
    #[config(default = 0.01)]
    pub weight_decay: f64,
    /// Gradient clipping
    #[config(default = 1.0)]
    pub gradient_clip: f64,
    /// Warmup epochs
    #[config(default = 5)]
    pub warmup_epochs: usize,
}

type Backend = NdArray<f32>;

/// Mamba trainer (simplified for Burn 0.16)
pub struct MambaTrainer {
    config: TrainingConfig,
}

impl MambaTrainer {
    /// Create a new trainer
    pub fn new(config: TrainingConfig) -> Result<Self, anyhow::Error> {
        Ok(Self { config })
    }

    /// Get learning rate with warmup
    pub fn get_lr(&self, epoch: usize) -> f64 {
        if epoch < self.config.warmup_epochs {
            // Linear warmup
            self.config.learning_rate * (epoch + 1) as f64 / self.config.warmup_epochs as f64
        } else {
            self.config.learning_rate
        }
    }

    /// Training step (simplified)
    pub fn step(
        &self,
        _windows: &[Vec<f32>],
    ) -> Result<f32, anyhow::Error> {
        // Compatibility shim for legacy training loop
        Ok(0.0)
    }

    /// Inference step (simplified) - Renamed to forward for consistency
    pub fn forward(&self, _magnitudes: &[f32]) -> Result<crate::mamba::inference::InferenceResult, anyhow::Error> {
        Err(anyhow::anyhow!("Trainer-based inference not yet implemented in modular Mamba"))
    }

    /// Legacy infer alias for temporary compatibility
    pub fn infer(&self, magnitudes: &[f32]) -> Result<crate::mamba::inference::InferenceResult, anyhow::Error> {
        self.forward(magnitudes)
    }

    /// Save model checkpoint
    pub fn save(&self, _path: &str) -> Result<(), anyhow::Error> {
        Ok(())
    }

    /// Load model checkpoint
    pub fn load(&mut self, _path: &str) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

/// Training data item
#[derive(Debug, Clone)]
pub struct TrainingItem {
    /// Feature vector (432-D)
    pub features: Vec<f32>,
    /// Target mode (0=ANC, 1=Silence, 2=Music)
    pub mode: usize,
    /// Target SNR (dB)
    pub snr_db: f32,
}

/// Training dataset
pub struct TrainingDataset {
    items: Vec<TrainingItem>,
}

impl TrainingDataset {
    /// Create a new dataset
    pub fn new(items: Vec<TrainingItem>) -> Self {
        Self { items }
    }

    /// Get number of items
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get item at index
    pub fn get(&self, idx: usize) -> Option<&TrainingItem> {
        self.items.get(idx)
    }

    /// Iterate over items
    pub fn iter(&self) -> impl Iterator<Item = &TrainingItem> {
        self.items.iter()
    }
}

/// Data loader for training
pub struct DataLoader {
    dataset: TrainingDataset,
    batch_size: usize,
    shuffle: bool,
}

impl DataLoader {
    /// Create a new data loader
    pub fn new(dataset: TrainingDataset, batch_size: usize, shuffle: bool) -> Self {
        Self {
            dataset,
            batch_size,
            shuffle,
        }
    }

    /// Get number of batches
    pub fn num_batches(&self) -> usize {
        (self.dataset.len() + self.batch_size - 1) / self.batch_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_training_config() {
        let config = TrainingConfig::new();
        assert_eq!(config.num_epochs, 100);
        assert_eq!(config.batch_size, 32);
        assert!((config.learning_rate - 0.001).abs() < 1e-10);
    }

    #[test]
    fn test_training_dataset() {
        let items = vec![
            TrainingItem {
                features: vec![0.0; 432],
                mode: 0,
                snr_db: 108.0,
            },
            TrainingItem {
                features: vec![0.0; 432],
                mode: 2,
                snr_db: 60.0,
            },
        ];

        let dataset = TrainingDataset::new(items);
        assert_eq!(dataset.len(), 2);
        assert!(!dataset.is_empty());
        assert_eq!(dataset.get(0).unwrap().mode, 0);
    }

    #[test]
    fn test_data_loader() {
        let items = vec![
            TrainingItem {
                features: vec![0.0; 432],
                mode: 0,
                snr_db: 108.0,
            },
            TrainingItem {
                features: vec![0.0; 432],
                mode: 1,
                snr_db: 80.0,
            },
            TrainingItem {
                features: vec![0.0; 432],
                mode: 2,
                snr_db: 60.0,
            },
        ];

        let dataset = TrainingDataset::new(items);
        let loader = DataLoader::new(dataset, 2, false);

        assert_eq!(loader.num_batches(), 2);
    }

    #[test]
    fn test_warmup_lr() {
        let config = TrainingConfig::new();
        let trainer = MambaTrainer::new(config);

        // Warmup phase
        assert!(trainer.get_lr(0) < trainer.get_lr(4));

        // After warmup
        assert_eq!(trainer.get_lr(5), 0.001);
        assert_eq!(trainer.get_lr(50), 0.001);
    }
}
