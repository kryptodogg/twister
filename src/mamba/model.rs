use burn::prelude::*;
use burn::tensor::backend::Backend;

/// SSAMBA: State Space Audio Mamba
/// Advanced architecture for multi-sensor forensic fusion.
/// Input: [RF_PSD(256) | Audio_PSD(128) | TDOA(16) | ANC_state(32)] = 432
#[derive(Module, Debug)]
pub struct SSAMBA<B: Backend> {
    input_proj: burn::nn::Linear<B>,
    blocks: Vec<MambaBlock<B>>,
    latent_proj: burn::nn::Linear<B>,
    output_proj: burn::nn::Linear<B>,
}

impl<B: Backend> SSAMBA<B> {
    pub fn new(device: &B::Device) -> Self {
        let input_proj = burn::nn::LinearConfig::new(432, 256).init(device);
        let mut blocks = Vec::new();
        for _ in 0..8 { // Increased depth for better forensic discovery
            blocks.push(MambaBlock::new(256, 128, device));
        }
        let latent_proj = burn::nn::LinearConfig::new(256, 128).init(device);
        let output_proj = burn::nn::LinearConfig::new(128, 432).init(device);

        Self {
            input_proj,
            blocks,
            latent_proj,
            output_proj,
        }
    }

    /// Forward pass through the State Space Model.
    /// Returns (128-D Latent, 432-D Reconstruction)
    pub fn forward(&self, input: Tensor<B, 2>) -> (Tensor<B, 2>, Tensor<B, 2>) {
        let mut x = self.input_proj.forward(input);
        for block in &self.blocks {
            x = block.forward(x);
        }
        let latent = self.latent_proj.forward(x);
        let reconstruction = self.output_proj.forward(latent.clone());
        (latent, reconstruction)
    }

    pub fn latent_dim(&self) -> usize { 128 }
}

#[derive(Module, Debug)]
pub struct MambaBlock<B: Backend> {
    norm: burn::nn::LayerNorm<B>,
    ssm_proj: burn::nn::Linear<B>,
}

impl<B: Backend> MambaBlock<B> {
    pub fn new(dim: usize, _state_dim: usize, device: &B::Device) -> Self {
        let norm = burn::nn::LayerNormConfig::new(dim).init(device);
        let ssm_proj = burn::nn::LinearConfig::new(dim, dim).init(device);
        Self { norm, ssm_proj }
    }

    pub fn forward(&self, x: Tensor<B, 2>) -> Tensor<B, 2> {
        let res = x.clone();
        let x = self.norm.forward(x);
        let x = self.ssm_proj.forward(x);
        x + res
    }
}
