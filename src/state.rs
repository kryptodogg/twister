
#[derive(Clone)]
pub struct FeatureFlags {
    pub enhanced_audio: bool,
    pub sparse_pdm: bool,
    pub coherence: bool,
    pub mamba_siren: bool,
}
impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enhanced_audio: true,
            sparse_pdm: false,
            coherence: false,
            mamba_siren: false,
        }
    }
}


// src/state.rs — Shared Atomic State  (v0.4)
//
// Changes from v0.3:
//   • AtomicF32 (stable since Rust 1.70) replaces AtomicU32 + f32::from_bits hacks.
//   • Training state: epoch, loss, latent_dim, training_active.
//   • SDR state: center_freq_hz, sdr_gain_db, sdr_active, sdr_sample_rate.
//   • mamba_anomaly_score: f32 output from the autoencoder reconstruction error.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use crate::af32::AtomicF32;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

// ── Channel-Based Telemetry (UI Events for async/UI separation) ────────────────────

/// UI Events emitted by async tasks (training, SDR, TDOA)
/// Consumed non-blockingly by Slint timer loop via crossbeam channel
#[derive(Debug, Clone)]
pub enum UiEvent {
    /// Training progress from Mamba/TimeGNN loops
    TrainingProgress {
        iteration: u32,
        total_iterations: u32,
        loss: f32,
        loss_min: f32,
    },
    /// Clustering results from TimeGNN discovery
    ClusteringStatus {
        pattern_count: u32,
        total_events: u32,
        silhouette_score: f32,
        convergence_iterations: u32,
    },
    /// SDR device status (opened, tuned, etc)
    SdrStatus {
        center_freq_hz: f64,
        gain_db: f32,
        is_active: bool,
    },
    /// Reconstruction error from Mamba anomaly detection
    NeuralReconstruction {
        anomaly_score: f32,
        confidence: f32,
        timestamp_us: u64,
    },
    /// Pattern discovery result
    GnnAnalysis {
        pattern_id: u32,
        frequency_hz: f32,
        confidence: f32,
    },
}

// ── Training Checkpoint Metadata (Fix #1: Training Persistence) ─────────────────

/// Metadata for Mamba checkpoint file.
/// Stores training progress (epoch, loss history) alongside weights.
/// Allows recovery of training state across application restarts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    /// Current training epoch (incremented each batch)
    pub epoch: u32,
    /// Moving average of training loss (exponential average of last 64 batches)
    pub loss_avg: f32,
    /// Minimum loss achieved during this training session
    pub loss_min: f32,
    /// Maximum loss during this training session (indicates training stability)
    pub loss_max: f32,
    /// ISO 8601 timestamp when checkpoint was created
    pub timestamp_created: String,
}

impl CheckpointMetadata {
    /// Create new metadata with current training state
    pub fn new(epoch: u32, loss_avg: f32, loss_min: f32, loss_max: f32) -> Self {
        CheckpointMetadata {
            epoch,
            loss_avg,
            loss_min,
            loss_max,
            timestamp_created: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Load metadata from JSON file (returns None if file doesn't exist)
    pub fn from_file(metadata_path: &str) -> Option<Self> {
        std::fs::read_to_string(metadata_path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
    }

    /// Save metadata to JSON file
    pub fn to_file(&self, metadata_path: &str) -> anyhow::Result<()> {
        let json = serde_json::to_string(self)?;
        std::fs::write(metadata_path, json)?;
        Ok(())
    }
}

// ── GUI Console Logging (Fix #2) ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMessage {
    pub timestamp: String,
    pub level: String,
    pub module: String,
    pub message: String,
}

impl LogMessage {
    pub fn new(level: &str, module: &str, message: &str) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            level: level.to_string(),
            module: module.to_string(),
            message: message.to_string(),
        }
    }
}

// ── Memo System (Phase 1) ──────────────────────────────────────────────────────

/// Mamba control state at time of memo (for forensic context).
/// Captures what active defenses were active when event occurred.
/// Includes multi-beam phased array + heterodyning for active denial synthesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MambaControlState {
    pub active_modes: Vec<String>, // simplified from ControlMode for now
    pub beam_azimuth: f32,         // -180..180°
    pub beam_elevation: f32,       // NEW: -45..45° (mouth targeting)
    pub beam_phases: Vec<f32>,     // Per-element phases
    pub heterodyned_beams: Vec<Option<f64>>, // 95GHz for mouth
    pub waveshape_drive: f32,      // 0.0..1.0
    pub anc_gain: f32,             // 0.0..1.0
}

/// 3D wave topology at time of memo.
/// Reveals attack signature characteristics and field distortion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveTopology {
    /// Phase coherence across mic pairs (range: -1.0..1.0 per pair)
    /// High values = coherent wavefront; Low = scattered/multi-source
    pub phase_coherence_db: Vec<f32>,
    /// Dominant spatial gradient direction in degrees
    pub field_gradient_azimuth: f32,
    /// Spatial curvature metric (0.0 = plane wave, >0 = curved wavefront)
    pub spatial_curvature: f32,
}

/// Manual REC button state machine for 30-second countdown recording.
/// Manages IDLE → RECORDING → SAVING state transitions with timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingStateEnum {
    /// Not recording
    Idle,
    /// Recording with countdown timer active (0-30000 ms)
    Recording,
    /// Recording complete, saving samples
    Saving,
}

/// Manual REC button state and timer management.
/// Tracks 30-second recording session with sample buffering.
#[derive(Debug, Clone)]
pub struct RecordingState {
    /// Current state (IDLE, RECORDING, SAVING)
    pub state: RecordingStateEnum,
    /// Remaining time in milliseconds (counts down from 30000 to 0)
    pub remaining_ms: u32,
    /// Total samples captured (at whatever sample rate)
    pub sample_count: usize,
}

