// src/ml/rt_blind_rays.rs
// Blind Ray Generation: Spawn particles from FDFD RF propagation field
//
// This module implements the RT-Core inspired particle generation:
// 1. Reads FDFD E-field solution (Complex<f32>)
// 2. Identifies high-energy collision voxels
// 3. Generates FieldParticle data at each collision
// 4. Feeds to HitListAccumulator for batch processing
//
// The "Blind Ray" metaphor: We don't trace rays explicitly;
// instead we sample the solved FDFD field to find where waves are interacting.

use super::data_contracts::FieldParticle;
use super::field_particle::{FieldParticleGPU, HitListAccumulator};
use crate::resonance::rf_propagation::RFWavePropagation;
use crate::resonance::voxel_grid::VoxelGrid;
use crate::resonance::material_absorption::Material;
use std::f32::consts::PI;

/// Configuration for blind ray generation from FDFD solution
#[derive(Clone, Debug)]
pub struct BlindRayConfig {
    /// Energy threshold (0.0-1.0) below which voxels are ignored
    pub energy_threshold: f32,
    /// Maximum particles to generate per frame
    pub max_particles_per_frame: usize,
    /// Phase reference point (usually SDR/source location)
    pub reference_position: [f32; 3],
    /// Frequency in Hz (for phase calculation)
    pub frequency_hz: f32,
}

impl Default for BlindRayConfig {
    fn default() -> Self {
        Self {
            energy_threshold: 0.1,  // Only high-energy voxels
            max_particles_per_frame: 512,  // Batch in groups
            reference_position: [0.0, 1.5, 0.0],  // Eye-level
            frequency_hz: 2.4e9,  // 2.4 GHz (nRF24 band)
        }
    }
}

/// Generates hit list particles from FDFD RF propagation field
pub struct BlindRayGenerator {
    config: BlindRayConfig,
    accumulated_particles: Vec<(FieldParticleGPU, u32, u64)>,
    frame_counter: u32,
}

impl BlindRayGenerator {
    /// Create a new blind ray generator
    pub fn new(config: BlindRayConfig) -> Self {
        let capacity = config.max_particles_per_frame;
        Self {
            config,
            accumulated_particles: Vec::with_capacity(capacity),
            frame_counter: 0,
        }
    }

    /// Generate particles from FDFD solution and material grid
    ///
    /// # Physics
    /// - Energy: |E(x,y,z)|² extracted from FDFD solver
    /// - Phase: arg(E) provides IQ data (I=Re(E), Q=Im(E))
    /// - Material: From material_grid; influences attenuation and phase velocity
    /// - Energy Gradient: ∇|E|² computed via finite differences
    pub fn generate_hit_list(
        &mut self,
        rf_prop: &RFWavePropagation,
        material_grid: &VoxelGrid<Material>,
    ) -> Result<Vec<(FieldParticleGPU, u32, u64)>, Box<dyn std::error::Error>> {
        self.accumulated_particles.clear();

        // Compute max energy for normalization
        let mut max_energy = 0.0f32;

        // Scan FDFD field for high-energy voxels
        let (dim_x, dim_y, dim_z) = rf_prop.grid.dimensions;

        for z in 1..dim_z - 1 {
            for y in 1..dim_y - 1 {
                for x in 1..dim_x - 1 {
                    if self.accumulated_particles.len() >= self.config.max_particles_per_frame {
                        break;
                    }

                    // Get E-field at voxel
                    let e_field = rf_prop.grid.get(x, y, z);
                    let energy = (e_field.norm_sqr()).sqrt();

                    // Track max for normalization
                    if energy > max_energy {
                        max_energy = energy;
                    }

                    // Skip low-energy voxels
                    if energy < self.config.energy_threshold {
                        continue;
                    }

                    // Normalized energy (0.0-1.0)
                    let norm_energy = if max_energy > 0.0 {
                        (energy / max_energy).min(1.0)
                    } else {
                        0.0
                    };

                    // IQ data: I = Re(E), Q = Im(E), normalized
                    let i_component = e_field.re / (max_energy + 1e-9);
                    let q_component = e_field.im / (max_energy + 1e-9);

                    // Voxel center position in world coordinates
                    // grid_to_world: position = (grid_idx * voxel_size) + origin
                    let world_pos = (
                        x as f32 * rf_prop.grid.voxel_size_m + rf_prop.grid.origin.0,
                        y as f32 * rf_prop.grid.voxel_size_m + rf_prop.grid.origin.1,
                        z as f32 * rf_prop.grid.voxel_size_m + rf_prop.grid.origin.2,
                    );

                    // Material properties at this voxel
                    let material = material_grid.get(x, y, z);

                    // Energy gradient: ∇|E|² via finite differences
                    let energy_grad = self.compute_energy_gradient(
                        rf_prop,
                        x,
                        y,
                        z,
                        max_energy,
                    );

                    // Create particle
                    let particle = FieldParticleGPU {
                        position: [world_pos.0, world_pos.1, world_pos.2],
                        phase_amp: [i_component, q_component],
                        material: [material.hardness, material.roughness, material.wetness],
                        energy_gradient: energy_grad,
                        _padding: norm_energy,
                    };

                    // Hilbert curve index (placeholder - will be computed in Phase 3)
                    let hilbert_idx = self.compute_hilbert_index(x as u32, y as u32, z as u32);

                    // Timestamp in microseconds since epoch
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_micros() as u64;

                    self.accumulated_particles.push((particle, hilbert_idx, timestamp));
                }
            }
        }

        self.frame_counter += 1;
        Ok(self.accumulated_particles.clone())
    }

