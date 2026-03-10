//! Performance metrics and latency monitoring

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::Mutex;

/// Latency statistics
#[derive(Debug, Clone, Default)]
pub struct LatencyStats {
    pub count: usize,
    pub min_ns: u64,
    pub max_ns: u64,
    pub sum_ns: u64,
    pub p50_ns: u64,
    pub p95_ns: u64,
    pub p99_ns: u64,
}

impl LatencyStats {
    pub fn min_ms(&self) -> f64 {
        self.min_ns as f64 / 1_000_000.0
    }

    pub fn max_ms(&self) -> f64 {
        self.max_ns as f64 / 1_000_000.0
    }

    pub fn avg_ms(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            (self.sum_ns / self.count as u64) as f64 / 1_000_000.0
        }
    }

    pub fn p50_ms(&self) -> f64 {
        self.p50_ns as f64 / 1_000_000.0
    }

    pub fn p95_ms(&self) -> f64 {
        self.p95_ns as f64 / 1_000_000.0
    }

    pub fn p99_ms(&self) -> f64 {
        self.p99_ns as f64 / 1_000_000.0
    }
}

/// Latency monitor for tracking pipeline performance
#[derive(Debug)]
pub struct LatencyMonitor {
    samples: Arc<Mutex<Vec<u64>>>,
    max_samples: usize,
    count: AtomicUsize,
    sum_ns: AtomicU64,
    min_ns: AtomicU64,
    max_ns: AtomicU64,
}

impl LatencyMonitor {
    pub fn new() -> Self {
        Self::with_capacity(10000)
    }

    pub fn with_capacity(max_samples: usize) -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::with_capacity(max_samples))),
            max_samples,
            count: AtomicUsize::new(0),
            sum_ns: AtomicU64::new(0),
            min_ns: AtomicU64::new(u64::MAX),
            max_ns: AtomicU64::new(0),
        }
    }

    /// Record a latency sample
    pub fn record(&self, duration: Duration) {
        let ns = duration.as_nanos() as u64;
        
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum_ns.fetch_add(ns, Ordering::Relaxed);
        
        // Update min
        let mut current_min = self.min_ns.load(Ordering::Relaxed);
        while ns < current_min {
            match self.min_ns.compare_exchange_weak(
                current_min,
                ns,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_min = x,
            }
        }

        // Update max
        let mut current_max = self.max_ns.load(Ordering::Relaxed);
        while ns > current_max {
            match self.max_ns.compare_exchange_weak(
                current_max,
                ns,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }

        // Store sample for percentile calculation
        let mut samples = self.samples.lock();
        if samples.len() >= self.max_samples {
            samples.remove(0);
        }
        samples.push(ns);
    }

    /// Record latency from an Instant (start time)
    pub fn record_since(&self, start: Instant) {
        self.record(start.elapsed());
    }

    /// Get current statistics
    pub fn stats(&self) -> LatencyStats {
        let samples = self.samples.lock();
        let count = self.count.load(Ordering::Relaxed);
        
        if count == 0 || samples.is_empty() {
            return LatencyStats::default();
        }

        let mut sorted: Vec<u64> = samples.clone();
        sorted.sort_unstable();

        LatencyStats {
            count,
            min_ns: self.min_ns.load(Ordering::Relaxed),
            max_ns: self.max_ns.load(Ordering::Relaxed),
            sum_ns: self.sum_ns.load(Ordering::Relaxed),
            p50_ns: sorted[sorted.len() * 50 / 100],
            p95_ns: sorted[sorted.len() * 95 / 100],
            p99_ns: sorted[sorted.len() * 99 / 100],
        }
    }

    /// Reset all statistics
    pub fn reset(&self) {
        self.count.store(0, Ordering::Relaxed);
        self.sum_ns.store(0, Ordering::Relaxed);
        self.min_ns.store(u64::MAX, Ordering::Relaxed);
        self.max_ns.store(0, Ordering::Relaxed);
        self.samples.lock().clear();
    }

    /// Check if latency exceeds target
    pub fn exceeds_target(&self, target: Duration) -> bool {
        let stats = self.stats();
        stats.p95_ns > target.as_nanos() as u64
    }
}

impl Default for LatencyMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance metrics collector
#[derive(Debug, Default)]
pub struct MetricsCollector {
    pub latency: LatencyMonitor,
    pub rf_capture_count: AtomicUsize,
    pub audio_capture_count: AtomicUsize,
    pub mamba_inference_count: AtomicUsize,
    pub control_update_count: AtomicUsize,
    pub forensics_logged_count: AtomicUsize,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a summary of all metrics
    pub fn summary(&self) -> MetricsSummary {
        MetricsSummary {
            latency: self.latency.stats(),
            rf_captures: self.rf_capture_count.load(Ordering::Relaxed),
            audio_captures: self.audio_capture_count.load(Ordering::Relaxed),
            mamba_inferences: self.mamba_inference_count.load(Ordering::Relaxed),
            control_updates: self.control_update_count.load(Ordering::Relaxed),
            forensics_logged: self.forensics_logged_count.load(Ordering::Relaxed),
        }
    }

