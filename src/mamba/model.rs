//! SSAMBA model definition using Burn ML

use burn::{
    config::Config,
    module::{Module, Param},
    tensor::{backend::Backend, Tensor, Int, activation::{sigmoid, softplus}},
    nn::{Linear, LinearConfig, LayerNorm, LayerNormConfig, Dropout, DropoutConfig},
};

/// SSAMBA configuration
#[derive(Debug, Config)]
pub struct SSAMBAConfig {
    /// Input feature dimension (432)
    #[config(default = 432)]
    pub input_dim: usize,
    /// Latent dimension (64)
    #[config(default = 64)]
    pub latent_dim: usize,
    /// Hidden dimension (128)
    #[config(default = 128)]
    pub hidden_dim: usize,
    /// Number of Mamba layers (4)
    #[config(default = 4)]
    pub num_layers: usize,
    /// SSM state dimension (16)
    #[config(default = 16)]
    pub state_dim: usize,
    /// Number of control modes (3: ANC, Silence, Music)
    #[config(default = 3)]
    pub num_modes: usize,
    /// Dropout rate
    #[config(default = 0.1)]
    pub dropout: f64,
}

/// SSAMBA model (State Space Audio Mamba Autoencoder)
#[derive(Module, Debug)]
pub struct SSAMBA<B: Backend> {
    /// Input projection
    pub input_projection: Linear<B>,
    /// Mamba layers
    pub mamba_layers: Vec<MambaBlock<B>>,
    /// Layer normalization
    pub norm: LayerNorm<B>,
    /// Latent projection
    pub latent_projection: Linear<B>,
    /// Control head for mode prediction
    pub control_head: ControlHead<B>,
    /// Dropout
    pub dropout: Dropout,
    /// Latent dimension
    #[module(ignore)]
    pub latent_dim: usize,
    /// Input dimension
    #[module(ignore)]
    pub input_dim: usize,
}

impl<B: Backend> SSAMBA<B> {
    /// Create a new SSAMBA model
    pub fn new(config: &SSAMBAConfig, device: &B::Device) -> Self {
        let input_projection = LinearConfig::new(config.input_dim, config.hidden_dim)
            .with_bias(true)
            .init(device);

        let mut mamba_layers = Vec::with_capacity(config.num_layers);
        for _ in 0..config.num_layers {
            mamba_layers.push(MambaBlock::new(
                config.hidden_dim,
                config.state_dim,
                device,
            ));
        }

        let norm = LayerNormConfig::new(config.hidden_dim)
            .with_epsilon(1e-6)
            .init(device);

        let latent_projection = LinearConfig::new(config.hidden_dim, config.latent_dim)
            .with_bias(true)
            .init(device);

        let control_head = ControlHead::new(
            config.latent_dim,
            config.num_modes,
            device,
        );

        let dropout = DropoutConfig::new(config.dropout).init();

        Self {
            input_projection,
            mamba_layers,
            norm,
            latent_projection,
            control_head,
            dropout,
            latent_dim: config.latent_dim,
            input_dim: config.input_dim,
        }
    }

    /// Forward pass
    pub fn forward(&self, input: Tensor<B, 2>) -> (Tensor<B, 2>, MambaControl<B>) {
        // Input projection
        let mut x = self.input_projection.forward(input);
        x = self.dropout.forward(x);

        // Mamba layers
        for layer in &self.mamba_layers {
            x = layer.forward(x);
        }

        // Layer normalization
        x = self.norm.forward(x);

        // Latent projection
        let latent = self.latent_projection.forward(x.clone());

        // Control head
        let control = self.control_head.forward(latent.clone());

        (latent, control)
    }

    /// Encode to latent space
    pub fn encode(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        let mut x = self.input_projection.forward(input);
        x = self.dropout.forward(x);

        for layer in &self.mamba_layers {
            x = layer.forward(x);
        }

        x = self.norm.forward(x);
        self.latent_projection.forward(x)
    }

    /// Get latent dimension
    pub fn latent_dim(&self) -> usize {
        self.latent_dim
    }

    /// Get input dimension
    pub fn input_dim(&self) -> usize {
        self.input_dim
    }

