// src/hardware_io/iq_staging_buffer.rs — CPU Staging Buffer for IQ Samples
//
// Host-visible staging buffer for raw [u8; 2] IQ samples from RTL-SDR/Pluto+.
// Enables zero-copy DMA transfer to GPU without f32 conversion on CPU.
//
// Architecture:
//   RTL-SDR read_sync() → IqStagingBuffer → DMA → GPU VRAM
//
// Key properties:
// - Double-buffered for continuous streaming (ping-pong buffers)
// - Lock-free atomic state for thread-safe producer/consumer
// - Backpressure handling when GPU falls behind

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

/// Size of a single IQ sample (I: u8, Q: u8).
pub const IQ_SAMPLE_SIZE: usize = 2;

/// Default staging buffer size: 16384 complex samples = 32768 bytes.
/// Matches DMA_CHUNK_SAMPLES in dma_vbuffer.rs for efficient transfer.
pub const DEFAULT_STAGING_SAMPLES: usize = 16384;
pub const DEFAULT_STAGING_BYTES: usize = DEFAULT_STAGING_SAMPLES * IQ_SAMPLE_SIZE;

/// Number of staging buffers in the pool (double-buffering).
pub const STAGING_BUFFER_COUNT: usize = 2;

/// State of a staging buffer slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferState {
    /// Buffer is empty, ready for CPU to write IQ samples.
    Empty,
    /// Buffer contains valid IQ data, ready for DMA transfer to GPU.
    ReadyForDma,
    /// DMA transfer in progress.
    DmaInProgress,
}

/// Metadata for a single staging buffer slot.
pub struct StagingSlotMeta {
    /// Current state of this slot.
    pub state: AtomicUsize, // BufferState as usize
    /// Number of valid bytes in this slot (≤ DEFAULT_STAGING_BYTES).
    pub valid_bytes: AtomicUsize,
    /// Sequence number for ordering (monotonically increasing).
    pub sequence: AtomicU64,
}

impl StagingSlotMeta {
    fn new() -> Self {
        Self {
            state: AtomicUsize::new(BufferState::Empty as usize),
            valid_bytes: AtomicUsize::new(0),
            sequence: AtomicU64::new(0),
        }
    }

    #[inline]
    pub fn state(&self) -> BufferState {
        match self.state.load(Ordering::Acquire) {
            0 => BufferState::Empty,
            1 => BufferState::ReadyForDma,
            2 => BufferState::DmaInProgress,
            _ => unreachable!("Invalid BufferState"),
        }
    }

    #[inline]
    pub fn set_state(&self, state: BufferState) {
        self.state.store(state as usize, Ordering::Release);
    }

