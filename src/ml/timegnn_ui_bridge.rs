/// UI Bridge for TimeGNN Parameter Hot-Swap
///
/// This module provides utilities for connecting Slint UI sliders to the TimeGNN training
/// loop without interrupting gradient descent or losing model weights.
///
/// Architecture:
/// ```
/// Slint UI (temperature slider)
///     ↓ [user moves slider]
/// UI callback sends ParameterUpdate
///     ↓ [tokio::sync::mpsc channel]
/// train_timegnn() receives at batch boundary
///     ↓ [non-blocking try_recv()]
/// config.loss_config.temperature updated
///     ↓ [no weight loss, no training pause]
/// Next batch uses new τ value
/// ```

use crate::ml::timegnn_trainer::{ParameterUpdate, TimeGnnTrainingConfig};
use std::sync::{Arc, Mutex};

/// UI parameter state for real-time display and control
/// Used to synchronize UI sliders with training loop
pub struct UIParameterState {
    /// Current temperature (τ) value in UI
    pub temperature: Arc<Mutex<f32>>,
    /// Current learning rate in UI
    pub learning_rate: Arc<Mutex<f32>>,
    /// Current attention window in milliseconds
    pub attention_window_ms: Arc<Mutex<f32>>,
    /// Channel sender for parameter updates to training loop
    pub update_tx: tokio::sync::mpsc::UnboundedSender<ParameterUpdate>,
}

impl UIParameterState {
    /// Create new UI parameter state with channel
    pub fn new() -> (
        Self,
        tokio::sync::mpsc::UnboundedReceiver<ParameterUpdate>,
    ) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        let state = Self {
            temperature: Arc::new(Mutex::new(0.07)),
            learning_rate: Arc::new(Mutex::new(1e-3)),
            attention_window_ms: Arc::new(Mutex::new(1000.0)),
            update_tx: tx,
        };

        (state, rx)
    }

    /// Called when temperature slider moves in Slint UI
    /// Updates UI state and sends parameter update to training loop
    pub fn on_temperature_changed(&self, new_tau: f32) {
        if let Ok(mut tau) = self.temperature.lock() {
            *tau = new_tau;
        }

        // Send update to training loop
        let update = ParameterUpdate::new(
            Some(new_tau),
            None,
            None,
            chrono::Local::now().timestamp_micros(),
        );

        let _ = self.update_tx.send(update);
    }

    /// Called when learning rate slider moves in Slint UI
    pub fn on_learning_rate_changed(&self, new_lr: f32) {
        if let Ok(mut lr) = self.learning_rate.lock() {
            *lr = new_lr;
        }

        let update = ParameterUpdate::new(
            None,
            Some(new_lr),
            None,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_micros() as i64)
                .unwrap_or(0),
        );

        let _ = self.update_tx.send(update);
    }

    /// Called when attention window slider moves
    pub fn on_attention_window_changed(&self, new_window_ms: u64) {
        if let Ok(mut aw) = self.attention_window_ms.lock() {
            *aw = new_window_ms as f32;
        }

        let update = ParameterUpdate::new(
            None,
            None,
            Some(new_window_ms),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_micros() as i64)
                .unwrap_or(0),
        );

        let _ = self.update_tx.send(update);
    }

    /// Called when multiple parameters change simultaneously
    /// Sends a single update with all changes
    pub fn on_parameters_changed(
        &self,
        tau: Option<f32>,
        learning_rate: Option<f32>,
        attention_window_ms: Option<u64>,
    ) {
        if let Some(t) = tau {
            if let Ok(mut temp) = self.temperature.lock() {
                *temp = t;
            }
        }
        if let Some(lr) = learning_rate {
            if let Ok(mut lr_val) = self.learning_rate.lock() {
                *lr_val = lr;
            }
        }
        if let Some(aw) = attention_window_ms {
            if let Ok(mut aw_val) = self.attention_window_ms.lock() {
                *aw_val = aw as f32;
            }
        }

        let update = ParameterUpdate::new(
            tau,
            learning_rate,
            attention_window_ms,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_micros() as i64)
                .unwrap_or(0),
        );

        let _ = self.update_tx.send(update);
    }

    /// Get current UI parameter values (for display in Slint)
    pub fn get_current_config(&self) -> (f32, f32, u64) {
        let tau = self.temperature.lock().map(|t| *t).unwrap_or(0.07);
        let lr = self.learning_rate.lock().map(|l| *l).unwrap_or(1e-3);
        let aw = self
            .attention_window_ms
            .lock()
            .map(|a| *a as u64)
            .unwrap_or(1000);

        (tau, lr, aw)
    }
}

impl Default for UIParameterState {
    fn default() -> Self {
        Self::new().0
    }
}

/// Helper: Create a Slint callback wrapper for temperature slider
/// Usage in Slint UI:
/// ```slint
/// slider_temperature := Slider {
///     min: 0.01;
///     max: 1.0;
///     value: 0.07;
///     changed(value) => {
///         ui_params.on_temperature_changed(value);
///     }
/// }
/// ```
pub fn temperature_slider_callback(
    ui_params: &UIParameterState,
) -> impl Fn(f32) + '_ {
    move |tau: f32| {
        ui_params.on_temperature_changed(tau);
    }
}

/// Helper: Create a Slint callback wrapper for learning rate slider
pub fn learning_rate_slider_callback(
    ui_params: &UIParameterState,
) -> impl Fn(f32) + '_ {
    move |lr: f32| {
        ui_params.on_learning_rate_changed(lr);
    }
}

/// Helper: Create a Slint callback wrapper for attention window slider
pub fn attention_window_slider_callback(
    ui_params: &UIParameterState,
) -> impl Fn(f32) + '_ {
    move |window_ms: f32| {
        ui_params.on_attention_window_changed(window_ms as u64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_state_creation() {
        let (state, _rx) = UIParameterState::new();
        let (tau, lr, window) = state.get_current_config();

        assert!((tau - 0.07).abs() < 1e-6);
        assert!((lr - 1e-3).abs() < 1e-6);
        assert_eq!(window, 1000);
    }

    #[test]
    fn test_temperature_update() {
        let (state, mut rx) = UIParameterState::new();

        state.on_temperature_changed(0.15);

        let update = rx.try_recv().expect("Should have update");
        assert_eq!(update.temperature, Some(0.15));
    }

    #[test]
    fn test_learning_rate_update() {
        let (state, mut rx) = UIParameterState::new();

        state.on_learning_rate_changed(5e-4);

        let update = rx.try_recv().expect("Should have update");
        assert_eq!(update.learning_rate, Some(5e-4));
    }

    #[test]
    fn test_multi_parameter_update() {
        let (state, mut rx) = UIParameterState::new();

        state.on_parameters_changed(Some(0.12), Some(2e-3), Some(2000));

        let update = rx.try_recv().expect("Should have update");
        assert_eq!(update.temperature, Some(0.12));
        assert_eq!(update.learning_rate, Some(2e-3));
        assert_eq!(update.attention_window_ms, Some(2000));
    }
}
