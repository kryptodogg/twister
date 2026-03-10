//! Hardware synchronization module

use crate::utils::error::Result;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::Mutex;

/// Clock source for synchronization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockSource {
    /// Internal clock (default)
    Internal,
    /// External word clock
    ExternalWordClock,
    /// PTP (Precision Time Protocol)
    PTP,
    /// GPS disciplined
    GPS,
}

/// Synchronization status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    /// All devices synchronized
    Synchronized,
    /// Synchronization in progress
    Syncing,
    /// Synchronization failed
    Desynchronized,
    /// Not attempting sync
    NotSyncing,
}

/// Hardware synchronizer for multi-device coordination
pub struct HardwareSynchronizer {
    clock_source: ClockSource,
    status: Arc<Mutex<SyncStatus>>,
    rf_offset_samples: Arc<AtomicI64>,
    audio_offset_samples: Arc<AtomicI64>,
    last_sync_time: Arc<Mutex<Option<Instant>>>,
    running: Arc<AtomicBool>,
}

/// Sync measurement result
#[derive(Debug, Clone)]
pub struct SyncMeasurement {
    pub rf_timestamp: Instant,
    pub audio_timestamp: Instant,
    pub offset_samples: i64,
    pub confidence: f32,
}

impl HardwareSynchronizer {
    /// Create a new hardware synchronizer
    pub fn new(clock_source: ClockSource) -> Self {
        Self {
            clock_source,
            status: Arc::new(Mutex::new(SyncStatus::NotSyncing)),
            rf_offset_samples: Arc::new(AtomicI64::new(0)),
            audio_offset_samples: Arc::new(AtomicI64::new(0)),
            last_sync_time: Arc::new(Mutex::new(None)),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start synchronization process
    pub async fn start_sync(&self) -> Result<()> {
        *self.status.lock() = SyncStatus::Syncing;
        self.running.store(true, Ordering::SeqCst);

        // Perform initial synchronization
        self.perform_sync().await?;

        Ok(())
    }

    /// Stop synchronization
    pub fn stop_sync(&self) {
        self.running.store(false, Ordering::SeqCst);
        *self.status.lock() = SyncStatus::NotSyncing;
    }

    /// Perform synchronization measurement
    async fn perform_sync(&self) -> Result<SyncMeasurement> {
        let rf_timestamp = Instant::now();
        
        // Simulate audio timestamp (in real implementation, would measure actual hardware)
        let audio_timestamp = Instant::now();
        
        // Calculate offset (simplified - real implementation would use correlation)
        let offset_samples = 0i64;

        let measurement = SyncMeasurement {
            rf_timestamp,
            audio_timestamp,
            offset_samples,
            confidence: 0.95,
        };

        // Update offsets
        self.rf_offset_samples.store(offset_samples, Ordering::SeqCst);
        self.audio_offset_samples.store(offset_samples, Ordering::SeqCst);
        *self.last_sync_time.lock() = Some(Instant::now());

        if measurement.confidence > 0.8 {
            *self.status.lock() = SyncStatus::Synchronized;
        } else {
            *self.status.lock() = SyncStatus::Desynchronized;
        }

        Ok(measurement)
    }

    /// Get current sync status
    pub fn status(&self) -> SyncStatus {
        *self.status.lock()
    }

    /// Get RF offset in samples
    pub fn rf_offset(&self) -> i64 {
        self.rf_offset_samples.load(Ordering::Relaxed)
    }

    /// Get audio offset in samples
    pub fn audio_offset(&self) -> i64 {
        self.audio_offset_samples.load(Ordering::Relaxed)
    }

    /// Get time since last sync
    pub fn time_since_sync(&self) -> Option<Duration> {
        self.last_sync_time.lock().map(|t| t.elapsed())
    }

    /// Check if synchronization is valid
    pub fn is_valid(&self, max_age: Duration) -> bool {
        self.time_since_sync()
            .map(|age| age < max_age && self.status() == SyncStatus::Synchronized)
            .unwrap_or(false)
    }

    /// Apply offset correction to samples
    pub fn correct_offset(&self, samples: &[f32], offset_samples: i64) -> Vec<f32> {
        if offset_samples == 0 {
            return samples.to_vec();
        }

        if offset_samples > 0 {
            // Delay: prepend zeros
            let mut result = vec![0.0f32; offset_samples as usize];
            result.extend_from_slice(samples);
            result
        } else {
            // Advance: skip samples
            let skip = (-offset_samples) as usize;
            if skip >= samples.len() {
                vec![]
            } else {
                samples[skip..].to_vec()
            }
        }
    }

    /// Get clock source
    pub fn clock_source(&self) -> ClockSource {
        self.clock_source
    }
}

impl Default for HardwareSynchronizer {
    fn default() -> Self {
        Self::new(ClockSource::Internal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synchronizer_creation() {
        let sync = HardwareSynchronizer::new(ClockSource::Internal);
        assert_eq!(sync.status(), SyncStatus::NotSyncing);
        assert_eq!(sync.rf_offset(), 0);
    }

    #[test]
    fn test_offset_correction_zero() {
        let sync = HardwareSynchronizer::default();
        let samples = vec![1.0f32, 2.0, 3.0];
        let corrected = sync.correct_offset(&samples, 0);
        assert_eq!(corrected, samples);
    }

    #[test]
    fn test_offset_correction_delay() {
        let sync = HardwareSynchronizer::default();
        let samples = vec![1.0f32, 2.0, 3.0];
        let corrected = sync.correct_offset(&samples, 2);
        assert_eq!(corrected.len(), 5);
        assert_eq!(corrected[0], 0.0);
        assert_eq!(corrected[1], 0.0);
        assert_eq!(corrected[2], 1.0);
    }

    #[test]
    fn test_offset_correction_advance() {
        let sync = HardwareSynchronizer::default();
        let samples = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        let corrected = sync.correct_offset(&samples, -2);
        assert_eq!(corrected.len(), 3);
        assert_eq!(corrected[0], 3.0);
        assert_eq!(corrected[1], 4.0);
        assert_eq!(corrected[2], 5.0);
    }
}
