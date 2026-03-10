use bytemuck::{Pod, Zeroable};

/// Universal FieldParticle Struct
/// Represents a single unified particle for any signal type (Audio, RF, Video)
/// Total Size: 32 bytes (6 * 4 bytes data + 3 * 4 bytes padding = 36? Wait, 3*4=12. 3 f32=12. phase_i=4, phase_q=4, energy=4, material_id=4. Total 32 bytes! Wait, 12+4+4+4+4=28. Wait, 3 padding u32s = 12 bytes. 28 + 12 = 40 bytes)
/// GPU Buffer Layout: std140 with proper alignment for compute shaders
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct FieldParticle {
    /// Spatial coordinate in the Chronos Slate
    pub position: [f32; 3],

    /// In-phase component
    pub phase_i: f32,

    /// Quadrature component
    pub phase_q: f32,

    /// Instantaneous power/magnitude
    pub energy: f32,

    /// Mamba latent cluster motif (e.g., 60Hz vs 750THz)
    pub material_id: u32,

    /// 16-byte alignment for GPU buffers
    pub _padding: [u32; 3],
}

// WGSL Equivalent (add as a comment for the GPU shader pipeline):
// struct FieldParticle {
//     position: vec3<f32>,
//     phase_i: f32,
//     phase_q: f32,
//     energy: f32,
//     material_id: u32,
//     _padding: vec3<u32>,
// };
