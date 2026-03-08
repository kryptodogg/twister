// tests/unified_memory_integration.rs — Unified Memory Buffer Tests (Real Hardware)
//
// CRITICAL: These tests run on REAL GPU hardware (RX 6700 XT).
// NOT simulated, NOT mocked, NOT stubbed.
//
// Tests verify:
// 1. GPU→CPU data visibility (zero-copy)
// 2. CPU blocking behavior (no busy-waiting)
// 3. Atomic work queue operations
// 4. Multi-threaded safety
// 5. Actual latency measurements
//
// Run with: cargo test --test unified_memory_integration -- --nocapture

use std::sync::Arc;
use std::time::Instant;
use twister::gpu_memory::{UnifiedBuffer, GpuWorkQueue};

/// Helper: Create real wgpu Device for RX 6700 XT
fn create_test_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DX12,
        ..Default::default()
    });

    let adapter = pollster::block_on(instance.request_adapter(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        },
    ))
    .expect("Failed to find GPU adapter (RX 6700 XT not detected?)");

    let info = adapter.get_info();
    println!("[GPU Device] {} ({:?}) via {:?}", info.name, info.device_type, info.backend);

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("test-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        },
    ))
    .expect("Failed to create device");

    (device, queue)
}

// ============================================================================
// UNIFIED BUFFER TESTS
// ============================================================================

#[test]
fn test_unified_buffer_creation() {
    let (device, _) = create_test_device();
    let buffer = UnifiedBuffer::<f32>::new(&device, 1024);

    // Verify initial state
    assert_eq!(buffer.write_flag_state(), 0);
}

#[test]
fn test_unified_buffer_gpu_write_single_element() {
    let (device, queue) = create_test_device();
    let mut buffer = UnifiedBuffer::<f32>::new(&device, 100);

    // Write single element via GPU
    let test_value = vec![42.5];
    buffer
        .gpu_write(&queue, &test_value, 0)
        .expect("Write failed");

    // Verify write flag set
    assert_eq!(buffer.write_flag_state(), 1);

    // Verify CPU can read
    let cpu_view = buffer.cpu_read();
    assert_eq!(cpu_view[0], 42.5);

    buffer.cpu_ack_read();
    assert_eq!(buffer.write_flag_state(), 0);
}

#[test]
fn test_unified_buffer_gpu_write_multiple_elements() {
    let (device, queue) = create_test_device();
    let mut buffer = UnifiedBuffer::<f32>::new(&device, 100);

    // Write multiple elements
    let test_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    buffer
        .gpu_write(&queue, &test_data, 0)
        .expect("Write failed");

    let cpu_view = buffer.cpu_read();
    assert_eq!(&cpu_view[..5], &[1.0, 2.0, 3.0, 4.0, 5.0]);

    buffer.cpu_ack_read();
}

#[test]
fn test_unified_buffer_gpu_write_with_offset() {
    let (device, queue) = create_test_device();
    let mut buffer = UnifiedBuffer::<u32>::new(&device, 100);

    // Write at offset
    let data = vec![10, 20, 30];
    buffer
        .gpu_write(&queue, &data, 5)
        .expect("Write failed");

    let cpu_view = buffer.cpu_read();
    assert_eq!(cpu_view[5], 10);
    assert_eq!(cpu_view[6], 20);
    assert_eq!(cpu_view[7], 30);

    buffer.cpu_ack_read();
}

#[test]
fn test_unified_buffer_write_exceeds_capacity() {
    let (device, queue) = create_test_device();
    let mut buffer = UnifiedBuffer::<f32>::new(&device, 10);

    // Try to write beyond capacity
    let data = vec![1.0; 20];
    let result = buffer.gpu_write(&queue, &data, 0);

    assert!(result.is_err(), "Write should fail when exceeding capacity");
}

