// src/visualization/data_contracts.rs
use bytemuck::{Pod, Zeroable};

#[derive(Clone, Debug, Default)]
pub struct PoseFrame {
    pub keypoints: Vec<(f32, f32, f32)>, // 33 keypoints
}

#[derive(Clone, Debug)]
pub struct RoomGeometry {
    pub min_bound: (f32, f32, f32),
    pub max_bound: (f32, f32, f32),
}

impl Default for RoomGeometry {
    fn default() -> Self {
        Self {
            min_bound: (-5.0, 0.0, -5.0),
            max_bound: (5.0, 3.0, 5.0),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct VoxelGridData {
    pub energy: f32,
    pub phase_coherence: f32, // Gamma value
    pub padding: [f32; 2],
}

pub struct VoxelGrid {
    pub dimensions: [u32; 3],
    pub data: Vec<VoxelGridData>,
}

impl VoxelGrid {
    pub fn new(dim_x: u32, dim_y: u32, dim_z: u32) -> Self {
        Self {
            dimensions: [dim_x, dim_y, dim_z],
            data: vec![VoxelGridData { energy: 0.0, phase_coherence: 0.0, padding: [0.0; 2] }; (dim_x * dim_y * dim_z) as usize],
        }
    }

    pub fn get(&self, x: u32, y: u32, z: u32) -> VoxelGridData {
        if x < self.dimensions[0] && y < self.dimensions[1] && z < self.dimensions[2] {
            let index = x + y * self.dimensions[0] + z * self.dimensions[0] * self.dimensions[1];
            self.data[index as usize]
        } else {
            VoxelGridData { energy: 0.0, phase_coherence: 0.0, padding: [0.0; 2] }
        }
    }

    pub fn sample(&self, pos: (f32, f32, f32)) -> f32 {
        let x = (pos.0.max(0.0).min((self.dimensions[0] - 1) as f32)) as u32;
        let y = (pos.1.max(0.0).min((self.dimensions[1] - 1) as f32)) as u32;
        let z = (pos.2.max(0.0).min((self.dimensions[2] - 1) as f32)) as u32;
        self.get(x, y, z).energy
    }

    pub fn iter_voxels(&self) -> impl Iterator<Item = (u32, u32, u32)> + '_ {
        (0..self.dimensions[0]).flat_map(move |x| {
            (0..self.dimensions[1]).flat_map(move |y| {
                (0..self.dimensions[2]).map(move |z| (x, y, z))
            })
        })
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ParticleGPU {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub color: [f32; 4],
    pub life: f32,
}
