// src/ml/batch_accumulation.rs
// Batch Accumulation Orchestrator for Unified Field Mamba
//
// Coordinates the flow:
// 1. Particles accumulate in CPU HitListAccumulator
// 2. When threshold met (N ≥ batch_threshold), snapshot accumulator
// 3. Compute Hilbert indices for each particle
// 4. Radix sort by Hilbert index (optimal cache locality)
// 5. Prepare tensor [Batch, N, 9] for Mamba input
// 6. Upload to GPU StorageBuffer
// 7. Dispatch Mamba Neural Operator
// 8. Collect results and feed to renderer

use super::data_contracts::FieldParticle;
use super::field_particle::{FieldParticleBuffer, FieldParticleGPU, HitListAccumulator};
use super::hilbert_sort::{hilbert_index, radix_sort_hilbert, HilbertKey};
use wgpu::Queue;

/// Batch accumulation configuration and state machine
#[derive(Clone, Debug)]
pub struct BatchAccumulationConfig {
    /// Batch size threshold (e.g., 4096 particles trigger Mamba)
    pub batch_threshold: usize,
    /// Grid resolution level for Hilbert curve (e.g., 6 = 64³ grid)
    pub hilbert_level: u32,
    /// Maximum time to wait before forcing dispatch (seconds)
    pub max_wait_seconds: f32,
}

impl Default for BatchAccumulationConfig {
    fn default() -> Self {
        Self {
            batch_threshold: 4096,
            hilbert_level: 6, // 64³ = 262,144 max Hilbert indices
            max_wait_seconds: 1.0,
        }
    }
}

/// State machine for batch processing
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccumulationState {
    /// Waiting for particles to accumulate
    Accumulating,
    /// Snapshot taken, ready for sorting
    Snapshotted,
    /// Sorting in progress
    Sorting,
    /// Ready to dispatch to Mamba
    ReadyForMamba,
    /// Currently processing in Mamba
    ProcessingInMamba,
}

/// Orchestrates batch accumulation, sorting, and Mamba dispatch
pub struct BatchAccumulator {
    /// Configuration
    pub config: BatchAccumulationConfig,
    /// Current state
    pub state: AccumulationState,
    /// Snapshotted particles (copied from HitListAccumulator)
    pub snapshot: Vec<FieldParticle>,
    /// Hilbert indices for each particle
    pub hilbert_keys: Vec<HilbertKey>,
    /// Sorted particle indices (permutation)
    pub sorted_indices: Vec<u32>,
    /// Timestamp when accumulation started
    pub accumulation_start_us: u64,
    /// Number of batches processed so far
    pub batches_processed: u32,
}

impl BatchAccumulator {
    /// Create new batch accumulator
    pub fn new(config: BatchAccumulationConfig) -> Self {
        Self {
            config,
            state: AccumulationState::Accumulating,
            snapshot: Vec::new(),
            hilbert_keys: Vec::new(),
            sorted_indices: Vec::new(),
            accumulation_start_us: 0,
            batches_processed: 0,
        }
    }

    /// Check if accumulation threshold is met
    pub fn should_dispatch(&self, accumulated_count: usize, elapsed_us: u64) -> bool {
        accumulated_count >= self.config.batch_threshold
            || (elapsed_us as f32 / 1_000_000.0) > self.config.max_wait_seconds
    }

    /// Snapshot particles from accumulator and prepare for sorting
    pub fn snapshot_and_sort(&mut self, accumulator: &HitListAccumulator) {
        self.state = AccumulationState::Snapshotted;

        // Convert FieldParticleGPU to FieldParticle and copy from accumulator
        self.snapshot.clear();
        for gpu_particle in &accumulator.particles {
            let particle = FieldParticle {
                position: gpu_particle.position,
                phase_amp: gpu_particle.phase_amp,
                material: gpu_particle.material,
                energy_gradient: gpu_particle.energy_gradient,
                _padding: gpu_particle._padding,
            };
            self.snapshot.push(particle);
        }

        // Compute Hilbert indices (or use pre-computed ones from accumulator)
        self.hilbert_keys.clear();
        for (idx, particle) in self.snapshot.iter().enumerate() {
            // Use pre-computed Hilbert index if available, otherwise compute
            let h_idx = if idx < accumulator.hilbert_indices.len() {
                accumulator.hilbert_indices[idx] as u64
            } else {
                let x = (particle.position[0] as u32).min(63);
                let y = (particle.position[1] as u32).min(63);
                let z = (particle.position[2] as u32).min(63);
                hilbert_index(x, y, z, self.config.hilbert_level)
            };

            self.hilbert_keys.push(HilbertKey {
                index: h_idx,
                particle_idx: idx as u32,
            });
        }

        // Radix sort by Hilbert index
        self.state = AccumulationState::Sorting;
        self.sorted_indices = radix_sort_hilbert(&self.hilbert_keys);
        self.state = AccumulationState::ReadyForMamba;
    }

