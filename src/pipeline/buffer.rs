//! Pipeline buffer management

use std::sync::Arc;
use parking_lot::Mutex;
use crossbeam_channel::{bounded, Sender, Receiver};

/// Pipeline buffer for sample storage
pub struct PipelineBuffer<T> {
    data: Arc<Mutex<Vec<T>>>,
    capacity: usize,
}

impl<T: Clone + Default> PipelineBuffer<T> {
    /// Create a new pipeline buffer
    pub fn new(capacity: usize) -> Self {
        Self {
            data: Arc::new(Mutex::new(vec![T::default(); capacity])),
            capacity,
        }
    }

    /// Get buffer reference
    pub fn get(&self) -> Vec<T> {
        self.data.lock().clone()
    }

    /// Set buffer contents
    pub fn set(&self, data: Vec<T>) {
        let mut buf = self.data.lock();
        *buf = data;
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Clear buffer
    pub fn clear(&self) {
        let mut buf = self.data.lock();
        buf.clear();
        buf.resize(self.capacity, T::default());
    }
}

impl<T: Clone + Default> Clone for PipelineBuffer<T> {
    fn clone(&self) -> Self {
        Self {
            data: Arc::clone(&self.data),
            capacity: self.capacity,
        }
    }
}

/// Buffer pool for efficient memory reuse
pub struct BufferPool<T: Clone + Default> {
    buffers: Vec<PipelineBuffer<T>>,
    free_queue: Receiver<usize>,
    used_queue: Sender<usize>,
}

impl<T: Clone + Default + Send + 'static> BufferPool<T> {
    /// Create a new buffer pool
    pub fn new(num_buffers: usize, capacity: usize) -> Self {
        let (tx, rx) = bounded(num_buffers);

        let mut buffers = Vec::with_capacity(num_buffers);
        for i in 0..num_buffers {
            buffers.push(PipelineBuffer::new(capacity));
            let _ = tx.send(i);
        }

        Self {
            buffers,
            free_queue: rx,
            used_queue: tx,
        }
    }

    /// Acquire a buffer from the pool
    pub fn acquire(&self) -> Option<(usize, PipelineBuffer<T>)> {
        if let Ok(idx) = self.free_queue.try_recv() {
            Some((idx, self.buffers[idx].clone()))
        } else {
            None
        }
    }

    /// Release a buffer back to the pool
    pub fn release(&self, idx: usize) {
        let _ = self.used_queue.try_send(idx);
    }

    /// Get number of free buffers
    pub fn free_count(&self) -> usize {
        self.free_queue.try_iter().count()
    }

    /// Get total buffers
    pub fn total_count(&self) -> usize {
        self.buffers.len()
    }
}

/// Ring buffer for streaming data
pub struct RingBuffer<T> {
    data: Vec<T>,
    head: usize,
    tail: usize,
    full: bool,
}

impl<T: Clone + Default> RingBuffer<T> {
    /// Create a new ring buffer
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![T::default(); capacity],
            head: 0,
            tail: 0,
            full: false,
        }
    }

    /// Push data to the buffer
    pub fn push(&mut self, item: T) -> Option<T> {
        let evicted = if self.full {
            Some(self.data[self.head].clone())
        } else {
            None
        };

        self.data[self.tail] = item;
        self.tail = (self.tail + 1) % self.data.len();

        if self.full {
            self.head = (self.head + 1) % self.data.len();
        }

        self.full = self.tail == self.head;

        evicted
    }

    /// Pop data from the buffer
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let item = self.data[self.head].clone();
        self.head = (self.head + 1) % self.data.len();
        self.full = false;
        Some(item)
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        !self.full && self.head == self.tail
    }

    /// Check if buffer is full
    pub fn is_full(&self) -> bool {
        self.full
    }

    /// Get current size
    pub fn len(&self) -> usize {
        if self.full {
            self.data.len()
        } else if self.tail >= self.head {
            self.tail - self.head
        } else {
            self.data.len() - self.head + self.tail
        }
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.data.len()
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.full = false;
    }

    /// Get all items
    pub fn to_vec(&self) -> Vec<T> {
        let mut result = Vec::with_capacity(self.len());
        
        if self.full {
            for i in 0..self.data.len() {
                result.push(self.data[i].clone());
            }
        } else if self.tail >= self.head {
            for i in self.head..self.tail {
                result.push(self.data[i].clone());
            }
        } else {
            for i in self.head..self.data.len() {
                result.push(self.data[i].clone());
            }
            for i in 0..self.tail {
                result.push(self.data[i].clone());
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_buffer() {
        let buffer: PipelineBuffer<f32> = PipelineBuffer::new(1024);
        assert_eq!(buffer.capacity(), 1024);

        let data = vec![1.0f32; 1024];
        buffer.set(data);

        let retrieved = buffer.get();
        assert_eq!(retrieved.len(), 1024);
        assert!(retrieved.iter().all(|&x| x == 1.0));
    }

    #[test]
    fn test_buffer_pool() {
        let pool: BufferPool<f32> = BufferPool::new(4, 1024);
        
        assert_eq!(pool.total_count(), 4);
        assert_eq!(pool.free_count(), 4);

        let (idx, buf) = pool.acquire().unwrap();
        assert_eq!(pool.free_count(), 3);

        pool.release(idx);
        assert_eq!(pool.free_count(), 4);
    }

    #[test]
    fn test_ring_buffer_push_pop() {
        let mut ring: RingBuffer<i32> = RingBuffer::new(4);
        
        assert!(ring.is_empty());
        assert!(!ring.is_full());

        ring.push(1);
        ring.push(2);
        ring.push(3);

        assert_eq!(ring.len(), 3);
        assert_eq!(ring.pop(), Some(1));
        assert_eq!(ring.pop(), Some(2));
        assert_eq!(ring.pop(), Some(3));
        assert_eq!(ring.pop(), None);
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let mut ring: RingBuffer<i32> = RingBuffer::new(3);
        
        ring.push(1);
        ring.push(2);
        ring.push(3);
        assert!(ring.is_full());

        let evicted = ring.push(4);
        assert_eq!(evicted, Some(1));
        assert!(ring.is_full());

        assert_eq!(ring.to_vec(), vec![2, 3, 4]);
    }

    #[test]
    fn test_ring_buffer_wrap() {
        let mut ring: RingBuffer<i32> = RingBuffer::new(4);
        
        ring.push(1);
        ring.push(2);
        ring.pop();
        ring.pop();
        
        ring.push(3);
        ring.push(4);
        
        assert_eq!(ring.to_vec(), vec![3, 4]);
    }
}
