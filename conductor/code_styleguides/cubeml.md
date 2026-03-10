# CubeCL Style Guide

## Purpose
Define standards for CubeCL low-level GPU compute primitives in Project Twister, focusing on real-time signal reconstruction, waterfall visualization, and kernel performance optimization for AMD RX 6700 XT (12GB VRAM).

## Core Principles
1. **VRAM-First Design**: All kernels must explicitly manage VRAM allocation/deallocation
2. **Zero-Copy Kernels**: Direct memory access between DSP buffers and GPU compute shaders
3. **Deterministic Execution**: Kernel runtime must be bounded and predictable
4. **AMD RDNA2 Optimization**: Leverage matrix cores and infinity cache

## Kernel Definitions

### Waterfall Downsampling Kernel
```rust
use cubecl::prelude::*;

#[cube(launch)]
pub fn waterfall_downsample_kernel(
    src: &Tensor<f32>,
    dst: &Tensor<f32>,
    src_cols: u32,
    src_rows: u32,
    dst_cols: u32,
    dst_rows: u32,
) {
    let col = ABSOLUTE_POS_X;
    let row = ABSOLUTE_POS_Y;

    if col >= dst_cols || row >= dst_rows {
        return;
    }

    // Bilinear interpolation from 256×128 → 128×64
    let src_col = (col * src_cols / dst_cols) as usize;
    let src_row = (row * src_rows / dst_rows) as usize;
    let src_idx = src_row * src_cols as usize + src_col;
    let dst_idx = row as usize * dst_cols as usize + col as usize;

    dst[dst_idx] = src[src_idx];
}
```

### Bispectrum Coherence Kernel (O(N²) → O(N) via parallel reduction)
```rust
#[cube(launch_unchecked)]
pub fn bispectrum_coherence_kernel(
    spectrum: &Tensor<f32>,
    coherence: &Tensor<f32>,
    n_bins: u32,
    threshold: f32,
) {
    let f1 = ABSOLUTE_POS_X;
    let f2 = ABSOLUTE_POS_Y;

    if f1 >= n_bins || f2 >= n_bins || f2 < f1 {
        return;
    }

    let idx = (f2 * n_bins + f1) as usize;

    // Skip low-magnitude bins (sparse coherence)
    let mag1 = spectrum[f1 as usize].abs();
    let mag2 = spectrum[f2 as usize].abs();

    if mag1 < threshold || mag2 < threshold {
        coherence[idx] = 0.0;
        return;
    }

    // Phase coherence calculation
    let phase_diff = (spectrum[f2 as usize] / spectrum[f1 as usize]).atan2();
    coherence[idx] = phase_diff.cos().abs();
}
```

## VRAM Management Rules

### Allocation Strategy
```rust
use cubecl::runtime::Runtime;
use cubecl::wgpu::WgpuRuntime;

pub struct VramManager {
    runtime: WgpuRuntime,
    // Reserve 2GB for DSP + 8GB for ML models + 2GB framebuffer
    reserved_mb: usize,
    allocated_mb: AtomicUsize,
}

impl VramManager {
    pub fn new(runtime: WgpuRuntime) -> Self {
        Self {
            runtime,
            reserved_mb: 2048, // 2GB for waterfall/bispectrum
            allocated_mb: AtomicUsize::new(0),
        }
    }

    pub fn can_allocate(&self, size_mb: usize) -> bool {
        let current = self.allocated_mb.load(Ordering::Relaxed);
        current + size_mb <= self.reserved_mb
    }

    pub fn allocate<T: CubeElement>(&self, n_elements: usize) -> Result<Buffer<T>, VramError> {
        let size_mb = (n_elements * std::mem::size_of::<T>()) / (1024 * 1024);
        if !self.can_allocate(size_mb) {
            return Err(VramError::OutOfMemory);
        }

        self.allocated_mb.fetch_add(size_mb, Ordering::Relaxed);
        Ok(self.runtime.create_buffer(n_elements))
    }
}
```

### Deallocation Policy
```rust
impl Drop for VramManager {
    fn drop(&mut self) {
        // Explicit VRAM release on shutdown
        self.runtime.clear_cache();
    }
}

// RAII wrapper for GPU buffers
pub struct GpuBuffer<T: CubeElement> {
    buffer: Buffer<T>,
    size_mb: usize,
    manager: Arc<VramManager>,
}

impl<T: CubeElement> Drop for GpuBuffer<T> {
    fn drop(&mut self) {
        self.manager.allocated_mb.fetch_sub(self.size_mb, Ordering::Relaxed);
    }
}
```

