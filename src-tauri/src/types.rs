// === PRE-FLIGHT ===
// Task:           Track 0-A — Core Types
// Files read:     AGENTS.md, ROADMAP.md, README.md, skills/rust-gpu-parallelism/SKILL.md, models/AGENTS.md
// Files in scope: src-tauri/src/types.rs (new), src-tauri/src/lib.rs or main.rs (for mod declaration)
// Acceptance:     `cargo check` clean. Every struct assertion passes. No anonymous `_pad` bytes anywhere.
// Findings:
// - `FieldParticle` is the core GPU-boundary struct, subject to the 128-byte law for RDNA2 Infinity Cache alignment.
// - All bytes must be named, with future-track fields using `reserved_for_` prefixes as a contract.
// - `RawIQPoint` is the 32-byte ingestion struct, carrying raw data and metadata like jitter.
// - `AtomicF32` is a required utility type implemented via `AtomicU32` bit reinterpretation.
// - All GPU-facing structs must derive `bytemuck::Pod` and `bytemuck::Zeroable` for safe, zero-cost casting.
// === END PRE-FLIGHT ===

use std::mem::{align_of, size_of};
use std::sync::atomic::{AtomicU32, Ordering};

/// A single, fused point of evidence in the unified field.
///
/// This struct is the fundamental unit of data that crosses the CPU-GPU boundary
/// for processing and rendering. It is subject to the **128-Byte Law**, ensuring
/// it occupies exactly one RDNA2 Infinity Cache line.
///
/// Every byte is explicitly named, either as an active field or a reservation
/// for a planned future track. There is no anonymous padding.
#[repr(C, align(128))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FieldParticle {
    // --- Geometric Data (40 bytes) ---
    /// Position in 3D space (meters).
    pub position: [f32; 3], // 12 bytes
    /// Covariance matrix for Gaussian splat (6 unique values of symmetric 3x3).
    pub covariance: [f32; 6], // 24 bytes
    /// Opacity for rendering (0.0 to 1.0).
    pub opacity: f32, // 4 bytes

    // --- Color/Visual Data (16 bytes) ---
    /// RGBA color. Hue from frequency, Sat from variance, Val from coherence.
    pub color: [f32; 4], // 16 bytes

    // --- Core Physics & Timestamps (24 bytes) ---
    /// Timestamp in microseconds, slaved to Pico 2 PPS via QPC.
    pub timestamp_us: u64, // 8 bytes
    /// Center frequency in Hz.
    pub frequency_hz: f32, // 4 bytes
    /// Signal energy.
    pub energy: f32, // 4 bytes
    /// Phase coherence (0.0 to 1.0).
    pub phase_coherence: f32, // 4 bytes
    /// Carrier variance (discriminant for synthesized signals).
    pub carrier_variance: f32, // 4 bytes

    // --- Forensic & Inference Data (16 bytes) ---
    /// Anomaly score from UnifiedFieldMamba (0.0 to 1.0).
    pub anomaly_score: f32, // 4 bytes
    /// Bitmask of contributing sensor IDs.
    pub sensor_id_mask: u32, // 4 bytes
    /// First 7 bytes of the SHA-256 hash of the forensic corpus block.
    pub corpus_hash: [u8; 7], // 7 bytes
    /// Jury flags (e.g., unanimous, dissent, which voter dissented).
    pub jury_flags: u8, // 1 byte

    // --- Reserved for Future Tracks (32 bytes) ---
    /// Phase for counter-waveform null synthesis (Track H2).
    pub reserved_for_h2_null_phase: f32, // 4 bytes
    /// Biometric data (e.g., pulse, breath rate) (Track I1).
    pub reserved_for_i1_biometrics: [f32; 2], // 8 bytes
    /// Proprioceptive mapping data (Track I2).
    pub reserved_for_i2_proprioception: f32, // 4 bytes
    /// Equivariant feature hash (Track I3).
    pub reserved_for_i3_equivariant_hash: u64, // 8 bytes
    /// General purpose reservation for future expansion.
    pub reserved_future: [u8; 8], // 8 bytes
}

// Compile-time assertions to enforce the 128-Byte Law.
const _: () = assert!(size_of::<FieldParticle>() == 128);
const _: () = assert!(align_of::<FieldParticle>() == 128);

/// A raw, unprocessed sample from an IQ-based sensor (e.g., RTL-SDR, PlutoSDR).
///
/// This struct is the standard format for all data entering the ingestion pipeline
/// before it is processed by the space-time Laplacian on the GPU. It contains no
/// derived information; FFT and other preprocessing are explicitly forbidden at
/// this stage.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RawIQPoint {
    /// In-phase component.
    pub i: f32, // 4 bytes
    /// Quadrature component.
    pub q: f32, // 4 bytes
    /// Timestamp in microseconds, slaved to Pico 2 PPS.
    pub timestamp_us: u64, // 8 bytes
    /// ID of the source sensor.
    pub sensor_id: u32, // 4 bytes
    /// Jitter in microseconds observed during USB packet reception. This is a feature, not an error.
    pub jitter_us: u16, // 2 bytes
    /// Count of lost packets preceding this one. This is a feature, not an error.
    pub packet_loss_count: u16, // 2 bytes
    /// Reserved for future use (e.g., sequence numbers, flags).
    pub reserved: [u8; 8], // 8 bytes
}

// Compile-time assertion to enforce the 32-byte size.
const _: () = assert!(size_of::<RawIQPoint>() == 32);

/// A particle in the scene-space, ready for rendering.
///
/// This is the primary input to the Gaussian splat renderer. For now, it is a
/// direct mapping from `FieldParticle`, but will evolve in Phase G to include
/// screen-space projections and other render-specific data.
pub type AetherParticle = FieldParticle;

/// The consensus decision from the three independent voters.
///
/// This struct is an intermediate representation on the CPU/GPU before the final
/// `FieldParticle` is formed. It holds the raw outputs from each voter,
/// allowing for the logging of dissent, which is critical forensic data.
#[derive(Debug, Clone, Copy)]
pub struct JuryVerdict {
    pub timestamp_us: u64,
    pub position: [f32; 3],
    pub frequency_hz: f32,
    pub gpu_mamba_score: f32,
    pub coral_mamba_score: f32,
    /// A score from 0.0 to 1.0 based on geometric consistency.
    pub pico_tdoa_confidence: f32,
    /// The divergence signal: `abs(gpu_score - coral_score)`.
    pub divergence: f32,
}

/// An atomic `f32` type, implemented by bit-reinterpreting an `AtomicU32`.
///
/// This is necessary because `std::sync::atomic::AtomicF32` is not yet stable.
/// This implementation correctly preserves NaN bit patterns across atomic
/// operations, which is essential for forensic integrity as NaN can be a
/// valid diagnostic signal.
#[derive(Debug)]
#[repr(transparent)]
pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    /// Creates a new `AtomicF32`.
    pub fn new(v: f32) -> Self {
        Self(AtomicU32::new(v.to_bits()))
    }

    /// Loads a value from the atomic float.
    pub fn load(&self, ord: Ordering) -> f32 {
        f32::from_bits(self.0.load(ord))
    }

    /// Stores a value into the atomic float.
    pub fn store(&self, v: f32, ord: Ordering) {
        self.0.store(v.to_bits(), ord)
    }
}

impl Default for AtomicF32 {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl From<f32> for AtomicF32 {
    fn from(v: f32) -> Self {
        Self::new(v)
    }
}