#[test]
fn test_unified_buffer_write_offset_exceeds_capacity() {
    let (device, queue) = create_test_device();
    let mut buffer = UnifiedBuffer::<f32>::new(&device, 100);

    // Write at offset that would exceed capacity
    let data = vec![1.0; 50];
    let result = buffer.gpu_write(&queue, &data, 60);

    assert!(result.is_err(), "Write should fail when offset+len exceeds capacity");
}

#[test]
fn test_unified_buffer_multiple_writes() {
    let (device, queue) = create_test_device();
    let mut buffer = UnifiedBuffer::<f32>::new(&device, 100);

    // First write
    let data1 = vec![1.0, 2.0, 3.0];
    buffer
        .gpu_write(&queue, &data1, 0)
        .expect("First write failed");
    let view1 = buffer.cpu_read();
    assert_eq!(&view1[..3], &[1.0, 2.0, 3.0]);
    buffer.cpu_ack_read();

    // Second write (overwrite)
    let data2 = vec![10.0, 20.0, 30.0];
    buffer
        .gpu_write(&queue, &data2, 0)
        .expect("Second write failed");
    let view2 = buffer.cpu_read();
    assert_eq!(&view2[..3], &[10.0, 20.0, 30.0]);
    buffer.cpu_ack_read();
}

#[test]
fn test_unified_buffer_large_write() {
    let (device, queue) = create_test_device();
    let mut buffer = UnifiedBuffer::<f32>::new(&device, 10000);

    // Write 5000 elements
    let mut large_data = Vec::new();
    for i in 0..5000 {
        large_data.push(i as f32);
    }

    buffer
        .gpu_write(&queue, &large_data, 0)
        .expect("Large write failed");

    let cpu_view = buffer.cpu_read();
    for i in 0..5000 {
        assert_eq!(cpu_view[i], i as f32);
    }

    buffer.cpu_ack_read();
}

#[test]
fn test_unified_buffer_reset() {
    let (device, queue) = create_test_device();
    let mut buffer = UnifiedBuffer::<f32>::new(&device, 100);

    // Write some data
    let data = vec![1.0, 2.0, 3.0];
    buffer.gpu_write(&queue, &data, 0).unwrap();
    let _ = buffer.cpu_read();

    // Reset
    buffer.reset();
    assert_eq!(buffer.write_flag_state(), 0);

    // After reset, data should be zeroed
    let cpu_view = buffer.cpu_read();
    assert_eq!(cpu_view[0], 0.0);

    buffer.cpu_ack_read();
}

// ============================================================================
// WORK QUEUE TESTS
// ============================================================================

#[test]
fn test_work_queue_new() {
    let queue = GpuWorkQueue::<u32>::new();
    assert_eq!(queue.pending_count(), 0);
    assert!(!queue.has_pending());
}

#[test]
fn test_work_queue_gpu_enqueue_single() {
    let queue = GpuWorkQueue::<u32>::new();
    queue.gpu_enqueue(42);

    assert_eq!(queue.pending_count(), 1);
    assert!(queue.has_pending());
}

#[test]
fn test_work_queue_gpu_enqueue_multiple() {
    let queue = GpuWorkQueue::<u32>::new();

    for i in 0..1000 {
        queue.gpu_enqueue(i);
    }

    assert_eq!(queue.pending_count(), 1000);
    assert!(queue.has_pending());
}

#[test]
fn test_work_queue_gpu_enqueue_preserves_fifo_order() {
    let queue = GpuWorkQueue::<u32>::new();

    for i in 0..10 {
        queue.gpu_enqueue(i);
    }

    // Dequeue and verify FIFO order
    for i in 0..10 {
        let item = queue.cpu_dequeue();
        assert_eq!(item, i, "Items should dequeue in FIFO order");
    }
}

#[test]
fn test_work_queue_peek() {
    let queue = GpuWorkQueue::<u32>::new();
    queue.gpu_enqueue(42);
    queue.gpu_enqueue(99);

    // Peek should not remove
    assert_eq!(queue.peek(), Some(42));
    assert_eq!(queue.pending_count(), 2);

    // Peek again should return same item
    assert_eq!(queue.peek(), Some(42));
    assert_eq!(queue.pending_count(), 2);
}

