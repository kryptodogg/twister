//! Pipeline latency tracking

use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use crate::utils::metrics::{LatencyMonitor, LatencyStats};

/// Pipeline latency tracker
pub struct PipelineLatency {
    /// Overall latency monitor
    monitor: LatencyMonitor,
    /// Per-stage latencies
    stages: Arc<Mutex<Vec<StageLatency>>>,
    /// Target latency
    target_ms: f32,
    /// Start time
    start_time: Instant,
}

/// Stage latency record
#[derive(Debug, Clone)]
pub struct StageLatency {
    /// Stage name
    pub name: String,
    /// Latency in ms
    pub latency_ms: f32,
    /// Timestamp
    pub timestamp: Instant,
}

/// Latency budget allocation
#[derive(Debug, Clone)]
pub struct LatencyBudget {
    /// RF capture budget (ms)
    pub rf_capture_ms: f32,
    /// Audio capture budget (ms)
    pub audio_capture_ms: f32,
    /// DSP processing budget (ms)
    pub dsp_processing_ms: f32,
    /// ML inference budget (ms)
    pub ml_inference_ms: f32,
    /// Control policy budget (ms)
    pub control_ms: f32,
    /// Output budget (ms)
    pub output_ms: f32,
    /// Total budget (ms)
    pub total_ms: f32,
}

impl Default for LatencyBudget {
    fn default() -> Self {
        Self {
            rf_capture_ms: 5.0,
            audio_capture_ms: 5.0,
            dsp_processing_ms: 10.0,
            ml_inference_ms: 8.0,
            control_ms: 2.0,
            output_ms: 5.0,
            total_ms: 35.0,
        }
    }
}

impl PipelineLatency {
    /// Create a new latency tracker
    pub fn new(target_ms: f32) -> Self {
        Self {
            monitor: LatencyMonitor::new(),
            stages: Arc::new(Mutex::new(Vec::new())),
            target_ms,
            start_time: Instant::now(),
        }
    }

    /// Record a frame latency
    pub fn record_frame(&self, duration: Duration) {
        self.monitor.record(duration);
    }

    /// Record stage latency
    pub fn record_stage(&self, name: &str, duration: Duration) {
        let mut stages = self.stages.lock();
        stages.push(StageLatency {
            name: name.to_string(),
            latency_ms: duration.as_secs_f32() * 1000.0,
            timestamp: Instant::now(),
        });

        // Keep only last 1000 records
        if stages.len() > 1000 {
            stages.remove(0);
        }
    }

    /// Get current latency stats
    pub fn stats(&self) -> LatencyStats {
        self.monitor.stats()
    }

    /// Check if latency exceeds target
    pub fn exceeds_target(&self) -> bool {
        let stats = self.stats();
        stats.p95_ms() > self.target_ms as f64
    }

    /// Get latency margin (target - current)
    pub fn margin(&self) -> f32 {
        let stats = self.stats();
        self.target_ms - stats.p95_ms() as f32
    }

    /// Get stage statistics
    pub fn stage_stats(&self, stage_name: &str) -> StageStats {
        let stages = self.stages.lock();
        let latencies: Vec<f32> = stages
            .iter()
            .filter(|s| s.name == stage_name)
            .map(|s| s.latency_ms)
            .collect();

        if latencies.is_empty() {
            return StageStats::default();
        }

        let mut sorted = latencies.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let sum: f32 = latencies.iter().sum();
        let count = latencies.len();

        StageStats {
            count,
            min_ms: latencies.iter().fold(f32::INFINITY, |a, &b| a.min(b)),
            max_ms: latencies.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b)),
            avg_ms: sum / count as f32,
            p50_ms: sorted[sorted.len() * 50 / 100],
            p95_ms: sorted[sorted.len() * 95 / 100],
            p99_ms: sorted[sorted.len() * 99 / 100],
        }
    }

    /// Get all stage names
    pub fn stage_names(&self) -> Vec<String> {
        let stages = self.stages.lock();
        let mut names: Vec<String> = stages.iter().map(|s| s.name.clone()).collect();
        names.sort();
        names.dedup();
        names
    }

    /// Reset all statistics
    pub fn reset(&self) {
        self.monitor.reset();
        self.stages.lock().clear();
    }

    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get target latency
    pub fn target(&self) -> f32 {
        self.target_ms
    }
}

/// Stage statistics
#[derive(Debug, Clone, Default)]
pub struct StageStats {
    pub count: usize,
    pub min_ms: f32,
    pub max_ms: f32,
    pub avg_ms: f32,
    pub p50_ms: f32,
    pub p95_ms: f32,
    pub p99_ms: f32,
}

impl StageStats {
    /// Format as string
    pub fn to_string(&self) -> String {
        format!(
            "StageStats {{ count: {}, min: {:.2}ms, max: {:.2}ms, avg: {:.2}ms, p50: {:.2}ms, p95: {:.2}ms, p99: {:.2}ms }}",
            self.count, self.min_ms, self.max_ms, self.avg_ms, self.p50_ms, self.p95_ms, self.p99_ms
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_tracker_creation() {
        let tracker = PipelineLatency::new(35.0);
        assert_eq!(tracker.target(), 35.0);
        assert!(!tracker.exceeds_target());
    }

    #[test]
    fn test_latency_recording() {
        let tracker = PipelineLatency::new(35.0);
        
        tracker.record_frame(Duration::from_millis(10));
        tracker.record_frame(Duration::from_millis(20));
        tracker.record_frame(Duration::from_millis(30));

        let stats = tracker.stats();
        assert_eq!(stats.count, 3);
        assert!((stats.min_ms - 10.0).abs() < 0.01);
        assert!((stats.max_ms - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_stage_recording() {
        let tracker = PipelineLatency::new(35.0);
        
        tracker.record_stage("rf_capture", Duration::from_millis(5));
        tracker.record_stage("dsp", Duration::from_millis(10));
        tracker.record_stage("rf_capture", Duration::from_millis(6));

        let names = tracker.stage_names();
        assert!(names.contains(&"rf_capture".to_string()));
        assert!(names.contains(&"dsp".to_string()));

        let stats = tracker.stage_stats("rf_capture");
        assert_eq!(stats.count, 2);
    }

    #[test]
    fn test_latency_budget() {
        let budget = LatencyBudget::default();
        assert_eq!(budget.total_ms, 35.0);
        assert!(budget.rf_capture_ms > 0.0);
        assert!(budget.ml_inference_ms > 0.0);
    }

    #[test]
    fn test_stage_stats() {
        let stats = StageStats {
            count: 100,
            min_ms: 1.0,
            max_ms: 50.0,
            avg_ms: 10.0,
            p50_ms: 8.0,
            p95_ms: 30.0,
            p99_ms: 45.0,
        };

        let s = stats.to_string();
        assert!(s.contains("count: 100"));
        assert!(s.contains("avg: 10.00ms"));
    }
}