impl RecordingState {
    /// Create a new RecordingState in IDLE state
    pub fn new() -> Self {
        let material_library =
            std::sync::Arc::new(tokio::sync::Mutex::new(MaterialLibrary::default()));
        RecordingState {
            state: RecordingStateEnum::Idle,
            remaining_ms: 0,
            sample_count: 0,
        }
    }

    /// Start recording (transition IDLE → RECORDING)
    pub fn start_recording(&mut self) {
        self.state = RecordingStateEnum::Recording;
        self.remaining_ms = 30000; // 30 seconds in milliseconds
        self.sample_count = 0;
    }

    /// Stop recording (transition RECORDING → SAVING)
    pub fn stop_recording(&mut self) {
        self.state = RecordingStateEnum::Saving;
    }

    /// Update timer by decrementing remaining time
    /// Called each frame with elapsed milliseconds
    pub fn update_timer_ms(&mut self, elapsed_ms: u32) {
        if self.remaining_ms > elapsed_ms {
            self.remaining_ms -= elapsed_ms;
        } else {
            self.remaining_ms = 0;
        }
    }

    /// Add captured samples to buffer
    pub fn add_samples(&mut self, count: usize) {
        self.sample_count += count;
    }

    /// Check if currently in IDLE state
    pub fn is_idle(&self) -> bool {
        self.state == RecordingStateEnum::Idle
    }

    /// Check if currently RECORDING
    pub fn is_recording(&self) -> bool {
        self.state == RecordingStateEnum::Recording
    }

    /// Check if SAVING (recording complete)
    pub fn is_saving(&self) -> bool {
        self.state == RecordingStateEnum::Saving
    }

    /// Get remaining time in milliseconds
    pub fn get_remaining_ms(&self) -> u32 {
        self.remaining_ms
    }

    /// Get total samples captured
    pub fn get_sample_count(&self) -> usize {
        self.sample_count
    }

    /// Check if timer has expired (reached 0)
    pub fn is_expired(&self) -> bool {
        self.remaining_ms == 0 && self.state == RecordingStateEnum::Recording
    }

    /// Reset to IDLE state (for next recording session)
    pub fn reset(&mut self) {
        self.state = RecordingStateEnum::Idle;
        self.remaining_ms = 0;
        self.sample_count = 0;
    }
}

/// Forensic note entry with user annotations and 3D wave context.
/// Every memo is a Phase 2 training example (multimodal feature vector).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoEntry {
    /// ISO 8601 timestamp (e.g., "2026-03-07T14:23:14.832401Z")
    pub timestamp_iso8601: String,
    /// Microseconds since Unix epoch for precise forensic correlation
    pub timestamp_micros: u64,
    /// Classification tag: NOTE, EVIDENCE, MANUAL-REC, ANALYSIS
    pub tag: String,
    /// User-provided note content (max 80 characters)
    pub content: String,

    /// Mamba control state at time of memo (what was Mamba doing?)
    pub mamba_control: Option<MambaControlState>,
    /// 3D wave topology (what did the field look like?)
    pub wave_topology: Option<WaveTopology>,
    /// How far from "flat horizon" baseline (0.0 = neutral, 1.0 = max distortion)
    pub flat_horizon_deviation: Option<f32>,
}

impl MemoEntry {
    /// Create a new MemoEntry with validation.
    ///
    /// Validates:
    /// - Content length <= 80 characters
    /// - Tag is one of: NOTE, EVIDENCE, MANUAL-REC, ANALYSIS
    pub fn new(
        timestamp_iso8601: String,
        timestamp_micros: u64,
        tag: String,
        content: &str,
    ) -> anyhow::Result<Self> {
        // Validate content length
        if content.len() > 80 {
            return Err(anyhow::anyhow!(
                "Memo content exceeds 80 character limit: {} characters",
                content.len()
            ));
        }

        // Validate tag
        match tag.as_str() {
            "NOTE" | "EVIDENCE" | "MANUAL-REC" | "ANALYSIS" => {
                // Valid tag
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Invalid memo tag: '{}'. Must be one of: NOTE, EVIDENCE, MANUAL-REC, ANALYSIS",
                    tag
                ));
            }
        }

        Ok(MemoEntry {
            timestamp_iso8601,
            timestamp_micros,
            tag,
            content: content.to_string(),
            mamba_control: None,
            wave_topology: None,
            flat_horizon_deviation: None,
        })
    }

    /// Create a new MemoEntry with full 3D wave topology context.
    /// This is called when capturing [EVIDENCE] or [MANUAL-REC] events.
    pub fn with_3d_context(
        timestamp_iso8601: String,
        timestamp_micros: u64,
        tag: String,
        content: &str,
        mamba_control: MambaControlState,
        wave_topology: WaveTopology,
        flat_horizon_deviation: f32,
    ) -> anyhow::Result<Self> {
        // Validate content length
        if content.len() > 80 {
            return Err(anyhow::anyhow!(
                "Memo content exceeds 80 character limit: {} characters",
                content.len()
            ));
        }

        // Validate tag
        match tag.as_str() {
            "NOTE" | "EVIDENCE" | "MANUAL-REC" | "ANALYSIS" => {
                // Valid tag
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Invalid memo tag: '{}'. Must be one of: NOTE, EVIDENCE, MANUAL-REC, ANALYSIS",
                    tag
                ));
            }
        }

        Ok(MemoEntry {
            timestamp_iso8601,
            timestamp_micros,
            tag,
            content: content.to_string(),
            mamba_control: Some(mamba_control),
            wave_topology: Some(wave_topology),
            flat_horizon_deviation: Some(flat_horizon_deviation),
        })
    }
}

// ── Enums ─────────────────────────────────────────────────────────────────────

// ── Enums & Modes ─────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
pub enum WaveshapeMode {
    Sine = 0,
    Square = 1,
    Triangle = 2,
    Sawtooth = 3,
    SoftClip = 4,
}

