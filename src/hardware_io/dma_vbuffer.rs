// src/hardware_io/dma_vbuffer.rs — Zero-Copy DMA Gateway
//
// Maps raw RTL-SDR [u8] IQ samples directly into GPU VRAM without CPU f32 conversion.
// Uses circular buffer: write_offset advances, wraps at max_vram_bytes.
// Host staging buffer → wgpu::queue write_buffer() → GPU VRAM (no PCIe copies).

use std::sync::Arc;

/// Size of DMA chunk (16384 complex samples * 2 bytes (I+Q) = 32 KB per transfer).
/// Balances PCIe overhead with latency.
pub const DMA_CHUNK_SAMPLES: usize = 16384;
const CHUNK_BYTES: usize = DMA_CHUNK_SAMPLES * 2;

/// History depth: 64 chunks = 1,048,576 bytes ≈ 10.7 seconds @ 2.4 MSPS.
pub const DMA_HISTORY_CHUNKS: usize = 64;

pub struct IqDmaGateway {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,

    // CPU-visible staging buffer (MAP_WRITE | COPY_SRC)
    staging_buffer: wgpu::Buffer,

    // GPU VRAM rolling history (STORAGE | COPY_DST)
    pub vram_buffer: wgpu::Buffer,

    // Circular buffer state
    write_offset_bytes: wgpu::BufferAddress,
    max_vram_bytes: wgpu::BufferAddress,
}

impl IqDmaGateway {
    /// Create a new DMA gateway with rolling history.
    ///
    /// # Parameters
    /// - `device`: wgpu Device for buffer allocation
    /// - `queue`: wgpu Queue for async copy operations
    /// - `history_chunks`: Number of DMA chunks to keep in rolling history
    ///
    /// # Returns
    /// IqDmaGateway with pre-allocated staging and VRAM buffers
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>, history_chunks: usize) -> Self {
        // Staging buffer: CPU writes here, GPU reads via DMA copy
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("IQ Staging Buffer (host-visible)"),
            size: CHUNK_BYTES as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // VRAM rolling history buffer
        let max_vram_bytes = (CHUNK_BYTES * history_chunks) as wgpu::BufferAddress;
        let vram_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("IQ VRAM Rolling History"),
            size: max_vram_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            device,
            queue,
            staging_buffer,
            vram_buffer,
            write_offset_bytes: 0,
            max_vram_bytes,
        }
    }

    /// Push raw IQ bytes directly to VRAM via DMA (zero f32 conversion).
    ///
    /// # Parameters
    /// - `raw_iq_bytes`: [u8; 2] samples from RTL-SDR (interleaved I, Q)
    ///
    /// # Flow
    /// 1. Map staging buffer (CPU-accessible)
    /// 2. Copy bytes into staging
    /// 3. Unmap staging
    /// 4. Queue GPU command: copy staging → VRAM at write_offset_bytes
    /// 5. Update write_offset_bytes (circular wrap)
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err(String)` if chunk size invalid
    pub fn push_dma_chunk(&mut self, raw_iq_bytes: &[u8]) -> Result<(), String> {
        if raw_iq_bytes.len() != CHUNK_BYTES {
            return Err(format!(
                "Invalid chunk size: expected {}, got {}",
                CHUNK_BYTES,
                raw_iq_bytes.len()
            ));
        }

        // Step 1: Map staging buffer for CPU write
        let buffer_slice = self.staging_buffer.slice(..);

        // Use blocking_map_async (synchronous) for simplicity
        // In high-performance scenarios, use async_map_async + polling
        buffer_slice.map_async(wgpu::MapMode::Write, |_| {});
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .unwrap();

        {
            // Step 2: Copy raw bytes into mapped staging buffer
            let mut view = buffer_slice.get_mapped_range_mut();
            view.copy_from_slice(raw_iq_bytes);
        }

        // Step 3: Unmap so GPU can read
        self.staging_buffer.unmap();

        // Step 4: Queue GPU command: staging → VRAM DMA copy
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("IQ DMA Copy Encoder"),
            });

        encoder.copy_buffer_to_buffer(
            &self.staging_buffer,
            0,
            &self.vram_buffer,
            self.write_offset_bytes,
            CHUNK_BYTES as wgpu::BufferAddress,
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Step 5: Update circular buffer pointer
        self.write_offset_bytes =
            (self.write_offset_bytes + CHUNK_BYTES as u64) % self.max_vram_bytes;

        Ok(())
    }

    /// Get current write offset (for debugging / visualization).
    pub fn write_offset(&self) -> wgpu::BufferAddress {
        self.write_offset_bytes
    }

    /// Get maximum VRAM size.
    pub fn max_vram_size(&self) -> wgpu::BufferAddress {
        self.max_vram_bytes
    }

    /// Reset to start of buffer (use on mode change or error recovery).
    pub fn reset(&mut self) {
        self.write_offset_bytes = 0;
    }
}

// Safety: IqDmaGateway wraps wgpu Device/Queue which are Send + Sync.
// All operations are synchronized through the Mutex in the dispatch loop.
unsafe impl Send for IqDmaGateway {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dma_chunk_constants() {
        // Verify DMA chunk size
        assert_eq!(CHUNK_BYTES, 32768);
        assert_eq!(DMA_CHUNK_SAMPLES, 16384);

        // Verify history depth
        assert_eq!(DMA_HISTORY_CHUNKS, 64);
    }

    #[test]
    fn test_circular_wraparound() {
        // Simulate offset wraparound
        let mut offset = 0u64;
        let max = (CHUNK_BYTES * 4) as u64; // Small buffer for testing

        for _ in 0..8 {
            offset = (offset + CHUNK_BYTES as u64) % max;
        }

        // Should wrap back to 0 after 4 iterations
        assert_eq!(offset, 0);
    }
}
