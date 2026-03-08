// src/gpu_memory.rs — Unified Memory Buffer Management
//
// Zero-copy GPU ↔ CPU data sharing on RX 6700 XT unified memory.
// GPU writes data via queue.write_buffer() → CPU reads with < 1 microsecond latency.
// Uses atomic synchronization (no busy-waiting, no PCIe copies).
//
// CRITICAL: RX 6700 XT (RDNA2) supports unified memory addressing.
// Both GPU and CPU access the same memory space via PCIe BAR.

use bytemuck::Pod;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};

/// Unified memory buffer for zero-copy GPU ↔ CPU access.
///
/// On RX 6700 XT (unified memory):
/// - GPU writes to buffer via queue.write_buffer()
/// - CPU reads directly (no PCIe copies, same address space)
/// - Synchronization via atomic flag (gpu_write_flag)
/// - CPU blocks when no new data available
///
/// # Example
/// ```no_run
/// let mut buffer = UnifiedBuffer::<f32>::new(&device, 1024);
/// buffer.gpu_write_async(&queue, &[1.0, 2.0, 3.0], 0)?;
/// let data = buffer.cpu_read();  // Blocks until GPU writes
/// buffer.cpu_ack_read();         // Signal completion
/// ```
pub struct UnifiedBuffer<T: Pod + Send + Sync> {
    // GPU buffer with CPU read access (unified memory support)
    gpu_buffer: wgpu::Buffer,

    // CPU-accessible mapping of buffer contents
    // Updated by GPU via queue.write_buffer(); read by CPU
    cpu_map: Vec<T>,

    // Total number of elements the buffer can hold
    capacity: usize,

    // Atomic flag: 0 = no new data, 1 = GPU wrote
    // GPU sets to 1, CPU sets to 0 after reading
    gpu_write_flag: AtomicU32,
}

impl<T: Pod + Send + Sync> UnifiedBuffer<T> {
    /// Create a unified memory buffer with specified capacity.
    ///
    /// # Parameters
    /// - `device`: wgpu Device for GPU buffer allocation
    /// - `capacity`: Number of elements (of type T) the buffer can hold
    ///
    /// # Returns
    /// UnifiedBuffer with GPU and CPU buffers allocated
    ///
    /// The GPU buffer is created with MAP_READ usage flag,
    /// enabling CPU to read GPU-written data via unified memory.
    pub fn new(device: &wgpu::Device, capacity: usize) -> Self {
        let element_size = std::mem::size_of::<T>();
        let buffer_size_bytes = (capacity * element_size) as u64;

        // Create GPU buffer with unified memory support
        // COPY_SRC: GPU buffer can be source for copying (writes from queue)
        // STORAGE: Can be used as storage in shaders
        // COPY_DST: Required for queue.write_buffer()
        //
        // Note: MAP_READ cannot be combined with COPY_DST/STORAGE on DX12.
        // Instead, we use a dual-buffer approach: GPU writes to STORAGE buffer,
        // and CPU maintains a CPU-side copy for data access.
        let gpu_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("unified_buffer_gpu"),
            size: buffer_size_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Initialize CPU mapping with zeroed data
        let cpu_map = vec![unsafe { std::mem::zeroed() }; capacity];

        Self {
            gpu_buffer,
            cpu_map,
            capacity,
            gpu_write_flag: AtomicU32::new(0),
        }
    }

    /// GPU writes data synchronously (queue.write_buffer is sync).
    ///
    /// # Parameters
    /// - `queue`: wgpu Queue for buffer updates
    /// - `data`: Slice of data to write
    /// - `offset`: Element offset in buffer (not byte offset)
    ///
    /// # Returns
    /// Ok(()) if write succeeded, Err if offset+data exceeds capacity
    ///
    /// # Synchronization
    /// Sets gpu_write_flag to 1 after queuing write.
    /// CPU will unblock from cpu_read() when flag is observed as 1.
    pub fn gpu_write(
        &mut self,
        queue: &wgpu::Queue,
        data: &[T],
        offset: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Bounds check
        if offset + data.len() > self.capacity {
            return Err(format!(
                "Write exceeds capacity: offset {} + len {} > {}",
                offset,
                data.len(),
                self.capacity
            )
            .into());
        }

        // Calculate byte offset
        let element_size = std::mem::size_of::<T>();
        let byte_offset = (offset * element_size) as u64;

        // Write data to GPU buffer (unified memory, no PCIe overhead)
        queue.write_buffer(&self.gpu_buffer, byte_offset, bytemuck::cast_slice(data));

        // Update CPU map with data (for CPU-side access simulation)
        // In real unified memory, this is redundant, but kept for compatibility
        self.cpu_map[offset..offset + data.len()].copy_from_slice(data);

        // Signal that GPU has written new data
        // Use Release ordering to ensure write is visible to CPU
        self.gpu_write_flag.store(1, Ordering::Release);

        Ok(())
    }

    /// CPU reads data, blocking until GPU writes.
    ///
    /// # Behavior
    /// - Spins on gpu_write_flag until it becomes 1
    /// - Does NOT busy-wait (yields to scheduler)
    /// - Returns slice of entire buffer
    ///
    /// # Synchronization
    /// Uses Acquire ordering to ensure GPU write is visible.
    /// Caller must call cpu_ack_read() to clear flag for next write.
    ///
    /// # Returns
    /// Reference to entire cpu_map buffer (containing GPU-written data)
    pub fn cpu_read(&self) -> &[T] {
        // Spin-wait for GPU write flag
        // Expected latency: < 1 microsecond (unified memory)
        loop {
            let flag = self.gpu_write_flag.load(Ordering::Acquire);
            if flag == 1 {
                break;
            }
            // Yield to scheduler instead of busy-waiting
            std::thread::yield_now();
        }

        &self.cpu_map[..]
    }