    #[inline]
    pub fn try_acquire_for_write(&self) -> bool {
        self.state
            .compare_exchange(
                BufferState::Empty as usize,
                BufferState::DmaInProgress as usize,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    #[inline]
    pub fn mark_ready_for_dma(&self, bytes: usize, seq: u64) {
        self.valid_bytes.store(bytes, Ordering::Release);
        self.sequence.store(seq, Ordering::Release);
        self.set_state(BufferState::ReadyForDma);
    }

    #[inline]
    pub fn mark_dma_complete(&self) {
        self.set_state(BufferState::Empty);
        self.valid_bytes.store(0, Ordering::Release);
    }
}

/// CPU staging buffer for IQ samples.
///
/// Double-buffered design allows continuous streaming:
/// - While one buffer is being filled by RTL-SDR read_sync()
/// - The other buffer can be transferred to GPU via DMA
///
/// Thread-safe: Multiple producers (Tokio tasks) can write,
/// single consumer (DMA engine) reads.
pub struct IqStagingBuffer {
    /// Raw byte storage for all slots (contiguous memory).
    data: Vec<u8>,

    /// Metadata for each slot.
    slots: Vec<Arc<StagingSlotMeta>>,

    /// Next sequence number for ordering.
    next_sequence: AtomicU64,

    /// Current write slot index (0 or 1).
    write_slot: AtomicUsize,

    /// Total bytes written (for monitoring).
    total_bytes_written: AtomicU64,

    /// Overflow counter (backpressure events).
    overflow_count: AtomicU64,
}

// Safety: IqStagingBuffer uses atomics for thread-safe access.
unsafe impl Send for IqStagingBuffer {}
unsafe impl Sync for IqStagingBuffer {}

impl IqStagingBuffer {
    /// Create a new staging buffer with default size.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_STAGING_SAMPLES)
    }

    /// Create a staging buffer with custom capacity.
    ///
    /// # Parameters
    /// - `samples`: Number of complex IQ samples per slot
    pub fn with_capacity(samples: usize) -> Self {
        let bytes_per_slot = samples * IQ_SAMPLE_SIZE;
        let total_bytes = bytes_per_slot * STAGING_BUFFER_COUNT;

        // Pre-allocate contiguous storage
        let mut data = Vec::with_capacity(total_bytes);
        data.resize(total_bytes, 0u8);

        // Create metadata for each slot
        let slots: Vec<Arc<StagingSlotMeta>> = (0..STAGING_BUFFER_COUNT)
            .map(|_| Arc::new(StagingSlotMeta::new()))
            .collect();

        Self {
            data,
            slots,
            next_sequence: AtomicU64::new(0),
            write_slot: AtomicUsize::new(0),
            total_bytes_written: AtomicU64::new(0),
            overflow_count: AtomicU64::new(0),
        }
    }

    /// Get the byte capacity per slot.
    #[inline]
    pub fn slot_capacity(&self) -> usize {
        self.data.len() / STAGING_BUFFER_COUNT
    }

    /// Get mutable slice for writing IQ samples.
    ///
    /// # Returns
    /// - `Some(&mut [u8])` if a slot is available for writing
    /// - `None` if all slots are busy (backpressure)
    ///
    /// # Usage
    /// ```ignore
    /// if let Some(buf) = staging.acquire_write_slot() {
    ///     let bytes_read = device.read_sync(buf)?;
    ///     staging.release_write_slot(slot_idx, bytes_read);
    /// }
    /// ```
    pub fn acquire_write_slot(&self) -> Option<(usize, &mut [u8])> {
        // Try current write slot first
        let current = self.write_slot.load(Ordering::Acquire);

        if self.slots[current].try_acquire_for_write() {
            let start = current * self.slot_capacity();
            let _end = start + self.slot_capacity();
            // Safety: We have exclusive access via try_acquire_for_write
            let slice = unsafe {
                std::slice::from_raw_parts_mut(
                    self.data.as_ptr().add(start) as *mut u8,
                    self.slot_capacity(),
                )
            };
            return Some((current, slice));
        }

        // Try the other slot
        let other = 1 - current;
        if self.slots[other].try_acquire_for_write() {
            let start = other * self.slot_capacity();
            let _end = start + self.slot_capacity();
            let slice = unsafe {
                std::slice::from_raw_parts_mut(
                    self.data.as_ptr().add(start) as *mut u8,
                    self.slot_capacity(),
                )
            };
            self.write_slot.store(other, Ordering::Release);
            return Some((other, slice));
        }

        // Both slots busy - backpressure
        self.overflow_count.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Release a write slot after filling with IQ data.
    ///
    /// # Parameters
    /// - `slot_idx`: Index returned from acquire_write_slot()
    /// - `valid_bytes`: Number of bytes actually written (≤ slot_capacity)
    ///
    /// # Panics
    /// Panics if slot was not in DmaInProgress state.
    pub fn release_write_slot(&self, slot_idx: usize, valid_bytes: usize) {
        assert!(slot_idx < STAGING_BUFFER_COUNT, "Invalid slot index");
        assert!(
            valid_bytes <= self.slot_capacity(),
            "valid_bytes exceeds capacity"
        );

        let seq = self.next_sequence.fetch_add(1, Ordering::AcqRel);
        self.slots[slot_idx].mark_ready_for_dma(valid_bytes, seq);
        self.total_bytes_written
            .fetch_add(valid_bytes as u64, Ordering::Relaxed);

        // Update write slot hint for next iteration
        self.write_slot.store(1 - slot_idx, Ordering::Release);
    }

    /// Acquire a slot ready for DMA transfer.
    ///
    /// # Returns
    /// - `Some((slot_idx, bytes, sequence))` if a slot is ready
    /// - `None` if no slots are ready for DMA
    pub fn acquire_ready_slot(&self) -> Option<(usize, usize, u64)> {
        for (idx, slot) in self.slots.iter().enumerate() {
            if slot.state() == BufferState::ReadyForDma {
                let bytes = slot.valid_bytes.load(Ordering::Acquire);
                let seq = slot.sequence.load(Ordering::Acquire);
                return Some((idx, bytes, seq));
            }
        }
        None
    }

    /// Mark a slot's DMA transfer as complete.
    pub fn complete_dma(&self, slot_idx: usize) {
        assert!(slot_idx < STAGING_BUFFER_COUNT, "Invalid slot index");
        self.slots[slot_idx].mark_dma_complete();
    }

    /// Get raw bytes from a slot (for DMA copy).
    ///
    /// # Safety
    /// Caller must ensure slot is in ReadyForDma or DmaInProgress state.
    pub unsafe fn get_slot_bytes(&self, slot_idx: usize) -> &[u8] {
        let valid_bytes = self.slots[slot_idx].valid_bytes.load(Ordering::Acquire);
        let start = slot_idx * self.slot_capacity();
        // Safety: Caller guarantees valid slot and bounds
        unsafe {
            std::slice::from_raw_parts(
                self.data.as_ptr().add(start),
                valid_bytes,
            )
        }
    }

    /// Get metadata for a slot.
    pub fn slot_meta(&self, slot_idx: usize) -> &StagingSlotMeta {
        &self.slots[slot_idx]
    }

    /// Get total bytes written since creation.
    pub fn total_bytes_written(&self) -> u64 {
        self.total_bytes_written.load(Ordering::Relaxed)
    }

    /// Get overflow count (backpressure events).
    pub fn overflow_count(&self) -> u64 {
        self.overflow_count.load(Ordering::Relaxed)
    }

    /// Check if any slot is ready for DMA.
    pub fn has_ready_data(&self) -> bool {
        self.slots
            .iter()
            .any(|s| s.state() == BufferState::ReadyForDma)
    }

    /// Reset all slots to empty state (for error recovery).
    pub fn reset(&self) {
        for slot in &self.slots {
            slot.set_state(BufferState::Empty);
            slot.valid_bytes.store(0, Ordering::Release);
        }
        self.write_slot.store(0, Ordering::Release);
    }

    /// Get statistics for monitoring.
    pub fn stats(&self) -> StagingBufferStats {
        StagingBufferStats {
            total_bytes_written: self.total_bytes_written(),
            overflow_count: self.overflow_count(),
            has_ready_data: self.has_ready_data(),
            write_slot: self.write_slot.load(Ordering::Acquire),
            slot_states: self
                .slots
                .iter()
                .map(|s| s.state())
                .collect::<Vec<_>>(),
        }
    }
}

impl Default for IqStagingBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for monitoring staging buffer health.
#[derive(Debug, Clone)]
pub struct StagingBufferStats {
    pub total_bytes_written: u64,
    pub overflow_count: u64,
    pub has_ready_data: bool,
    pub write_slot: usize,
    pub slot_states: Vec<BufferState>,
}

/// Zero-copy view into a staging buffer slot.
///
/// Provides safe access to raw IQ bytes without copying.
pub struct StagingBufferView<'a> {
    staging: &'a IqStagingBuffer,
    slot_idx: usize,
    bytes: usize,
}

impl<'a> StagingBufferView<'a> {
    /// Create a new view (caller must ensure slot is ready).
    pub fn new(staging: &'a IqStagingBuffer, slot_idx: usize, bytes: usize) -> Self {
        Self {
            staging,
            slot_idx,
            bytes,
        }
    }

