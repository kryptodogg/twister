//! Mamba autoencoder benchmark

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use twister::mamba::{SSAMBAConfig, MambaInference};
use twister::dsp::features::FeatureVector;

fn benchmark_mamba_inference(c: &mut Criterion) {
    let mut group = c.benchmark_group("mamba_inference");
    
    let config = SSAMBAConfig::new();
    let inference = MambaInference::new(&config);
    let features = FeatureVector::zeros();
    
    group.bench_function("infer_zeros", |b| {
        b.iter(|| {
            black_box(inference.infer(&features));
        })
    });
    
    group.finish();
}

fn benchmark_mamba_configurations(c: &mut Criterion) {
    let mut group = c.benchmark_group("mamba_configs");
    
    for latent_dim in [32, 64, 128].iter() {
        let config = SSAMBAConfig {
            latent_dim: *latent_dim,
            ..SSAMBAConfig::new()
        };
        
        let inference = MambaInference::new(&config);
        let features = FeatureVector::zeros();
        
        group.bench_with_input(
            BenchmarkId::new("latent_dim", latent_dim),
            &features,
            |b, features| {
                b.iter(|| {
                    black_box(inference.infer(features));
                })
            },
        );
    }
    
    group.finish();
}

fn benchmark_feature_extraction(c: &mut Criterion) {
    use twister::dsp::{FeatureExtractor, WelchPSD, PSDConfig};
    use num_complex::Complex;
    
    let mut group = c.benchmark_group("feature_extraction");
    
    let extractor = FeatureExtractor::new(2_048_000, 192_000);
    
    // Generate test IQ data
    let iq: Vec<Complex<f32>> = (0..1024)
        .map(|i| Complex::new((i as f32 * 0.1).sin(), (i as f32 * 0.1).cos()))
        .collect();
    
    // Generate test audio data
    let audio: Vec<Vec<f32>> = (0..3)
        .map(|ch| (0..1024).map(|i| ((i + ch * 100) as f32 * 0.01).sin()).collect())
        .collect();
    
    group.bench_function("extract_rf_features", |b| {
        b.iter(|| {
            black_box(extractor.extract_rf_features(&iq));
        })
    });
    
    group.bench_function("extract_audio_features", |b| {
        b.iter(|| {
            black_box(extractor.extract_audio_features(&audio));
        })
    });
    
    group.bench_function("extract_features_combined", |b| {
        b.iter(|| {
            black_box(extractor.extract_features_default(&iq, &audio));
        })
    });
    
    group.finish();
}

fn benchmark_psd_computation(c: &mut Criterion) {
    use twister::dsp::{WelchPSD, PSDConfig};
    
    let mut group = c.benchmark_group("psd_computation");
    
    for fft_size in [256, 512, 1024].iter() {
        let config = PSDConfig {
            fft_size: *fft_size,
            ..PSDConfig::default()
        };
        
        let psd = WelchPSD::new(config);
        let samples: Vec<f32> = (0..*fft_size * 4).map(|i| (i as f32 * 0.01).sin()).collect();
        
        group.bench_with_input(
            BenchmarkId::new("fft_size", fft_size),
            &samples,
            |b, samples| {
                b.iter(|| {
                    black_box(psd.compute(samples));
                })
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(100);
    targets = benchmark_mamba_inference, 
              benchmark_mamba_configurations, 
              benchmark_feature_extraction,
              benchmark_psd_computation
);

criterion_main!(benches);
