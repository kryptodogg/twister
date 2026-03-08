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
pub const V_BUF_BYTES: u64 = (V_BUF_CELLS * 4) as u64;

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
            size: (V_FREQ_BINS * 4) as u64, // one row at a time
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
    pub fn push_frame(&mut self, queue: &wgpu::Queue, magnitudes: &[f32]) {
        let slot = (self.meta.version as usize) % V_DEPTH;
        let offset = (slot * V_FREQ_BINS * 4) as u64;

        let mut row = [0.0f32; V_FREQ_BINS];
        let n = magnitudes.len().min(V_FREQ_BINS);
        row[..n].copy_from_slice(&magnitudes[..n]);

        queue.write_buffer(&self.buffer, offset, bytemuck::cast_slice(&row));
        self.meta.version += 1;
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

fn vbuf_read(vbuf: ptr<storage, array<f32>, read>,
             slot: u32, bin: u32, freq_bins: u32) -> f32 {
    return (*vbuf)[slot * freq_bins + bin];
}

fn vbuf_lookup(vbuf: ptr<storage, array<f32>, read>,
               pc: VBufferPC, frames_back: u32, bin: u32) -> f32 {
    let slot = vbuf_slot(pc.write_version - frames_back, pc.depth);
    return vbuf_read(vbuf, slot, bin, pc.freq_bins);
}
"#;
