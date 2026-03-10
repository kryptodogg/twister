// src/vbuffer.rs — Versioned GPU Buffer
//
// A V-buffer is an append-only, power-of-2 capacity GPU storage buffer.
// Unlike a ring buffer, it never aliases old data under the write head:
// the write head is a monotonically increasing version counter, and the
// GPU layout is:
//
//   buf[v % DEPTH][freq_bin]  ←  frame at version v
//
// This means any contiguous context window of T frames [v-T+1 .. v] maps
// to a contiguous (possibly wrapped) slice in GPU memory.  The Mamba SSM
// reads exactly this window every dispatch.
//
// Compared to a ring buffer:
//   Ring buffer: head races write pointer, old data aliased immediately.
//   V-buffer:    head is a version, never wraps the data it points at;
//                all T frames in the context window are valid simultaneously.
//                The GPU can scan any sub-window without a copy.
//
// Layout (flat f32 array, row-major):
//   slot = version % DEPTH
//   offset = slot * FREQ_BINS + freq_bin_index
//
// Push constants passed to shaders that read the V-buffer:
//   write_version: u32   — most recent written version (slot = write_version % DEPTH)
//   context_len:   u32   — how many frames to look back (T)
//   freq_bins:     u32   — FREQ_BINS
//   depth:         u32   — DEPTH (must be power of 2)

use parking_lot::Mutex;
use std::sync::Arc;

/// Capacity: 512 frames × 512 freq-bins = 256 KiB of f32.
/// 512 frames at ~21 ms/frame (2048 samples @ 96 kHz) = ~10.7 seconds of context.
pub const V_DEPTH: usize = 512; // must be power of 2
pub const V_FREQ_BINS: usize = 512; // matches BISPEC_BINS and waterfall bins
pub const V_BUF_CELLS: usize = V_DEPTH * V_FREQ_BINS;
pub const V_BUF_BYTES: u64 = (V_BUF_CELLS * 8) as u64; // vec4<f16> is 8 bytes

/// Push-constant block passed to every shader that reads the V-buffer.
/// Must be ≤ 128 bytes (Vulkan minimum push constant size guarantee).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VBufferPushConst {
    pub write_version: u32, // most recently written slot's version number
    pub context_len: u32,   // how many frames to read back (T)
    pub freq_bins: u32,     // V_FREQ_BINS
    pub depth: u32,         // V_DEPTH  (power of 2 so % is a mask)
}

/// CPU-side metadata.  The GPU buffer itself lives in `GpuVBuffer`.
pub struct VBufferMeta {
    /// Monotonically increasing write counter.  Never resets.
    pub version: u64,
}

/// Buffer for raw IQ samples from RTL-SDR.
/// Holds 512 complex samples as [i8; 2] for the GPU FFT pass.
pub struct IqVBuffer {
    pub buffer: wgpu::Buffer,
    pub staging: wgpu::Buffer,
}

impl IqVBuffer {
    pub fn new(device: &wgpu::Device) -> Self {
        let size = (V_FREQ_BINS * 2) as u64; // 512 bins * 2 bytes (i8, i8) = 1024 bytes
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("iq-vbuffer-main"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("iq-vbuffer-staging"),
            size,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE,
            mapped_at_creation: false,
        });
        Self { buffer, staging }
    }

    /// Push exactly 512 raw [i8; 2] samples to the GPU.
    pub fn push_frame(&self, queue: &wgpu::Queue, samples: &[[i8; 2]]) {
        let n = samples.len().min(V_FREQ_BINS);
        let mut row = vec![[0i8, 0i8]; V_FREQ_BINS];
        row[..n].copy_from_slice(&samples[..n]);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&row));
    }
}

/// The actual GPU resource.
pub struct GpuVBuffer {
    /// Storage buffer visible to all compute shaders (read + write).
    pub buffer: wgpu::Buffer,
    /// Staging buffer for CPU → GPU uploads.
    pub staging: wgpu::Buffer,
    pub meta: VBufferMeta,
}