    /// Upload sorted particles to GPU buffer
    pub fn upload_to_gpu(
        &self,
        queue: &Queue,
        gpu_buffer: &mut FieldParticleBuffer,
        accumulator: &HitListAccumulator,
    ) {
        if self.state != AccumulationState::ReadyForMamba {
            panic!("Batch not ready for GPU upload (must be ReadyForMamba state)");
        }

        // Create reordered GPU particle buffer and corresponding Hilbert/timestamp arrays
        let mut reordered_gpu = Vec::with_capacity(self.snapshot.len());
        let mut reordered_hilbert = Vec::with_capacity(self.snapshot.len());
        let mut reordered_timestamps = Vec::with_capacity(self.snapshot.len());

        for &orig_idx in &self.sorted_indices {
            if orig_idx < accumulator.particles.len() as u32 {
                let idx = orig_idx as usize;
                reordered_gpu.push(accumulator.particles[idx]);

                let h_idx = if idx < accumulator.hilbert_indices.len() {
                    accumulator.hilbert_indices[idx]
                } else {
                    0
                };
                reordered_hilbert.push(h_idx);

                let ts = if idx < accumulator.timestamps.len() {
                    accumulator.timestamps[idx]
                } else {
                    0
                };
                reordered_timestamps.push(ts);
            }
        }

        // Upload to GPU
        gpu_buffer.upload(queue, &reordered_gpu, &reordered_hilbert, &reordered_timestamps);
    }

    /// Get particles in sorted order for CPU-side processing
    pub fn get_sorted_particles(&self) -> Vec<FieldParticle> {
        let mut result = Vec::with_capacity(self.snapshot.len());
        for &idx in &self.sorted_indices {
            if let Some(&particle) = self.snapshot.get(idx as usize) {
                result.push(particle);
            }
        }
        result
    }

    /// Reset accumulator for next batch
    pub fn reset(&mut self) {
        self.state = AccumulationState::Accumulating;
        self.snapshot.clear();
        self.hilbert_keys.clear();
        self.sorted_indices.clear();
        self.batches_processed += 1;
    }

    /// Statistics about current batch
    pub fn stats(&self) -> BatchStats {
        BatchStats {
            particle_count: self.snapshot.len(),
            state: self.state,
            batches_processed: self.batches_processed,
            hilbert_min: self.hilbert_keys.iter().map(|k| k.index).min().unwrap_or(0),
            hilbert_max: self.hilbert_keys.iter().map(|k| k.index).max().unwrap_or(0),
        }
    }
}

/// Statistics snapshot
#[derive(Clone, Debug)]
pub struct BatchStats {
    pub particle_count: usize,
    pub state: AccumulationState,
    pub batches_processed: u32,
    pub hilbert_min: u64,
    pub hilbert_max: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_accumulator_creation() {
        let config = BatchAccumulationConfig::default();
        let acc = BatchAccumulator::new(config);
        assert_eq!(acc.state, AccumulationState::Accumulating);
        assert_eq!(acc.batches_processed, 0);
    }

    #[test]
    fn test_should_dispatch_threshold() {
        let config = BatchAccumulationConfig {
            batch_threshold: 100,
            ..Default::default()
        };
        let acc = BatchAccumulator::new(config);

        assert!(!acc.should_dispatch(50, 0));
        assert!(acc.should_dispatch(100, 0));
        assert!(acc.should_dispatch(150, 0));
    }

    #[test]
    fn test_should_dispatch_timeout() {
        let config = BatchAccumulationConfig {
            batch_threshold: 10000,
            max_wait_seconds: 1.0,
            ..Default::default()
        };
        let acc = BatchAccumulator::new(config);

        // No timeout yet
        assert!(!acc.should_dispatch(50, 500_000)); // 0.5s

        // Timeout triggered
        assert!(acc.should_dispatch(50, 1_100_000)); // 1.1s
    }

    #[test]
    fn test_batch_stats() {
        let config = BatchAccumulationConfig::default();
        let mut acc = BatchAccumulator::new(config);

        // Create dummy particles
        for i in 0..10 {
            acc.snapshot.push(FieldParticle {
                position: [i as f32, i as f32, i as f32],
                phase_amp: [0.0, 0.0],
                material: [0.5, 0.5, 0.5],
                energy_gradient: 1.0,
                _padding: 0.0,
            });
        }

        let stats = acc.stats();
        assert_eq!(stats.particle_count, 10);
        assert_eq!(stats.state, AccumulationState::Accumulating);
    }

    #[test]
    fn test_reset() {
        let config = BatchAccumulationConfig::default();
        let mut acc = BatchAccumulator::new(config);

        acc.snapshot.push(FieldParticle {
            position: [0.0, 0.0, 0.0],
            phase_amp: [0.0, 0.0],
            material: [0.5, 0.5, 0.5],
            energy_gradient: 1.0,
            _padding: 0.0,
        });
        assert_eq!(acc.snapshot.len(), 1);
        assert_eq!(acc.batches_processed, 0);

        acc.reset();
        assert_eq!(acc.snapshot.len(), 0);
        assert_eq!(acc.batches_processed, 1);
        assert_eq!(acc.state, AccumulationState::Accumulating);
    }
}
