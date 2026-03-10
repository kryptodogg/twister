/// Chronos Backend Configuration Bridge
///
/// Thread-safe configuration management for TimeGNN training with live parameter injection
/// from Slint UI. Uses Arc<RwLock<>> for async-safe read/write operations.
///
/// Architecture:
/// ```
/// Slint UI (main thread)
///     ↓ [user moves slider]
///     ↓
/// update_config(new_config) [async write]
///     ↓
/// Arc<RwLock<ChronosConfig>>
///     ↑
/// train_timegnn loop [async read at each epoch]
///     ↓
/// Uses latest config for loss computation
/// ```

use std::sync::Arc;
use tokio::sync::RwLock;

/// Critical configuration for TimeGNN contrastive training
/// Swappable at runtime via Slint UI
#[derive(Debug, Clone, Copy)]
pub struct ChronosConfig {
    /// Temperature parameter (τ) for NT-Xent loss
    /// Range: 0.01 (sharp, numerical stability risk) to 1.0 (soft, loses discrimination)
    /// Default: 0.07 (generation-critical for harassment pattern discovery)
    pub temperature: f32,

    /// Adam optimizer learning rate
    /// Range: 1e-6 (very slow) to 1e-1 (unstable)
    /// Default: 1e-3 (stable convergence)
    pub learning_rate: f32,

    /// Prediction horizon in seconds
    /// Controls temporal window for attention mechanism
    /// Range: 1 second to 3600 seconds (1 hour)
    /// Default: 60 seconds
    pub prediction_horizon_secs: u32,

    /// Weight decay for L2 regularization
    /// Default: 1e-5
    pub weight_decay: f32,

    /// Batch size for gradient descent
    /// Default: 32 samples
    pub batch_size: usize,
}

impl Default for ChronosConfig {
    fn default() -> Self {
        Self {
            temperature: 0.07,  // Generation-critical
            learning_rate: 1e-3,
            prediction_horizon_secs: 60,
            weight_decay: 1e-5,
            batch_size: 32,
        }
    }
}

impl ChronosConfig {
    /// Validate configuration values
    pub fn validate(&self) -> Result<(), String> {
        // Temperature validation (0.01 to 1.0)
        if self.temperature < 0.01 {
            return Err(format!(
                "Temperature {:.4} below minimum 0.01",
                self.temperature
            ));
        }
        if self.temperature > 1.0 {
            return Err(format!(
                "Temperature {:.4} above maximum 1.0",
                self.temperature
            ));
        }

        // Learning rate validation (1e-6 to 1e-1)
        if self.learning_rate < 1e-6 {
            return Err(format!("Learning rate {:.6} below minimum 1e-6", self.learning_rate));
        }
        if self.learning_rate > 1e-1 {
            return Err(format!("Learning rate {:.6} above maximum 1e-1", self.learning_rate));
        }

        // Prediction horizon validation (1 to 3600 seconds)
        if self.prediction_horizon_secs < 1 {
            return Err(format!(
                "Prediction horizon {} seconds below minimum 1",
                self.prediction_horizon_secs
            ));
        }
        if self.prediction_horizon_secs > 3600 {
            return Err(format!(
                "Prediction horizon {} seconds above maximum 3600",
                self.prediction_horizon_secs
            ));
        }

        // Batch size must be positive
        if self.batch_size == 0 {
            return Err("Batch size cannot be zero".to_string());
        }

        Ok(())
    }

    /// Clamp configuration to safe ranges
    pub fn clamp(&mut self) {
        self.temperature = self.temperature.clamp(0.01, 1.0);
        self.learning_rate = self.learning_rate.clamp(1e-6, 1e-1);
        self.prediction_horizon_secs = self.prediction_horizon_secs.clamp(1, 3600);
    }
}

/// Thread-safe bridge for runtime configuration updates
/// Allows Slint UI and training loop to synchronize on configuration state
pub struct ChronosBridge {
    /// Shared mutable configuration behind RwLock
    /// Writers: Slint UI (update_config)
    /// Readers: Training loop (get_config, read_config_async)
    config: Arc<RwLock<ChronosConfig>>,
}

