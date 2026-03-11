// === PRE-FLIGHT ===
// Task:           Track E, Milestone E2 (Toto widget live on Windows 11 with real data)
// Files read:     ROADMAP.md, ui/AGENTS.md
// Files in scope: src/ui/toto_widget.rs
// Acceptance:     E2: Toto widget live on Windows 11 with real data
// Findings:       Dispatch loop expects a lightweight UI-side model with update_values().
// === END PRE-FLIGHT ===

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Configuration for the Toto widget model.
#[derive(Clone, Debug)]
pub struct TotoWidgetConfig {
    pub update_interval_ms: u64,
    pub anomaly_threshold: f32,
    pub neural_auto_steer_enabled: bool,
}

impl Default for TotoWidgetConfig {
    fn default() -> Self {
        Self {
            update_interval_ms: 16, // ~60 FPS
            anomaly_threshold: 0.8,
            neural_auto_steer_enabled: true,
        }
    }
}

/// Lightweight UI-side state model for Toto.
///
/// This is used by the dispatch loop to push real-time values into the UI layer.
pub struct TotoWidget {
    config: TotoWidgetConfig,

    // Shared anomaly score (stored as milli-units to allow atomic updates)
    anomaly_score_milli: Arc<AtomicU32>,

    neural_auto_steer: bool,
    drive: f32,
    fold: f32,
    asym: f32,
}

impl TotoWidget {
    pub fn new() -> Self {
        Self::with_config(TotoWidgetConfig::default())
    }

    pub fn with_config(config: TotoWidgetConfig) -> Self {
        Self {
            config,
            anomaly_score_milli: Arc::new(AtomicU32::new(0)),
            neural_auto_steer: false,
            drive: 0.0,
            fold: 0.0,
            asym: 0.0,
        }
    }

    pub fn config(&self) -> &TotoWidgetConfig {
        &self.config
    }

    /// Update values from Mamba inference.
    pub fn update_values(
        &mut self,
        anomaly_score: f32,
        neural_auto_steer: bool,
        drive: f32,
        fold: f32,
        asym: f32,
    ) {
        let anomaly = anomaly_score.clamp(0.0, 1.0);
        self.anomaly_score_milli
            .store((anomaly * 1000.0) as u32, Ordering::Relaxed);

        self.neural_auto_steer = neural_auto_steer;

        // Keep these bounded for UI presentation.
        self.drive = drive.clamp(0.0, 2.0);
        self.fold = fold.clamp(0.0, 1.0);
        self.asym = asym.clamp(0.0, 1.0);
    }

    pub fn get_anomaly_score(&self) -> f32 {
        self.anomaly_score_milli.load(Ordering::Relaxed) as f32 / 1000.0
    }

    pub fn is_neural_auto_steer_enabled(&self) -> bool {
        self.neural_auto_steer
    }

    pub fn get_parameters(&self) -> (f32, f32, f32) {
        (self.drive, self.fold, self.asym)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_with_defaults() {
        let w = TotoWidget::new();
        assert_eq!(w.config().update_interval_ms, 16);
        assert!((w.config().anomaly_threshold - 0.8).abs() < 1e-6);
    }

    #[test]
    fn clamps_values() {
        let mut w = TotoWidget::new();
        w.update_values(2.0, true, 3.0, 1.5, -0.5);

        assert!((w.get_anomaly_score() - 1.0).abs() < 1e-6);
        let (d, f, a) = w.get_parameters();
        assert_eq!(d, 2.0);
        assert_eq!(f, 1.0);
        assert_eq!(a, 0.0);
    }
}