    /// Log metrics summary
    pub fn log_summary(&self) {
        let summary = self.summary();
        tracing::info!("=== Metrics Summary ===");
        tracing::info!("  RF Captures: {}", summary.rf_captures);
        tracing::info!("  Audio Captures: {}", summary.audio_captures);
        tracing::info!("  Mamba Inferences: {}", summary.mamba_inferences);
        tracing::info!("  Control Updates: {}", summary.control_updates);
        tracing::info!("  Forensics Logged: {}", summary.forensics_logged);
        tracing::info!("  Latency P50: {:.2}ms", summary.latency.p50_ms());
        tracing::info!("  Latency P95: {:.2}ms", summary.latency.p95_ms());
        tracing::info!("  Latency P99: {:.2}ms", summary.latency.p99_ms());
    }
}

/// Summary of all metrics
#[derive(Debug, Clone, Default)]
pub struct MetricsSummary {
    pub latency: LatencyStats,
    pub rf_captures: usize,
    pub audio_captures: usize,
    pub mamba_inferences: usize,
    pub control_updates: usize,
    pub forensics_logged: usize,
}

/// SNR metrics tracker
#[derive(Debug)]
pub struct SNRMetrics {
    current_snr_db: AtomicU64, // Stored as fixed-point (×1000)
    min_snr_db: AtomicU64,
    max_snr_db: AtomicU64,
    sum_snr_db: AtomicU64,
    count: AtomicUsize,
}

impl SNRMetrics {
    pub fn new() -> Self {
        Self {
            current_snr_db: AtomicU64::new(0),
            min_snr_db: AtomicU64::new(u64::MAX),
            max_snr_db: AtomicU64::new(0),
            sum_snr_db: AtomicU64::new(0),
            count: AtomicUsize::new(0),
        }
    }

    pub fn update(&self, snr_db: f32) {
        let fixed = (snr_db * 1000.0) as u64;
        self.current_snr_db.store(fixed, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum_snr_db.fetch_add(fixed, Ordering::Relaxed);

        // Update min
        let mut current_min = self.min_snr_db.load(Ordering::Relaxed);
        while fixed < current_min {
            match self.min_snr_db.compare_exchange_weak(
                current_min,
                fixed,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_min = x,
            }
        }

        // Update max
        let mut current_max = self.max_snr_db.load(Ordering::Relaxed);
        while fixed > current_max {
            match self.max_snr_db.compare_exchange_weak(
                current_max,
                fixed,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }
    }

    pub fn current_db(&self) -> f32 {
        self.current_snr_db.load(Ordering::Relaxed) as f32 / 1000.0
    }

    pub fn average_db(&self) -> f32 {
        let count = self.count.load(Ordering::Relaxed);
        if count == 0 {
            0.0
        } else {
            (self.sum_snr_db.load(Ordering::Relaxed) / count as u64) as f32 / 1000.0
        }
    }

    pub fn min_db(&self) -> f32 {
        let min = self.min_snr_db.load(Ordering::Relaxed);
        if min == u64::MAX { 0.0 } else { min as f32 / 1000.0 }
    }

    pub fn max_db(&self) -> f32 {
        self.max_snr_db.load(Ordering::Relaxed) as f32 / 1000.0
    }
}

impl Default for SNRMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_monitor() {
        let monitor = LatencyMonitor::new();
        
        monitor.record(Duration::from_millis(10));
        monitor.record(Duration::from_millis(20));
        monitor.record(Duration::from_millis(30));
        
        let stats = monitor.stats();
        assert_eq!(stats.count, 3);
        assert!((stats.min_ms() - 10.0).abs() < 0.01);
        assert!((stats.max_ms() - 30.0).abs() < 0.01);
        assert!((stats.avg_ms() - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_snr_metrics() {
        let metrics = SNRMetrics::new();
        
        metrics.update(50.0);
        metrics.update(60.0);
        metrics.update(55.0);
        
        assert!((metrics.current_db() - 55.0).abs() < 0.01);
        assert!((metrics.average_db() - 55.0).abs() < 0.01);
        assert!((metrics.min_db() - 50.0).abs() < 0.01);
        assert!((metrics.max_db() - 60.0).abs() < 0.01);
    }
}