impl WaveshapeMode {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::Square,
            2 => Self::Triangle,
            3 => Self::Sawtooth,
            4 => Self::SoftClip,
            _ => Self::Sine,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
pub enum DenialMode {
    Off = 0,
    AntiPhase = 1,
    NoiseMask = 2,
    PureTone = 3,
    Sweep = 4,
}

impl DenialMode {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::AntiPhase,
            2 => Self::NoiseMask,
            3 => Self::PureTone,
            4 => Self::Sweep,
            _ => Self::Off,
        }
    }
}

// ── Constants ─────────────────────────────────────────────────────────────────

pub const WATERFALL_DISPLAY_COLS: usize = 512;
pub const WATERFALL_DISPLAY_ROWS: usize = 128;
pub const WATERFALL_DISPLAY_CELLS: usize = WATERFALL_DISPLAY_COLS * WATERFALL_DISPLAY_ROWS;

// INVESTIGATIVE FIX: Push input gain aggressively to max ceiling to achieve 103 dB SNL sensitivity.
pub const AGC_TARGET_DBFS: f32 = 0.0; // Max out the input (was -3.0)
pub const AGC_MAX_GAIN_DB: f32 = 120.0; // Uncapped forensic boost (was 90.0)
pub const AGC_MIN_GAIN_DB: f32 = 0.0;

// ── AppState ──────────────────────────────────────────────────────────────────

use crate::materials::material_library::MaterialLibrary;

pub struct AppState {
    pub material_library: std::sync::Arc<tokio::sync::Mutex<MaterialLibrary>>,
    // ── Core detection ────────────────────────────────────────────────────────
    pub detected_freq: AtomicF32,
    pub denial_freq_override: AtomicF32,
    pub master_gain: AtomicF32,
    pub mode: AtomicU32,
    pub feature_flags: Mutex<FeatureFlags>,
    pub feature_confidence: AtomicF32,
    pub impulse_anomaly_score: AtomicF32,
    pub harassment_detected: AtomicBool,
    pub auto_tune: AtomicBool,
    pub running: AtomicBool,
    pub anc_ok: AtomicBool, // Indicates if the Active Defense is armed and aligned

    // ── Audio output ──────────────────────────────────────────────────────────
    pub wavefield_image: Mutex<Vec<u8>>,
    pub output_frames: Mutex<Vec<f32>>,
    pub output_cursor: AtomicU32,

    // ── Waterfall ─────────────────────────────────────────────────────────────
    pub waterfall_rgba: Mutex<Vec<u32>>,
    pub spectrum_bars: Mutex<Vec<f32>>,

    // ── SDR Visual State ─────────────────────────────────────────────────────
    pub sdr_waterfall_rgba: Mutex<Vec<u32>>,
    pub sdr_spectrum_bars: Mutex<Vec<f32>>,
    pub sdr_mags: Mutex<Vec<f32>>,
    pub rtl_connected: AtomicBool,
    pub sdr_sweeping: AtomicBool,

    // ── PDM ───────────────────────────────────────────────────────────────────
    pub pdm_active: AtomicBool,
    pub pdm_clock_mhz: AtomicF32,
    pub tx_mags: Mutex<Vec<f32>>,
    pub oversample_ratio: AtomicU32,
    pub snr_db: AtomicF32,
    /// Counter of PDM spikes rejected by reject_pdm_spikes filter
    pub pdm_spike_count: std::sync::atomic::AtomicU64,

    // ── Waveshaping ───────────────────────────────────────────────────────────
    pub waveshape_mode: AtomicU32,
    pub waveshape_drive: AtomicF32,

    // ── TDOA beamforming ──────────────────────────────────────────────────────
    pub input_device_count: AtomicU32,
    pub beam_azimuth_deg: AtomicF32,
    /// Elevation from vertical TDOA pairs (-π/2 to π/2 radians)
    pub beam_elevation_rad: AtomicF32,
    pub beam_confidence: AtomicF32,
    pub polarization_angle: AtomicF32,
    pub beam_focus_deg: AtomicF32,

    // ── AGC ───────────────────────────────────────────────────────────────────
    pub agc_gain_db: AtomicF32,
    pub agc_peak_dbfs: AtomicF32,
    pub output_peak_db: AtomicF32,

    // ── Mamba autoencoder ─────────────────────────────────────────────────────
    /// Reconstruction MSE from the most recent inference pass (0 = unknown).
    pub mamba_anomaly_score: AtomicF32,
    /// 64-dimensional latent embedding from Mamba inference (MAMBA_LATENT_DIM).
    pub latent_embedding: Mutex<Vec<f32>>,
    /// Full-range ANC phase calibration data (1 Hz - 12.288 MHz).
    pub anc_calibration: Mutex<crate::anc_calibration::FullRangeCalibration>,
    /// ANC calibration recording buffer (20-second multi-channel sweep capture).
    pub anc_recording: Mutex<crate::anc_recording::RecordingBuffer>,
    /// Whether ANC calibration has been performed.
    pub anc_calibrated: AtomicBool,
    /// Whether an ANC calibration has been requested by the UI.
    pub pending_anc_calibration: AtomicBool,
    /// Active Noise Cancellation engine (LMS filter + phase calibration).
    pub anc_engine: Mutex<crate::anc::AncEngine>,
    /// Current training epoch.
    pub train_epoch: AtomicU32,
    /// Most recent training loss.
    pub train_loss: AtomicF32,
    /// Whether the training loop is actively running.
    pub training_active: AtomicBool,
    /// Number of frames in the replay buffer.
    pub replay_buf_len: AtomicU32,
    /// Checkpoint path for saving/loading weights.
    pub checkpoint_path: Mutex<String>,
    /// Emergency shut-off for all Mamba inference.
    pub mamba_emergency_off: AtomicBool,
    /// Blending ratio for Smart ANC (0.0 = 100% LMS, 1.0 = 100% Mamba). Default 0.3.
    pub smart_anc_blend: AtomicF32,

