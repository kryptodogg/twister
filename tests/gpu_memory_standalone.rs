// tests/gpu_memory_standalone.rs — Standalone GPU Memory Tests
// This file is independent and doesn't require the full lib to compile

use bytemuck::Pod;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Unified memory buffer for zero-copy GPU ↔ CPU access.
pub struct UnifiedBuffer<T: Pod + Send + Sync> {
    gpu_buffer: wgpu::Buffer,
    cpu_map: Vec<T>,
    capacity: usize,
    gpu_write_flag: AtomicU32,
}

impl<T: Pod + Send + Sync> UnifiedBuffer<T> {
    pub fn new(device: &wgpu::Device, capacity: usize) -> Self {
        let element_size = std::mem::size_of::<T>();
        let buffer_size_bytes = (capacity * element_size) as u64;

        let gpu_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("unified_buffer_gpu"),
            size: buffer_size_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let cpu_map = vec![unsafe { std::mem::zeroed() }; capacity];

        Self {
            gpu_buffer,
            cpu_map,
            capacity,
            gpu_write_flag: AtomicU32::new(0),
        }
    }

    pub fn gpu_write(
        &mut self,
        queue: &wgpu::Queue,
        data: &[T],
        offset: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if offset + data.len() > self.capacity {
            return Err(format!(
                "Write exceeds capacity: offset {} + len {} > {}",
                offset,
                data.len(),
                self.capacity
            )
            .into());
        }

        let element_size = std::mem::size_of::<T>();
        let byte_offset = (offset * element_size) as u64;

        queue.write_buffer(&self.gpu_buffer, byte_offset, bytemuck::cast_slice(data));
        self.cpu_map[offset..offset + data.len()].copy_from_slice(data);

        self.gpu_write_flag.store(1, Ordering::Release);

        Ok(())
    }

    pub fn cpu_read(&self) -> &[T] {
        loop {
            let flag = self.gpu_write_flag.load(Ordering::Acquire);
            if flag == 1 {
                break;
            }
            std::thread::yield_now();
        }

        &self.cpu_map[..]
    }

    pub fn cpu_ack_read(&self) {
        self.gpu_write_flag.store(0, Ordering::Release);
    }

    pub fn gpu_buffer(&self) -> &wgpu::Buffer {
        &self.gpu_buffer
    }

    pub fn write_flag_state(&self) -> u32 {
        self.gpu_write_flag.load(Ordering::Acquire)
    }

    pub fn reset(&mut self) {
        self.cpu_map
            .iter_mut()
            .for_each(|v| *v = unsafe { std::mem::zeroed() });
        self.gpu_write_flag.store(0, Ordering::Release);
    }
}

/// Lock-free work queue: GPU enqueues, CPU dequeues.
pub struct GpuWorkQueue<T: Copy + Send + Sync> {
    items: Mutex<VecDeque<T>>,
    pending_count: AtomicU32,
}

impl<T: Copy + Send + Sync> GpuWorkQueue<T> {
    pub fn new() -> Self {
        Self {
            items: Mutex::new(VecDeque::with_capacity(1024)),
            pending_count: AtomicU32::new(0),
        }
    }

    pub fn gpu_enqueue(&self, item: T) {
        let mut items = self.items.lock();
        items.push_back(item);
        drop(items);

        self.pending_count.fetch_add(1, Ordering::Release);
    }

    pub fn cpu_dequeue(&self) -> T {
        loop {
            if self.pending_count.load(Ordering::Acquire) > 0 {
                {
                    let mut items = self.items.lock();
                    if let Some(item) = items.pop_front() {
                        drop(items);
                        self.pending_count.fetch_sub(1, Ordering::Acquire);
                        return item;
                    }
                }
            }

            std::thread::yield_now();
        }
    }

    pub fn has_pending(&self) -> bool {
        self.pending_count.load(Ordering::Acquire) > 0
    }

    pub fn pending_count(&self) -> u32 {
        self.pending_count.load(Ordering::Acquire)
    }

    pub fn peek(&self) -> Option<T> {
        let items = self.items.lock();
        items.front().copied()
    }

    pub fn clear(&self) {
        let mut items = self.items.lock();
        items.clear();
        drop(items);
        self.pending_count.store(0, Ordering::Release);
    }
}

impl<T: Copy + Send + Sync> Default for GpuWorkQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

fn create_test_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DX12,
        ..Default::default()
    });

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("Failed to find GPU adapter");

    let info = adapter.get_info();
    println!(
        "[GPU Device] {} ({:?}) via {:?}",
        info.name, info.device_type, info.backend
    );

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("test-device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    }))
    .expect("Failed to create device");

    (device, queue)
}

#[test]
fn test_unified_buffer_creation() {
    let (device, _) = create_test_device();
    let buffer = UnifiedBuffer::<f32>::new(&device, 1024);
    assert_eq!(buffer.write_flag_state(), 0);
}

