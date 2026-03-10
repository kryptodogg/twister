//! Pipeline synchronization

use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use crossbeam_channel::{bounded, Sender, Receiver};

/// Sync point for coordinating pipeline stages
#[derive(Debug, Clone)]
pub struct SyncPoint {
    /// Stage name
    pub stage: String,
    /// Timestamp
    pub timestamp: Instant,
    /// Sequence number
    pub sequence: u64,
}

/// Pipeline synchronizer
pub struct PipelineSynchronizer {
    /// Current sequence number
    sequence: Arc<Mutex<u64>>,
    /// Sync points for each stage
    sync_points: Arc<Mutex<Vec<SyncPoint>>>,
    /// Frame timing channel
    frame_tx: Sender<FrameTiming>,
    frame_rx: Receiver<FrameTiming>,
    /// Target frame duration
    target_frame_duration: Duration,
}

/// Frame timing information
#[derive(Debug, Clone)]
pub struct FrameTiming {
    /// Frame sequence number
    pub sequence: u64,
    /// Capture start time
    pub capture_start: Instant,
    /// Processing start time
    pub processing_start: Instant,
    /// Output time
    pub output_time: Instant,
}

impl PipelineSynchronizer {
    /// Create a new pipeline synchronizer
    pub fn new(target_latency_ms: u32) -> Self {
        let (tx, rx) = bounded(16);

        Self {
            sequence: Arc::new(Mutex::new(0)),
            sync_points: Arc::new(Mutex::new(Vec::new())),
            frame_tx: tx,
            frame_rx: rx,
            target_frame_duration: Duration::from_millis(target_latency_ms as u64),
        }
    }

    /// Get next sequence number
    pub fn next_sequence(&self) -> u64 {
        let mut seq = self.sequence.lock();
        *seq += 1;
        *seq
    }

    /// Record a sync point
    pub fn record_sync(&self, stage: &str) -> SyncPoint {
        let sequence = *self.sequence.lock();
        let point = SyncPoint {
            stage: stage.to_string(),
            timestamp: Instant::now(),
            sequence,
        };

        let mut points = self.sync_points.lock();
        points.push(point.clone());

        // Keep only last 1000 points
        if points.len() > 1000 {
            points.remove(0);
        }

        point
    }

    /// Send frame timing
    pub fn send_frame_timing(&self, timing: FrameTiming) -> Result<(), String> {
        self.frame_tx.try_send(timing)
            .map_err(|e| e.to_string())
    }

    /// Receive frame timing
    pub fn recv_frame_timing(&self) -> Option<FrameTiming> {
        self.frame_rx.try_recv().ok()
    }

    /// Get sync points for a sequence
    pub fn get_sequence_points(&self, sequence: u64) -> Vec<SyncPoint> {
        let points = self.sync_points.lock();
        points
            .iter()
            .filter(|p| p.sequence == sequence)
            .cloned()
            .collect()
    }

    /// Calculate stage latency
    pub fn stage_latency(&self, from_stage: &str, to_stage: &str, sequence: u64) -> Option<Duration> {
        let points = self.get_sequence_points(sequence);
        
        let from_time = points.iter().find(|p| p.stage == from_stage)?.timestamp;
        let to_time = points.iter().find(|p| p.stage == to_stage)?.timestamp;

        Some(to_time.duration_since(from_time))
    }

    /// Get total pipeline latency for a sequence
    pub fn total_latency(&self, sequence: u64) -> Option<Duration> {
        self.stage_latency("capture", "output", sequence)
    }

    /// Wait for next frame boundary
    pub fn wait_for_frame(&self, frame_start: Instant) {
        let elapsed = frame_start.elapsed();
        if elapsed < self.target_frame_duration {
            let sleep_time = self.target_frame_duration - elapsed;
            std::thread::sleep(sleep_time);
        }
    }

    /// Check if frame is on schedule
    pub fn is_on_schedule(&self, frame_start: Instant) -> bool {
        frame_start.elapsed() < self.target_frame_duration
    }

    /// Get target frame duration
    pub fn target_duration(&self) -> Duration {
        self.target_frame_duration
    }

    /// Get current sequence
    pub fn current_sequence(&self) -> u64 {
        *self.sequence.lock()
    }

    /// Reset synchronizer
    pub fn reset(&self) {
        *self.sequence.lock() = 0;
        self.sync_points.lock().clear();
    }
}

