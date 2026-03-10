//! BSS (Blind Source Separation) benchmark

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use twister::dsp::{BSSProcessor, BSSConfig, RLSFilter};

fn benchmark_bss_processor(c: &mut Criterion) {
    let mut group = c.benchmark_group("bss_processor");
    
    for num_channels in [2, 3, 4].iter() {
        let config = BSSConfig {
            num_channels: *num_channels,
            filter_length: 64,
            forgetting_factor: 0.99,
            regularization: 0.001,
            block_size: 256,
            algorithm: twister::dsp::bss::BSSAlgorithm::RLS,
        };
        
        let mut processor = BSSProcessor::new(config);
        
        // Create test inputs
        let inputs: Vec<Vec<f32>> = (0..*num_channels)
            .map(|ch| (0..256).map(|i| ((i + ch * 100) as f32 * 0.01).sin()).collect())
            .collect();
        
        group.bench_with_input(
            BenchmarkId::new("process", num_channels),
            &inputs,
            |b, inputs| {
                b.iter(|| {
                    let mut p = BSSProcessor::new(BSSConfig {
                        num_channels: *num_channels,
                        ..BSSConfig::default()
                    });
                    black_box(p.process(inputs));
                })
            },
        );
    }
    
    group.finish();
}

fn benchmark_rls_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("rls_filter");
    
    for filter_length in [32, 64, 128].iter() {
        let mut filter = RLSFilter::new(2, *filter_length, 0.99, 0.001);
        
        let input_data: Vec<f32> = (0..256).map(|i| (i as f32 * 0.01).sin()).collect();
        let desired: Vec<f32> = (0..256).map(|i| (i as f32 * 0.01).sin()).collect();
        
        let input_view = ndarray::ArrayView1::from(&input_data);
        
        group.bench_with_input(
            BenchmarkId::new("process", filter_length),
            &input_view,
            |b, input| {
                b.iter(|| {
                    let mut f = RLSFilter::new(2, *filter_length, 0.99, 0.001);
                    black_box(f.process(&[*input], &desired));
                })
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(100);
    targets = benchmark_bss_processor, benchmark_rls_filter
);

criterion_main!(benches);
