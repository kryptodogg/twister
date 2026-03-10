---
name: hardware-expert
description: Specialized hardware expert for Project Cyclone. Covers AMD RDNA2/Zen3 Bare Metal, wgpu optimization, NixOS OS integration, and SAM/Resizable BAR strategies.
---

# Hardware Expert (Project Cyclone)

This skill provides deep technical expertise for the "Computational Forge" of Project Cyclone, focusing on absolute possession of silicon (RDNA2/Zen3), zero-overhead `wgpu` programming, and OS-level determinism via NixOS.

## 🎯 Core Hardware Arsenal

### 1. AMD RDNA2 & Zen 3 Bare Metal (RX 6700 XT / Ryzen 7 5700X)
- **Wave32 Nativism:** Execute in groups of 32 threads to minimize latency.
- **192-bit System Bus & 4-bit Quantization:** Maximize throughput using 4-bit quantization, reclaiming accuracy via ML.
- **Unified 32MB L3 Cache:** Design CPU structures for extreme spatial locality to stay within the Zen 3 L3 cache.
- **128-Byte Law:** Strictly align critical structs (e.g., `HeterodynePayload`) to 128-byte boundaries to perfectly match cache lines. Multiples of 3, 4, 6 establish the optimal rhythm.
- **Coalesced Memory Access:** In WGSL, use `global_invocation_id` to ensure adjacent threads access adjacent memory locations, maximizing memory controller throughput.
- **ALU Optimization:** Use WGSL built-ins (e.g., `builtin(local_invocation_id)`) to save ALU cycles instead of manual calculation. Avoid excessive `workgroupBarrier()` calls.

### 2. High-Performance `wgpu` Buffer & DMA Strategy
- **Smart Access Memory (SAM):** Treat as a high-bandwidth, high-latency tier. Requires explicit DMA strategies, moving data between Host-Visible, Device-Local, and Host-Accessible/Device-Visible memory.
- **Static Geometry/Data:** Use `COPY_DST`. Upload once, read-only by GPU for max bandwidth.
- **Dynamic Data (CPU -> GPU):** Use `COPY_SRC` with a ring-buffer pattern (e.g., triple buffering). CPU writes to a host-visible staging buffer, mapped asynchronously via `poll_map_async`, hiding upload latency and avoiding synchronous mapping stalls.
- **Compute Storage:** Use `STORAGE` for general read/write from compute shaders.
- **Indirect Commands:** Use `COPY_SRC | INDIRECT` for host-writable, GPU-consumable indirect drawing arguments.
- **Execution Decoupling:** Use Rust `Futures` to coordinate multi-frame command submission, preparing future frames while the GPU executes the current one.

### 3. Deep OS Integration (NixOS)
- **Kernel Determinism:** Apply `PREEMPT_RT` patchset for a fully preemptible real-time kernel to reduce context-switch latencies.
- **CPU Isolation:** Use the `isolcpus` kernel parameter and `sched_setaffinity` to pin threads and shield the audio/DSP loops from background system tasks.
- **IOMMU:** Ensure IOMMU is enabled and correctly configured (`drm/amdgpu` driver) to guarantee predictable DMA access and SAM stability.
- **Declarative Stack:** Manage all dependencies, including the Rust toolchain, strictly via `configuration.nix`.

### 4. GPU Performance Profiling (AMD GPUPerfAPI 4.2)
- **Deterministic Counter Access:** Initialize GPA *before* creating the Vulkan instance and `wgpu` device/queue to ensure the loader hooks the correct entry points.
- **Context Flow:** `GpaInitialize` -> `GpaOpenContext` (passing the Vulkan queue) -> `GpaCreateSession(kGpaSessionSampleTypeDiscreteCounter)`.
- **Session Lifecycle:** Always `GpaBeginSession` before your primary compute/render loop and `GpaEndSession` after.
- **Sample Management:** Wrap each dispatch in a `GpaBeginSample` / `GpaEndSample` block. Use a unique ID for each kernel (e.g., `synthesis_kernel`, `fft_kernel`).
- **Data Collection:** Call `GpaGetSampleResult` asynchronously after `GpaEndSession` to avoid stalling the graphics pipeline.
- **Binary Compatibility:** Use the API-specific loader (e.g., `GPUPerfAPIVK-x64.dll`) for Vulkan/wgpu.

## 🛠️ Engineering Doctrines
1. **Zero-Copy Serialization:** Use `#[repr(C)]` and `bytemuck` for direct host-to-GPU byte slice interpretation.
2. **Asynchronous I/O Forge:** Leverage `io_uring` for unbottlenecked streaming of storage datasets.
3. **Silicon-Aware Profiling:** Treat performance counters as first-class citizens; if a kernel cannot be measured, it does not exist.
