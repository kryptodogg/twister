// src/anc_recording.rs — Real-time ANC multi-channel sweep recording
//
// Captures 20-second responses from audio inputs during calibration sweep:
//   Device 0: C925e physical microphone (16-bit 32 kHz, rank-0 primary)
//   Device 1: Rear Panel Mic (Pink, 192 kHz)
//   Device 2: Rear Line-In (Blue, 192 kHz) — optional
//
// All channels resampled to 192 kHz reference before FFT analysis

use std::collections::HashMap;

/// ANC calibration recording state machine
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CalibrationState {
    Idle,
    Recording,
    Analyzing,
    Complete,
}

/// Multi-channel sweep recording buffer
#[derive(Clone, Debug)]
pub struct RecordingBuffer {
    /// Samples per device (indexed by device ID)
    pub channels: HashMap<usize, Vec<f32>>,
    /// Target: 20 seconds @ 192 kHz = 3,840,000 samples
    pub target_samples: usize,
    /// Reference sample rate (192 kHz)
    pub ref_sr: f32,
    /// Current state
    pub state: CalibrationState,
}

impl RecordingBuffer {
    pub fn new(ref_sr: f32) -> Self {
        Self {
            channels: HashMap::new(),
            target_samples: (20.0 * ref_sr) as usize,
            ref_sr,
            state: CalibrationState::Idle,
        }
    }

    /// Start recording
    pub fn start_recording(&mut self) {
        self.channels.clear();
        self.state = CalibrationState::Recording;
    }

    /// Add tagged samples from a device
    pub fn push_samples(&mut self, device_idx: usize, samples: &[f32]) {
        if self.state != CalibrationState::Recording {
            return;
        }

        self.channels
            .entry(device_idx)
            .or_insert_with(Vec::new)
            .extend_from_slice(samples);
    }

    /// Check if recording is complete (all channels have >= target samples)
    pub fn is_complete(&self) -> bool {
        if self.channels.is_empty() {
            return false;
        }

        // At least 2 channels (requirement: C925e + one of rear devices)
        let mut active_channels = 0;
        for samples in self.channels.values() {
            if samples.len() >= self.target_samples {
                active_channels += 1;
            }
        }

        active_channels >= 2
    }

    /// Finalize recording and transition to analysis
    pub fn finalize(&mut self) -> HashMap<usize, Vec<f32>> {
        self.state = CalibrationState::Analyzing;

        // Trim all channels to exactly target length
        let mut result = HashMap::new();
        for (dev_id, mut samples) in self.channels.drain() {
            samples.truncate(self.target_samples);
            result.insert(dev_id, samples);
        }

        result
    }

    /// Get recording progress (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        if self.channels.is_empty() {
            return 0.0;
        }

        let mut total_progress = 0.0;
        let mut count = 0;

        for samples in self.channels.values() {
            total_progress += (samples.len() as f32 / self.target_samples as f32).min(1.0);
            count += 1;
        }

        if count == 0 {
            0.0
        } else {
            total_progress / count as f32
        }
    }

    /// Get recording status string
    pub fn status(&self) -> String {
        match self.state {
            CalibrationState::Idle => "Ready".to_string(),
            CalibrationState::Recording => {
                let pct = (self.progress() * 100.0) as u32;
                format!("Recording... {}%", pct)
            }
            CalibrationState::Analyzing => "Analyzing FFT + Mamba...".to_string(),
            CalibrationState::Complete => "✓ Calibration complete".to_string(),
        }
    }
}

/// Resample audio to common reference rate
pub fn resample_to_ref(samples: &[f32], from_sr: f32, to_sr: f32) -> Vec<f32> {
    if (from_sr - to_sr).abs() < 1.0 {
        // Already at target rate
        return samples.to_vec();
    }

    let ratio = to_sr / from_sr;
    let out_len = (samples.len() as f32 * ratio) as usize;
    let mut result = Vec::with_capacity(out_len);

    for i in 0..out_len {
        let t = i as f32 / ratio;
        let idx = t.floor() as usize;
        let frac = t - idx as f32;

        if idx + 1 < samples.len() {
            let a = samples[idx];
            let b = samples[idx + 1];
            result.push(a * (1.0 - frac) + b * frac);
        } else if idx < samples.len() {
            result.push(samples[idx]);
        }
    }

    result
}

/// Map device indices to human-readable names
pub fn device_name(idx: usize) -> &'static str {
    match idx {
        0 => "C925e Physical Mic (32 kHz)",
        1 => "Rear Mic (Pink, 192 kHz)",
        2 => "Rear Line-In (Blue, 192 kHz)",
        _ => "Unknown Device",
    }
}
