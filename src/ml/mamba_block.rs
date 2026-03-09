use burn::prelude::Module;
use burn::tensor::{backend::Backend, Distribution, Tensor, TensorData};

/// Single Mamba block with selective scan
#[derive(Module, Debug)]
pub struct MambaBlock<B: Backend> {
    // State transition matrices (function-packed, no dead padding)
    state_a: Tensor<B, 2>,  // (128, 128)
    input_b: Tensor<B, 1>,  // (128,)
    output_c: Tensor<B, 1>, // (128,)
    gate_w: Tensor<B, 1>,   // (128,)
}

impl<B: Backend> MambaBlock<B> {
    pub fn new(
        state_a: Tensor<B, 2>,
        input_b: Tensor<B, 1>,
        output_c: Tensor<B, 1>,
        gate_w: Tensor<B, 1>,
    ) -> Self {
        Self {
            state_a,
            input_b,
            output_c,
            gate_w,
        }
    }

    /// Forward: (batch, num_points, 128) → (batch, num_points, 128) + residual
    ///
    /// Function-packed selective scan: all 128-D state operations in single scope.
    /// No dead padding. Intermediates freed immediately after use.
    pub fn forward(&self, input: &Tensor<B, 3>) -> Tensor<B, 3> {
        let [batch, num_points, _dim] = input.dims();
        let original = input.clone();

        // Function-packed selective scan: tight scope, immediate cleanup
        let output = {
            // Gating: Δ_i = sigmoid(W_g · u_i) ∈ [0, 1]
            // input: [batch, n, 128], gate_w: [128]
            // Expand gate_w for broadcasting: [128] -> [1, 1, 128]
            let gate_w_expanded = self.gate_w.clone()
                .unsqueeze_dim::<2>(0)  // [1, 128]
                .unsqueeze_dim::<3>(0); // [1, 1, 128]
            let gate_logits = input.clone().mul(gate_w_expanded);
            // Sum over last dimension: [batch, n, 128] -> [batch, n, 1]
            let gate_logits = gate_logits.sum_dim(2);
            let delta = burn::tensor::activation::sigmoid(gate_logits);

            // State evolution (highly simplified parallel formulation for MVP)
            // In a real Mamba, this would be a scan; here we approximate with a gated linear layer
            // input: [batch, n, 128], delta: [batch, n, 1]
            // h_updated: [batch, n, 128]
            let h_updated = input
                .clone()
                .matmul(self.state_a.clone().unsqueeze_dim::<3>(0)) // [batch, n, 128]
                .mul(delta.clone());

            // Readout: y = C ⊙ h
            h_updated.mul(self.output_c.clone().unsqueeze_dim::<2>(0).unsqueeze_dim::<3>(0))
        }; // All intermediates freed here

        // Residual connection (skip connection for gradient flow)
        original.add(output)
    }
}


