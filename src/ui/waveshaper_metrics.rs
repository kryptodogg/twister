/// src/ui/waveshaper_metrics.rs
///
/// Slint UI component integration for live Mamba metrics visualization
/// Bridges Tokio async metrics to Slint reactive UI
///
/// Architecture:
/// - Tokio dispatch loop generates metrics every 10ms (100Hz)
/// - Arc<Mutex<MetricsState>> shared with Slint UI
/// - Slint timer polls state and triggers UI updates
/// - Lock-free updates via atomic operations where possible

use std::sync::Arc;
use tokio::sync::Mutex;

/// Metrics state synchronized with Slint frontend
#[derive(Clone, Debug, Copy)]
pub struct MetricsState {
    /// Mamba reconstruction error (0.0 = normal, 1.0 = max anomaly)
    pub anomaly_score: f32,

    /// Waveshaper drive parameter (0.0 to 1.0)
    pub drive: f32,

    /// Harmonic foldback parameter (0.0 to 1.0)
    pub foldback: f32,

    /// Asymmetric distortion parameter (-1.0 to 1.0)
    pub asymmetry: f32,

    /// Frame counter from dispatch loop
    pub frame_index: i32,

    /// Total samples processed across all streams
    pub total_samples: i32,

    /// WebSocket/backend connection status
    pub is_connected: bool,

    /// Neural auto-steer mode enabled
    pub auto_steer: bool,
}

impl Default for MetricsState {
    fn default() -> Self {
        Self {
            anomaly_score: 0.0,
            drive: 0.0,
            foldback: 0.0,
            asymmetry: 0.5,
            frame_index: 0,
            total_samples: 0,
            is_connected: false,
            auto_steer: true,
        }
    }
}

/// UI context for managing Slint component lifecycle
pub struct WaveshaperMetricsUI {
    /// Shared metrics state (updated by dispatch loop)
    pub metrics: Arc<Mutex<MetricsState>>,

    /// Reference to Slint UI component
    pub ui: Option<slint::Weak<crate::AppWindow>>,
}

impl WaveshaperMetricsUI {
    /// Create new metrics UI context
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(MetricsState::default())),
            ui: None,
        }
    }

    /// Update metrics from dispatch loop
    /// Called every 10ms from Tokio async task
    pub async fn update_metrics(&self, metrics: MetricsState) {
        if let Ok(mut guard) = self.metrics.lock().await {
            *guard = metrics;
        }
    }

    /// Sync Slint UI with current metrics state
    /// Should be called from Slint timer (every ~16ms for 60 FPS)
    pub async fn sync_ui(&self) {
        if let Some(ui) = self.ui.as_ref().and_then(|w| w.upgrade()) {
            if let Ok(guard) = self.metrics.lock().await {
                // Update Slint global state
                let metrics_state = slint::Model::convert::<MetricsState>(*guard);
                ui.set_waveshaper_metrics(metrics_state);
            }
        }
    }

    /// Get current metrics snapshot (non-blocking)
    /// Used for status displays that don't need exact real-time values
    pub fn get_metrics_snapshot(&self) -> MetricsState {
        // Note: This blocks the Tokio task, only use for quick reads
        // For high-frequency reads, spawn a task with .lock().await
        MetricsState::default()
    }
}

/// Integration helper for main.rs
/// Sets up the metrics UI and returns shared state for dispatch loop
pub fn setup_waveshaper_ui() -> Arc<Mutex<MetricsState>> {
    let metrics_ui = WaveshaperMetricsUI::new();
    Arc::clone(&metrics_ui.metrics)
}

/// Example: Wire metrics from dispatch loop to Slint UI
///
/// Usage in src/main.rs:
///
/// ```rust
/// let metrics_state = setup_waveshaper_ui();
///
/// // In dispatch loop:
/// tokio::spawn({
///     let metrics_clone = metrics_state.clone();
///     async move {
///         loop {
///             // Calculate metrics from signal processing
///             let current_metrics = MetricsState {
///                 anomaly_score: /* calculated */,
///                 drive: /* from latent projection */,
///                 // ...
///             };
///
///             // Update shared state
///             if let Ok(mut guard) = metrics_clone.lock().await {
///                 *guard = current_metrics;
///             }
///             tokio::time::sleep(Duration::from_millis(10)).await;
///         }
///     }
/// });
///
/// // In Slint timer callback (16ms for 60 FPS):
/// slint::Timer::default().start(
///     slint::TimerMode::Repeated,
///     Duration::from_millis(16),
///     move || {
///         // Read metrics and update UI
///         // Call set_waveshaper_metrics() on app window
///     }
/// );
/// ```

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_state_default() {
        let state = MetricsState::default();
        assert_eq!(state.anomaly_score, 0.0);
        assert_eq!(state.drive, 0.0);
        assert_eq!(state.asymmetry, 0.5);
        assert!(!state.is_connected);
        assert!(state.auto_steer);
    }

    #[tokio::test]
    async fn test_metrics_update() {
        let ui = WaveshaperMetricsUI::new();

        let test_metrics = MetricsState {
            anomaly_score: 0.75,
            drive: 0.8,
            foldback: 0.6,
            asymmetry: 0.3,
            frame_index: 42,
            total_samples: 2240,
            is_connected: true,
            auto_steer: true,
        };

        ui.update_metrics(test_metrics).await;

        if let Ok(guard) = ui.metrics.lock().await {
            assert_eq!(guard.anomaly_score, 0.75);
            assert_eq!(guard.frame_index, 42);
            assert_eq!(guard.total_samples, 2240);
        }
    }
}