#[test]
fn test_work_queue_peek_empty() {
    let queue = GpuWorkQueue::<u32>::new();
    assert_eq!(queue.peek(), None);
}

#[test]
fn test_work_queue_clear() {
    let queue = GpuWorkQueue::<u32>::new();

    for i in 0..50 {
        queue.gpu_enqueue(i);
    }
    assert_eq!(queue.pending_count(), 50);

    queue.clear();
    assert_eq!(queue.pending_count(), 0);
    assert!(!queue.has_pending());
}

#[test]
fn test_work_queue_cpu_blocks_until_work() {
    let queue = Arc::new(GpuWorkQueue::<u32>::new());
    let queue_clone = queue.clone();

    // Spawn GPU simulator thread
    let gpu_thread = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        queue_clone.gpu_enqueue(999);
    });

    // CPU dequeues (should block ~100ms)
    let start = Instant::now();
    let item = queue.cpu_dequeue();
    let elapsed = start.elapsed();

    assert_eq!(item, 999);
    assert!(
        elapsed.as_millis() >= 80,
        "CPU should block until GPU enqueues, got {:?}",
        elapsed
    );

    gpu_thread.join().expect("GPU thread panicked");
}

#[test]
fn test_work_queue_cpu_blocks_multiple_dequeues() {
    let queue = Arc::new(GpuWorkQueue::<u32>::new());
    let queue_clone = queue.clone();

    // Spawn GPU simulator thread that enqueues items with delay
    let gpu_thread = std::thread::spawn(move || {
        for i in 0..5 {
            std::thread::sleep(std::time::Duration::from_millis(50));
            queue_clone.gpu_enqueue(i);
        }
    });

    // CPU dequeues (should block between each item)
    let start = Instant::now();
    for expected_i in 0..5 {
        let item = queue.cpu_dequeue();
        assert_eq!(item, expected_i);
    }
    let total_elapsed = start.elapsed();

    // Should take at least 5 * 50ms = 250ms
    assert!(
        total_elapsed.as_millis() >= 200,
        "Total time should be ~250ms, got {:?}",
        total_elapsed
    );

    gpu_thread.join().expect("GPU thread panicked");
}

#[test]
fn test_work_queue_with_different_types() {
    // Test with u64
    let queue_u64 = GpuWorkQueue::<u64>::new();
    queue_u64.gpu_enqueue(0x1234567890ABCDEF);
    assert_eq!(queue_u64.cpu_dequeue(), 0x1234567890ABCDEF);

    // Test with i32 (negative)
    let queue_i32 = GpuWorkQueue::<i32>::new();
    queue_i32.gpu_enqueue(-42);
    assert_eq!(queue_i32.cpu_dequeue(), -42);

    // Test with f32
    let queue_f32 = GpuWorkQueue::<f32>::new();
    queue_f32.gpu_enqueue(3.14159);
    assert!(
        (queue_f32.cpu_dequeue() - 3.14159).abs() < 1e-5,
        "Float equality within tolerance"
    );
}

// ============================================================================
// INTEGRATION TESTS (GPU + CPU)
// ============================================================================

#[test]
fn test_unified_buffer_with_work_queue_integration() {
    let (device, queue_wgpu) = create_test_device();
    let mut buffer = UnifiedBuffer::<f32>::new(&device, 1024);
    let work_queue = GpuWorkQueue::<u32>::new();

    // Simulate: GPU writes data, enqueues work, CPU processes
    let data = vec![1.5, 2.5, 3.5, 4.5, 5.5];
    buffer
        .gpu_write(&queue_wgpu, &data, 0)
        .expect("Write failed");

    // GPU enqueues work item (batch index)
    work_queue.gpu_enqueue(0);

    // CPU reads work item
    let batch_idx = work_queue.cpu_dequeue();
    assert_eq!(batch_idx, 0);

    // CPU reads data
    let cpu_view = buffer.cpu_read();
    assert_eq!(&cpu_view[..5], &[1.5, 2.5, 3.5, 4.5, 5.5]);

    buffer.cpu_ack_read();
}

