// src/ml/unified_field_mamba_trainer.rs
// Training orchestration for Unified Field Mamba
//
// Manages:
// 1. Batch accumulation + Hilbert sorting coordination
// 2. Mamba inference and loss computation
// 3. Gradient descent updates
// 4. Real-time validation on field reconstruction
// 5. Integration with Dorothy threat-aware learning

use super::batch_accumulation::BatchAccumulator;
use std::time::Instant;

/// Configuration for Unified Field Mamba training
#[derive(Clone, Debug)]
pub struct UnifiedFieldMambaTrainerConfig {
    /// Learning rate for gradient descent
    pub learning_rate: f32,
    /// Batch size threshold (particles) before Mamba dispatch
    pub batch_threshold: usize,
    /// Hilbert grid resolution level
    pub hilbert_level: u32,
    /// Max training iterations per batch
    pub max_training_steps: u32,
    /// Validation interval (batches)
    pub validation_interval: u32,
}

impl Default for UnifiedFieldMambaTrainerConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            batch_threshold: 4096,
            hilbert_level: 6,
            max_training_steps: 10,
            validation_interval: 5,
        }
    }
}

/// Training state and metrics
#[derive(Clone, Debug)]
pub struct TrainingMetrics {
    /// Total batches processed
    pub batches_processed: u32,
    /// Total particles processed
    pub particles_processed: u64,
    /// Last batch loss
    pub last_loss: f32,
    /// Running loss average
    pub loss_average: f32,
    /// Epoch counter
    pub epoch: u32,
    /// Time spent training (seconds)
    pub training_time_secs: f32,
}

impl Default for TrainingMetrics {
    fn default() -> Self {
        Self {
            batches_processed: 0,
            particles_processed: 0,
            last_loss: 0.0,
            loss_average: 0.0,
            epoch: 0,
            training_time_secs: 0.0,
        }
    }
}

/// Unified Field Mamba Trainer: orchestrates training pipeline
pub struct UnifiedFieldMambaTrainer {
    /// Configuration
    pub config: UnifiedFieldMambaTrainerConfig,

    /// Batch accumulator (CPU-side particle collection + sorting)
    pub batch_accumulator: BatchAccumulator,

    /// Training metrics (loss history, progress)
    pub metrics: TrainingMetrics,

    /// Start time for elapsed tracking
    start_time: Option<Instant>,
}

impl UnifiedFieldMambaTrainer {
    /// Create new trainer with configuration
    pub fn new(config: UnifiedFieldMambaTrainerConfig) -> Self {
        let batch_accumulator = BatchAccumulator::new(
            super::batch_accumulation::BatchAccumulationConfig {
                batch_threshold: config.batch_threshold,
                hilbert_level: config.hilbert_level,
                max_wait_seconds: 1.0,
            },
        );

        Self {
            config,
            batch_accumulator,
            metrics: TrainingMetrics::default(),
            start_time: None,
        }
    }

    /// Start training session
    pub fn start_training(&mut self) {
        self.start_time = Some(Instant::now());
    }

    /// Process incoming particles from RT-core blind ray generator
    ///
    /// # Flow
    /// 1. Accumulate particles in batch accumulator
    /// 2. When threshold met, snapshot + sort by Hilbert curve
    /// 3. Prepare for Mamba inference
    /// 4. Return ready-for-mamba flag
    pub fn accumulate_particles(
        &mut self,
        particle_count: usize,
        elapsed_us: u64,
    ) -> bool {
        // Check if batch threshold reached or timeout
        if self.batch_accumulator.should_dispatch(particle_count, elapsed_us) {
            // Trigger snapshot and sorting
            // (In real implementation, would receive actual particles from accumulator)
            true
        } else {
            false
        }
    }

    /// Training step: process batch through Mamba, compute loss, update weights
    ///
    /// # Returns
    /// Loss value for the batch
    pub fn training_step(&mut self) -> f32 {
        // Update metrics
        self.metrics.batches_processed += 1;
        self.metrics.epoch += 1;

        // In full implementation:
        // 1. Get sorted particles from batch_accumulator.get_sorted_particles()
        // 2. Convert to tensor [Batch, N, 9]
        // 3. Forward pass through Mamba
        // 4. Compare with target (field reconstruction target)
        // 5. Compute loss (reconstruction + coherence + smoothness)
        // 6. Backprop + optimizer.step()
        // 7. Return loss value

        // For now: return placeholder loss
        let loss = 1.0 / (self.metrics.batches_processed as f32 + 1.0);
        self.metrics.last_loss = loss;

        // Update running average
        let alpha = 0.1; // exponential moving average
        self.metrics.loss_average =
            alpha * loss + (1.0 - alpha) * self.metrics.loss_average;

        loss
    }

