/// src/dispatch/signal_metadata.rs
/// Multi-rate signal metadata with native sample rates and delta-time tagging
///
/// Purpose: Tag disparate input streams (C925e @ 32kHz, SDR @ 6.144MHz) with their exact
/// physical timesteps so Mamba's SSM can integrate them as a single continuous observation.

use std::time::{SystemTime, UNIX_EPOCH};

/// Physical timestep for a single sample in a stream
#[derive(Clone, Copy, Debug)]
pub struct SampleDeltaTime {
    /// Nanoseconds between consecutive samples (exact reciprocal of sample rate)
    pub dt_nanos: u64,
    /// Corresponding frequency in Hz for validation
    pub sample_rate_hz: u32,
}

impl SampleDeltaTime {
    /// Create from sample rate in Hz
    pub fn from_sample_rate(sample_rate_hz: u32) -> Self {
        if sample_rate_hz == 0 {
            panic!("Sample rate must be non-zero");
        }
        let dt_nanos = 1_000_000_000u64 / sample_rate_hz as u64;
        Self {
            dt_nanos,
            sample_rate_hz,
        }
    }

    /// Get the step size in seconds (for Mamba SSM parameter)
    pub fn as_seconds(&self) -> f32 {
        (self.dt_nanos as f32) / 1_000_000_000.0
    }

    /// Get the step size in microseconds
    pub fn as_micros(&self) -> f32 {
        (self.dt_nanos as f32) / 1_000.0
    }

    /// Validate consistency with a sample count and time window
    pub fn validate_window(&self, sample_count: usize, expected_duration_nanos: u64) -> bool {
        let actual_duration = sample_count as u64 * self.dt_nanos;
        let tolerance = expected_duration_nanos / 20; // Allow 5% drift
        actual_duration >= (expected_duration_nanos.saturating_sub(tolerance))
            && actual_duration <= (expected_duration_nanos + tolerance)
    }
}

/// A signal stream tagged with its native sample rate and physical timestamp
#[derive(Clone, Debug)]
pub struct TaggedSignalBuffer {
    /// Unique identifier for this stream (e.g., "c925e_mic", "sdr_2p4ghz")
    pub stream_id: String,

    /// Raw audio/RF samples as 32-bit floats (normalized to [-1.0, 1.0])
    pub samples: Vec<f32>,

    /// Native sample rate (do NOT normalize; preserve physical reality)
    pub dt: SampleDeltaTime,

    /// Wall-clock timestamp when this buffer was captured (Unix nanoseconds)
    pub timestamp_nanos: u64,

    /// UNIX timestamp of first sample (for multi-buffer alignment)
    pub buffer_start_nanos: u64,
}

impl TaggedSignalBuffer {
    /// Create a new tagged signal buffer
    pub fn new(
        stream_id: String,
        samples: Vec<f32>,
        sample_rate_hz: u32,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_nanos() as u64;

        let dt = SampleDeltaTime::from_sample_rate(sample_rate_hz);

        Self {
            stream_id,
            samples,
            dt,
            timestamp_nanos: now,
            buffer_start_nanos: now,
        }
    }

    /// Get the physical duration of this buffer in nanoseconds
    pub fn duration_nanos(&self) -> u64 {
        self.samples.len() as u64 * self.dt.dt_nanos
    }

    /// Get the timestamp of the last sample in this buffer
    pub fn end_timestamp_nanos(&self) -> u64 {
        self.buffer_start_nanos + self.duration_nanos()
    }

    /// Check if this buffer overlaps with another in time
    pub fn overlaps_with(&self, other: &TaggedSignalBuffer) -> bool {
        let self_start = self.buffer_start_nanos;
        let self_end = self.end_timestamp_nanos();
        let other_start = other.buffer_start_nanos;
        let other_end = other.end_timestamp_nanos();

        self_start < other_end && other_start < self_end
    }

    /// Validate that sample count matches expected window duration
    pub fn validate(&self, expected_window_nanos: u64) -> bool {
        self.dt.validate_window(self.samples.len(), expected_window_nanos)
    }
}

/// Collection of multi-rate streams ready for Mamba inference
#[derive(Clone, Debug)]
pub struct MultiRateSignalFrame {
    /// All input streams (disparate sample rates, native resolution)
    pub streams: Vec<TaggedSignalBuffer>,

    /// The physical time window this frame represents (in nanoseconds)
    pub window_duration_nanos: u64,

    /// Frame index in the dispatch loop
    pub frame_index: u64,
}

impl MultiRateSignalFrame {
    /// Create a new multi-rate frame from disparate streams
    pub fn new(streams: Vec<TaggedSignalBuffer>, window_duration_nanos: u64, frame_index: u64) -> Self {
        Self {
            streams,
            window_duration_nanos,
            frame_index,
        }
    }

    /// Validate all streams are within the expected time window
    pub fn validate_alignment(&self) -> bool {
        self.streams
            .iter()
            .all(|stream| stream.validate(self.window_duration_nanos))
    }

    /// Get statistics about the frame (for diagnostics)
    pub fn get_stats(&self) -> FrameStats {
        let total_samples: usize = self.streams.iter().map(|s| s.samples.len()).sum();
        let min_rate = self.streams.iter().map(|s| s.dt.sample_rate_hz).min();
        let max_rate = self.streams.iter().map(|s| s.dt.sample_rate_hz).max();

        FrameStats {
            num_streams: self.streams.len(),
            total_samples,
            min_sample_rate_hz: min_rate.unwrap_or(0),
            max_sample_rate_hz: max_rate.unwrap_or(0),
            duration_ms: (self.window_duration_nanos as f32) / 1_000_000.0,
        }
    }
}

/// Statistics about a multi-rate frame
#[derive(Clone, Debug)]
pub struct FrameStats {
    pub num_streams: usize,
    pub total_samples: usize,
    pub min_sample_rate_hz: u32,
    pub max_sample_rate_hz: u32,
    pub duration_ms: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_time_c925e() {
        let dt = SampleDeltaTime::from_sample_rate(32_000);
        assert_eq!(dt.dt_nanos, 31_250); // 1/32000 = 31.25 µs
        assert!((dt.as_micros() - 31.25).abs() < 0.01);
    }

    #[test]
    fn test_delta_time_sdr() {
        let dt = SampleDeltaTime::from_sample_rate(6_144_000);
        assert_eq!(dt.dt_nanos, 162); // 1/6.144MHz ≈ 162.76 ns
        assert!((dt.as_micros() - 0.16276).abs() < 0.001);
    }

    #[test]
    fn test_buffer_validation() {
        let samples = vec![0.5; 320]; // 10ms @ 32kHz
        let buffer = TaggedSignalBuffer::new("c925e".to_string(), samples, 32_000);

        let window_10ms = 10_000_000; // 10ms in nanos
        assert!(buffer.validate(window_10ms));
        assert!(buffer.duration_nanos() == window_10ms);
    }

    #[test]
    fn test_multi_rate_alignment() {
        let c925e = TaggedSignalBuffer::new("c925e".to_string(), vec![0.0; 320], 32_000);
        let sdr = TaggedSignalBuffer::new("sdr_2p4ghz".to_string(), vec![0.0; 1920], 192_000);

        let frame = MultiRateSignalFrame::new(vec![c925e, sdr], 10_000_000, 0);
        assert!(frame.validate_alignment());

        let stats = frame.get_stats();
        assert_eq!(stats.num_streams, 2);
        assert_eq!(stats.total_samples, 2240);
    }
}