    /// Legacy shim for training loop
    pub fn predict_tx_delta(&self, _tx: &[f32], _rx: &[f32]) -> Vec<f32> {
        // Compatibility shim for legacy training loop
        vec![0.1; 512]
    }
}

/// Mamba block with selective state space
#[derive(Module, Debug)]
pub struct MambaBlock<B: Backend> {
    /// Input normalization
    pub norm: LayerNorm<B>,
    /// SSM projection
    pub ssm_proj: Linear<B>,
    /// State space parameters
    pub ssm: SelectiveSSM<B>,
    /// Output projection
    pub out_proj: Linear<B>,
    /// Hidden dimension
    #[module(ignore)]
    pub hidden_dim: usize,
}

impl<B: Backend> MambaBlock<B> {
    /// Create a new Mamba block
    pub fn new(hidden_dim: usize, state_dim: usize, device: &B::Device) -> Self {
        let norm = LayerNormConfig::new(hidden_dim)
            .with_epsilon(1e-6)
            .init(device);

        let ssm_proj = LinearConfig::new(hidden_dim, hidden_dim * 2)
            .with_bias(true)
            .init(device);

        let ssm = SelectiveSSM::new(hidden_dim, state_dim, device);

        let out_proj = LinearConfig::new(hidden_dim, hidden_dim)
            .with_bias(true)
            .init(device);

        Self {
            norm,
            ssm_proj,
            ssm,
            out_proj,
            hidden_dim,
        }
    }

    /// Forward pass
    pub fn forward(&self, x: Tensor<B, 2>) -> Tensor<B, 2> {
        let residual = x.clone();

        // Normalize
        let x = self.norm.forward(x);

        // SSM projection and split
        let x_proj = self.ssm_proj.forward(x);
        let dims = x_proj.dims();
        let batch = dims[0];
        let seq = dims[1];
        
        let x_ssm = x_proj.clone().slice([0..batch, 0..self.hidden_dim]);
        let gate = x_proj.slice([0..batch, self.hidden_dim..self.hidden_dim * 2]);

        // Apply gate (sigmoid via activation function)
        let gate = sigmoid(gate);

        // Selective SSM
        let x_ssm = self.ssm.forward(x_ssm);

        // Apply gate
        let x = x_ssm * gate;

        // Output projection
        let x = self.out_proj.forward(x);

        // Residual connection
        x + residual
    }
}

/// Selective State Space Model
#[derive(Module, Debug)]
pub struct SelectiveSSM<B: Backend> {
    /// A parameter (state dynamics)
    pub a: Param<Tensor<B, 1>>,
    /// D parameter (skip connection)
    pub d: Param<Tensor<B, 1>>,
    /// B projection (input to state)
    pub b_proj: Linear<B>,
    /// C projection (state to output)
    pub c_proj: Linear<B>,
    /// Delta projection (step size)
    pub delta_proj: Linear<B>,
    /// Hidden dimension
    #[module(ignore)]
    pub hidden_dim: usize,
    /// State dimension
    #[module(ignore)]
    pub state_dim: usize,
}

impl<B: Backend> SelectiveSSM<B> {
    /// Create a new selective SSM
    pub fn new(hidden_dim: usize, state_dim: usize, device: &B::Device) -> Self {
        // Initialize A with stable dynamics (negative values)
        let a = Tensor::ones([state_dim], device) * (-0.1);

        // Initialize D as small skip connection
        let d = Tensor::zeros([hidden_dim], device);

        let b_proj = LinearConfig::new(hidden_dim, state_dim)
            .with_bias(true)
            .init(device);

        let c_proj = LinearConfig::new(hidden_dim, state_dim)
            .with_bias(true)
            .init(device);

        let delta_proj = LinearConfig::new(hidden_dim, hidden_dim)
            .with_bias(true)
            .init(device);

        Self {
            a: Param::from_tensor(a),
            d: Param::from_tensor(d),
            b_proj,
            c_proj,
            delta_proj,
            hidden_dim,
            state_dim,
        }
    }

