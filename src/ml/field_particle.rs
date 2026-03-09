// src/ml/field_particle.rs — GPU-resident FieldParticle buffer management
//
// Manages StorageBuffers for FieldParticle data on the GPU.
// Handles batch accumulation, sorting triggers, and device transfers.

use bytemuck::{Pod, Zeroable};
use std::mem;
use wgpu::{Buffer, Device, Queue};

use crate::ml::data_contracts::FieldParticle;

/// GPU-safe wrapper for FieldParticle
/// Ensures proper byte layout and alignment for GPU consumption
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct FieldParticleGPU {
    pub position: [f32; 3],
    pub phase_amp: [f32; 2],
    pub material: [f32; 3],
    pub energy_gradient: f32,
    pub _padding: f32,
}

impl From<FieldParticle> for FieldParticleGPU {
    fn from(particle: FieldParticle) -> Self {
        FieldParticleGPU {
            position: particle.position,
            phase_amp: particle.phase_amp,
            material: particle.material,
            energy_gradient: particle.energy_gradient,
            _padding: particle._padding,
        }
    }
}

/// Storage for accumulated Hit Lists from RT-core ray tracing
/// Acts as fast staging area before Mamba batch
#[derive(Debug)]
pub struct FieldParticleBuffer {
    /// GPU storage buffer for FieldParticle data
    pub buffer: Buffer,
    /// Maximum capacity (allocated at init)
    pub capacity: usize,
    /// Current number of valid particles in buffer
    pub len: usize,
    /// Hilbert curve indices for sorting
    pub hilbert_buffer: Buffer,
    /// Timestamps (microseconds since epoch)
    pub timestamp_buffer: Buffer,
}

impl FieldParticleBuffer {
    /// Create a new GPU buffer for particles
    ///
    /// # Arguments
    /// * `device`: WGPU device
    /// * `capacity`: Maximum particles to hold (e.g., 4096)
    pub fn new(device: &Device, capacity: usize) -> Self {
        let buffer_size = (capacity * mem::size_of::<FieldParticleGPU>()) as u64;
        let hilbert_size = (capacity * mem::size_of::<u32>()) as u64;
        let timestamp_size = (capacity * mem::size_of::<u64>()) as u64;

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("FieldParticleBuffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let hilbert_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("HilbertIndexBuffer"),
            size: hilbert_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let timestamp_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("TimestampBuffer"),
            size: timestamp_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        FieldParticleBuffer {
            buffer,
            capacity,
            len: 0,
            hilbert_buffer,
            timestamp_buffer,
        }
    }

    /// Upload particle data to GPU
    ///
    /// # Arguments
    /// * `queue`: WGPU queue for command submission
    /// * `particles`: Particle data to upload
    /// * `hilbert_indices`: Pre-computed Hilbert indices
    /// * `timestamps`: Microsecond timestamps
    pub fn upload(
        &mut self,
        queue: &Queue,
        particles: &[FieldParticleGPU],
        hilbert_indices: &[u32],
        timestamps: &[u64],
    ) {
        debug_assert_eq!(particles.len(), hilbert_indices.len());
        debug_assert_eq!(particles.len(), timestamps.len());
        debug_assert!(particles.len() <= self.capacity);

        self.len = particles.len();

        // Upload particles
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(particles));

        // Upload Hilbert indices
        queue.write_buffer(
            &self.hilbert_buffer,
            0,
            bytemuck::cast_slice(hilbert_indices),
        );

        // Upload timestamps
        queue.write_buffer(
            &self.timestamp_buffer,
            0,
            bytemuck::cast_slice(timestamps),
        );
    }

    /// Get buffer binding for compute shader
    pub fn as_binding(&self, _offset: u64) -> wgpu::BindingResource {
        wgpu::BindingResource::Buffer(self.buffer.as_entire_buffer_binding())
    }

    /// Check if buffer should trigger Mamba batch solve
    /// Returns true when N >= batch_threshold (e.g., 4096)
    pub fn should_dispatch_mamba(&self, batch_threshold: usize) -> bool {
        self.len >= batch_threshold
    }

    /// Clear buffer for next accumulation cycle
    pub fn reset(&mut self) {
        self.len = 0;
    }

    /// Get current utilization (0.0 = empty, 1.0 = full)
    pub fn utilization(&self) -> f32 {
        self.len as f32 / self.capacity as f32
    }
}

/// Hit List accumulator for RT-core streaming
/// Collects collision data before batch processing
#[derive(Debug, Default)]
pub struct HitListAccumulator {
    /// Accumulated particles from ray tracing
    pub particles: Vec<FieldParticleGPU>,
    /// Hilbert indices (computed during accumulation)
    pub hilbert_indices: Vec<u32>,
    /// Timestamps for causality
    pub timestamps: Vec<u64>,
    /// Frame counter for diagnostics
    pub frame_count: u32,
}

impl HitListAccumulator {
    pub fn new() -> Self {
        HitListAccumulator::default()
    }

    /// Add a hit to the accumulator
    pub fn add_hit(
        &mut self,
        particle: FieldParticleGPU,
        hilbert_idx: u32,
        timestamp: u64,
    ) {
        self.particles.push(particle);
        self.hilbert_indices.push(hilbert_idx);
        self.timestamps.push(timestamp);
    }

    /// Get current hit count
    pub fn len(&self) -> usize {
        self.particles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    /// Clear for next frame
    pub fn clear(&mut self) {
        self.particles.clear();
        self.hilbert_indices.clear();
        self.timestamps.clear();
        self.frame_count += 1;
    }

    /// Check if ready for batch dispatch
    pub fn is_ready(&self, batch_threshold: usize) -> bool {
        self.particles.len() >= batch_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_particle_gpu_alignment() {
        // Verify 48-byte layout
        assert_eq!(mem::size_of::<FieldParticleGPU>(), 48);
    }

    #[test]
    fn test_hit_list_accumulator() {
        let mut acc = HitListAccumulator::new();

        let particle = FieldParticleGPU {
            position: [1.0, 2.0, 3.0],
            phase_amp: [0.5, 0.25],
            material: [0.8, 0.6, 0.5],
            energy_gradient: 10.5,
            _padding: 0.0,
        };

        acc.add_hit(particle, 42, 1709990400000000);

        assert_eq!(acc.len(), 1);
        assert!(!acc.is_empty());
        assert!(!acc.is_ready(4096));
    }
}
