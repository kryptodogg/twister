//! Pipeline processing

use anyhow::Result;
use std::time::{Duration, Instant};

/// Pipeline configuration
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub target_latency_ms: f32,
    pub buffer_size: usize,
    pub enable_forensics: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            target_latency_ms: 35.0,
            buffer_size: 1024,
            enable_forensics: true,
        }
    }
}

/// Pipeline processor state
#[derive(Debug)]
pub struct PipelineState {
    pub mode: String,
    pub snr_db: f32,
    pub latency_ms: f32,
    pub events_processed: u64,
}

/// Main pipeline processor
pub struct PipelineProcessor {
    config: PipelineConfig,
    running: bool,
    events_processed: u64,
    start_time: Option<Instant>,
    last_latency_check: Option<Instant>,
    measured_latency_ms: f32,
}

impl PipelineProcessor {
    pub fn new(config: PipelineConfig) -> Self {
        Self {
            config,
            running: false,
            events_processed: 0,
            start_time: None,
            last_latency_check: None,
            measured_latency_ms: 0.0,
        }
    }

    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    pub fn target_latency(&self) -> Duration {
        Duration::from_millis((self.config.target_latency_ms) as u64)
    }

    pub fn buffer_size(&self) -> usize {
        self.config.buffer_size
    }

    pub fn forensics_enabled(&self) -> bool {
        self.config.enable_forensics
    }

    pub fn start(&mut self) -> Result<()> {
        log::info!("Starting pipeline with config: {:?}", self.config);
        self.running = true;
        self.start_time = Some(Instant::now());
        self.last_latency_check = Some(Instant::now());
        Ok(())
    }

    pub fn stop(&mut self) {
        log::info!("Stopping pipeline after {} events", self.events_processed);
        self.running = false;
        self.start_time = None;
    }

    /// Process a single event and track latency
    pub fn process_event(&mut self) {
        if !self.running {
            return;
        }

        self.events_processed += 1;

        // Measure latency periodically
        if let Some(last_check) = self.last_latency_check {
            if last_check.elapsed() >= Duration::from_millis(100) {
                self.measure_latency();
                self.last_latency_check = Some(Instant::now());
            }
        }
    }

    /// Measure current pipeline latency
    fn measure_latency(&mut self) {
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed().as_millis() as f32;
            self.measured_latency_ms = elapsed / self.events_processed.max(1) as f32;
        }
    }

    /// Check if latency is within target
    pub fn latency_ok(&self) -> bool {
        self.measured_latency_ms <= self.config.target_latency_ms
    }

    /// Get current latency in milliseconds
    pub fn current_latency_ms(&self) -> f32 {
        self.measured_latency_ms
    }

    pub fn state(&self) -> PipelineState {
        let mode = if self.latency_ok() { "ANC" } else { "BYPASS" };
        PipelineState {
            mode: mode.to_string(),
            snr_db: 108.0,
            latency_ms: self.measured_latency_ms,
            events_processed: self.events_processed,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn events_processed(&self) -> u64 {
        self.events_processed
    }
}