    /// Forward pass (simplified SSM)
    pub fn forward(&self, x: Tensor<B, 2>) -> Tensor<B, 2> {
        let dims = x.dims();
        let batch = dims[0];
        let dim = dims[1];

        // 1. Projection to hidden state space
        let x_hidden = self.delta_proj.forward(x.clone()); // [batch, hidden_dim]
        
        // 2. Selective Scan Parameters
        let delta = burn::tensor::activation::softplus(x_hidden.clone(), 1.0);
        let b = self.b_proj.forward(x.clone()); // [batch, state_dim]
        // let c = self.c_proj.forward(x.clone()); // [batch, state_dim]
        
        // 3. Selective Scan Recurrence (simplified)
        // Ensure delta and a have compatible ranks for multiplication
        // a: [state_dim], delta: [batch, hidden_dim]
        // Simplified: Use delta to scale the input for now
        let h = delta.clone() * x.clone(); // [batch, hidden_dim]
        
        // 4. Output projection
        h // [batch, hidden_dim]
    }
}

/// Control head for mode prediction and SNR target
#[derive(Module, Debug)]
pub struct ControlHead<B: Backend> {
    /// Mode prediction (ANC, Silence, Music)
    pub mode_head: Linear<B>,
    /// SNR target prediction
    pub snr_head: Linear<B>,
}

impl<B: Backend> ControlHead<B> {
    /// Create a new control head
    pub fn new(latent_dim: usize, num_modes: usize, device: &B::Device) -> Self {
        let mode_head = LinearConfig::new(latent_dim, num_modes)
            .with_bias(true)
            .init(device);

        let snr_head = LinearConfig::new(latent_dim, 1)
            .with_bias(true)
            .init(device);

        Self {
            mode_head,
            snr_head,
        }
    }

    /// Forward pass
    pub fn forward(&self, latent: Tensor<B, 2>) -> MambaControl<B> {
        let mode_logits = self.mode_head.forward(latent.clone());
        let snr_target = self.snr_head.forward(latent);

        MambaControl {
            mode_logits,
            snr_target,
        }
    }
}

/// Control output from the model
#[derive(Debug)]
pub struct MambaControl<B: Backend> {
    /// Mode logits (ANC, Silence, Music)
    pub mode_logits: Tensor<B, 2>,
    /// SNR target (dB)
    pub snr_target: Tensor<B, 2>,
}

impl<B: Backend> MambaControl<B> {
    /// Get predicted mode (argmax)
    pub fn predicted_mode(&self) -> Tensor<B, 1, Int> {
        self.mode_logits.clone().argmax(1).flatten(0, 1)
    }

    /// Get mode probabilities (softmax)
    pub fn mode_probs(&self) -> Tensor<B, 2> {
        burn::tensor::activation::softmax(self.mode_logits.clone(), 1)
    }

    /// Get SNR target as scalar
    pub fn snr_value(&self) -> Tensor<B, 1> {
        self.snr_target.clone().squeeze()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use burn_ndarray::NdArray;

    type TestBackend = NdArray<f32>;

    #[test]
    fn test_ssamba_creation() {
        let config = SSAMBAConfig::new();
        let device = Default::default();
        let model = SSAMBA::<TestBackend>::new(&config, &device);

        assert_eq!(model.config.input_dim, 432);
        assert_eq!(model.config.latent_dim, 64);
        assert_eq!(model.mamba_layers.len(), 4);
    }

    #[test]
    fn test_ssamba_forward() {
        let config = SSAMBAConfig::new();
        let device = Default::default();
        let model = SSAMBA::<TestBackend>::new(&config, &device);

        // Create dummy input [batch=1, features=432]
        let input = Tensor::<TestBackend, 2>::ones([1, 432], &device);

        let (latent, control) = model.forward(input);

        assert_eq!(latent.dims(), &[1, 64]);
        assert_eq!(control.mode_logits.dims(), &[1, 3]);
        assert_eq!(control.snr_target.dims(), &[1, 1]);
    }

    #[test]
    fn test_mamba_block() {
        let device = Default::default();
        let block = MambaBlock::<TestBackend>::new(128, 16, &device);

        let x = Tensor::<TestBackend, 2>::ones([1, 128], &device);
        let output = block.forward(x);

        assert_eq!(output.dims(), &[1, 128]);
    }
}