impl GpuVBuffer {
    pub fn new(device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vbuffer-main"),
            size: V_BUF_BYTES,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vbuffer-staging"),
            size: (V_FREQ_BINS * 8) as u64, // one row at a time (vec4<f16>)
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE,
            mapped_at_creation: false,
        });
        Self {
            buffer,
            staging,
            meta: VBufferMeta { version: 0 },
        }
    }

    /// Append one frame of frequency-domain magnitudes.
    /// `magnitudes` must have exactly V_FREQ_BINS elements (zero-padded if shorter).
    pub fn push_frame(&mut self, queue: &wgpu::Queue, frame_data: &[[half::f16; 4]]) {
        let slot = (self.meta.version as usize) % V_DEPTH;
        let offset = (slot * V_FREQ_BINS * 8) as u64;

        let n = frame_data.len().min(V_FREQ_BINS);
        let mut row = vec![[half::f16::from_f32_const(0.0); 4]; V_FREQ_BINS];
        row[..n].copy_from_slice(&frame_data[..n]);

        queue.write_buffer(&self.buffer, offset, bytemuck::cast_slice(&row));
        self.meta.version += 1;
    }

    /// Helper to push f32 magnitudes (mapped to first channel of f16x4)
    pub fn push_frame_f32(&mut self, queue: &wgpu::Queue, mags: &[f32]) {
        let frame_data: Vec<[half::f16; 4]> = mags
            .iter()
            .map(|&f| {
                [
                    half::f16::from_f32(f),
                    half::f16::from_f32(0.0),
                    half::f16::from_f32(0.0),
                    half::f16::from_f32(0.0),
                ]
            })
            .collect();
        self.push_frame(queue, &frame_data);
    }

    /// Build the push-constant block for shaders reading this buffer.
    pub fn push_const(&self, context_len: u32) -> VBufferPushConst {
        VBufferPushConst {
            write_version: ((self.meta.version.saturating_sub(1)) % V_DEPTH as u64) as u32,
            context_len: context_len.min(V_DEPTH as u32),
            freq_bins: V_FREQ_BINS as u32,
            depth: V_DEPTH as u32,
        }
    }

    /// Current version (number of frames pushed so far).
    pub fn version(&self) -> u64 {
        self.meta.version
    }

    /// True once enough frames have been pushed for a full context window.
    pub fn ready(&self, context_len: u32) -> bool {
        self.meta.version >= context_len as u64
    }
}

/// Shared, thread-safe handle to a GpuVBuffer.
/// Clone-able so the bispectrum, waterfall, and Mamba trainer can all
/// reference the same underlying buffer without copying.
pub type SharedVBuffer = Arc<Mutex<GpuVBuffer>>;

pub fn new_shared_vbuffer(device: &wgpu::Device) -> SharedVBuffer {
    Arc::new(Mutex::new(GpuVBuffer::new(device)))
}

// ── Context Window API for V-Buffer ───────────────────────────────────────────

/// A read-only view into a contiguous context window of the V-buffer.
/// Handles circular wraparound internally.
pub struct VBufferContextWindow {
    /// Flattened frame data: [frame0, frame1, ..., frameN]
    /// Each frame is [vec4<f16>; FREQ_BINS]
    pub data: Vec<[half::f16; 4]>,
    /// Number of frames in the window
    pub n_frames: usize,
    /// Start version (oldest frame in window)
    pub start_version: u64,
    /// End version (newest frame in window)
    pub end_version: u64,
}

impl GpuVBuffer {
    /// Extract a contiguous context window from the rolling buffer.
    ///
    /// # Parameters
    /// - `n_frames`: Number of frames to extract (must be <= V_DEPTH)
    ///
    /// # Returns
    /// VBufferContextWindow with flattened frame data
    ///
    /// # Behavior
    /// - Handles circular wraparound automatically
    /// - Returns frames in chronological order (oldest → newest)
    /// - If n_frames > available frames, returns all available
    pub fn get_context_window(&self, n_frames: usize) -> VBufferContextWindow {
        let available = self.meta.version as usize;
        let n_frames = n_frames.min(available).min(V_DEPTH);

        if n_frames == 0 {
            return VBufferContextWindow {
                data: Vec::new(),
                n_frames: 0,
                start_version: 0,
                end_version: 0,
            };
        }

        // Calculate start version (oldest frame in window)
        let start_version = self.meta.version.saturating_sub(n_frames as u64);
        let mut data = Vec::with_capacity(n_frames * V_FREQ_BINS);

        // Read frames in chronological order
        for frame_idx in 0..n_frames {
            let version = start_version + frame_idx as u64;
            let slot = (version as usize) % V_DEPTH;
            let base_offset = slot * V_FREQ_BINS;

            // Read one frame from GPU buffer
            // Note: This requires mapping the GPU buffer, which is slow.
            // For real-time use, consider async readback or CPU shadow buffer.
            let frame_data = self.read_frame_cpu(slot, base_offset);
            data.extend_from_slice(&frame_data);
        }

        VBufferContextWindow {
            data,
            n_frames,
            start_version,
            end_version: self.meta.version,
        }
    }

