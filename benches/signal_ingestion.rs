// benches/signal_ingestion.rs — Signal Ingestion Performance Benchmarks
//
// Benchmarks for Track B components:
// - IQ dispatch throughput
// - STFT FFT latency
// - V-Buffer context window extraction
//
// Run with: cargo bench --bench signal_ingestion

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::sync::Arc;
use tokio::runtime::Runtime;

use twister::hardware_io::dma_vbuffer::{IqDmaGateway, DMA_CHUNK_SAMPLES};
use twister::visualization::stft_pipeline::{StftProcessor, FFT_SIZE};
use twister::vbuffer::{GpuVBuffer, V_DEPTH, V_FREQ_BINS};

/// Benchmark IQ DMA throughput
fn bench_dma_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (device, queue) = rt.block_on(create_wgpu_device());
    let device = Arc::new(device);
    let queue = Arc::new(queue);

    let mut gateway = IqDmaGateway::new(Arc::clone(&device), Arc::clone(&queue), 64);
    let test_buffer = vec![0u8; DMA_CHUNK_SAMPLES * 2];

    let mut group = c.benchmark_group("dma_throughput");
    group.throughput(criterion::Throughput::Bytes(DMA_CHUNK_SAMPLES as u64 * 2));

    group.bench_function("push_dma_chunk", |b| {
        b.iter(|| {
            gateway.push_dma_chunk(black_box(&test_buffer)).unwrap();
        });
    });

    group.finish();
}

/// Benchmark STFT FFT latency
fn bench_stft_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (device, queue) = rt.block_on(create_wgpu_device());
    let device = Arc::new(device);
    let queue = Arc::new(queue);

    let mut processor = StftProcessor::new(Arc::clone(&device), Arc::clone(&queue)).unwrap();
    let test_iq_samples = vec![[0i8, 0i8]; FFT_SIZE];

    let mut group = c.benchmark_group("stft_latency");
    group.measurement_time(std::time::Duration::from_secs(10));

    group.bench_function("process_frame_512", |b| {
        b.iter(|| {
            processor.process_frame(black_box(&test_iq_samples)).unwrap();
        });
    });

    group.finish();
}

/// Benchmark V-Buffer context window extraction
fn bench_vbuffer_context_window(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (device, queue) = rt.block_on(create_wgpu_device());
    let device = Arc::new(device);
    let queue = Arc::new(queue);

    let mut vbuffer = GpuVBuffer::new(&device);

    // Pre-fill buffer with test data
    let test_data = vec![[half::f16::from_f32(0.5); 4]; V_FREQ_BINS];
    for _ in 0..100 {
        vbuffer.push_frame(&queue, &test_data);
    }

    let mut group = c.benchmark_group("vbuffer_context_window");

    for n_frames in [10, 50, 100, 256] {
        group.bench_with_parameter(
            BenchmarkId::from_parameter(n_frames),
            n_frames,
            |b, n_frames| {
                b.iter(|| {
                    let window = vbuffer.get_context_window(*n_frames);
                    black_box(window);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark V-Buffer push_frame latency
fn bench_vbuffer_push_frame(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (device, queue) = rt.block_on(create_wgpu_device());
    let device = Arc::new(device);
    let queue = Arc::new(queue);

    let mut vbuffer = GpuVBuffer::new(&device);
    let test_data = vec![[half::f16::from_f32(0.5); 4]; V_FREQ_BINS];

    let mut group = c.benchmark_group("vbuffer_push_frame");
    group.throughput(criterion::Throughput::Elements(1));

    group.bench_function("push_frame", |b| {
        b.iter(|| {
            vbuffer.push_frame(black_box(&queue), black_box(&test_data));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_dma_throughput,
    bench_stft_latency,
    bench_vbuffer_context_window,
    bench_vbuffer_push_frame,
);

criterion_main!(benches);

/// Helper to create a wgpu device for benchmarking
async fn create_wgpu_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("Failed to find an appropriate adapter");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Benchmark Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        )
        .await
        .expect("Failed to create device");

    (device, queue)
}
