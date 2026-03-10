//! Latency monitoring and profiling

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Latency statistics
#[derive(Debug, Clone, Default)]
pub struct LatencyStats {
    pub count: usize,
    pub min_ms: f64,
    pub max_ms: f64,
    pub avg_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

/// Simple latency tracker
pub struct LatencyTracker {
    samples: Mutex<Vec<u64>>,
    max_samples: usize,
}

impl LatencyTracker {
    fn new(max_samples: usize) -> Self {
        Self {
            samples: Mutex::new(Vec::with_capacity(max_samples)),
            max_samples,
        }
    }
    
    fn record(&self, ns: u64) {
        if let Ok(mut samples) = self.samples.lock() {
            samples.push(ns);
            if samples.len() > self.max_samples {
                samples.remove(0);
            }
        }
    }
    
    fn stats(&self) -> LatencyStats {
        let samples = self.samples.lock().unwrap_or_else(|e| e.into_inner());
        let count = samples.len();
        
        if count == 0 {
            return LatencyStats::default();
        }
        
        let mut sorted = samples.clone();
        sorted.sort();
        
        let min_ns = *sorted.first().unwrap_or(&0);
        let max_ns = *sorted.last().unwrap_or(&0);
        let total_ns: u64 = sorted.iter().sum();
        let avg_ns = total_ns / count as u64;
        
        let p50_idx = count * 50 / 100;
        let p95_idx = count * 95 / 100;
        let p99_idx = count * 99 / 100;
        
        LatencyStats {
            count,
            min_ms: min_ns as f64 / 1_000_000.0,
            max_ms: max_ns as f64 / 1_000_000.0,
            avg_ms: avg_ns as f64 / 1_000_000.0,
            p50_ms: sorted.get(p50_idx).copied().unwrap_or(0) as f64 / 1_000_000.0,
            p95_ms: sorted.get(p95_idx).copied().unwrap_or(0) as f64 / 1_000_000.0,
            p99_ms: sorted.get(p99_idx).copied().unwrap_or(0) as f64 / 1_000_000.0,
        }
    }
}

/// Latency monitor for pipeline profiling
pub struct LatencyMonitor {
    tracker: LatencyTracker,
    sample_count: AtomicUsize,
    total_ns: AtomicU64,
    min_ns: AtomicU64,
    max_ns: AtomicU64,
}

impl LatencyMonitor {
    /// Create new latency monitor
    pub fn new() -> Self {
        Self {
            tracker: LatencyTracker::new(10000),
            sample_count: AtomicUsize::new(0),
            total_ns: AtomicU64::new(0),
            min_ns: AtomicU64::new(u64::MAX),
            max_ns: AtomicU64::new(0),
        }
    }
    
    /// Record a latency measurement
    pub fn record(&self, duration: Duration) {
        let ns = duration.as_nanos() as u64;
        
        self.tracker.record(ns);
        self.sample_count.fetch_add(1, Ordering::Relaxed);
        self.total_ns.fetch_add(ns, Ordering::Relaxed);
        
        // Update min/max
        let mut current_min = self.min_ns.load(Ordering::Relaxed);
        while ns < current_min {
            match self.min_ns.compare_exchange_weak(current_min, ns, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(x) => current_min = x,
            }
        }
        
        let mut current_max = self.max_ns.load(Ordering::Relaxed);
        while ns > current_max {
            match self.max_ns.compare_exchange_weak(current_max, ns, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }
    }
    
    /// Get current statistics
    pub fn stats(&self) -> LatencyStats {
        let mut stats = self.tracker.stats();
        let count = self.sample_count.load(Ordering::Relaxed);
        if count > 0 {
            let total_ns = self.total_ns.load(Ordering::Relaxed);
            stats.count = count;
            stats.avg_ms = (total_ns as f64 / count as f64) / 1_000_000.0;
        }
        stats
    }
}

impl Default for LatencyMonitor {
    fn default() -> Self {
        Self::new()
    }
}
