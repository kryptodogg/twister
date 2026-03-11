use bytemuck::{Pod, Zeroable};

/// Universal FieldParticle Struct
/// Represents a single unified particle for any signal type (Audio, RF, Video)
/// Total Size: Exactly 128 bytes (one AMD Infinity Cache line)
/// GPU Buffer Layout: std140 with proper alignment for compute shaders
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct FieldParticle {
    /// Hardware-sourced microsecond timestamp
    pub timestamp_us: u64,

    /// Center frequency of observation
    pub freq_hz: f64,

    /// Normalized energy (0.0 - 1.0)
    pub energy: f32,

    /// Phase coherence (Γ), 0.0 (null) to 1.0 (constructive)
    pub phase_coherence: f32,

    /// Spatial estimate in meters
    pub position_xyz: [f32; 3],

    /// Octave-folded harmonic bucket (0-11)
    pub material_id: u8,

    /// Data source identifier:
    /// 0=AudioHost, 1=PlutoOnboard, 2=HostProcessed, 3=Pico
    pub source: u8,

    /// Alignment padding to 4 bytes
    pub _pad0: [u8; 2],

    /// Pre-computed Doppler shift heuristic
    pub doppler_shift: f32,

    /// Pre-computed phase velocity heuristic
    pub phase_velocity: f32,

    /// Pre-computed scattering cross-section heuristic
    pub scattering_cross_section: f32,

    /// Spectral bandwidth of this observation in Hz
    pub bandwidth_hz: f32,

    /// Anomaly score from Coral NF (0.0 if unavailable)
    pub anomaly_score: f32,

    /// Last ESN classification from Pico (255 = unknown)
    pub motif_hint: u8,

    /// Alignment padding
    pub _pad1: [u8; 3],

    /// First 16 dimensions of the 128-D Mamba latent embedding
    /// (Full 128-D lives in GPU; this is the CPU-resident summary)
    pub embedding: [f32; 16],
}

// Forensic constraint: The struct MUST be exactly 128 bytes.
const _: () = assert!(std::mem::size_of::<FieldParticle>() == 128);