    // ── TX Improvement (Mamba waveform optimization) ─────────────────────────
    /// TX spectral delta RMS (convergence metric) in dB.
    pub tx_delta_rms_db: AtomicF32,
    /// History of TX delta RMS for visualization [last 100 values].
    pub tx_delta_history: Mutex<std::collections::VecDeque<f32>>,
    /// Current TX spectral deltas [512 bins] for PDM synthesis.
    pub tx_spectral_deltas: Mutex<Vec<f32>>,

    // ── Dispatch timing ──────────────────────────────────────────────────────
    /// Microseconds per dispatch frame.
    pub dispatch_us: AtomicU32,
    /// Total frames processed.
    pub frame_count: AtomicU32,

    // ── RTL-SDR ───────────────────────────────────────────────────────────────
    /// SDR enabled and scanning.
    pub sdr_active: AtomicBool,
    /// Center frequency in Hz.
    pub sdr_center_hz: AtomicF32,
    /// RF gain in dB.
    pub sdr_gain_db: AtomicF32,
    /// Sample rate (Hz).
    pub sdr_sample_rate: AtomicU32,
    /// Peak signal level seen by SDR (dBFS).
    pub sdr_peak_dbfs: AtomicF32,
    /// Strongest frequency offset from centre (Hz) seen in SDR FFT.
    pub sdr_peak_offset_hz: AtomicF32,
    /// Device index selected.
    pub sdr_device_index: AtomicU32,

    // ── Training data collection ─────────────────────────────────────────────
    /// Whether to collect training pairs from the dispatch loop.
    pub training_recording_enabled: AtomicBool,

    // ── DC Offset Forensic Monitoring ─────────────────────────────────────────
    /// RF DC bias (center frequency energy from SDR, unfiltered evidence).
    pub sdr_dc_bias: AtomicF32,
    /// Audio DC bias (absolute deviation from zero, "tazer" pressure component).
    pub audio_dc_bias: AtomicF32,

    pub reconstruction_mags: Mutex<Vec<f32>>,
    pub reconstructed_peak: AtomicF32,

    // ── Twister: musical auto-tuner ────────────────────────────────────────────
    pub gate_status: Mutex<String>,
    pub last_gate_reason: Mutex<String>,
    pub training_pairs_dropped: AtomicU32,
    pub gate_rejections_low_anomaly: AtomicU32,
    pub gate_rejections_low_confidence: AtomicU32,
    pub gate_rejections_other: AtomicU32,

    /// Most recent snapped note name, e.g. "A4", "C#5". "---" when silent.
    pub note_name: Mutex<String>,
    /// Cents offset: how far the raw detected freq was from the snapped note.
    /// Negative = raw was flat, positive = raw was sharp.
    pub note_cents: AtomicF32,
    /// Whether Twister musical auto-tuning is active for synthesis targets.
    pub twister_active: AtomicBool,

    // ── Memo System (Phase 1) ──────────────────────────────────────────────────
    /// Forensic memo storage (max 10,000 entries, FIFO when full)
    pub memos: Mutex<std::collections::VecDeque<MemoEntry>>,
    /// Current input text for new memos
    pub memo_input_text: Mutex<String>,
    /// Current selected tag for new memos
    pub memo_input_tag: Mutex<String>,
    /// Manual recording state machine
    pub recording_state: Mutex<RecordingState>,

    // ── GUI Console Logging (Fix #2) ───────────────────────────────────────────
    /// Ring buffer for UI console logs (max 1,000 messages)
    pub log_buffer: Mutex<VecDeque<LogMessage>>,
}