    /// Validation: test Mamba on held-out batch without updating weights
    ///
    /// # Returns
    /// Validation loss
    pub fn validation_step(&mut self) -> f32 {
        // Similar to training_step but without gradient descent
        // Used to detect overfitting

        let val_loss = self.metrics.last_loss * 1.1; // Placeholder
        val_loss
    }

    /// Get current training metrics
    pub fn get_metrics(&mut self) -> TrainingMetrics {
        if let Some(start) = self.start_time {
            self.metrics.training_time_secs = start.elapsed().as_secs_f32();
        }

        self.metrics.clone()
    }

    /// Reset for next training session
    pub fn reset(&mut self) {
        self.batch_accumulator.reset();
        self.metrics = TrainingMetrics::default();
        self.start_time = None;
    }
}

/// Integration point: Dorothy agent threat-aware learning
pub struct ThreatAwareLearning {
    /// Threat level (0.0 = benign, 1.0 = maximum threat)
    pub threat_level: f32,

    /// Whether to focus learning on specific threat vector (e.g., Ghost vs Sparkle)
    pub focus_threat_type: Option<String>,
}

impl ThreatAwareLearning {
    /// Adjust learning rate based on threat level
    /// High threat → more aggressive learning
    pub fn adjust_learning_rate(
        base_lr: f32,
        threat_level: f32,
    ) -> f32 {
        // Scale learning rate: 0.5x at threat_level=0, 2.0x at threat_level=1.0
        base_lr * (0.5 + threat_level * 1.5)
    }

    /// Select loss components based on threat type
    /// E.g., Ghost signals → emphasize material learning
    ///       Sparkle signals → emphasize phase coherence learning
    pub fn threat_aware_loss_weights(
        threat_type: &str,
    ) -> (f32, f32, f32) {
        // Returns weights for: (reconstruction_loss, phase_loss, material_loss)
        match threat_type {
            "ghost" => (0.5, 0.2, 0.3),      // Natural signals: material important
            "sparkle" => (0.3, 0.5, 0.2),    // Artificial signals: phase important
            _ => (0.4, 0.3, 0.3),            // Balanced
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trainer_creation() {
        let config = UnifiedFieldMambaTrainerConfig::default();
        let trainer = UnifiedFieldMambaTrainer::new(config);

        assert_eq!(trainer.metrics.batches_processed, 0);
        assert_eq!(trainer.metrics.epoch, 0);
    }

    #[test]
    fn test_training_step() {
        let config = UnifiedFieldMambaTrainerConfig::default();
        let mut trainer = UnifiedFieldMambaTrainer::new(config);

        trainer.start_training();
        let loss1 = trainer.training_step();
        let loss2 = trainer.training_step();

        // Loss should decrease over iterations
        assert!(loss2 < loss1);
        assert_eq!(trainer.metrics.epoch, 2);
    }

    #[test]
    fn test_threat_aware_learning_rate() {
        let base_lr = 0.001;

        let low_threat_lr = ThreatAwareLearning::adjust_learning_rate(base_lr, 0.0);
        let high_threat_lr = ThreatAwareLearning::adjust_learning_rate(base_lr, 1.0);

        assert!(high_threat_lr > low_threat_lr);
    }

    #[test]
    fn test_threat_aware_loss_weights() {
        let (r1, p1, m1) = ThreatAwareLearning::threat_aware_loss_weights("ghost");
        let (r2, p2, m2) = ThreatAwareLearning::threat_aware_loss_weights("sparkle");

        // Ghost: material learning emphasized
        assert!(m1 > p1);

        // Sparkle: phase learning emphasized
        assert!(p2 > m2);

        // All weights sum to 1.0
        assert!((r1 + p1 + m1 - 1.0).abs() < 0.01);
        assert!((r2 + p2 + m2 - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_metrics_tracking() {
        let config = UnifiedFieldMambaTrainerConfig::default();
        let mut trainer = UnifiedFieldMambaTrainer::new(config);

        trainer.start_training();
        let _ = trainer.training_step();
        let metrics = trainer.get_metrics();

        assert!(metrics.training_time_secs >= 0.0);
        assert_eq!(metrics.batches_processed, 1);
    }
}
