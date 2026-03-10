// src/ml/field_dispatch.rs
// Unified Field Dispatch Pipeline
//
// Orchestrates the flow of particles from RF propagation → blind ray generation →
// accumulation → Mamba Neural Operator → output field
//
// The dispatch loop:
// 1. RF FDFD solver generates E-field solution
// 2. Blind ray generator samples E-field → particles
// 3. Accumulate particles in GPU buffer (StorageBuffer)
// 4. When N ≥ threshold, sort by Hilbert curve
// 5. Feed to Mamba Neural Operator
// 6. Write results back to particle buffer for rendering

use super::data_contracts::FieldParticle;
use super::field_particle::{FieldParticleBuffer, FieldParticleGPU, HitListAccumulator};
use super::rt_blind_rays::{BlindRayConfig, BlindRayGenerator};
use crate::resonance::rf_propagation::RFWavePropagation;
use crate::resonance::voxel_grid::VoxelGrid;
use crate::resonance::material_absorption::Material;
use wgpu::{Device, Queue};

/// Configuration for the unified field dispatch pipeline
#[derive(Clone, Debug)]
pub struct FieldDispatchConfig {
    /// Batch size threshold for Mamba execution
    pub batch_threshold: usize,
    /// Maximum GPU buffer capacity
    pub gpu_buffer_capacity: usize,
    /// Blind ray generation config
    pub blind_ray_config: BlindRayConfig,
}

impl Default for FieldDispatchConfig {
    fn default() -> Self {
        Self {
            batch_threshold: 4096,
            gpu_buffer_capacity: 8192,
            blind_ray_config: BlindRayConfig::default(),
        }
    }
}

/// Main dispatch pipeline state
pub struct FieldDispatchPipeline {
    config: FieldDispatchConfig,
    blind_ray_gen: BlindRayGenerator,
    accumulator: HitListAccumulator,
    gpu_buffer: Option<FieldParticleBuffer>,
    frame_count: u32,
    total_particles_processed: u64,
}

impl FieldDispatchPipeline {
    /// Create a new dispatch pipeline
    pub fn new(config: FieldDispatchConfig) -> Self {
        Self {
            config: config.clone(),
            blind_ray_gen: BlindRayGenerator::new(config.blind_ray_config),
            accumulator: HitListAccumulator::new(),
            gpu_buffer: None,
            frame_count: 0,
            total_particles_processed: 0,
        }
    }

    /// Initialize GPU buffer (call after WGPU device is available)
    pub fn initialize_gpu(&mut self, device: &Device) {
        self.gpu_buffer = Some(FieldParticleBuffer::new(device, self.config.gpu_buffer_capacity));
    }

    /// Main dispatch step: Generate particles from RF field and accumulate
    ///
    /// Returns:
    /// - `Ok(true)` if batch is ready for Mamba processing
    /// - `Ok(false)` if still accumulating
    /// - `Err(...)` if processing error
    pub fn dispatch_step(
        &mut self,
        rf_prop: &RFWavePropagation,
        material_grid: &VoxelGrid<Material>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Step 1: Generate particles from FDFD field
        let hit_particles = self.blind_ray_gen.generate_hit_list(rf_prop, material_grid)?;

        // Step 2: Accumulate in CPU buffer
        for (particle_gpu, hilbert_idx, timestamp) in hit_particles {
            let _particle = FieldParticle {
                position: particle_gpu.position,
                phase_amp: particle_gpu.phase_amp,
                material: particle_gpu.material,
                energy_gradient: particle_gpu.energy_gradient,
                _padding: particle_gpu._padding,
            };

            self.accumulator.add_hit(particle_gpu, hilbert_idx, timestamp);
        }

        // Step 3: Check if ready to dispatch to Mamba
        let ready = self.accumulator.is_ready(self.config.batch_threshold);

        if ready {
            self.total_particles_processed += self.accumulator.len() as u64;
        }

        self.frame_count += 1;

        Ok(ready)
    }

    /// Upload accumulated particles to GPU buffer (after Mamba dispatch)
    pub fn upload_to_gpu(&mut self, queue: &Queue) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(buffer) = &mut self.gpu_buffer {
            // Convert accumulated particles to GPU format
            let particles: Vec<FieldParticleGPU> = self
                .accumulator
                .particles
                .iter()
                .map(|p| FieldParticleGPU {
                    position: p.position,
                    phase_amp: p.phase_amp,
                    material: p.material,
                    energy_gradient: p.energy_gradient,
                    _padding: 0.0,
                })
                .collect();

            buffer.upload(
                queue,
                &particles,
                &self.accumulator.hilbert_indices,
                &self.accumulator.timestamps,
            );

            Ok(())
        } else {
            Err("GPU buffer not initialized".into())
        }
    }

    /// Reset accumulator after Mamba processing
    pub fn reset_accumulator(&mut self) {
        self.accumulator.clear();
    }

    /// Get current accumulator state
    pub fn accumulator_len(&self) -> usize {
        self.accumulator.len()
    }

    /// Get GPU buffer utilization (0.0-1.0)
    pub fn gpu_buffer_utilization(&self) -> f32 {
        if let Some(buffer) = &self.gpu_buffer {
            buffer.utilization()
        } else {
            0.0
        }
    }

    /// Get statistics
    pub fn stats(&self) -> DispatchStats {
        DispatchStats {
            frame_count: self.frame_count,
            total_particles_processed: self.total_particles_processed,
            current_accumulation: self.accumulator_len(),
            gpu_utilization: self.gpu_buffer_utilization(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DispatchStats {
    pub frame_count: u32,
    pub total_particles_processed: u64,
    pub current_accumulation: usize,
    pub gpu_utilization: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatch_config_default() {
        let cfg = FieldDispatchConfig::default();
        assert_eq!(cfg.batch_threshold, 4096);
        assert_eq!(cfg.gpu_buffer_capacity, 8192);
    }

    #[test]
    fn test_dispatch_pipeline_creation() {
        let pipeline = FieldDispatchPipeline::new(FieldDispatchConfig::default());
        assert_eq!(pipeline.frame_count, 0);
        assert_eq!(pipeline.total_particles_processed, 0);
        assert_eq!(pipeline.accumulator_len(), 0);
    }
}
