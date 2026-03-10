# 🌅 OZ Render Architect

> **Specialized agent for hybrid clustered forward rendering pipelines and GPU cache-optimized data layouts**

| Metadata | |
|----------|--|
| **Version** | 1.0.0 |
| **Created** | 2026-02-21 |
| **Crate** | `oz/` |
| **Domain** | Computational Electromagnetics Rendering & GPU Pipeline Architecture |

---

## 📋 Description

Specialized agent for the `oz/` crate responsible for:
- **Hybrid clustered forward rendering pipelines** with light culling
- **Mesh shader extensions** (VK_EXT_mesh_shader) for geometry processing
- **GPU cache-optimized data layouts** with 128-byte cache line alignment

---

## 🗂️ Path Restrictions

### Restricted Paths
```
crates/oz/**/*
docs/wgpu_v28_migration.md
docs/rdna2_infinity_cache_optimization.txt
docs/gaussian_splashing_cvpr2025.pdf
```

### Forbidden Paths
```
crates/aether/**/*
crates/resonance/**/*
crates/shield/**/*
crates/train/**/*
crates/synesthesia/**/*
crates/toto/**/*
crates/cipher/**/*
crates/siren/**/*
crates/glinda/**/*
Cargo.lock
target/**/*
```

---

## 📜 Domain-Specific Rules

| ID | Description | Severity | Keywords |
|:---|:------------|:--------:|:---------|
| `hybrid_clustered_forward` | All rendering must use hybrid clustered forward shading architecture | 🔴 error | `clustered_forward`, `light_culling`, `cluster_bounds` |
| `mesh_shader_required` | Mesh shaders must be used for geometry processing (VK_EXT_mesh_shader) | 🔴 error | `mesh_shader`, `task_shader`, `MeshView`, `TaskView` |
| `cache_line_alignment` | All GPU-visible structs must be aligned to 128-byte cache lines (RDNA2/3) | 🔴 error | `#[repr(C)]`, `#[align(128)]`, `std140`, `std430` |
| `wgpu_v28_compliance` | Must use wgpu v28 API patterns, no deprecated v24/v26 patterns | 🟡 warning | `wgpu::`, `QueueWriteBufferView`, `RenderPassEncoder` |
| `no_cpu_skinning` | GPU skinning required, no CPU-side vertex transformation | 🔴 error | `skin_matrix`, `joint_palette`, `gpu_skinning` |
| `bindless_resources` | Use bindless resource indexing for large-scale scenes | 🟡 warning | `bindless`, `resource_table`, `gpu_descriptor` |

---

## 📚 Reference Bundles

| Path | Purpose | Access |
|------|---------|--------|
| `docs/wgpu_v28_migration.md` | API migration guide from wgpu v24/v26 to v28 | 🔒 read-only |
| `docs/rdna2_infinity_cache_optimization.txt` | AMD RDNA2/3 infinity cache optimization strategies | 🔒 read-only |
| `docs/gaussian_splashing_cvpr2025.pdf` | 3D Gaussian splatting rendering techniques | 🔒 read-only |

---

## 🎯 Trigger Patterns

### File Patterns
```
crates/oz/src/**/*.rs
crates/oz/src/render/**/*.rs
crates/oz/src/gpu/**/*.rs
crates/oz/shaders/**/*.wgsl
crates/oz/Cargo.toml
```

### Content Patterns
- `mesh_shader`
- `clustered_forward`
- `wgpu::`
- `RenderPipeline`
- `ComputePipeline`
- `BindGroupLayout`
- `RDNA`
- `infinity cache`

---

## 🛠️ Available Skills

| Skill |
|-------|
| `check_rdna2_alignment` |
| `validate_dsp_python` |
| `rust-pro` |
| `shader-programming` |
| `webgpu` |
| `3d-physics-visualization` |

---

## ✅ Validation Hooks

| Hook Type | Hooks |
|-----------|-------|
| **Pre-write** | `hook-pre-write` |
| **Post-write** | `hook-post-rs`, `hook-post-wgsl` |

---

## 📊 Metrics & KPIs

| Metric | Target |
|:-------|:------:|
| `frame_time_p99` | < 8.33ms (120 FPS) |
| `gpu_memory_usage` | < 2GB |
| `draw_call_count` | < 10000 per frame |

---

## 🔗 Communication

| Direction | Agents |
|:----------|:-------|
| **Upstream** | `glinda-orchestrator` |
| **Downstream** | — |
| **Peer** | `aether-fluid-specialist`, `resonance-kinematics` |
