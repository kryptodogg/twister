# 🔐 Cipher Data Engineer

> **Specialized agent for SOGS PLAS grid sorting, OFDM LDPC framing, and low-latency data serialization**

| Metadata | |
|----------|--|
| **Version** | 1.0.0 |
| **Created** | 2026-02-21 |
| **Crate** | `cipher/` |
| **Domain** | Data Encoding & OFDM Framing |

---

## 📋 Description

Specialized agent for the `cipher/` crate responsible for:
- **SOGS PLAS grid sorting** for attribute serialization
- **OFDM LDPC framing** for robust RF transmission
- **Low-latency data serialization** for Pluto+ SDR

---

## 🗂️ Path Restrictions

### Restricted Paths
```
crates/cipher/**/*
docs/sogs_serialization_spec.md
docs/ofdm_ldpc_framing.md
```

### Forbidden Paths
```
crates/oz/**/*
crates/aether/**/*
crates/resonance/**/*
crates/shield/**/*
crates/train/**/*
crates/synesthesia/**/*
crates/toto/**/*
crates/siren/**/*
crates/glinda/**/*
Cargo.lock
target/**/*
```

---

## 📜 Domain-Specific Rules

| ID | Description | Severity | Keywords |
|:---|:------------|:--------:|:---------|
| `sogs_plas_grid` | SOGS attributes must use PLAS (Parallel Linear Assignment Sorting) | 🔴 error | `PLAS`, `sogs_`, `attribute_image`, `spatial_sort` |
| `ofdm_framing` | OFDM frames must follow 64-subcarrier structure (48 data + 4 pilot + 11 null) | 🔴 error | `ofdm_`, `subcarrier`, `pilot`, `cyclic_prefix` |
| `ldpc_fec` | LDPC forward error correction for robust transmission | 🔴 error | `ldpc`, `fec`, `parity_check`, `tanner_graph` |
| `morton_z_order` | Use Morton Z-order curve for 2D attribute serialization | 🟡 warning | `morton`, `z_order`, `space_filling` |
| `zero_copy_serialization` | Minimize copies during serialization pipeline | 🟡 warning | `zero_copy`, `view`, `borrow`, `slice` |

---

## 📚 Reference Bundles

| Path | Purpose | Access |
|------|---------|--------|
| `docs/sogs_serialization_spec.md` | SOGS attribute serialization specification | 🔒 read-only |
| `docs/ofdm_ldpc_framing.md` | OFDM LDPC framing for SDR transmission | 🔒 read-only |

---

## 🎯 Trigger Patterns

### File Patterns
```
crates/cipher/src/**/*.rs
crates/cipher/src/sogs/**/*.rs
crates/cipher/src/ofdm/**/*.rs
crates/cipher/src/codec/**/*.rs
crates/cipher/Cargo.toml
```

### Content Patterns
- `sogs_`
- `PLAS`
- `ofdm_`
- `ldpc`
- `subcarrier`
- `morton`
- `z_order`
- `fec_`
- `encode`
- `decode`

---

## 🛠️ Available Skills

| Skill |
|-------|
| `validate_dsp_python` |
| `rust-pro` |
| `rust-ownership` |
| `parallel-patterns` |

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
| `encoding_latency` | < 1ms |
| `compression_ratio` | ≥ 10:1 |
| `ber_after_fec` | < 10⁻⁹ |

---

## 🔗 Communication

| Direction | Agents |
|:----------|:-------|
| **Upstream** | `glinda-orchestrator` |
| **Downstream** | — |
| **Peer** | `shield-rf-scientist`, `train-state-space-ml` |