## Kernel Performance Optimization

### RDNA2-Specific Optimizations

#### Matrix Core Usage (for bispectrum N×N matrix)
```rust
#[cube]
pub fn bispectrum_matrix_core(
    spectrum: &Tensor<f32>,
    output: &Tensor<f32>,
) {
    // Use RDNA2 matrix cores for 16×16 tile processing
    let tile_size = 16;
    let tile_x = ABSOLUTE_POS_X / tile_size;
    let tile_y = ABSOLUTE_POS_Y / tile_size;

    // Load tile into shared memory
    let mut tile = Array::<f32, 16>::new();

    for i in 0..tile_size {
        for j in 0..tile_size {
            let idx = (tile_y * tile_size + i) * 256 + (tile_x * tile_size + j);
            tile[i * 16 + j] = spectrum[idx];
        }
    }

    // Matrix multiplication via matrix cores
    // ... (implementation uses RDNA2 MFMA instructions)
}
```

#### Infinity Cache Optimization
```rust
// Align tensor data to 64-byte cache lines
#[repr(align(64))]
pub struct AlignedTensor {
    data: Vec<f32>,
}

impl AlignedTensor {
    pub fn new(n: usize) -> Self {
        // Allocate with 64-byte alignment for infinity cache efficiency
        let mut data = Vec::with_capacity(n);
        data.resize(n, 0.0);
        Self { data }
    }
}
```

### Kernel Launch Configuration
```rust
pub fn launch_waterfall_kernel(
    runtime: &WgpuRuntime,
    src: &Tensor<f32>,
    dst: &Tensor<f32>,
) {
    let dst_cols = 128;
    let dst_rows = 64;

    // Cube count for RX 6700 XT (40 compute units)
    let cube_count = CubeCount::Static(dst_cols / 16, dst_rows / 16, 1);

    unsafe {
        waterfall_downsample_kernel::launch::<f32>(
            runtime,
            cube_count,
            src,
            dst,
            256,
            128,
            dst_cols,
            dst_rows,
        );
    }
}
```

## Real-Time Waterfall Visualization

### Frame Pipeline
```rust
pub struct WaterfallPipeline {
    gpu_buffer: GpuBuffer<f32>,
    cpu_buffer: Vec<f32>,
    vram_manager: Arc<VramManager>,
}

impl WaterfallPipeline {
    pub fn render_frame(&mut self, spectrum: &[f32]) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        // 1. Upload spectrum to GPU (zero-copy via mapped buffer)
        self.gpu_buffer.write(spectrum);

        // 2. Launch downsampling kernel
        launch_waterfall_kernel(&self.runtime, &self.gpu_buffer, &self.display_buffer);

        // 3. Read back for display (async, non-blocking)
        let pixels = self.display_buffer.read_async();

        // 4. Convert to RGBA image
        self.pixels_to_rgba(pixels)
    }

    // Target: ≤2ms per frame (500 FPS)
    pub fn frame_budget_ms() -> u64 { 2 }
}
```

## Performance Benchmarks

### Target Metrics (RX 6700 XT)
| Kernel | Target Runtime | VRAM Usage |
|--------|---------------|------------|
| Waterfall Downsample (256×128 → 128×64) | ≤0.3ms | 1 MB |
| Bispectrum Coherence (sparse, 256 bins) | ≤1.5ms | 64 MB |
| Spectrum FFT (2048 bins) | ≤0.5ms | 8 MB |
| Total Visualization Pipeline | ≤3ms | 100 MB |

### Memory Safety Checklist
- [ ] All GPU buffers use `GpuBuffer` RAII wrapper
- [ ] VRAM budget checked before kernel launch
- [ ] Kernel cube counts match hardware (40 CU for RX 6700 XT)
- [ ] 64-byte alignment for infinity cache efficiency
- [ ] Async read-back for non-blocking display
- [ ] Explicit `Drop` for all GPU resources

## References
- [CubeCL Documentation](https://docs.rs/cubecl/latest/cubecl/)
- [AMD RDNA2 Architecture Whitepaper](https://www.amd.com/content/dam/amd/en/documents/rdna-architecture-whitepaper.pdf)
- [WGPU Compute Shader Best Practices](https://docs.rs/wgpu/latest/wgpu/)