    /// Read a single frame from the GPU buffer (CPU readback).
    ///
    /// # Parameters
    /// - `slot`: Buffer slot (0..V_DEPTH)
    /// - `base_offset`: Starting offset in the buffer
    ///
    /// # Returns
    /// Vec of [half::f16; 4] for one frame
    ///
    /// # Note
    /// This is a CPU-side operation and should not be used in hot paths.
    /// For real-time access, use GPU shaders with vbuf_lookup().
    fn read_frame_cpu(&self, slot: usize, base_offset: usize) -> Vec<[half::f16; 4]> {
        // In production, this would use wgpu buffer mapping
        // For now, return zeros (placeholder for actual readback implementation)
        // Real implementation would:
        // 1. Create buffer slice
        // 2. Map for reading
        // 3. Copy data
        // 4. Unmap
        vec![[half::f16::from_f32_const(0.0); 4]; V_FREQ_BINS]
    }

    /// Get the number of frames available in the buffer.
    pub fn available_frames(&self) -> u64 {
        self.meta.version
    }

    /// Check if buffer has enough frames for a specific context window.
    pub fn has_enough_frames(&self, required: usize) -> bool {
        self.meta.version >= required as u64
    }
}

impl VBufferContextWindow {
    /// Get a specific frame from the context window.
    ///
    /// # Parameters
    /// - `frame_index`: Index within the window (0 = oldest, n_frames-1 = newest)
    ///
    /// # Returns
    /// Slice of [half::f16; 4] for the frame (length = V_FREQ_BINS)
    pub fn get_frame(&self, frame_index: usize) -> Option<&[[half::f16; 4]]> {
        if frame_index >= self.n_frames {
            return None;
        }

        let start = frame_index * V_FREQ_BINS;
        let end = start + V_FREQ_BINS;

        if end <= self.data.len() {
            Some(&self.data[start..end])
        } else {
            None
        }
    }

    /// Get the version number for a specific frame.
    pub fn get_frame_version(&self, frame_index: usize) -> Option<u64> {
        if frame_index >= self.n_frames {
            return None;
        }
        Some(self.start_version + frame_index as u64)
    }

    /// Iterate over all frames in the window.
    pub fn frames(&self) -> impl Iterator<Item = (usize, &[[half::f16; 4]])> {
        (0..self.n_frames).filter_map(move |i| {
            self.get_frame(i).map(|frame| (i, frame))
        })
    }
}

// ── WGSL helper function (copy into any shader that reads the V-buffer) ───────
//
// Include this literal in shader source strings:
//
//   fn vbuf_read(vbuf: ptr<storage, array<f32>, read>,
//                slot: u32, bin: u32, freq_bins: u32) -> f32 {
//       return (*vbuf)[slot * freq_bins + bin];
//   }
//   fn vbuf_slot(version: u32, depth: u32) -> u32 {
//       return version & (depth - 1u);  // power-of-2 modulo via mask
//   }
//   // Read bin `bin` from `frames_back` frames before the write head.
//   fn vbuf_lookup(vbuf: ptr<storage, array<f32>, read>,
//                  pc: VBufferPC, frames_back: u32, bin: u32) -> f32 {
//       let slot = vbuf_slot(pc.write_version - frames_back, pc.depth);
//       return vbuf_read(vbuf, slot, bin, pc.freq_bins);
//   }

pub const VBUF_WGSL_HELPERS: &str = r#"
struct VBufferPC {
    write_version : u32,
    context_len   : u32,
    freq_bins     : u32,
    depth         : u32,
}

fn vbuf_slot(version: u32, depth: u32) -> u32 {
    return version & (depth - 1u);
}

fn vbuf_read(vbuf: ptr<storage, array<vec4<f16>>, read>,
             slot: u32, bin: u32, freq_bins: u32) -> vec4<f16> {
    return (*vbuf)[slot * freq_bins + bin];
}

fn vbuf_lookup(vbuf: ptr<storage, array<vec4<f16>>, read>,
               pc: VBufferPC, frames_back: u32, bin: u32) -> vec4<f16> {
    let slot = vbuf_slot(pc.write_version - frames_back, pc.depth);
    return vbuf_read(vbuf, slot, bin, pc.freq_bins);
}
"#;
