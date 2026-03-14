---
name: async-gpu-parallelism
description: >
  Use this skill when writing Rust code involving high-performance async pipelines (Tokio),
  GPU compute (wgpu/WGSL), and real-time sensor ingestion. It prioritizes async/await over
  manual threading to leverage OS-level scheduling and minimal latency. Triggers include:
  Tokio task design, wgpu pipeline layout, WGSL workgroup sizing (Wave64), SAM/ReBAR 
  memory strategies, and alignment-critical CPU-GPU data transfers.
<example>
Context: User wants to optimize a hardware ingestion loop.
user: "How should I structure the PlutoSDR ingestion loop for minimum latency?"
assistant: "Use a dedicated Tokio task that selecting over a crossbeam channel from the
hardware callback, immediately issuing a SAM write to VRAM via wgpu::Queue::write_buffer."
<commentary>
The user is asking about async ingestion and low-latency GPU pipelines.
</commentary>
</example>
<example>
Context: Designing a compute shader for RDNA2.
user: "What's the best workgroup size for my Laplacian compute shader?"
assistant: "Use @workgroup_size(64, 1, 1). This is the Wave64 mandate for RDNA2/RDNA3,
ensuring a single workgroup maps exactly to one wavefront."
<commentary>
The user is asking about GPU parallelism and hardware-specific optimizations.
</commentary>
</example>
---

# Async-GPU Parallelism (Antigravity Style)

This skill encodes the architectural principles for **Synesthesia**, prioritizing **async/await** over manual thread management. It ensures that the CPU manages the control plane and data movement efficiently while the GPU handles all signal processing.

## 🚀 The Core Philosophy: Async/Await > Threads

In this architecture, manual thread management is an anti-pattern. We leverage **Tokio** to describe dependencies and let the OS handle the actual hardware threading and scheduling.

### Why Async/Await?
1. **OS-Cooperative Scheduling**: The OS handles preemption; `.await` handles voluntary yielding.
2. **Resource Efficiency**: Thousands of tasks can run on a handful of OS threads.
3. **Native Performance**: Modern OS schedulers are highly optimized for these throughput patterns.

---

## 🏗️ CPU Architecture (Tokio)

### Hardware Ingestion Topology
Never block the hardware callback. Use **Crossbeam** as the bridge to **Tokio**.

```rust
// CORRECT: High-performance boundary
// 1. Hardware callback (Dedicated RT thread)
fn hardware_callback(data: &[f32]) {
    // Non-blocking bridge to async land
    tx.try_send(data.to_vec()).ok(); 
}

// 2. Ingestion Dispatcher (Tokio Task)
tokio::spawn(async move {
    let mut rx = rx;
    while let Some(data) = rx.recv().await {
        // SAM Write: CPU -> VRAM (GDDR6) directly
        queue.write_buffer(&target_buffer, 0, bytemuck::cast_slice(&data));
        // GPU submit is async and non-blocking
        queue.submit([]); 
    }
});
```

### The Three Task Tiers:
- **T1: Hardware Ingesters**: Dedicated `tokio::spawn` loops per device (Pico 2, SDR, etc.).
- **T2: GPU Controller**: A single task owning the `wgpu::Queue` to ensure strict submission order.
- **T3: Forensic I/O**: Async file writes (append-only) for forensic integrity.

---

## ⚡ GPU Architecture (wgpu/WGSL)

### The Wave64 Mandate (RDNA2/RDNA3)
Always use `@workgroup_size(64, 1, 1)`. This maps one workgroup directly to one hardware wavefront, maximizing SIMD utilization.

### The 128-Byte Struct Law
Align all CPU->GPU structs to 128 bytes to match RDNA2's **Infinity Cache** line size. Single-transaction memory fetches are critical for performance.

```rust
#[repr(C, align(128))]
pub struct FieldParticle {
    pub position: [f32; 3],
    pub energy: f32,
    pub timestamp_us: u64,
    pub sensor_id: u32,
    pub _reserved: [u8; 104], // Zero anonymous padding; all bytes are contracts.
}
const _: () = assert!(std::mem::size_of::<FieldParticle>() == 128);
```

### The Single-Cross Rule (SAM/ReBAR)
Data crosses from CPU to GPU **exactly once**. 
1. CPU writes to VRAM via `Queue::write_buffer`.
2. All processing (FFT, Mamba, Rendering) happens in-place in VRAM.
3. No readbacks until forensic finalization or frame swap.

---

## 🧪 Common Anti-Patterns

| Mistake | Fix |
|---------|-----|
| `std::thread::sleep` | `tokio::time::sleep` |
| `@workgroup_size(32, 1, 1)` | `@workgroup_size(64, 1, 1)` |
| `std::sync::Mutex` in hot path | `tokio::sync::Mutex` or (preferred) lock-free channels |
| Reallocating GPU buffers per frame | Pre-allocate high-water mark; reuse storage buffers |
| Anonymous padding | Named reservations (`_reserved: [u8; N]`) |

---

## 📓 Physical Constants (Constants as Contracts)
These values are measured from the hardware and the environment. They are never "learned" or "config".

- `F_MIN`: 1.0 (Infrasound)
- `F_MAX`: 700e12 (Visible Light)
- `WAVEFRONT_WIDTH`: 64 (RDNA Hardware)
- `PARTICLE_SIZE`: 128 (Cache Line Match)