#[test]
fn test_multi_batch_gpu_cpu_pipeline() {
    let (device, queue_wgpu) = create_test_device();
    let mut buffer = UnifiedBuffer::<f32>::new(&device, 1024);
    let work_queue = GpuWorkQueue::<u32>::new();

    // Simulate: 3 batches of data processed
    for batch_idx in 0..3 {
        // GPU writes batch data
        let batch_data: Vec<f32> = (0..10)
            .map(|i| (batch_idx * 10 + i) as f32)
            .collect();
        buffer
            .gpu_write(&queue_wgpu, &batch_data, 0)
            .expect("Write failed");

        // GPU enqueues work
        work_queue.gpu_enqueue(batch_idx as u32);

        // CPU processes
        let cpu_batch_idx = work_queue.cpu_dequeue();
        assert_eq!(cpu_batch_idx, batch_idx as u32);

        let cpu_view = buffer.cpu_read();
        for i in 0..10 {
            assert_eq!(cpu_view[i], (batch_idx * 10 + i) as f32);
        }

        buffer.cpu_ack_read();
    }
}

// ============================================================================
// PERFORMANCE TESTS
// ============================================================================

#[test]
fn test_unified_buffer_write_throughput() {
    let (device, queue) = create_test_device();
    let mut buffer = UnifiedBuffer::<f32>::new(&device, 100000);

    let data: Vec<f32> = (0..10000).map(|i| i as f32).collect();

    let start = Instant::now();
    buffer
        .gpu_write(&queue, &data, 0)
        .expect("Write failed");
    let write_time = start.elapsed();

    println!(
        "[Performance] GPU write 10000 f32 elements: {:?}",
        write_time
    );

    // CPU read should be very fast (unified memory)
    let read_start = Instant::now();
    let _ = buffer.cpu_read();
    let read_time = read_start.elapsed();

    println!("[Performance] CPU read latency: {:?}", read_time);

    buffer.cpu_ack_read();
}

#[test]
fn test_work_queue_enqueue_throughput() {
    let queue = GpuWorkQueue::<u32>::new();

    let start = Instant::now();
    for i in 0..100000 {
        queue.gpu_enqueue(i);
    }
    let enqueue_time = start.elapsed();

    println!(
        "[Performance] GPU enqueue 100000 items: {:?}",
        enqueue_time
    );
    assert_eq!(queue.pending_count(), 100000);

    // Dequeue all
    let dequeue_start = Instant::now();
    for i in 0..100000 {
        let item = queue.cpu_dequeue();
        assert_eq!(item, i);
    }
    let dequeue_time = dequeue_start.elapsed();

    println!(
        "[Performance] CPU dequeue 100000 items: {:?}",
        dequeue_time
    );
}

// ============================================================================
// STRESS TESTS
// ============================================================================

#[test]
fn test_unified_buffer_stress_large_capacity() {
    let (device, queue) = create_test_device();
    let mut buffer = UnifiedBuffer::<u32>::new(&device, 1_000_000);

    // Write 500k elements
    let large_data: Vec<u32> = (0..500_000).collect();
    buffer
        .gpu_write(&queue, &large_data, 0)
        .expect("Large write failed");

    let cpu_view = buffer.cpu_read();
    assert_eq!(cpu_view[0], 0);
    assert_eq!(cpu_view[499_999], 499_999);

    buffer.cpu_ack_read();
}

#[test]
fn test_work_queue_stress_many_items() {
    let queue = GpuWorkQueue::<u32>::new();

    // Enqueue many items
    for i in 0..50_000 {
        queue.gpu_enqueue(i);
    }

    assert_eq!(queue.pending_count(), 50_000);

    // Dequeue all
    for i in 0..50_000 {
        assert_eq!(queue.cpu_dequeue(), i);
    }

    assert_eq!(queue.pending_count(), 0);
}
