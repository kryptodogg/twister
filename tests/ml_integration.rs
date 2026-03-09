use burn::backend::NdArray;
use burn::tensor::Tensor;
use burn::tensor::backend::Backend;
use twister::ml::modular_features::{
    FeatureFlags, ModularFeatureEncoder, SignalFeaturePayload, VideoFrame,
};

type B = NdArray;

#[test]
fn test_feature_dims() {
    let mut flags = FeatureFlags::default();

    // Baseline audio only
    assert_eq!(flags.total_audio_dim(), 196);
    assert_eq!(flags.total_visual_dim(), 0);
    assert_eq!(flags.total_dim(), 196);

    // Audio with visual
    flags.use_visual_microphone = true;
    flags.visual_preserve_rgb_separation = true;
    flags.visual_num_frequency_bins = 3;

    // RGB visual: (3*3) + 12 + 3 + 4 + 4 = 9 + 23 = 32
    assert_eq!(flags.total_visual_dim(), 32);
    assert_eq!(flags.total_dim(), 196 + 32);

    // With more features
    flags.use_anc_phase = true; // +64
    assert_eq!(flags.total_dim(), 196 + 32 + 64);
}

#[test]
fn test_modular_encoder_forward() {
    let device = Default::default();

    let mut flags = FeatureFlags::default();
    flags.use_visual_microphone = true;
    flags.visual_preserve_rgb_separation = true;
    flags.visual_num_frequency_bins = 3;

    let encoder = ModularFeatureEncoder::<B>::new(flags, &device);

    // total_dim should be 196 + 32 = 228
    let total_dim = flags.total_dim();

    // Create random input
    let batch_size = 16;
    let input: Tensor<B, 2> = Tensor::zeros([batch_size, total_dim], &device);

    let (latent, mse, importance) = encoder.forward(input);

    assert_eq!(latent.dims(), [batch_size, 128]);
    assert_eq!(mse.dims(), [batch_size]);
    assert_eq!(importance.flags_visual_dim, 32);
}
