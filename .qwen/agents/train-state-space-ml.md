# 🚂 Train State-Space ML

> **Specialized agent for Burn 0.21 IQUMamba-1D complex-valued selective state-space models**

| Metadata | |
|----------|--|
| **Version** | 1.0.0 |
| **Created** | 2026-02-21 |
| **Crate** | `train/` |
| **Domain** | State-Space Machine Learning & Neural Inference |

---

## 📋 Description

Specialized agent for the `train/` crate responsible for:
- **Burn 0.21 IQUMamba-1D** complex-valued selective state-space models
- **Zero-copy cubecl-wgpu backends** for GPU tensor operations
- **Real-time RF signal classification** with selective state-space (S6) blocks

---

## 🗂️ Path Restrictions

### Restricted Paths
```
crates/train/**/*
docs/burn_custom_wgpu_backend_guide.md
docs/iqumamba_architecture.md
```

### Forbidden Paths
```
crates/oz/**/*
crates/aether/**/*
crates/resonance/**/*
crates/shield/**/*
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
| `burn_021_version` | Must use Burn v0.21-pre1 for IQUMamba-1D compatibility | 🔴 error | `burn = "0.21"`, `burn-wgpu`, `cubecl-wgpu` |
| `iqumamba_1d` | Use complex-valued IQUMamba-1D for baseband signal processing | 🔴 error | `IquMamba1D`, `complex_ssm`, `selective_state` |
| `zero_copy_backend` | Use zero-copy cubecl-wgpu backend for GPU tensor operations | 🔴 error | `cubecl::wgpu`, `zero_copy`, `wgpu_buffer` |
| `iq_dtype` | I/Q data must use `Complex<f32>` or `Complex<f64>` tensors | 🔴 error | `Complex<f32>`, `Complex<f64>`, `num_complex` |
| `gradient_checkpointing` | Use gradient checkpointing for long sequence training | 🟡 warning | `checkpoint`, `gradient_accumulation`, `backward` |

---

## 📚 Reference Bundles

| Path | Purpose | Access |
|------|---------|--------|
| `docs/burn_custom_wgpu_backend_guide.md` | Custom WGPU backend implementation for Burn | 🔒 read-only |
| `docs/iqumamba_architecture.md` | IQUMamba-1D architecture specification | 🔒 read-only |

---

## 🎯 Trigger Patterns

### File Patterns
```
crates/train/src/**/*.rs
crates/train/src/ml/**/*.rs
crates/train/src/mamba/**/*.rs
crates/train/Cargo.toml
```

### Content Patterns
- `burn::`
- `IquMamba`
- `state_space`
- `cubecl`
- `wgpu_backend`
- `Complex<`
- `tensor::`
- `autodiff`

---

## 🛠️ Available Skills

| Skill |
|-------|
| `validate_dsp_python` |
| `rust-pro` |
| `domain-ml` |
| `ml-pipeline-workflow` |
| `langchain-architecture` |

---

## ✅ Validation Hooks

| Hook Type | Hooks |
|-----------|-------|
| **Pre-write** | `hook-pre-write` |
| **Post-write** | `hook-post-rs` |

---

## 📊 Metrics

| Metric | Target |
|:-------|:------:|
| `inference_latency` | < 1ms |
| `sequence_length` | ≥ 4096 |
| `training_throughput` | ≥ 1000 samples/sec |

---

## 🔗 Communication

| Direction | Agents |
|:----------|:-------|
| **Upstream** | `glinda-orchestrator` |
| **Downstream** | — |
| **Peer** | `shield-rf-scientist`, `cipher-data-engineer` |
