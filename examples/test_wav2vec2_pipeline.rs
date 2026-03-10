use burn::backend::Wgpu;
use twister::ml::wav2vec2_loader::Wav2Vec2Model;
use twister::ml::multimodal_fusion::MultimodalFeature;
use burn::tensor::Device;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Testing Wav2Vec2 Pipeline on GPU ===");

    // 1. Initialize Device
    let device = Device::<Wgpu>::default();
    println!("Device Initialized.");

    // 2. Load Model
    println!("Loading facebook/wav2vec2-base-960h model...");
    let model = Wav2Vec2Model::<Wgpu>::load(&device).await?;
    println!("Model loaded successfully.");

    // 3. Create Dummy Audio (1 second at 16kHz)
    let dummy_audio = vec![0.1f32; 16000];

    // 4. Inference
    println!("Running inference on 1s of audio...");
    let start = std::time::Instant::now();
    let embedding = model.embed(&dummy_audio)?;
    let duration = start.elapsed();

    println!("Inference complete in {:?}", duration);
    println!("Embedding shape: [{}]", embedding.len());
    assert_eq!(embedding.len(), 768, "Embedding should be 768-D");

    // 5. Dummy features for fusion
    let dummy_audio_feat = [0.05f32; 196];
    let dummy_ray_feat = [0.15f32; 128];

    // Convert Vec to array for fusion (safe since length is verified above)
    let mut embedding_array = [0.0f32; 768];
    embedding_array.copy_from_slice(&embedding);

    // 6. Multimodal Fusion
    println!("Fusing modalities: [196D audio + 128D ray + 768D wav2vec2]...");
    let fused_feature = MultimodalFeature::fuse(
        &dummy_audio_feat,
        &dummy_ray_feat,
        &embedding_array
    );

    println!("Fused feature shape: [{}]", fused_feature.fused.len());
    assert_eq!(fused_feature.fused.len(), 1092, "Fused output should be 1092-D");

    // Check normalization on fused feature
    let mut sum_sq = 0.0;
    for &val in &fused_feature.fused[0..196] {
        sum_sq += val * val;
    }
    println!("L2 Norm of Audio section in Fused Feature: {:.4} (should be ~1.0)", sum_sq.sqrt());

    println!("=== Pipeline Test Successful ===");

    Ok(())
}