impl ChronosBridge {
    /// Create new Chronos bridge with default configuration
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(ChronosConfig::default())),
        }
    }

    /// Create new Chronos bridge with custom initial configuration
    pub fn with_config(initial: ChronosConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(initial)),
        }
    }

    /// Get current configuration (blocking)
    /// Use in synchronous contexts (UI callbacks, tests)
    pub fn get_config(&self) -> ChronosConfig {
        // This blocks if a writer has the lock
        // For Slint callbacks (main thread), this is acceptable
        match self.config.try_read() {
            Ok(guard) => *guard,
            Err(_) => {
                eprintln!("[Chronos] Warning: Could not acquire read lock, using default config");
                ChronosConfig::default()
            }
        }
    }

    /// Get current configuration (async)
    /// Use in training loop (async context)
    pub async fn read_config_async(&self) -> ChronosConfig {
        *self.config.read().await
    }

    /// Update configuration from Slint UI
    /// This is the critical method that Slint will call when a slider moves
    pub async fn update_config(&self, mut new_config: ChronosConfig) -> Result<(), String> {
        // Validate new configuration
        new_config.validate()?;

        // Acquire write lock and update
        let mut lock = self.config.write().await;
        let old_config = *lock;

        *lock = new_config;

        // Log the change
        if (new_config.temperature - old_config.temperature).abs() > 1e-6 {
            eprintln!(
                "[Chronos] Temperature update: {:.4} → {:.4}",
                old_config.temperature, new_config.temperature
            );
        }
        if (new_config.learning_rate - old_config.learning_rate).abs() > 1e-6 {
            eprintln!(
                "[Chronos] Learning rate update: {:.6} → {:.6}",
                old_config.learning_rate, new_config.learning_rate
            );
        }
        if new_config.prediction_horizon_secs != old_config.prediction_horizon_secs {
            eprintln!(
                "[Chronos] Prediction horizon update: {} → {} seconds",
                old_config.prediction_horizon_secs, new_config.prediction_horizon_secs
            );
        }

        Ok(())
    }

    /// Update single temperature parameter
    /// Convenience method for Slint slider
    pub async fn update_temperature(&self, temperature: f32) -> Result<(), String> {
        let mut config = self.read_config_async().await;
        config.temperature = temperature;
        self.update_config(config).await
    }

    /// Update single learning rate parameter
    pub async fn update_learning_rate(&self, learning_rate: f32) -> Result<(), String> {
        let mut config = self.read_config_async().await;
        config.learning_rate = learning_rate;
        self.update_config(config).await
    }

    /// Update single prediction horizon parameter
    pub async fn update_prediction_horizon(&self, secs: u32) -> Result<(), String> {
        let mut config = self.read_config_async().await;
        config.prediction_horizon_secs = secs;
        self.update_config(config).await
    }

    /// Clone the underlying Arc for sharing with training tasks
    pub fn clone_ref(&self) -> Arc<RwLock<ChronosConfig>> {
        Arc::clone(&self.config)
    }
}

impl Default for ChronosBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chronos_config_default() {
        let config = ChronosConfig::default();
        assert_eq!(config.temperature, 0.07);
        assert_eq!(config.learning_rate, 1e-3);
        assert_eq!(config.prediction_horizon_secs, 60);
        assert_eq!(config.batch_size, 32);
    }

    #[test]
    fn test_chronos_config_validation() {
        let mut config = ChronosConfig::default();

        // Valid configuration
        assert!(config.validate().is_ok());

        // Invalid temperature (too low)
        config.temperature = 0.001;
        assert!(config.validate().is_err());

        // Invalid temperature (too high)
        config.temperature = 1.5;
        assert!(config.validate().is_err());

        // Valid after clamping
        config.clamp();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_chronos_config_clamp() {
        let mut config = ChronosConfig {
            temperature: 0.001,  // Too low
            learning_rate: 1e-7, // Too low
            prediction_horizon_secs: 0,  // Too low
            weight_decay: 1e-5,
            batch_size: 32,
        };

        config.clamp();

        assert_eq!(config.temperature, 0.01);
        assert_eq!(config.learning_rate, 1e-6);
        assert_eq!(config.prediction_horizon_secs, 1);
    }

    #[tokio::test]
    async fn test_chronos_bridge_creation() {
        let bridge = ChronosBridge::new();
        let config = bridge.read_config_async().await;

        assert_eq!(config.temperature, 0.07);
    }

    #[tokio::test]
    async fn test_chronos_bridge_temperature_update() {
        let bridge = ChronosBridge::new();

        // Update temperature
        let result = bridge.update_temperature(0.15).await;
        assert!(result.is_ok());

        // Verify update
        let config = bridge.read_config_async().await;
        assert_eq!(config.temperature, 0.15);
    }

    #[tokio::test]
    async fn test_chronos_bridge_invalid_update() {
        let bridge = ChronosBridge::new();

        // Try to update with invalid temperature
        let result = bridge.update_temperature(0.001).await;
        assert!(result.is_err());

        // Original value should remain unchanged
        let config = bridge.read_config_async().await;
        assert_eq!(config.temperature, 0.07);
    }

    #[tokio::test]
    async fn test_chronos_bridge_multi_update() {
        let bridge = ChronosBridge::new();

        let mut config = bridge.read_config_async().await;
        config.temperature = 0.20;
        config.learning_rate = 5e-4;
        config.prediction_horizon_secs = 120;

        let result = bridge.update_config(config).await;
        assert!(result.is_ok());

        let updated = bridge.read_config_async().await;
        assert_eq!(updated.temperature, 0.20);
        assert_eq!(updated.learning_rate, 5e-4);
        assert_eq!(updated.prediction_horizon_secs, 120);
    }

    #[test]
    fn test_chronos_bridge_sync_read() {
        let bridge = ChronosBridge::new();
        let config = bridge.get_config();

        assert_eq!(config.temperature, 0.07);
    }
}