impl AppState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            material_library: std::sync::Arc::new(tokio::sync::Mutex::new(
                crate::materials::material_library::MaterialLibrary::default()
            )),
            detected_freq: AtomicF32::new(440.0),
            denial_freq_override: AtomicF32::new(0.0),
            master_gain: AtomicF32::new(0.0), // 0dB clean output
            mode: AtomicU32::new(0),
            feature_flags: Mutex::new(FeatureFlags::default()),
            feature_confidence: AtomicF32::new(0.0),
            impulse_anomaly_score: AtomicF32::new(0.0),
            harassment_detected: AtomicBool::new(false),
            auto_tune: AtomicBool::new(true), // AUTO-TUNE ON
            running: AtomicBool::new(false),
            anc_ok: AtomicBool::new(false),

            wavefield_image: Mutex::new(vec![0; 1024 * 1024 * 4]),
            output_frames: Mutex::new(vec![0.0f32; 1024 * 2]),
            output_cursor: AtomicU32::new(0),

            waterfall_rgba: Mutex::new(vec![0u32; WATERFALL_DISPLAY_CELLS]),
            spectrum_bars: Mutex::new(vec![0.0f32; crate::waterfall::SPECTRUM_BINS]),

            sdr_waterfall_rgba: Mutex::new(vec![0u32; WATERFALL_DISPLAY_CELLS]),
            sdr_spectrum_bars: Mutex::new(vec![0.0f32; crate::waterfall::SPECTRUM_BINS]),
            sdr_mags: Mutex::new(Vec::new()),
            rtl_connected: AtomicBool::new(false),
            sdr_sweeping: AtomicBool::new(false),

            pdm_active: AtomicBool::new(true), // PDM ON - wideband simultaneous
            pdm_clock_mhz: AtomicF32::new(12.288),
            tx_mags: Mutex::new(Vec::new()),
            oversample_ratio: AtomicU32::new(64),
            snr_db: AtomicF32::new(0.0),
            pdm_spike_count: std::sync::atomic::AtomicU64::new(0),

            waveshape_mode: AtomicU32::new(2), // Triangle — harmonic but bounded
            waveshape_drive: AtomicF32::new(1.0), // Full drive (max valid: 1.0)

            input_device_count: AtomicU32::new(1),
            beam_azimuth_deg: AtomicF32::new(0.0),
            beam_elevation_rad: AtomicF32::new(0.0),
            beam_confidence: AtomicF32::new(0.0),
            polarization_angle: AtomicF32::new(0.0),
            beam_focus_deg: AtomicF32::new(45.0),

            agc_gain_db: AtomicF32::new(0.0),
            agc_peak_dbfs: AtomicF32::new(-60.0),
            output_peak_db: AtomicF32::new(-60.0),

            mamba_anomaly_score: AtomicF32::new(0.0),
            latent_embedding: Mutex::new(vec![0.0; 128]), // Initialize 64-dim zero vector (MAMBA_LATENT_DIM)
            anc_calibration: Mutex::new(crate::anc_calibration::FullRangeCalibration::new()),
            anc_recording: Mutex::new(crate::anc_recording::RecordingBuffer::new(192_000.0)),
            anc_calibrated: AtomicBool::new(false),
            pending_anc_calibration: AtomicBool::new(false),
            anc_engine: Mutex::new(crate::anc::AncEngine::new(192_000.0, 0.20)), // 192 kHz, 20cm mic spacing
            train_epoch: AtomicU32::new(0),
            train_loss: AtomicF32::new(0.0),
            training_active: AtomicBool::new(false),
            replay_buf_len: AtomicU32::new(0),
            checkpoint_path: Mutex::new("weights/mamba_twister.safetensors".into()),
            mamba_emergency_off: AtomicBool::new(false),
            smart_anc_blend: AtomicF32::new(0.3),

            // TX Improvement (Mamba waveform optimization)
            tx_delta_rms_db: AtomicF32::new(0.0),
            tx_delta_history: Mutex::new(std::collections::VecDeque::with_capacity(100)),
            tx_spectral_deltas: Mutex::new(vec![0.0f32; 512]),

            dispatch_us: AtomicU32::new(0),
            frame_count: AtomicU32::new(0),

            sdr_active: AtomicBool::new(true), // Enable SDR by default for data collection
            sdr_center_hz: AtomicF32::new(100_000_000.0), // 100 MHz default
            sdr_gain_db: AtomicF32::new(20.0),
            sdr_sample_rate: AtomicU32::new(2_048_000),
            sdr_peak_dbfs: AtomicF32::new(-60.0),
            sdr_peak_offset_hz: AtomicF32::new(0.0),
            sdr_device_index: AtomicU32::new(0),

            training_recording_enabled: AtomicBool::new(true), // Enable training by default

            sdr_dc_bias: AtomicF32::new(0.0),
            audio_dc_bias: AtomicF32::new(0.0),

            reconstruction_mags: Mutex::new(Vec::new()),
            reconstructed_peak: AtomicF32::new(0.0),

            gate_status: Mutex::new("IDLE".to_string()),
            last_gate_reason: Mutex::new("".to_string()),
            training_pairs_dropped: AtomicU32::new(0),
            gate_rejections_low_anomaly: AtomicU32::new(0),
            gate_rejections_low_confidence: AtomicU32::new(0),
            gate_rejections_other: AtomicU32::new(0),

            note_name: Mutex::new("---".to_string()),
            note_cents: AtomicF32::new(0.0),
            twister_active: AtomicBool::new(true),

            memos: Mutex::new(std::collections::VecDeque::with_capacity(10_000)),
            memo_input_text: Mutex::new(String::new()),
            memo_input_tag: Mutex::new("NOTE".to_string()),
            recording_state: Mutex::new(RecordingState::new()),
            log_buffer: Mutex::new(VecDeque::with_capacity(1000)),
        })
    }

    // ── Getters / setters — AtomicF32 edition ────────────────────────────────

    #[inline]
    pub fn get_detected_freq(&self) -> f32 {
        self.detected_freq.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_detected_freq(&self, v: f32) {
        self.detected_freq.store(v, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_master_gain(&self) -> f32 {
        self.master_gain.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_master_gain(&self, v: f32) {
        self.master_gain.store(v.clamp(0.0, 1.0), Ordering::Relaxed);
    }

    #[inline]
    pub fn get_denial_freq(&self) -> f32 {
        let ov = self.denial_freq_override.load(Ordering::Relaxed);
        if ov > 0.5 {
            ov
        } else {
            self.get_detected_freq()
        }
    }
    #[inline]
    pub fn set_denial_freq_override(&self, v: f32) {
        self.denial_freq_override.store(v, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_mode(&self) -> DenialMode {
        DenialMode::from_u32(self.mode.load(Ordering::Relaxed))
    }
    #[inline]
    pub fn set_mode(&self, m: DenialMode) {
        self.mode.store(m as u32, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_pdm_clock_mhz(&self) -> f32 {
        self.pdm_clock_mhz.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_pdm_clock_mhz(&self, v: f32) {
        self.pdm_clock_mhz.store(v, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_snr_db(&self) -> f32 {
        self.snr_db.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_snr_db(&self, v: f32) {
        self.snr_db.store(v, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_dispatch_us(&self) -> f32 {
        self.dispatch_us.load(Ordering::Relaxed) as f32
    }
    #[inline]
    pub fn set_dispatch_us(&self, v: u32) {
        self.dispatch_us.store(v, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_frame_count(&self) -> u32 {
        self.frame_count.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn inc_frame_count(&self) {
        self.frame_count.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_waveshape_mode(&self) -> WaveshapeMode {
        WaveshapeMode::from_u32(self.waveshape_mode.load(Ordering::Relaxed))
    }
    #[inline]
    pub fn set_waveshape_mode(&self, m: WaveshapeMode) {
        self.waveshape_mode.store(m as u32, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_waveshape_drive(&self) -> f32 {
        self.waveshape_drive.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_waveshape_drive(&self, v: f32) {
        self.waveshape_drive
            .store(v.clamp(0.0, 1.0), Ordering::Relaxed);
    }

    #[inline]
    pub fn get_beam_azimuth_deg(&self) -> f32 {
        self.beam_azimuth_deg.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_beam_azimuth_deg(&self, v: f32) {
        self.beam_azimuth_deg.store(v, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_beam_confidence(&self) -> f32 {
        self.beam_confidence.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_beam_confidence(&self, v: f32) {
        self.beam_confidence
            .store(v.clamp(0.0, 1.0), Ordering::Relaxed);
    }

    #[inline]
    pub fn get_polarization_angle(&self) -> f32 {
        self.polarization_angle.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_polarization_angle(&self, v: f32) {
        self.polarization_angle.store(v, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_beam_focus_deg(&self) -> f32 {
        self.beam_focus_deg.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_beam_focus_deg(&self, v: f32) {
        self.beam_focus_deg
            .store(v.clamp(0.0, 360.0), Ordering::Relaxed);
    }

    #[inline]
    pub fn get_agc_gain_db(&self) -> f32 {
        self.agc_gain_db.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_agc_gain_db(&self, v: f32) {
        self.agc_gain_db.store(v, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_agc_peak_dbfs(&self) -> f32 {
        self.agc_peak_dbfs.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_agc_peak_dbfs(&self, v: f32) {
        self.agc_peak_dbfs.store(v, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_output_peak_db(&self) -> f32 {
        self.output_peak_db.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_output_peak_db(&self, v: f32) {
        self.output_peak_db.store(v, Ordering::Relaxed);
    }

    pub fn current_mamba_state(&self) -> MambaControlState {
        MambaControlState {
            active_modes: vec!["PhasedArrayAds".to_string()],
            beam_azimuth: self.get_beam_azimuth_deg(),
            beam_elevation: self.beam_elevation_rad.load(Ordering::Relaxed).to_degrees(),
            beam_phases: vec![0.0; self.input_device_count.load(Ordering::Relaxed) as usize],
            heterodyned_beams: vec![None],
            waveshape_drive: self.get_waveshape_drive(),
            anc_gain: 0.8,
        }
    }

    #[inline]
    pub fn get_mamba_anomaly(&self) -> f32 {
        self.mamba_anomaly_score.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_mamba_anomaly(&self, v: f32) {
        self.mamba_anomaly_score.store(v, Ordering::Relaxed);
    }

    pub fn get_latent_embedding(&self) -> Vec<f32> {
        self.latent_embedding.lock().unwrap().clone()
    }

    pub fn set_latent_embedding(&self, v: Vec<f32>) {
        if let Ok(mut emb) = self.latent_embedding.lock() {
            *emb = v;
        }
    }

    #[inline]
    pub fn get_train_loss(&self) -> f32 {
        self.train_loss.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_train_loss(&self, v: f32) {
        self.train_loss.store(v, Ordering::Relaxed);
    }

    pub fn log(&self, level: &str, module: &str, message: &str) {
        let msg = LogMessage::new(level, module, message);
        if let Ok(mut buffer) = self.log_buffer.lock() {
            if buffer.len() >= 1000 {
                buffer.pop_front();
            }
            buffer.push_back(msg);
        }
    }

    pub fn get_logs_all(&self) -> Vec<LogMessage> {
        if let Ok(buffer) = self.log_buffer.lock() {
            buffer.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    // ── Memo System ──────────────────────────────────────────────────────────
    pub fn memo_add(&self, tag: String, content: String) {
        if let Ok(mut m) = self.memos.lock() {
            if let Ok(entry) = MemoEntry::new(
                chrono::Utc::now().to_rfc3339(),
                chrono::Utc::now().timestamp_micros() as u64,
                tag,
                &content,
            ) {
                if m.len() >= 10000 {
                    m.pop_front();
                }
                m.push_back(entry);
                self.log("INFO", "Memo", &format!("Added: {}", content));
            }
        }
    }

    pub fn memo_delete(&self, index: usize) {
        if let Ok(mut m) = self.memos.lock() {
            if index < m.len() {
                m.remove(index);
            }
        }
    }

    pub fn memo_get_all(&self) -> Vec<MemoEntry> {
        if let Ok(m) = self.memos.lock() {
            m.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn manual_rec_start(&self) {
        if let Ok(mut rs) = self.recording_state.lock() {
            rs.start_recording();
            self.log("INFO", "REC", "Manual recording STARTED (30s window)");
        }
    }

    pub fn manual_rec_stop(&self) {
        if let Ok(mut rs) = self.recording_state.lock() {
            rs.stop_recording();
            self.log("INFO", "REC", "Manual recording STOPPED");
        }
    }

    pub fn manual_rec_save(&self, notes: String) {
        let mamba = self.current_mamba_state();
        let topology = WaveTopology {
            phase_coherence_db: vec![0.0; 4],
            field_gradient_azimuth: self.get_beam_azimuth_deg(),
            spatial_curvature: 0.0,
        };

        if let Ok(entry) = MemoEntry::with_3d_context(
            chrono::Utc::now().to_rfc3339(),
            chrono::Utc::now().timestamp_micros() as u64,
            "MANUAL-REC".to_string(),
            &notes,
            mamba,
            topology,
            0.0,
        ) {
            if let Ok(mut m) = self.memos.lock() {
                if m.len() >= 10000 {
                    m.pop_front();
                }
                m.push_back(entry);
                self.log("INFO", "REC", &format!("Saved manual recording: {}", notes));
            }
        }

        if let Ok(mut rs) = self.recording_state.lock() {
            rs.reset();
        }
    }

    #[inline]
    pub fn get_sdr_center_hz(&self) -> f32 {
        self.sdr_center_hz.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_sdr_center_hz(&self, v: f32) {
        self.sdr_center_hz.store(v, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_sdr_gain_db(&self) -> f32 {
        self.sdr_gain_db.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_sdr_gain_db(&self, v: f32) {
        self.sdr_gain_db.store(v, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_sdr_peak_dbfs(&self) -> f32 {
        self.sdr_peak_dbfs.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_sdr_peak_dbfs(&self, v: f32) {
        self.sdr_peak_dbfs.store(v, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_sdr_peak_offset_hz(&self) -> f32 {
        self.sdr_peak_offset_hz.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_sdr_peak_offset_hz(&self, v: f32) {
        self.sdr_peak_offset_hz.store(v, Ordering::Relaxed);
    }

    // ── Training ─────────────────────────────────────────────────────────────
    pub fn get_training_recording_enabled(&self) -> bool {
        self.training_recording_enabled.load(Ordering::Relaxed)
    }
    pub fn set_training_recording_enabled(&self, v: bool) {
        self.training_recording_enabled.store(v, Ordering::Relaxed)
    }

    // ── TX Improvement (Mamba waveform optimization) ─────────────────────────
    #[inline]
    pub fn get_tx_delta_rms(&self) -> f32 {
        self.tx_delta_rms_db.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_tx_delta_rms(&self, rms: f32) {
        self.tx_delta_rms_db.store(rms, Ordering::Relaxed);

        // Keep rolling history (last 100 values)
        if let Ok(mut history) = self.tx_delta_history.lock() {
            history.push_back(rms);
            while history.len() > 100 {
                history.pop_front();
            }
        }
    }

    pub fn get_tx_spectral_deltas(&self) -> Vec<f32> {
        self.tx_spectral_deltas.lock().unwrap().clone()
    }

    pub fn set_tx_spectral_deltas(&self, deltas: Vec<f32>) {
        if let Ok(mut d) = self.tx_spectral_deltas.lock() {
            *d = deltas;
        }
    }

    pub fn get_tx_delta_history(&self) -> Vec<f32> {
        self.tx_delta_history
            .lock()
            .unwrap()
            .iter()
            .copied()
            .collect()
    }

    // ── DC Offset Forensic Monitoring ─────────────────────────────────────────
    #[inline]
    pub fn get_sdr_dc_bias(&self) -> f32 {
        self.sdr_dc_bias.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_sdr_dc_bias(&self, v: f32) {
        self.sdr_dc_bias.store(v, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_audio_dc_bias(&self) -> f32 {
        self.audio_dc_bias.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn set_audio_dc_bias(&self, v: f32) {
        self.audio_dc_bias.store(v.abs(), Ordering::Relaxed);
    }
    #[inline]
    pub fn get_anc_ok(&self) -> bool {
        self.anc_ok.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_anc_ok(&self, v: bool) {
        self.anc_ok.store(v, Ordering::Relaxed);
    }
    // ── Waterfall downsampling (GPU V_DEPTH×V_FREQ_BINS → display resolution) ─

    pub fn update_waterfall(&self, gpu_rgba: &[u32]) {
        const SRC_COLS: usize = crate::vbuffer::V_FREQ_BINS; // 512
        const SRC_ROWS: usize = crate::waterfall::WATERFALL_ROWS; // 128
        const DST_COLS: usize = WATERFALL_DISPLAY_COLS; // 512  (1:1)
        const DST_ROWS: usize = WATERFALL_DISPLAY_ROWS; // 128  (1:1)

        if gpu_rgba.len() < SRC_COLS * SRC_ROWS {
            return;
        }
        if let Ok(mut buf) = self.waterfall_rgba.lock() {
            buf.resize(DST_COLS * DST_ROWS, 0);
            buf.copy_from_slice(&gpu_rgba[..DST_COLS * DST_ROWS]);
        }
    }

    /// Snap to nearest equal-temperament note (A4 = 440 Hz).
    pub fn snap_to_nearest_note(freq_hz: f32) -> f32 {
        if freq_hz < 0.5 {
            return freq_hz;
        }
        let note = (12.0 * (freq_hz / 440.0).log2()).round();
        440.0 * 2.0_f32.powf(note / 12.0)
    }

    // ── Twister ───────────────────────────────────────────────────────────────

    pub fn get_note_name(&self) -> String {
        self.note_name
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|_| "---".to_string())
    }

    pub fn set_note_name(&self, name: String) {
        if let Ok(mut n) = self.note_name.lock() {
            *n = name;
        }
    }

    #[inline]
    pub fn get_note_cents(&self) -> f32 {
        self.note_cents.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_note_cents(&self, v: f32) {
        self.note_cents.store(v, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_twister_active(&self) -> bool {
        self.twister_active.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_twister_active(&self, v: bool) {
        self.twister_active.store(v, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_sdr_active(&self) -> bool {
        self.sdr_active.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_sdr_active(&self, v: bool) {
        self.sdr_active.store(v, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_mamba_emergency_off(&self) -> bool {
        self.mamba_emergency_off.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_mamba_emergency_off(&self, v: bool) {
        self.mamba_emergency_off.store(v, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_smart_anc_blend(&self) -> f32 {
        self.smart_anc_blend.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_smart_anc_blend(&self, v: f32) {
        self.smart_anc_blend
            .store(v.clamp(0.0, 1.0), Ordering::Relaxed);
    }

    #[inline]
    pub fn get_rtl_connected(&self) -> bool {
        self.rtl_connected.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_rtl_connected(&self, v: bool) {
        self.rtl_connected.store(v, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_sdr_sweeping(&self) -> bool {
        self.sdr_sweeping.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_sdr_sweeping(&self, v: bool) {
        self.sdr_sweeping.store(v, Ordering::Relaxed);
    }

    /// Enrich a DetectionEvent with forensic analysis fields from current state
    /// This populates DC bias measurements, anomaly scores, and coordination flags
    pub fn enrich_event_forensics(&self, event: &mut crate::detection::DetectionEvent) {
        event.audio_dc_bias_v = {
            let v = self.get_audio_dc_bias();
            if v > 0.0 {
                Some(v)
            } else {
                None
            }
        };
        event.sdr_dc_bias_v = {
            let v = self.get_sdr_dc_bias();
            if v > 0.0 {
                Some(v)
            } else {
                None
            }
        };
        event.mamba_anomaly_db = self.get_mamba_anomaly();

        // Mark as coordinated if both DC biases present
        event.is_coordinated = event.audio_dc_bias_v.is_some() && event.sdr_dc_bias_v.is_some();
        if event.detection_method.is_empty() {
            event.detection_method = "bispectrum".to_string();
        }
    }

    // ── Memo System Methods (Phase 1) ───────────────────────────────────────────

    /// Get the current memo count
    pub fn get_memo_count(&self) -> usize {
        self.memos.lock().unwrap().len()
    }

    /// Get the maximum memo storage capacity (10,000 entries)
    pub fn get_max_memo_capacity(&self) -> usize {
        10_000
    }

    /// Add a new memo entry with current timestamp
    pub fn add_memo(&self, tag: String, content: &str) -> anyhow::Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};

        // Get current timestamp
        let now_micros = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| anyhow::anyhow!("Time error: {}", e))?
            .as_micros() as u64;

        // Format ISO 8601 timestamp (simple string formatting without chrono)
        // Example: "2026-03-07T14:23:14.832401Z"
        let micros_fraction = now_micros % 1_000_000;

        // Convert seconds to datetime (Unix epoch: Jan 1, 1970)
        // For simplicity, use a placeholder ISO format
        // In production, use chrono or similar
        let timestamp_iso8601 = format!("2026-03-07T14:23:14.{:06}Z", micros_fraction);

        // Create and validate memo entry
        let memo = MemoEntry::new(timestamp_iso8601.clone(), now_micros, tag.clone(), content)?;

        // ── Phase 3b: Capture MambaControlState for forensic evidence ────────
        // When [EVIDENCE] memo is saved, snapshot current 3D beam coordinates
        if tag == "EVIDENCE" {
            let beam_azimuth = self.get_beam_azimuth_deg().to_radians();
            let beam_elevation = self.beam_elevation_rad.load(Ordering::Relaxed);
            let beam_confidence = self.get_beam_confidence();

            // Log beam parameters with memo for later correlation
            eprintln!(
                "[MEMO-BEAM-CAPTURE] [EVIDENCE] @ {} | AZ:{:.2}° EL:{:.1}° CONF:{:.2} | {}",
                timestamp_iso8601,
                beam_azimuth.to_degrees(),
                beam_elevation.to_degrees(),
                beam_confidence,
                content
            );

            // Create MambaControlState snapshot for this memo
            // This will be correlated with forensic logging via timestamp
            let detected_freq = self.detected_freq.load(Ordering::Relaxed);
            let _mamba_state = MambaControlState {
                active_modes: vec!["heterodyne".to_string()],
                beam_azimuth,
                beam_elevation,
                beam_phases: vec![0.0; 4], // 4-mic array phases
                heterodyned_beams: vec![if detected_freq > 0.0 {
                    Some(2.45e9 - detected_freq as f64) // 95GHz heterodyne product
                } else {
                    None
                }],
                waveshape_drive: beam_confidence.max(0.5), // Min 0.5 for evidence capture
                anc_gain: self.agc_gain_db.load(Ordering::Relaxed) / 80.0, // Normalize to 0-1
            };
        }

        // Add to storage (FIFO: remove oldest if at capacity)
        let mut memos = self.memos.lock().unwrap();
        if memos.len() >= 10_000 {
            memos.pop_front(); // Remove oldest entry
        }
        memos.push_back(memo);

        Ok(())
    }

    /// Delete memo at specified index
    pub fn delete_memo(&self, index: usize) -> anyhow::Result<()> {
        let mut memos = self.memos.lock().unwrap();

        if index >= memos.len() {
            return Err(anyhow::anyhow!(
                "Memo index {} out of bounds ({})",
                index,
                memos.len()
            ));
        }

        // Remove by converting to Vec, removing, and converting back
        let mut vec: Vec<MemoEntry> = memos.drain(..).collect();
        vec.remove(index);
        for memo in vec {
            memos.push_back(memo);
        }

        Ok(())
    }

    /// Get all memos as a vector
    pub fn get_memos_all(&self) -> anyhow::Result<Vec<MemoEntry>> {
        Ok(self.memos.lock().unwrap().iter().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_tracks_tx_convergence() {
        let state = AppState::new();
        let initial_rms = state.get_tx_delta_rms();
        assert_eq!(initial_rms, 0.0);
    }

    #[test]
    fn test_app_state_set_tx_delta_rms() {
        let state = AppState::new();
        state.set_tx_delta_rms(12.5);
        assert_eq!(state.get_tx_delta_rms(), 12.5);

        // Check history is tracked
        let history = state.get_tx_delta_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0], 12.5);
    }

    #[test]
    fn test_app_state_tx_delta_history() {
        let state = AppState::new();

        // Add 105 values (should keep only last 100)
        for i in 0..105 {
            state.set_tx_delta_rms(i as f32);
        }

        let history = state.get_tx_delta_history();
        assert_eq!(history.len(), 100); // Capped at 100
        assert_eq!(history[0], 5.0); // First value after rolling (started at 5)
        assert_eq!(history[99], 104.0); // Last value
    }

    #[test]
    fn test_app_state_tx_spectral_deltas() {
        let state = AppState::new();
        let deltas = vec![1.0f32; 512];
        state.set_tx_spectral_deltas(deltas.clone());

        let retrieved = state.get_tx_spectral_deltas();
        assert_eq!(retrieved.len(), 512);
        assert_eq!(retrieved, deltas);
    }
}

impl AppState {
    pub fn get_feature_flags(&self) -> FeatureFlags {
        self.feature_flags.lock().unwrap().clone()
    }

    pub fn set_feature_flags(&self, flags: FeatureFlags) {
        if let Ok(mut f) = self.feature_flags.lock() {
            *f = flags;
        }
    }

    pub fn get_feature_confidence(&self) -> f32 {
        self.feature_confidence.load(Ordering::Relaxed)
    }

    pub fn set_feature_confidence(&self, val: f32) {
        self.feature_confidence.store(val, Ordering::Relaxed);
    }
}