    /// Get raw IQ bytes as slice.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { self.staging.get_slot_bytes(self.slot_idx) }
    }

    /// Get number of valid bytes.
    pub fn len(&self) -> usize {
        self.bytes
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.bytes == 0
    }

    /// Get slot index.
    pub fn slot_index(&self) -> usize {
        self.slot_idx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_staging_buffer_creation() {
        let staging = IqStagingBuffer::new();
        assert_eq!(staging.slot_capacity(), DEFAULT_STAGING_BYTES);
        assert_eq!(staging.total_bytes_written(), 0);
        assert_eq!(staging.overflow_count(), 0);
    }

    #[test]
    fn test_custom_capacity() {
        let staging = IqStagingBuffer::with_capacity(8192);
        assert_eq!(staging.slot_capacity(), 8192 * IQ_SAMPLE_SIZE);
    }

    #[test]
    fn test_acquire_write_slot() {
        let staging = IqStagingBuffer::new();

        // First slot should be available
        let result = staging.acquire_write_slot();
        assert!(result.is_some());
        let (slot_idx, slice) = result.unwrap();
        assert!(slot_idx < STAGING_BUFFER_COUNT);
        assert_eq!(slice.len(), DEFAULT_STAGING_BYTES);
    }

    #[test]
    fn test_release_write_slot() {
        let staging = IqStagingBuffer::new();

        let (slot_idx, slice) = staging.acquire_write_slot().unwrap();
        // Simulate writing some data
        slice[..1024].fill(0x42);

        staging.release_write_slot(slot_idx, 1024);

        assert_eq!(staging.total_bytes_written(), 1024);
        assert!(staging.has_ready_data());
    }

    #[test]
    fn test_acquire_ready_slot() {
        let staging = IqStagingBuffer::new();

        // No data initially
        assert!(staging.acquire_ready_slot().is_none());

        // Write and release a slot
        let (slot_idx, slice) = staging.acquire_write_slot().unwrap();
        slice[..2048].fill(0xFF);
        staging.release_write_slot(slot_idx, 2048);

        // Now should be ready
        let result = staging.acquire_ready_slot();
        assert!(result.is_some());
        let (ready_idx, bytes, seq) = result.unwrap();
        assert_eq!(ready_idx, slot_idx);
        assert_eq!(bytes, 2048);
        assert_eq!(seq, 0);
    }

    #[test]
    fn test_dma_complete() {
        let staging = IqStagingBuffer::new();

        let (slot_idx, slice) = staging.acquire_write_slot().unwrap();
        staging.release_write_slot(slot_idx, DEFAULT_STAGING_BYTES);
        staging.complete_dma(slot_idx);

        assert!(!staging.has_ready_data());
        assert_eq!(staging.slot_meta(slot_idx).state(), BufferState::Empty);
    }

    #[test]
    fn test_backpressure() {
        let staging = IqStagingBuffer::new();

        // Acquire both slots
        let (slot1, slice1) = staging.acquire_write_slot().unwrap();
        let _ = staging.acquire_write_slot(); // This might get slot2 or None

        // Try to acquire again - should trigger backpressure
        let result = staging.acquire_write_slot();
        if result.is_none() {
            assert_eq!(staging.overflow_count(), 1);
        }

        // Release one slot
        staging.release_write_slot(slot1, DEFAULT_STAGING_BYTES);

        // Now should be available again
        assert!(staging.acquire_write_slot().is_some());
    }

    #[test]
    fn test_staging_stats() {
        let staging = IqStagingBuffer::new();

        let stats = staging.stats();
        assert_eq!(stats.total_bytes_written, 0);
        assert_eq!(stats.overflow_count, 0);
        assert!(!stats.has_ready_data);
        assert_eq!(stats.write_slot, 0);
        assert_eq!(stats.slot_states.len(), STAGING_BUFFER_COUNT);
    }

    #[test]
    fn test_reset() {
        let staging = IqStagingBuffer::new();

        let (slot_idx, slice) = staging.acquire_write_slot().unwrap();
        staging.release_write_slot(slot_idx, 1024);
        assert!(staging.has_ready_data());

        staging.reset();
        assert!(!staging.has_ready_data());
        assert_eq!(staging.write_slot.load(Ordering::Acquire), 0);
    }

    #[test]
    fn test_staging_buffer_view() {
        let staging = IqStagingBuffer::new();

        let (slot_idx, slice) = staging.acquire_write_slot().unwrap();
        slice[..512].fill(0xAB);
        staging.release_write_slot(slot_idx, 512);

        let (ready_idx, bytes, _) = staging.acquire_ready_slot().unwrap();
        let view = StagingBufferView::new(&staging, ready_idx, bytes);

        assert_eq!(view.len(), 512);
        assert!(!view.is_empty());
        assert_eq!(view.as_bytes()[0], 0xAB);
    }

    #[test]
    fn test_sequence_ordering() {
        let staging = IqStagingBuffer::new();

        // Write multiple slots
        for i in 0..4 {
            let (slot_idx, slice) = staging.acquire_write_slot().unwrap();
            slice[..1024].fill(i as u8);
            staging.release_write_slot(slot_idx, 1024);

            let (_, _, seq) = staging.acquire_ready_slot().unwrap();
            assert_eq!(seq, i);

            staging.complete_dma(slot_idx);
        }
    }
}
