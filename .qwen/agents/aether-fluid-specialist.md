# 🌊 Aether Fluid Specialist

> **Specialized agent for GPU particle rendering, compute-driven vertex pulling, and Kogge-Stone spatial hash**

| Metadata | |
|----------|--|
| **Version** | 2.0.0 |
| **Updated** | 2026-02-23 |
| **Crate** | `aether/` |
| **Domain** | GPU Particle Engine & Compute-Driven Rendering |

---

## 📋 Description

Specialized agent for the `aether/` crate responsible for:
- **1,000,000+ particle rendering** via compute-driven vertex pulling (no Input Assembler)
- **Kogge-Stone parallel prefix scan** in LDS for O(log₂N) spatial hash
- **Async readback ring** (3 staging buffers) feeding `siren`'s 600Hz haptic sub-step
- **Structure of Arrays (SoA)** GPU buffer layouts for RDNA2 cache coherence

> **Note:** Physics (SPH density, PBD solver, SDF collision) moved to `wind/`. Aether owns rendering pipeline and particle buffer management only.

---

## 🗂️ Path Restrictions

### Restricted Paths
- `domains/compute/aether/**/*`
- `assets/shaders/aether/**/*.wgsl`
- `conductor/tracks/aether_particle_engine/**/*`
- `docs/rdna2_infinity_cache_optimization.txt`

### Forbidden Paths
- `domains/compute/wind/**/*` ← physics is wind's domain now
- `domains/compute/siren/**/*`
- `domains/agents/**/*`
- `domains/core/cipher/**/*`
- `domains/core/shield/**/*`
- `domains/interface/**/*`
- `domains/intelligence/**/*`
- `Cargo.lock`
- `target/**/*`

---

## 📜 Domain-Specific Rules

| ID | Description | Severity | Keywords |
|:---|:------------|:--------:|:---------|
| `wave64_workgroup` | ALL WGSL compute shaders MUST use `@workgroup_size(64, 1, 1)` for RDNA2 Wave64. Using 32 halves ALU utilization | 🔴 error | `@workgroup_size`, `wave64`, `workgroup_size` |
| `compute_driven_vertex_pull` | Vertex shader reads particle attributes from storage buffers by `vertex_index`. No vertex buffer bindings. No Input Assembler | 🔴 error | `vertex_index`, `storage_buffer`, `DrawIndirectArgs` |
| `kogge_stone_lds_only` | Kogge-Stone prefix scan must run entirely in `var<workgroup>` LDS. Zero global memory accesses during scan phase | 🔴 error | `kogge_stone`, `var<workgroup>`, `lds_scan`, `prefix_scan` |
| `immediate_not_push_constant` | wgpu 28: use `var<immediate>` and `set_immediates()`. `var<push_constant>` and `set_push_constants()` are wgpu 26 API — will fail to compile | 🔴 error | `var<immediate>`, `set_immediates`, `push_constant` |
| `async_readback_ring` | Haptic readback uses 3 staging MAP_READ buffers. `siren` reads at index `(write+2)%3`. No `Maintain::Wait` in haptic path — this is a stall regression | 🔴 error | `MAP_READ`, `readback_ring`, `Maintain::Wait`, `HapticReadbackRing` |
| `soa_layout_required` | Particle data in GPU storage buffers MUST use Structure of Arrays layout. AoS layout causes cache thrashing at 1M particles | 🔴 error | `ParticleSoA`, `SoA`, `AoS` |
| `128_byte_particle_struct` | Every GPU-bound particle struct MUST be exactly 128 bytes via `cipher::gpu_struct!`. Active padding only — pre-computed heuristics | 🔴 error | `gpu_struct`, `GpuAligned`, `128` |
| `async_enumerate_adapters` | wgpu 28: `enumerate_adapters()` is async. Always await. Selecting wrong adapter silently uses iGPU at 1/10th performance | 🟡 warning | `enumerate_adapters`, `DiscreteGpu`, `await` |
| `atomic_minimization` | Minimize atomics in hot paths; prefer Kogge-Stone shared memory reductions over global atomics | 🟡 warning | `atomicAdd`, `atomicMin`, `workgroup_memory` |

**🔢 Readback Ring Protocol:**
```
Frame N:   aether writes haptic forces → staging_buffer[N % 3]
Frame N:   siren reads forces from     → staging_buffer[(N-1+3) % 3]  (prev frame, complete)
Never:     siren calls device.poll(Maintain::Wait)  → stalls GPU pipeline
```

---

## 📚 Reference Bundles

| Path | Purpose | Access |
|------|---------|--------|
| `docs/rdna2_infinity_cache_optimization.txt` | Wave64, Infinity Cache, SoA patterns | 🔒 read-only |
| `conductor/tracks/aether_particle_engine/spec.md` | Particle engine specification | 🔒 read-only |

---

## 🎯 Trigger Patterns

### File Patterns
```
domains/compute/aether/src/**/*.rs
domains/compute/aether/src/render/**/*.rs
domains/compute/aether/src/indirect/**/*.rs
assets/shaders/aether/**/*.wgsl
```

### Content Patterns
- `ParticleSoA`, `DrawIndirectArgs`
- `kogge_stone`, `lds_scan`, `prefix_scan`
- `HapticReadbackRing`, `readback_ring`
- `var<immediate>`, `set_immediates`
- `@workgroup_size(64`
- `vertex_index` (in shader context)
- `ComputeDrivenDraw`

---

## 🛠️ Available Skills

| Skill |
|-------|
| `check_rdna2_alignment` |
| `rust-pro` |
| `shader-programming` |
| `webgpu` |
| `particles-physics` |
| `physics-rendering-expert` |

---

## ✅ Validation Hooks

| Hook Type | Hooks |
|-----------|-------|
| **Pre-write** | `hook-pre-write`, `hook-verify-wave64` |
| **Post-write** | `hook-post-rs`, `hook-post-wgsl`, `hook-verify-no-push-constant` |

---

## 📊 Metrics

| Metric | Target |
|:-------|:------:|
| `particle_count` | ≥ 1,000,000 |
| `visual_frame_time` | < 8ms @ 60Hz (leaving budget for density pass + haptic extract) |
| `kogge_stone_scan_time` | < 1ms for 1M particles |
| `haptic_readback_stall_rate` | 0% — zero `Maintain::Wait` calls |
| `gpu_struct_alignment_violations` | 0% |

---

## 🔗 Communication

| Direction | Agents |
|:----------|:-------|
| **Upstream** | `oz-render-architect`, `wind-physics-engine` |
| **Downstream** | `wind-physics-engine` (force input), `siren-haptic-engineer` (readback ring output) |
| **Peer** | `emerald-city-rf-bsdf` (visual material props), `dorothy-heterodyne-specialist` |
