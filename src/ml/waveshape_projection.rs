/// Neural Waveshape Parameters outputted by the projection
#[derive(Clone, Default, Debug)]
pub struct NeuralWaveshapeParams {
    pub drive: f32,
    pub fold: f32,
    pub asymmetry: f32,
}

/// Project the 128D latent tensor back into bounded physical waveshaping limits.
///
/// **Drive**: Overall field energy and density (dims 0..31)
/// **Fold**: Harmonic compression / clustering (dims 32..63)
/// **Asymmetry**: Directional asymmetry / phase skew (dims 64..95)
pub fn project_latent_to_waveshape(
    latent_embedding: &[f32],
    active_sample_rate_hz: f32,
) -> NeuralWaveshapeParams {
    if latent_embedding.len() < 96 {
        return NeuralWaveshapeParams::default();
    }

    // 1. DRIVE: Sigmoid projection of dimensions [0..31] -> [0.0, 1.0]
    let drive_raw: f32 = latent_embedding[0..32].iter().sum::<f32>() / 32.0;
    let drive = 1.0 / (1.0 + (-drive_raw).exp());

    // 2. FOLD: Tanh projection of dimensions [32..63] -> [0.0, 1.0]
    let fold_raw: f32 = latent_embedding[32..64].iter().sum::<f32>() / 32.0;
    let fold = fold_raw.tanh().abs() * (active_sample_rate_hz / 192000.0).min(1.0);

    // 3. ASYMMETRY: Tanh projection of dimensions [64..95] -> [-1.0, 1.0]
    let asym_raw: f32 = latent_embedding[64..96].iter().sum::<f32>() / 32.0;
    let asymmetry = asym_raw.tanh();

    NeuralWaveshapeParams {
        drive,
        fold,
        asymmetry,
    }
}
