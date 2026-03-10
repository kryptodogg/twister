// examples/test_iq_dispatch_loop.rs — IQ Dispatch Loop Integration Test
//
// Tests the Tokio-based IQ sample dispatch loop:
//   DeviceManager → read_sync() → IqDmaGateway → GPU VRAM
//
// Run with: cargo test --example test_iq_dispatch_loop

use std::sync::Arc;
use tokio::time::{Duration, timeout};

// Import twister modules
use twister::app_state::DirtyFlags;
use twister::dispatch::iq_dispatch::{BYTES_PER_FRAME, IqDispatchLoop, SAMPLES_PER_FRAME};
use twister::hardware_io::device_manager::DeviceManager;
use twister::hardware_io::dma_vbuffer::{DMA_CHUNK_SAMPLES, IqDmaGateway};

#[tokio::test]
async fn test_dispatch_loop_creation() {
    // Create device manager
    let dirty_flags = Arc::new(DirtyFlags::new());
    let device_manager = Arc::new(DeviceManager::new(dirty_flags));

    // Create DMA gateway (requires wgpu device)
    let (device, queue) = create_wgpu_device().await;
    let dma_gateway = Arc::new(std::sync::Mutex::new(IqDmaGateway::new(
        Arc::new(device),
        Arc::new(queue),
        64,
    )));

    // Create dispatch loop
    let mut dispatch = IqDispatchLoop::new(device_manager, dma_gateway);

    // Verify initial state
    assert_eq!(dispatch.frame_count(), 0);
    assert_eq!(dispatch.dropped_frames(), 0);

    // Stop immediately (no devices connected)
    dispatch.stop();
}

#[tokio::test]
async fn test_dispatch_loop_no_devices() {
    let dirty_flags = Arc::new(DirtyFlags::new());
    let device_manager = Arc::new(DeviceManager::new(dirty_flags));

    let (device, queue) = create_wgpu_device().await;
    let dma_gateway = Arc::new(std::sync::Mutex::new(IqDmaGateway::new(
        Arc::new(device),
        Arc::new(queue),
        64,
    )));

    let mut dispatch = IqDispatchLoop::new(device_manager, dma_gateway);

    // Run for 100ms with no devices (should not panic)
    let result = timeout(Duration::from_millis(100), dispatch.run()).await;

    // Should timeout (not error) because loop runs indefinitely
    assert!(result.is_err()); // timeout::error::Elapsed

    dispatch.stop();
}

#[tokio::test]
async fn test_dispatch_frame_constants() {
    // Verify frame timing constants
    assert_eq!(SAMPLES_PER_FRAME, 65536);
    assert_eq!(BYTES_PER_FRAME, 131072);
    assert_eq!(DMA_CHUNK_SAMPLES, 16384);

    // Verify chunk fits in frame
    assert!(BYTES_PER_FRAME % (DMA_CHUNK_SAMPLES * 2) == 0);
}

#[test]
fn test_iq_buffer_layout() {
    // IQ samples are interleaved [I, Q, I, Q, ...]
    // Each sample is 2 bytes (i8, i8)
    let sample_size = std::mem::size_of::<[i8; 2]>();
    assert_eq!(sample_size, 2);

    // Frame buffer layout
    let frame_buffer = vec![0u8; BYTES_PER_FRAME];
    assert_eq!(frame_buffer.len(), BYTES_PER_FRAME);
    assert_eq!(frame_buffer.len() / 2, SAMPLES_PER_FRAME); // Complex samples
}

/// Helper to create a wgpu device
async fn create_wgpu_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("Failed to find an appropriate adapter");

    println!("   Adapter: {:?}", adapter.get_info().name);

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("Signal Ingestion Demo Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
        })
        .await
        .expect("Failed to create device");

    (device, queue)
}

fn main() {
    println!("Run with: cargo test --example test_iq_dispatch_loop");
}