/// Clock synchronization for multi-device capture
pub struct ClockSync {
    /// Reference clock
    reference: Instant,
    /// Offset for each device
    offsets: Arc<Mutex<Vec<i64>>>, // in microseconds
    /// Drift rates
    drift_rates: Arc<Mutex<Vec<f64>>>, // ppm
}

impl ClockSync {
    /// Create a new clock synchronizer
    pub fn new(num_devices: usize) -> Self {
        Self {
            reference: Instant::now(),
            offsets: Arc::new(Mutex::new(vec![0i64; num_devices])),
            drift_rates: Arc::new(Mutex::new(vec![0.0f64; num_devices])),
        }
    }

    /// Set offset for a device
    pub fn set_offset(&self, device_idx: usize, offset_us: i64) {
        let mut offsets = self.offsets.lock();
        if device_idx < offsets.len() {
            offsets[device_idx] = offset_us;
        }
    }

    /// Get corrected timestamp for a device
    pub fn corrected_time(&self, device_idx: usize, local_time: Instant) -> Instant {
        let offsets = self.offsets.lock();
        let drift_rates = self.drift_rates.lock();

        if device_idx >= offsets.len() {
            return local_time;
        }

        let offset_us = offsets[device_idx];
        let drift_ppm = drift_rates[device_idx];

        let elapsed = local_time.duration_since(self.reference);
        let drift_correction = elapsed.as_secs_f64() * drift_ppm * 1e-6;
        let total_correction = offset_us as f64 + drift_correction * 1e6;

        if total_correction > 0.0 {
            local_time + Duration::from_micros(total_correction as u64)
        } else {
            local_time - Duration::from_micros((-total_correction) as u64)
        }
    }

    /// Update drift rate for a device
    pub fn update_drift(&self, device_idx: usize, measured_drift_ppm: f64) {
        let mut drift_rates = self.drift_rates.lock();
        if device_idx < drift_rates.len() {
            // Low-pass filter the drift estimate
            drift_rates[device_idx] = 0.9 * drift_rates[device_idx] + 0.1 * measured_drift_ppm;
        }
    }

    /// Get current offset
    pub fn offset(&self, device_idx: usize) -> i64 {
        let offsets = self.offsets.lock();
        offsets.get(device_idx).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synchronizer_creation() {
        let sync = PipelineSynchronizer::new(35);
        assert_eq!(sync.current_sequence(), 0);
        assert_eq!(sync.target_duration(), Duration::from_millis(35));
    }

    #[test]
    fn test_sequence_generation() {
        let sync = PipelineSynchronizer::new(35);
        
        let seq1 = sync.next_sequence();
        let seq2 = sync.next_sequence();
        let seq3 = sync.next_sequence();

        assert_eq!(seq1, 1);
        assert_eq!(seq2, 2);
        assert_eq!(seq3, 3);
    }

    #[test]
    fn test_sync_point_recording() {
        let sync = PipelineSynchronizer::new(35);
        
        let _ = sync.next_sequence();
        let point1 = sync.record_sync("capture");
        std::thread::sleep(Duration::from_millis(10));
        let point2 = sync.record_sync("processing");

        assert_eq!(point1.sequence, point2.sequence);
        assert!(point2.timestamp > point1.timestamp);

        let latency = sync.stage_latency("capture", "processing", 1);
        assert!(latency.is_some());
        assert!(latency.unwrap().as_millis() >= 10);
    }

    #[test]
    fn test_clock_sync() {
        let clock = ClockSync::new(3);
        
        clock.set_offset(0, 100); // Device 0 is 100us ahead
        clock.set_offset(1, -50); // Device 1 is 50us behind

        assert_eq!(clock.offset(0), 100);
        assert_eq!(clock.offset(1), -50);
        assert_eq!(clock.offset(2), 0);
    }

    #[test]
    fn test_frame_timing() {
        let sync = PipelineSynchronizer::new(35);
        
        let timing = FrameTiming {
            sequence: 1,
            capture_start: Instant::now(),
            processing_start: Instant::now(),
            output_time: Instant::now(),
        };

        assert!(sync.send_frame_timing(timing.clone()).is_ok());
        
        let received = sync.recv_frame_timing();
        assert!(received.is_some());
        assert_eq!(received.unwrap().sequence, 1);
    }
}
