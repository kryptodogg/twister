use bytemuck::{Pod, Zeroable};

/// Universal FieldParticle Struct
/// Represents a single unified particle for any signal type (Audio, RF, Video)
/// Total Size: 40 bytes (9 × f32 + 1 × u32 = 37 bytes, padded to 40)
/// GPU Buffer Layout: std140 with proper alignment for compute shaders
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct FieldParticle {
    /// Position in Chronos Slate (sensor-agnostic coordinate space)
    pub position: [f32; 3],     // [x, y, z]

    /// Phase information (In-Phase / Quadrature)
    /// For audio: q is 0.0 or Hilbert transform result
    /// For RF: native IQ from SDR receiver
    /// For video: color channels as phase proxies
    pub phase_i: f32,           // In-phase component
    pub phase_q: f32,           // Quadrature component

    /// Energy/magnitude (normalized 0-1)
    pub energy: f32,            // Power density

    /// Material identifier (latent cluster mapping)
    /// Allows Mamba to learn that 60Hz audio and 2.4GHz RF follow similar physics
    /// (e.g., water absorption, body coupling, material permittivity effects)
    pub material_id: u32,       // Bitflags or cluster ID

    /// Spatial derivative (energy gradient for dynamics)
    pub energy_gradient: f32,   // ∇|E|² for advection
}

// WGSL equivalent (for GPU shaders):
// struct FieldParticle {
//     position: vec3<f32>,
//     phase_i: f32,
//     phase_q: f32,
//     energy: f32,
//     material_id: u32,
//     energy_gradient: f32,
// }