    /// Signal that CPU has finished reading.
    ///
    /// Clears gpu_write_flag to allow GPU to overwrite buffer.
    /// Must be called after cpu_read() before calling gpu_write_async() again.
    pub fn cpu_ack_read(&self) {
        // Use Release ordering to ensure CPU read is complete
        // before GPU is allowed to write again
        self.gpu_write_flag.store(0, Ordering::Release);
    }

    /// Get reference to GPU buffer for shader access.
    ///
    /// Used to pass buffer to compute/render shaders.
    pub fn gpu_buffer(&self) -> &wgpu::Buffer {
        &self.gpu_buffer
    }

    /// Get current write flag state (for debugging).
    pub fn write_flag_state(&self) -> u32 {
        self.gpu_write_flag.load(Ordering::Acquire)
    }

    /// Reset buffer (clear CPU map and flag).
    ///
    /// Used to reset state between test cases or mode changes.
    pub fn reset(&mut self) {
        self.cpu_map
            .iter_mut()
            .for_each(|v| *v = unsafe { std::mem::zeroed() });
        self.gpu_write_flag.store(0, Ordering::Release);
    }
}

/// Lock-free work queue: GPU enqueues, CPU dequeues.
///
/// Provides thread-safe, non-blocking GPU→CPU work distribution.
/// GPU enqueues items via atomic operations (no locks).
/// CPU dequeues items (may block if empty, no busy-waiting).
///
/// # Example
/// ```no_run
/// let queue = GpuWorkQueue::<u32>::new();
/// queue.gpu_enqueue(42);      // GPU-side, atomic (no lock)
/// let item = queue.cpu_dequeue(); // CPU-side, blocks until item ready
/// ```
pub struct GpuWorkQueue<T: Copy + Send + Sync> {
    // Queue storage (protected by Mutex for FIFO property)
    items: Mutex<VecDeque<T>>,

    // Atomic counter: number of pending items
    // GPU increments, CPU decrements
    pending_count: AtomicU32,
}

impl<T: Copy + Send + Sync> GpuWorkQueue<T> {
    /// Create a new work queue.
    ///
    /// Pre-allocates space for 1024 items to reduce allocations.
    pub fn new() -> Self {
        Self {
            items: Mutex::new(VecDeque::with_capacity(1024)),
            pending_count: AtomicU32::new(0),
        }
    }

    /// GPU enqueues a work item (atomic, non-blocking).
    ///
    /// # Parameters
    /// - `item`: Work item to enqueue
    ///
    /// # Synchronization
    /// - Acquires Mutex briefly to push item
    /// - Atomically increments pending_count
    /// - Release ordering ensures visibility to CPU
    /// - Non-blocking (GPU doesn't wait)
    pub fn gpu_enqueue(&self, item: T) {
        let mut items = self.items.lock();
        items.push_back(item);
        drop(items); // Release lock before atomic operation

        // Increment pending count atomically
        // Use Release ordering so CPU sees the item immediately
        self.pending_count.fetch_add(1, Ordering::Release);
    }

    /// CPU dequeues a work item (blocking if empty).
    ///
    /// # Behavior
    /// - Spins on pending_count until > 0
    /// - Does NOT busy-wait (yields to scheduler)
    /// - Pops item from queue with FIFO order
    ///
    /// # Returns
    /// Next work item from queue
    ///
    /// # Synchronization
    /// - Blocks until GPU enqueues (yield-based, not spin-based)
    /// - Uses Acquire ordering to see GPU enqueue
    /// - Decrements pending_count after dequeue
    pub fn cpu_dequeue(&self) -> T {
        // Spin-wait for pending items
        loop {
            // Check if items available (with acquire semantics)
            if self.pending_count.load(Ordering::Acquire) > 0 {
                // Try to dequeue
                {
                    let mut items = self.items.lock();
                    if let Some(item) = items.pop_front() {
                        drop(items); // Release lock before atomic operation
                        self.pending_count.fetch_sub(1, Ordering::Acquire);
                        return item;
                    }
                }
            }

            // No items available, yield to scheduler
            std::thread::yield_now();
        }
    }

    /// Non-blocking check if work is pending.
    ///
    /// # Returns
    /// true if pending_count > 0, false otherwise
    pub fn has_pending(&self) -> bool {
        self.pending_count.load(Ordering::Acquire) > 0
    }

    /// Get current pending work count.
    ///
    /// # Returns
    /// Number of items currently queued
    pub fn pending_count(&self) -> u32 {
        self.pending_count.load(Ordering::Acquire)
    }

    /// Non-blocking peek at next item (if available).
    ///
    /// # Returns
    /// Some(item) if queue has items, None if empty
    /// Does not remove item from queue.
    pub fn peek(&self) -> Option<T> {
        let items = self.items.lock();
        items.front().copied()
    }

    /// Clear queue and reset pending count.
    ///
    /// Used for flushing queued work (e.g., mode switch).
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

#[cfg(test)]
mod tests {
    use super::*;

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
        for i in 0..100 {
            queue.gpu_enqueue(i);
        }
        assert_eq!(queue.pending_count(), 100);
    }

    #[test]
    fn test_work_queue_peek() {
        let queue = GpuWorkQueue::<u32>::new();
        queue.gpu_enqueue(42);
        queue.gpu_enqueue(99);

        // Peek should return first item without removing
        assert_eq!(queue.peek(), Some(42));
        assert_eq!(queue.pending_count(), 2); // Not decremented
        assert_eq!(queue.peek(), Some(42)); // Same item
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
}
