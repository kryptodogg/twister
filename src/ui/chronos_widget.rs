//! Chronos Widget stub for Track A dispatch loop
//!
//! Minimal implementation for compilation. Full TimeGNN integration planned later.

use std::time::Instant;

/// Configuration for the Chronos widget
#[derive(Clone, Debug)]
pub struct ChronosWidgetConfig {
    pub update_interval_ms: u64,
}

impl Default for ChronosWidgetConfig {
    fn default() -> Self {
        Self { update_interval_ms: 16 }
    }
}

/// Chronos widget state
pub struct ChronosWidget {
    config: ChronosWidgetConfig,
    last_update: Instant,
    temperature: f32,
    motif_name: String,
    confidence: f32,
    next_event_eta: f32,
}

impl ChronosWidget {
    pub fn new() -> Self {
        Self::with_config(ChronosWidgetConfig::default())
    }

    pub fn with_config(config: ChronosWidgetConfig) -> Self {
        Self {
            config,
            last_update: Instant::now(),
            temperature: 0.0,
            motif_name: String::new(),
            confidence: 0.0,
            next_event_eta: 0.0,
        }
    }

    pub fn config(&self) -> &ChronosWidgetConfig {
        &self.config
    }

    /// Update widget values from TimeGNN inference
    pub fn update_values(
        &mut self,
        temperature: f32,
        motif_name: &str,
        confidence: f32,
        next_event_eta: f32,
    ) {
        self.last_update = Instant::now();
        self.temperature = temperature;
        self.motif_name.clear();
        self.motif_name.push_str(motif_name);
        self.confidence = confidence.clamp(0.0, 1.0);
        self.next_event_eta = next_event_eta.max(0.0);
    }

    pub fn get_parameters(&self) -> (f32, &str, f32, f32) {
        (
            self.temperature,
            &self.motif_name,
            self.confidence,
            self.next_event_eta,
        )
    }

    pub fn last_update(&self) -> Instant {
        self.last_update
    }
}
