/// Zero-copy alignment buffer for GPU audio ingestion.
/// Packs heterogeneous audio streams into a flat, well-aligned `f32` buffer.
pub struct GpuStreamPacker {
    /// The flat buffer designed for zero-copy upload to Wgpu.
    pub staging_buffer: Vec<f32>,
    /// Offsets tracking the start of individual streams within the staging buffer.
    pub stream_offsets: Vec<usize>,
}

impl GpuStreamPacker {
    /// Create a new Stream Packer capable of holding up to `max_total_samples` floats.
    pub fn new(max_total_samples: usize) -> Self {
        Self {
            staging_buffer: Vec::with_capacity(max_total_samples),
            stream_offsets: Vec::new(),
        }
    }

    /// Reset the packer for a new frame, clearing old data but keeping capacity.
    pub fn reset_frame(&mut self) {
        self.staging_buffer.clear();
        self.stream_offsets.clear();
    }

    /// Packs raw 16-bit little-endian PCM bytes into normalized [-1.0, 1.0] f32s.
    /// Updates `cursor` to point to the next free index in the `staging_buffer`.
    pub fn pack_16bit_stream(&mut self, raw_bytes: &[u8], cursor: &mut usize) {
        self.stream_offsets.push(*cursor);

        let num_samples = raw_bytes.len() / 2;
        for i in 0..num_samples {
            let byte_idx = i * 2;
            let sample = i16::from_le_bytes([raw_bytes[byte_idx], raw_bytes[byte_idx + 1]]);

            // Normalize [-32768, 32767] to [-1.0, 1.0]
            let normalized = (sample as f32) / 32768.0;
            self.staging_buffer.push(normalized);
            *cursor += 1;
        }
    }

    /// Packs raw 24-bit little-endian PCM bytes into normalized [-1.0, 1.0] f32s.
    /// Updates `cursor` to point to the next free index in the `staging_buffer`.
    pub fn pack_24bit_stream(&mut self, raw_bytes: &[u8], cursor: &mut usize) {
        self.stream_offsets.push(*cursor);

        let num_samples = raw_bytes.len() / 3;
        for i in 0..num_samples {
            let byte_idx = i * 3;
            // 24-bit little endian into a 32-bit integer (sign-extended)
            let b0 = raw_bytes[byte_idx] as i32;
            let b1 = raw_bytes[byte_idx + 1] as i32;
            let b2 = (raw_bytes[byte_idx + 2] as i8) as i32; // sign extension on highest byte

            let sample = b0 | (b1 << 8) | (b2 << 16);

            // Normalize [-8388608, 8388607] to [-1.0, 1.0]
            let normalized = (sample as f32) / 8388608.0;
            self.staging_buffer.push(normalized);
            *cursor += 1;
        }
    }
}
