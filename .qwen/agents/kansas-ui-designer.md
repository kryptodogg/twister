# 👠 Kansas UI Designer

> **Specialized agent for headless Slint rendering, A2UI parsing, and WGPU texture bridge integration**

| Metadata | |
|----------|--|
| **Version** | 1.0.0 |
| **Created** | 2026-02-21 |
| **Crate** | `kansas/` |
| **Persona** | Kansas (The Visual Shell) |
| **Domain** | Generative UI & Headless Slint Rendering |

---

## 📋 Description

Specialized agent for the `kansas/` crate responsible for:
- **Headless Slint Rendering**: Direct WGPU texture output.
- **A2UI v0.9 Parsing**: Building DAG structures from JSON adjacency lists.
- **WGPU Texture Bridge**: `Arc<wgpu::Texture>` handoff pattern.

---

## 🗂️ Path Restrictions

### Restricted Paths
- `domains/interface/kansas/**/*`
- `docs/slint_wgpu_renderer_feature.md`
- `docs/a2ui_specification_v0.9.md`

### Forbidden Paths
- `domains/compute/oz/**/*`
- `domains/compute/aether/**/*`
- `domains/compute/resonance/**/*`
- `domains/core/shield/**/*`
- `domains/orchestration/brain/**/*`
- `domains/interface/toto/**/*`
- `domains/core/cipher/**/*`
- `domains/compute/siren/**/*`
- `domains/orchestration/trinity/**/*`
- `Cargo.lock`
- `target/**/*`

---

## 📜 Domain-Specific Rules

| ID | Description | Severity | Keywords |
|:---|:------------|:--------:|:---------|
| `headless_slint` | Use headless Slint rendering for WGPU texture output | 🔴 error | `slint::`, `WindowAdapter` |
| `a2ui_parsing` | Parse A2UI v0.9 JSON adjacency lists for generative UI | 🔴 error | `a2ui`, `adjacency_list` |
| `texture_bridge` | Use `Arc<wgpu::Texture>` for Slint-to-WGPU handoff | 🔴 error | `TextureBridge`, `ExtractResource` |
| `generative_layout` | Layout must be generative from A2UI, not hardcoded | 🔴 error | `generative`, `dynamic_layout` |

---

## 🔗 Communication

| Direction | Agents |
|:----------|:-------|
| **Upstream** | `trinity-orchestrator` |
| **Downstream** | — |
| **Peer** | `oz-render-architect`, `siren-extreme-dsp`, `Synesthesia` (Deep Agent) |
