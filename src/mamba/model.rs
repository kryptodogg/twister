use burn::prelude::*;
use burn::tensor::backend::Backend;

/// SSAMBA: State Space Audio Mamba
/// Input dimensionality: 432 [RF_PSD(256) | Audio_PSD(128) | TDOA(16) | ANC_state(32)]
/// Latent representation: 128-D
#[derive(Module, Debug)]
pub struct SSAMBA<B: Backend> {
    input_proj: burn::nn::Linear<B>,
    mamba_blocks: Vec<MambaBlock<B>>,
    latent_proj: burn::nn::Linear<B>,
    output_proj: burn::nn::Linear<B>,
}

impl<B: Backend> SSAMBA<B> {
    pub fn new(device: &B::Device) -> Self {
        let input_proj = burn::nn::LinearConfig::new(432, 256).init(device);
        let mut mamba_blocks = Vec::new();
        for _ in 0..4 {
            mamba_blocks.push(MambaBlock::new(256, 64, device));
        }
        let latent_proj = burn::nn::LinearConfig::new(256, 128).init(device);
        let output_proj = burn::nn::LinearConfig::new(128, 432).init(device);

        Self {
            input_proj,
            mamba_blocks,
            latent_proj,
            output_proj,
        }
    }

    pub fn forward(&self, input: Tensor<B, 2>) -> (Tensor<B, 2>, Tensor<B, 2>) {
        let mut x = self.input_proj.forward(input);
        for block in &self.mamba_blocks {
            x = block.forward(x);
        }
        let latent = self.latent_proj.forward(x);
        let reconstruction = self.output_proj.forward(latent.clone());
        (latent, reconstruction)
    }
}

#[derive(Module, Debug)]
pub struct MambaBlock<B: Backend> {
    norm: burn::nn::LayerNorm<B>,
    proj: burn::nn::Linear<B>,
}

impl<B: Backend> MambaBlock<B> {
    pub fn new(dim: usize, _state_dim: usize, device: &B::Device) -> Self {
        let norm = burn::nn::LayerNormConfig::new(dim).init(device);
        let proj = burn::nn::LinearConfig::new(dim, dim).init(device);
        Self { norm, proj }
    }

    pub fn forward(&self, x: Tensor<B, 2>) -> Tensor<B, 2> {
        let res = x.clone();
        let x = self.norm.forward(x);
        let x = self.proj.forward(x);
        x + res
    }
}

pub struct SSAMBAConfig;
impl SSAMBAConfig {
    pub fn new() -> Self { Self }
}

pub struct MambaControl<B: Backend> {
    pub mode_logits: Tensor<B, 2>,
    pub snr_target: Tensor<B, 2>,
}
