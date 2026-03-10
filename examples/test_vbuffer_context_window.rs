// examples/test_vbuffer_context_window.rs — V-Buffer Context Window Test
//
// Tests the context window API for extracting contiguous frame sequences:
//   get_context_window(n_frames) → VBufferContextWindow
//
// Run with: cargo test --example test_vbuffer_context_window

use std::sync::Arc;
use twister::vbuffer::{GpuVBuffer, V_DEPTH, V_FREQ_BINS};

#[tokio::test]
async fn test_vbuffer_context_window_empty() {
    let (device, _) = create_wgpu_device().await;
    let vbuffer = GpuVBuffer::new(&device);

    // Empty buffer should return empty window
    let window = vbuffer.get_context_window(10);
    assert_eq!(window.n_frames, 0);
    assert_eq!(window.data.len(), 0);
    assert_eq!(window.start_version, 0);
    assert_eq!(window.end_version, 0);
}

#[tokio::test]
async fn test_vbuffer_context_window_single_frame() {
    let (device, queue) = create_wgpu_device().await;
    let mut vbuffer = GpuVBuffer::new(&device);

    // Push one frame
    let test_data = create_test_frame(0.5);
    vbuffer.push_frame(&queue, &test_data);

    // Request context window
    let window = vbuffer.get_context_window(1);
    assert_eq!(window.n_frames, 1);
    assert_eq!(window.start_version, 0);
    assert_eq!(window.end_version, 1);
}

#[tokio::test]
async fn test_vbuffer_context_window_multiple_frames() {
    let (device, queue) = create_wgpu_device().await;
    let mut vbuffer = GpuVBuffer::new(&device);

    // Push 10 frames
    for i in 0..10 {
        let test_data = create_test_frame(i as f32 / 10.0);
        vbuffer.push_frame(&queue, &test_data);
    }

    assert_eq!(vbuffer.version(), 10);

    // Request context window of 5 frames
    let window = vbuffer.get_context_window(5);
    assert_eq!(window.n_frames, 5);
    assert_eq!(window.start_version, 5); // 10 - 5 = 5
    assert_eq!(window.end_version, 10);
    assert_eq!(window.data.len(), 5 * V_FREQ_BINS);
}

#[tokio::test]
async fn test_vbuffer_context_window_wraparound() {
    let (device, queue) = create_wgpu_device().await;
    let mut vbuffer = GpuVBuffer::new(&device);

    // Push more than V_DEPTH frames to trigger wraparound
    let n_frames = V_DEPTH + 10;
    for i in 0..n_frames {
        let test_data = create_test_frame(i as f32 / n_frames as f32);
        vbuffer.push_frame(&queue, &test_data);
    }

    // Request full depth window
    let window = vbuffer.get_context_window(V_DEPTH);
    assert_eq!(window.n_frames, V_DEPTH);
    assert_eq!(window.data.len(), V_DEPTH * V_FREQ_BINS);
}

#[tokio::test]
async fn test_vbuffer_context_window_get_frame() {
    let (device, queue) = create_wgpu_device().await;
    let mut vbuffer = GpuVBuffer::new(&device);

    // Push 5 frames with distinct values
    for i in 0..5 {
        let test_data = create_test_frame(i as f32);
        vbuffer.push_frame(&queue, &test_data);
    }

    let window = vbuffer.get_context_window(5);

    // Get first frame (oldest)
    let frame0 = window.get_frame(0);
    assert!(frame0.is_some());
    assert_eq!(frame0.unwrap().len(), V_FREQ_BINS);

    // Get last frame (newest)
    let frame4 = window.get_frame(4);
    assert!(frame4.is_some());

    // Out of bounds
    let frame_oob = window.get_frame(10);
    assert!(frame_oob.is_none());
}

#[tokio::test]
async fn test_vbuffer_context_window_iteration() {
    let (device, queue) = create_wgpu_device().await;
    let mut vbuffer = GpuVBuffer::new(&device);

    // Push 3 frames
    for i in 0..3 {
        let test_data = create_test_frame(i as f32);
        vbuffer.push_frame(&queue, &test_data);
    }

    let window = vbuffer.get_context_window(3);

    // Iterate over frames
    let mut count = 0;
    for (idx, frame) in window.frames() {
        assert_eq!(idx, count);
        assert_eq!(frame.len(), V_FREQ_BINS);
        count += 1;
    }

    assert_eq!(count, 3);
}

#[test]
fn test_vbuffer_constants() {
    assert_eq!(V_DEPTH, 512);
    assert_eq!(V_FREQ_BINS, 512);
    assert!(V_DEPTH.is_power_of_two());
}

/// Helper to create test frame data
fn create_test_frame(value: f32) -> Vec<[half::f16; 4]> {
    vec![
        [
            half::f16::from_f32(value),
            half::f16::from_f32(0.0),
            half::f16::from_f32(0.0),
            half::f16::from_f32(0.0),
        ];
        V_FREQ_BINS
    ]
}

/// Helper to create a wgpu device for testing
async fn create_wgpu_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("Failed to find an appropriate adapter");

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("Test Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
        })
        .await
        .expect("Failed to create device");

    (device, queue)
}

fn main() {
    println!("Run with: cargo test --example test_vbuffer_context_window");
}