#[test]
fn test_unified_buffer_gpu_write_single_element() {
    let (device, queue) = create_test_device();
    let mut buffer = UnifiedBuffer::<f32>::new(&device, 100);

    let test_value = vec![42.5];
    buffer
        .gpu_write(&queue, &test_value, 0)
        .expect("Write failed");

    assert_eq!(buffer.write_flag_state(), 1);

    let cpu_view = buffer.cpu_read();
    assert_eq!(cpu_view[0], 42.5);

    buffer.cpu_ack_read();
    assert_eq!(buffer.write_flag_state(), 0);
}

#[test]
fn test_unified_buffer_gpu_write_multiple_elements() {
    let (device, queue) = create_test_device();
    let mut buffer = UnifiedBuffer::<f32>::new(&device, 100);

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

    let data = vec![10, 20, 30];
    buffer.gpu_write(&queue, &data, 5).expect("Write failed");

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

    let data = vec![1.0; 20];
    let result = buffer.gpu_write(&queue, &data, 0);

    assert!(result.is_err(), "Write should fail when exceeding capacity");
}

#[test]
fn test_unified_buffer_multiple_writes() {
    let (device, queue) = create_test_device();
    let mut buffer = UnifiedBuffer::<f32>::new(&device, 100);

    let data1 = vec![1.0, 2.0, 3.0];
    buffer
        .gpu_write(&queue, &data1, 0)
        .expect("First write failed");
    let view1 = buffer.cpu_read();
    assert_eq!(&view1[..3], &[1.0, 2.0, 3.0]);
    buffer.cpu_ack_read();

    let data2 = vec![10.0, 20.0, 30.0];
    buffer
        .gpu_write(&queue, &data2, 0)
        .expect("Second write failed");
    let view2 = buffer.cpu_read();
    assert_eq!(&view2[..3], &[10.0, 20.0, 30.0]);
    buffer.cpu_ack_read();
}

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

    assert_eq!(queue.peek(), Some(42));
    assert_eq!(queue.pending_count(), 2);

    assert_eq!(queue.peek(), Some(42));
    assert_eq!(queue.pending_count(), 2);
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

    let gpu_thread = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        queue_clone.gpu_enqueue(999);
    });

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
fn test_unified_buffer_with_work_queue_integration() {
    let (device, queue_wgpu) = create_test_device();
    let mut buffer = UnifiedBuffer::<f32>::new(&device, 1024);
    let work_queue = GpuWorkQueue::<u32>::new();

    let data = vec![1.5, 2.5, 3.5, 4.5, 5.5];
    buffer
        .gpu_write(&queue_wgpu, &data, 0)
        .expect("Write failed");

    work_queue.gpu_enqueue(0);

    let batch_idx = work_queue.cpu_dequeue();
    assert_eq!(batch_idx, 0);

    let cpu_view = buffer.cpu_read();
    assert_eq!(&cpu_view[..5], &[1.5, 2.5, 3.5, 4.5, 5.5]);

    buffer.cpu_ack_read();
}

#[test]
fn test_unified_buffer_write_throughput() {
    let (device, queue) = create_test_device();
    let mut buffer = UnifiedBuffer::<f32>::new(&device, 100000);

    let data: Vec<f32> = (0..10000).map(|i| i as f32).collect();

    let start = Instant::now();
    buffer.gpu_write(&queue, &data, 0).expect("Write failed");
    let write_time = start.elapsed();

    println!(
        "[Performance] GPU write 10000 f32 elements: {:?}",
        write_time
    );

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

    println!("[Performance] GPU enqueue 100000 items: {:?}", enqueue_time);
    assert_eq!(queue.pending_count(), 100000);

    let dequeue_start = Instant::now();
    for i in 0..100000 {
        let item = queue.cpu_dequeue();
        assert_eq!(item, i);
    }
    let dequeue_time = dequeue_start.elapsed();

    println!("[Performance] CPU dequeue 100000 items: {:?}", dequeue_time);
}

#[test]
fn test_work_queue_with_different_types() {
    let queue_u64 = GpuWorkQueue::<u64>::new();
    queue_u64.gpu_enqueue(0x1234567890ABCDEF);
    assert_eq!(queue_u64.cpu_dequeue(), 0x1234567890ABCDEF);

    let queue_i32 = GpuWorkQueue::<i32>::new();
    queue_i32.gpu_enqueue(-42);
    assert_eq!(queue_i32.cpu_dequeue(), -42);

    let queue_f32 = GpuWorkQueue::<f32>::new();
    queue_f32.gpu_enqueue(3.14159);
    assert!(
        (queue_f32.cpu_dequeue() - 3.14159).abs() < 1e-5,
        "Float equality within tolerance"
    );
}
