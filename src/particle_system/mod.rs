pub mod frustum_culler;
pub mod renderer;
// pub mod streaming; // Temporarily disabled: Send trait issue with ThreadRng across await boundaries

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleGPU {
    pub position: [f32; 3], // 12 bytes
    pub color: [f32; 4],    // 16 bytes
    pub intensity: f32,     // 4 bytes
    pub hardness: f32,      // 4 bytes
    pub roughness: f32,     // 4 bytes
    pub wetness: f32,       // 4 bytes
} // 44 bytes total

// Assert size is exactly 44 bytes at compile time
const _: () = assert!(std::mem::size_of::<ParticleGPU>() == 44);
