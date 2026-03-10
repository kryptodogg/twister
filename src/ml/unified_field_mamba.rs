use burn::prelude::*;
use crate::ml::mamba_block::MambaBlock;
use burn::tensor::backend::Backend;

/// Unified Field Mamba: 8 cascaded Selective Scan blocks for 9D Hilbert-sorted particles
/// Input: [Batch, N_particles, 9] (Position: [x,y,z], [phase_i, phase_q], [hardness, roughness, wetness], energy_gradient)
/// Architecture: 9D -> 128D -> 8-block Selective Scan -> 9D refined predictions
#[derive(Module, Debug)]
pub struct UnifiedFieldMamba<B: Backend> {
    /// Projection from 9D input to 128D latent
    input_proj: burn::nn::Linear<B>,

    /// 8 cascaded Selective Scan blocks
    blocks: [MambaBlock<B>; 8],

    /// Projection from 128D back to 9D output
    output_proj: burn::nn::Linear<B>,
}

impl<B: Backend> UnifiedFieldMamba<B> {
    pub fn new(device: &B::Device) -> Self {
        // Initialize 9D -> 128D projection
        let input_proj = burn::nn::LinearConfig::new(9, 128)
            .with_bias(true)
            .init(device);

        // Initialize 8 Mamba blocks
        let blocks = [
            Self::create_block(device),
            Self::create_block(device),
            Self::create_block(device),
            Self::create_block(device),
            Self::create_block(device),
            Self::create_block(device),
            Self::create_block(device),
            Self::create_block(device),
        ];

        // Initialize 128D -> 9D projection
        let output_proj = burn::nn::LinearConfig::new(128, 9)
            .with_bias(true)
            .init(device);

        Self {
            input_proj,
            blocks,
            output_proj,
        }
    }

    /// Forward pass through the network.
    /// Returns a tuple of (Output Tensor [Batch, N_particles, 9], Latent Embeddings [Batch, N_particles, 128])
    pub fn forward(&self, input: Tensor<B, 3>) -> (Tensor<B, 3>, Tensor<B, 3>) {
        // Project 9D input to 128D latent embedding
        let mut latent = self.input_proj.forward(input);

        // Pass through 8 Selective Scan blocks
        for block in self.blocks.iter() {
            latent = block.forward(&latent);
        }

        // The final latent state after 8 blocks
        let final_latent = latent.clone();

        // Project 128D latent back to 9D refined predictions
        let output = self.output_proj.forward(final_latent.clone());

        (output, final_latent)
    }

    /// Helper to create a single Mamba block
    fn create_block(device: &B::Device) -> MambaBlock<B> {
        MambaBlock::new(
            Tensor::random([128, 128], burn::tensor::Distribution::Default, device),
            Tensor::random([128], burn::tensor::Distribution::Default, device),
            Tensor::random([128], burn::tensor::Distribution::Default, device),
            Tensor::random([128], burn::tensor::Distribution::Default, device),
        )
    }
}

/// Accumulator for gathering particles until threshold is met or timeout occurs
pub struct HitListAccumulator {
    particles: Vec<[f32; 9]>,
    last_flush: std::time::Instant,
}

impl Default for HitListAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl HitListAccumulator {
    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(4096),
            last_flush: std::time::Instant::now(),
        }
    }

    /// Add a particle to the accumulator. Returns true if a flush should be triggered.
    pub fn add(&mut self, particle: [f32; 9]) -> bool {
        self.particles.push(particle);
        self.should_flush()
    }

    /// Add multiple particles to the accumulator. Returns true if a flush should be triggered.
    pub fn extend(&mut self, new_particles: &[[f32; 9]]) -> bool {
        self.particles.extend_from_slice(new_particles);
        self.should_flush()
    }

    /// Check if accumulator has reached 4096 particles or 1s timeout
    pub fn should_flush(&self) -> bool {
        self.particles.len() >= 4096 || self.last_flush.elapsed().as_secs() >= 1
    }

    /// Flush the accumulated particles and reset the timer
    pub fn flush(&mut self) -> Vec<[f32; 9]> {
        let result = std::mem::take(&mut self.particles);
        self.last_flush = std::time::Instant::now();
        // Pre-allocate the next vector to avoid reallocations
        self.particles = Vec::with_capacity(4096);
        result
    }
}