    /// Compute energy gradient magnitude ∇|E|² via finite differences
    fn compute_energy_gradient(
        &self,
        rf_prop: &RFWavePropagation,
        x: usize,
        y: usize,
        z: usize,
        max_energy: f32,
    ) -> f32 {
        let e_center = rf_prop.grid.get(x, y, z);
        let _energy_center = e_center.norm();

        // Central differences for each axis
        let e_xp = rf_prop.grid.get(x + 1, y, z);
        let e_xm = rf_prop.grid.get(x - 1, y, z);
        let grad_x = (e_xp.norm() - e_xm.norm()) / (2.0 * rf_prop.grid.voxel_size_m);

        let e_yp = rf_prop.grid.get(x, y + 1, z);
        let e_ym = rf_prop.grid.get(x, y - 1, z);
        let grad_y = (e_yp.norm() - e_ym.norm()) / (2.0 * rf_prop.grid.voxel_size_m);

        let e_zp = rf_prop.grid.get(x, y, z + 1);
        let e_zm = rf_prop.grid.get(x, y, z - 1);
        let grad_z = (e_zp.norm() - e_zm.norm()) / (2.0 * rf_prop.grid.voxel_size_m);

        // Magnitude of gradient
        let grad_magnitude = (grad_x * grad_x + grad_y * grad_y + grad_z * grad_z).sqrt();

        // Normalize to (0.0-1.0) range
        (grad_magnitude / (max_energy + 1e-9)).min(1.0)
    }

    /// Compute 3D Hilbert curve index (placeholder)
    /// Full implementation in Phase 3
    fn compute_hilbert_index(&self, x: u32, y: u32, z: u32) -> u32 {
        // Simple space-filling curve approximation for now
        // Full 3D Hilbert curve will be implemented in Phase 3
        ((x + y * 256 + z * 256 * 256) % 1_000_000) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blind_ray_config_default() {
        let cfg = BlindRayConfig::default();
        assert!(cfg.energy_threshold > 0.0 && cfg.energy_threshold < 1.0);
        assert_eq!(cfg.frequency_hz, 2.4e9);
    }

    #[test]
    fn test_blind_ray_generator_creation() {
        let generator = BlindRayGenerator::new(BlindRayConfig::default());
        assert_eq!(generator.frame_counter, 0);
        assert!(generator.accumulated_particles.is_empty());
    }
}
