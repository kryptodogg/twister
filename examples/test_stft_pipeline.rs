// examples/test_stft_pipeline.rs — STFT GPU Pipeline Integration Test
//
// Tests the Short-Time Fourier Transform compute pipeline:
//   IQ Buffer → FFT Shader → Magnitude Output → V-Buffer
//
// Run with: cargo test --example test_stft_pipeline

use std::sync::Arc;
use twister::visualization::stft_pipeline::{StftPipeline, StftProcessor, FFT_SIZE, FREQ_BINS};
use twister::vbuffer::GpuVBuffer;

#[tokio::test]
async fn test_stft_pipeline_creation() {
    let (device, queue) = create_wgpu_device().await;

    let result = StftPipeline::new(
        Arc::new(device),
        Arc::new(queue),
    );

    assert!(result.is_ok(), "STFT pipeline creation failed: {:?}", result.err());
}

#[tokio::test]
async fn test_stft_processor_creation() {
    let (device, queue) = create_wgpu_device().await;

    let result = StftProcessor::new(
        Arc::new(device),
        Arc::new(queue),
    );

    assert!(result.is_ok(), "STFT processor creation failed: {:?}", result.err());
}

#[tokio::test]
async fn test_stft_constants() {
    // Verify FFT size matches shader expectation
    assert_eq!(FFT_SIZE, 512);
    assert_eq!(FREQ_BINS, 512);

    // Verify power of 2 for Radix-2 FFT
    assert!(FFT_SIZE.is_power_of_two());
    assert_eq!(FFT_SIZE.ilog2(), 9); // 2^9 = 512
}

#[test]
fn test_iq_sample_layout() {
    // IQ samples are [i8; 2] (interleaved I, Q)
    let sample_size = std::mem::size_of::<[i8; 2]>();
    assert_eq!(sample_size, 2);

    // Full frame size
    let frame_bytes = FFT_SIZE * 2;
    assert_eq!(frame_bytes, 1024);
}

#[tokio::test]
async fn test_vbuffer_integration() {
    let (device, _) = create_wgpu_device().await;

    // Create V-buffer
    let mut vbuffer = GpuVBuffer::new(&device);

    // Verify initial state
    assert_eq!(vbuffer.version(), 0);
    assert!(!vbuffer.ready(10));

    // Push some test frames
    let test_data = vec![[half::f16::from_f32(0.5); 4]; FREQ_BINS];
    vbuffer.push_frame(device.queue(), &test_data);

    assert_eq!(vbuffer.version(), 1);
    assert!(vbuffer.ready(1));
}

/// Helper to create a wgpu device for testing
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
                label: Some("Test Device"),
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
