pub mod voxel_grid;
pub mod material_absorption;
pub mod heterodyne_mixer;
pub mod body_interaction_model;
pub mod rf_propagation;

use std::error::Error;
use num_complex::Complex;
use crate::resonance::voxel_grid::VoxelGrid;
use crate::resonance::material_absorption::MaterialGrid;
use crate::resonance::rf_propagation::RFWavePropagation;
use crate::visualization::data_contracts::{PoseFrame, RoomGeometry};

pub struct EnergyDensityField {
    pub magnitude_grid: VoxelGrid<f32>,     // |E| per voxel
    pub phase_grid: VoxelGrid<f32>,         // ∠E per voxel
}

pub fn solve_rf_field(
    primary_freq: f32,
    source_pos: (f32, f32, f32),
    body_pose: &PoseFrame,
    room_geometry: &RoomGeometry,
) -> Result<VoxelGrid<Complex<f32>>, Box<dyn Error>> {
    let grid_size = 64; // Base voxel resolution
    let voxel_size_m = 0.1; // 10cm voxels

    let mut rf_sim = RFWavePropagation::new(grid_size, primary_freq);
    rf_sim.grid.voxel_size_m = voxel_size_m;

    let material_grid = MaterialGrid::from_room_geometry(room_geometry, primary_freq, voxel_size_m);
    let mut modified_materials = material_grid.grid.clone();

    // Voxelize human body into the material grid
    let human = crate::resonance::body_interaction_model::HumanBody::from_pose(body_pose, grid_size, voxel_size_m);
    for x in 0..modified_materials.dimensions.0 {
        for y in 0..modified_materials.dimensions.1 {
            for z in 0..modified_materials.dimensions.2 {
                if human.voxel_map.get(x, y, z) > 0.5 {
                    // Update material to lossy human tissue
                    let mut tissue = crate::resonance::material_absorption::Material::default();
                    tissue.name = "Human Tissue".to_string();
                    tissue.permittivity = 50.0;
                    tissue.conductivity = 1.0;
                    modified_materials.set(x, y, z, tissue);
                }
            }
        }
    }

    rf_sim.solve_wave_equation(source_pos, 1.0, &modified_materials)?;

    Ok(rf_sim.grid)
}
