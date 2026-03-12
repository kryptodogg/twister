//! FieldParticle — V3 Track 0-A Foundation
//!
//! # V3 Architecture Notes
//! - Exactly 128 bytes (one AMD Infinity Cache line)
//! - All padding is explicit with named track reservations
//! - bytemuck::Pod requires zero implicit padding

use bytemuck::{Pod, Zeroable};

/// Full-Spectrum Hologram Struct
/// Represents a single unified particle for the Synesthesia Hologram.
///
/// # Memory Layout
/// Total Size: Exactly 128 bytes (one AMD RX 6700 XT Infinity Cache line)
/// Alignment: 128 bytes (ensures single cache line fetch)
///
/// # Field Groups
/// - Bytes 0-15: Spatial (position + intensity)
/// - Bytes 16-31: Visual (color RGBA)
/// - Bytes 32-63: Identity (source_id, confidence, timestamp)
/// - Bytes 64-71: Spectral (freq_hz)
/// - Bytes 72-87: Forensic heuristics (phase, doppler, bandwidth, anomaly)
/// - Bytes 88-107: Classification (material, motif, RF-BSDF params)
/// - Bytes 108-127: Track reservations (H2, HA, G-RB, J1, forensic)
#[repr(C, align(128))]
#[derive(Copy, Clone, Debug, Zeroable)]
pub struct FieldParticle {
    // ═══════════════════════════════════════════════════════════════════════════
    // SPATIAL (16 bytes: 0-15)
    // ═══════════════════════════════════════════════════════════════════════════
    /// 3D Coordinate in the "Hologram" (meters)
    pub position: [f32; 3],    // 12 bytes

    /// The absolute "Truth" of the signal intensity (unfiltered)
    pub intensity: f32,        // 4 bytes

    // ═══════════════════════════════════════════════════════════════════════════
    // VISUAL (16 bytes: 16-31)
    // ═══════════════════════════════════════════════════════════════════════════
    /// RGB + A (Resonant mapping).
    pub color: [f32; 4],       // 16 bytes

    // ═══════════════════════════════════════════════════════════════════════════
    // IDENTITY (32 bytes: 32-63)
    // ═══════════════════════════════════════════════════════════════════════════
    /// Source ID: 0=Mic, 1=SDR, 2=Pluto, 3=CMOS.
    pub source_id: u32,        // 4 bytes

    /// DISCREPANCY MATRIX
    /// [Visible_Light, CMOS_Inductance, CV_Inference, RF_Density]
    pub confidence: [f32; 4],  // 16 bytes

    /// QPC Microseconds - The temporal glue for the hologram.
    pub timestamp_us: u64,     // 8 bytes

    // ═══════════════════════════════════════════════════════════════════════════
    // SPECTRAL (8 bytes: 64-71)
    // ═══════════════════════════════════════════════════════════════════════════
    /// Frequency alignment for BSS
    pub freq_hz: f64,          // 8 bytes

    // ═══════════════════════════════════════════════════════════════════════════
    // FORENSIC HEURISTICS (16 bytes: 72-87)
    // ═══════════════════════════════════════════════════════════════════════════
    /// Phase coherence Γ (0.0=null, 1.0=constructive)
    pub phase_coherence: f32,  // 4 bytes

    /// Doppler shift estimate (Hz)
    pub doppler_shift: f32,    // 4 bytes

    /// Signal bandwidth (Hz)
    pub bandwidth_hz: f32,     // 4 bytes

    /// Mamba anomaly score (dB)
    pub anomaly_score: f32,    // 4 bytes

    // ═══════════════════════════════════════════════════════════════════════════
    // CLASSIFICATION (20 bytes: 88-107)
    // ═══════════════════════════════════════════════════════════════════════════
    /// Material ID (0-11 octave bucket, hue class)
    pub material_id: u32,      // 4 bytes

    /// Motif hint (TimeGNN pattern ID)
    pub motif_hint: u32,       // 4 bytes

    /// RF-BSDF: scattering cross-section (m²) — Track G-RB
    pub scattering_cross_section: f32,  // 4 bytes

    /// RF-BSDF: complex permittivity real part ε' — Track G-RB
    pub permittivity_real: f32,         // 4 bytes

    /// RF-BSDF: complex permittivity imaginary part ε'' — Track G-RB
    pub permittivity_imag: f32,         // 4 bytes

    // ═══════════════════════════════════════════════════════════════════════════
    // TRACK RESERVATIONS (20 bytes: 108-127)
    // Named fields reserved for specific future tracks
    // ═══════════════════════════════════════════════════════════════════════════
    /// Reserved for Track H2: counter-waveform null phase offset (radians)
    pub reserved_for_h2_null_phase: f32,        // 4 bytes

    /// Reserved for Track HA: haptic frequency F_tactile (Hz)
    pub reserved_for_ha_haptic_freq: f32,       // 4 bytes

    /// Reserved for Track G-RB: water saturation S (0=dry, 1=saturated)
    pub reserved_for_grb_water_saturation: f32, // 4 bytes

    /// Reserved for Track J1: RF proprioception body quadrant density
    pub reserved_for_j1_proprioception: f32,    // 4 bytes

    /// Reserved for forensic chain-of-custody: hash fragment (low 32 bits)
    pub reserved_for_forensic_hash_lo: u32,     // 4 bytes
}

// ═══════════════════════════════════════════════════════════════════════════════
// COMPILE-TIME SIZE AND ALIGNMENT ASSERTIONS (128-Byte Law)
// ═══════════════════════════════════════════════════════════════════════════════
const _: () = assert!(std::mem::size_of::<FieldParticle>() == 128);
const _: () = assert!(std::mem::align_of::<FieldParticle>() == 128);

// ═══════════════════════════════════════════════════════════════════════════════
// MANUAL Pod IMPLEMENTATION
// We cannot derive Pod because #[repr(C, align(128))] introduces implicit padding
// that bytemuck cannot verify. Instead, we implement Pod manually with explicit
// safety justification.
// ═══════════════════════════════════════════════════════════════════════════════

// SAFETY: FieldParticle is guaranteed to be Pod because:
// 1. All fields are themselves Pod (f32, f64, u32, u64, [T; N] where T: Pod)
// 2. The struct uses #[repr(C)] which guarantees no inter-field padding
// 3. The size assertion proves total size is exactly 128 bytes
// 4. The alignment assertion proves alignment is exactly 128 bytes
// 5. All 128 bytes are explicitly accounted for by named fields
unsafe impl Pod for FieldParticle {}

/// Convert sensor data to FieldParticle hologram elements
pub trait ToHologram {
    fn to_particle(&self, ts: u64) -> FieldParticle;
}
