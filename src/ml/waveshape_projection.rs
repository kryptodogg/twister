/// Neural Waveshape Parameters outputted by the projection
#[derive(Clone, Default, Debug)]
pub struct NeuralWaveshapeParams {
    pub drive: f32,
    pub foldback: f32,
    pub asymmetry: f32,
}

/// Project the 128D latent tensor back into bounded physical waveshaping limits.
///
/// **Drive**: Sigmoid projection of dims [0..31] -> [0.0, 1.0]
/// **Foldback**: Tanh projection of dims [32..63] -> [0.0, 1.0], scaled dynamically by SR.
/// **Asymmetry**: Tanh projection of dims [64..95] -> [-1.0, 1.0]
pub fn project_latent_to_waveshape(
    latent_embedding: &[f32; 128],
    active_sample_rate_hz: f32,
) -> NeuralWaveshapeParams {
    // 1. DRIVE: Sigmoid projection of dimensions [0..31] -> [0.0, 1.0]
    let drive_raw: f32 = latent_embedding[0..32].iter().sum::<f32>() / 32.0;
    let drive = 1.0 / (1.0 + (-drive_raw).exp());

    // 2. FOLDBACK: Tanh projection of dimensions [32..63] -> [0.0, 1.0]
    // Scaled dynamically by active_sample_rate_hz / 192000.0
    let foldback_raw: f32 = latent_embedding[32..64].iter().sum::<f32>() / 32.0;
    let foldback = foldback_raw.tanh().abs() * (active_sample_rate_hz / 192000.0);

    // 3. ASYMMETRY: Tanh projection of dimensions [64..95] -> [-1.0, 1.0]
    let asym_raw: f32 = latent_embedding[64..96].iter().sum::<f32>() / 32.0;
    let asymmetry = asym_raw.tanh(); // Bounded [-1.0, 1.0]

    NeuralWaveshapeParams {
        drive,
        foldback,
        asymmetry,
    }
}
