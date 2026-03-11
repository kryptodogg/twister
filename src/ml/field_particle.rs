use bytemuck::{Pod, Zeroable};

/// Full-Spectrum Hologram Struct
/// Represents a single unified particle for the Synesthesia Hologram.
/// Total Size: Exactly 128 bytes (one AMD Infinity Cache line)
/// GPU Buffer Layout: std140 with proper alignment for compute shaders
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct FieldParticle {
    /// 3D Coordinate in the "Hologram" (meters)
    pub position: [f32; 3],

    /// The absolute "Truth" of the signal intensity (unfiltered)
    pub intensity: f32,

    /// RGB + A (Resonant mapping).
    /// Machine learning can correlate 'color' to 'frequency' later.
    pub color: [f32; 4],

    /// Source ID: 0=Mic, 1=SDR, 2=Pluto, 3=CMOS.
    /// This is the "Provenance" tag for forensic playback.
    pub source_id: u32,

    /// DISCREPANCY MATRIX
    /// [Visible_Light, CMOS_Inductance, CV_Inference, RF_Density]
    /// If CV_Inference is high but Visible_Light is zero, the hologram
    /// has captured an invisible "truth" (digital harassment).
    pub confidence: [f32; 4],

    /// QPC Microseconds - The temporal glue for the hologram.
    pub timestamp_us: u64,

    /// Frequency alignment for BSS
    pub freq_hz: f64,

    /// Pre-computed heuristics to hit 128 bytes
    pub phase_coherence: f32,
    pub doppler_shift: f32,
    pub bandwidth_hz: f32,
    pub anomaly_score: f32,
    pub material_id: u32,
    pub motif_hint: u32,
    pub _padding: [f32; 8],
}

// Forensic constraint: The struct MUST be exactly 128 bytes.
const _: () = assert!(std::mem::size_of::<FieldParticle>() == 128);
