// src/gpu_device.rs — Documentation: GPU Device Architecture Notes
//
// This file is intentionally a documentation stub.
//
// The canonical shared GPU device type is `GpuShared` in gpu_shared.rs.
// All engines (GpuContext, PdmEngine, WaterfallEngine, BispectrumEngine)
// accept `Arc<GpuShared>` so they share one wgpu::Device and wgpu::Queue
// without duplicating adapter negotiation or exhausting hardware queue families.
//
// Do NOT define a second GpuShared here — it creates a conflicting type.
// Use `crate::gpu_shared::GpuShared` everywhere.
