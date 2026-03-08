# Oz Render Architect Skill

Hybrid clustered forward rendering, mesh shaders, wgpu v28+, GPU resource
management, compute pipeline optimization, bind group layouts, buffer sync.

## Domain
- wgpu compute pipelines (DX12, Vulkan)
- WGSL shader development
- VRAM budget management
- Buffer lifecycle (uniform, storage, readback)
- Bind group layout optimization
- Async readback pipelines
- GPU instance singleton pattern

## Trigger Patterns
"wgpu", "WGSL", "compute shader", "VRAM", "bind group", "GPU pipeline",
"buffer", "shader", "rendering", "graphics", "gpu.rs", "waterfall.rs"

## Available Functions
- `create_gpu_instance()` — Singleton wgpu instance
- `create_compute_pipeline()` — WGSL compile + pipeline
- `manage_vram_budget()` — Track GPU memory usage
- `async_readback()` — Non-blocking buffer readback
- `create_bind_group_layout()` — Optimized BGL creation

## Constants
- `SYNTH_FRAMES = 512`
- `MAX_CHANNELS = 8`
- `MAX_DENIAL_TARGETS = 16`
- `WATERFALL_BINS = 512`
- `WATERFALL_ROWS = 128`

## Code Patterns

### GPU Instance Singleton
```rust
static GPU_INSTANCE: OnceCell<GpuInstance> = OnceCell::new();
```

### Uniform Buffer Alignment (128-byte)
```rust
#[repr(C)]
#[derive(Pod, Zeroable)]
struct UniformBlock {
    // vec4-aligned fields only
}
```

### Async Readback Pattern
```rust
slice.map_async(MapMode::Read, callback);
device.poll(PollType::Wait);
```

### Bind Group Layout Entry Helper
```rust
fn bgl_entry(binding: u32, ty: BufferBindingType) -> BindGroupLayoutEntry
```
