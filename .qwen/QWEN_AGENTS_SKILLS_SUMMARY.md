# Qwen CLI Sub-Agents and Skills for Project Oz

## Overview

This document summarizes the comprehensive set of Qwen CLI sub-agents and skills created for the SHIELD/Project Oz codebase.

---

## Created Agents (`.qwen/agents/`)

### 1. `dorothy-heterodyne-specialist.yml`
**Domain:** Active Heterodyne Engine & Super-Nyquist SDR  
**Crate:** `dorothy/`

**Responsibilities:**
- WGPU heterodyne compute shaders (complex I/Q mixing)
- Super-Nyquist folding mode for intentional aliasing
- PDM haptic encoding for infrasound (< 20 Hz)
- LangChain agent integration (radar-llm + point-llm)
- Pluto+ SDR programming (AD9363, ≤ 25 MSPS)

**Key Rules:**
- Complex mixing: `I_out = I_in·cos(θ) - Q_in·sin(θ)`
- Wave32 workgroups: `@workgroup_size(32, 1, 1)`
- I²t thermal guard for PDM (≤ 60% duty cycle)

---

### 2. `tri-modal-defense-specialist.yml`
**Domain:** RF/Acoustic/Optical Active Denial  
**Crate:** `shield/tri_modal/`

**Responsibilities:**
- Four-layer defense: Sense → Cloak → Reflect → Deny
- Cross-modal correlation (≥ 0.8 confidence)
- IQUMamba-1D classification (≥ 10 dB SI-SDR)
- RIS metasurface cloaking (> 10 dB RCS reduction)
- Active jamming (> 20 dB SNR reduction)

**Scenarios:**
- RF-1/RF-2/RF-4/RF-5: CPU EM, Wi-Fi sensing, SATA scanning
- AC-1 through AC-5: Fansmitter, PIXHELL, HDD, PSU, ultrasonic
- XM-5/XM-7: Multi-modal fusion, adaptive adversary

---

### 3. `mmwave-fusion-specialist.yml`
**Domain:** 60GHz FMCW Radar Integration  
**Crate:** `shield/hal/mmwave/`

**Responsibilities:**
- XIAO MR60BHA2 (all-in-one: 60GHz mmWave + ESP32-C6 integrated)
- HLK-2410 (24GHz mmWave + ESP32-WROOM UART bridge)
- mmWave + acoustic fusion for Fansmitter detection
- PIXHELL vibration detection (> 0.05mm @ 10-18 kHz)
- Backscatter tag interaction

**Hardware Inventory:**
- MR60BHA2: 60-61.5 GHz, 0.1mm resolution, UART via USB-C (no bridge needed)
- HLK-2410: 24-24.25 GHz, GPIO/UART output
- ESP32-WROOM × 2: UART bridge for HLK-2410 or WiFi sensor nodes

---

### 4. `crystal-ball-reconstruction.yml`
**Domain:** Inverse RF-BSDF Neural Reconstruction  
**Crate:** `crystal_ball/`

**Responsibilities:**
- RF-Vim (Vision Mamba for RF) architecture
- Bilinear (Tustin) discretization (NOT ZOH)
- Hermitian inner product loss for phase coherence
- Burn 0.21-pre1 WGPU backend
- Material property prediction (ε', ε'', σ, α)

**Model Architecture:**
- Input: [B, 2, 32768] (I/Q stacked)
- Patch embedding: Conv1d(kernel=32, stride=16)
- 6× Selective SSM blocks (state_dim=16, heads=8)
- Output: permittivity, conductivity, roughness

---

### 5. `synesthesia-holographic-ui.yml`
**Domain:** Holographic Generative UI  
**Crate:** `synesthesia/`

**Responsibilities:**
- A2UI → Slint translation (topological sort)
- Runtime Slint compilation via `slint_interpreter`
- Headless WGPU texture sharing
- Hologram shader (emissive bloom, scanlines)
- MCP server integration

**MCP Tools:**
- Read: `get_simulation_status`, `get_haptic_readback`, `get_rf_metrics`
- Write: `set_environment_wetness`, `spawn_fluid_volume`, `inject_a2ui_payload`

---

### 6. `glinda-mcp-orchestrator.yml`
**Domain:** Agentic Orchestration & MCP Server  
**Crate:** `glinda/`

**Responsibilities:**
- JSON-RPC 2.0 MCP server
- tokio::mpsc message passing
- Tool registration with schema validation
- Telemetry streaming (subscribe/listChanged)
- Safety bounds for write tools

**Message Types:**
- `McpCommand`: SetWetness, SpawnParticles, InjectA2UI, Subscribe
- `McpTelemetry`: SimulationStatus, HapticPayload, RfMetrics, StateDelta

---

## Created Skills (`.qwen/skills/`)

### 1. `validate_rf_bsdf_fresnel.md`
**Purpose:** Validate exact Fresnel equations (NO Schlick)  
**Agents:** shield-rf-scientist, tri-modal-defense-specialist, crystal-ball-reconstruction

**Validation:**
- Complex refractive index ñ = n + iκ
- Reflectance R_s, R_p for both polarizations
- Energy conservation: R + T = 1

**Script:** `scripts/validate_fresnel.py`

---

### 2. `verify_iqumamba_1d_inference.md`
**Purpose:** Validate RF-Vim inference pipeline  
**Agents:** crystal-ball-reconstruction, train-state-space-ml, tri-modal-defense-specialist

**Validation:**
- Input shape: [B, 2, 32768] (I/Q stacked)
- Bilinear discretization (Tustin transform)
- Hermitian loss for phase coherence
- Inference latency < 1 ms
- SI-SDR improvement ≥ 10 dB

**Script:** `scripts/validate_iqumamba.py` (TODO)

---

### 3. `validate_heterodyne_kernel.md`
**Purpose:** Validate WGPU heterodyne compute kernel  
**Agents:** dorothy-heterodyne-specialist, shield-rf-scientist

**Validation:**
- Complex mixing formula correctness
- Wave32 workgroup size
- 128-byte HeterodynePayload alignment
- Frequency accuracy < 0.01 Hz
- Phase error < 0.1° per sample

**Script:** `scripts/validate_heterodyne.py`

---

### 4. `verify_tri_modal_correlation.md`
**Purpose:** Validate multi-modal threat fusion  
**Agents:** tri-modal-defense-specialist, mmwave-fusion-specialist

**Validation:**
- Correlation confidence ≥ 0.8 for confirmed threats
- Detection latency: RF < 200ms, acoustic < 100ms, optical < 50ms
- RCS reduction > 10 dB
- Jamming effectiveness > 20 dB SNR reduction

**Script:** `scripts/validate_tri_modal.py` (TODO)

---

### 5. `validate_sph_parameters.md`
**Purpose:** Validate SPH fluid simulation  
**Agents:** aether-fluid-specialist, physics-mathematician

**Validation:**
- Kernel normalization: ∫W(r,h)dr = 1
- Density evaluation: ρ₀ = 1000 kg/m³ within 1%
- CFL condition: Δt < h / c_sound
- GPU performance: > 30 FPS at 1M particles

**Script:** `scripts/validate_sph.py` (TODO)

---

### 6. `validate_mcp_server.md`
**Purpose:** Validate JSON-RPC 2.0 MCP compliance  
**Agents:** glinda-mcp-orchestrator, synesthesia-holographic-ui

**Validation:**
- JSON-RPC 2.0 fields: jsonrpc, id, method, params
- Tool registration with schemas
- Read tool latency < 10 ms
- Write tool bounds validation
- Telemetry streaming (subscribe/listChanged)

**Script:** `scripts/validate_mcp.py` (TODO)

---

## Updated Documentation

### `.qwen/agents/README.md`
- Complete agent catalog with communication graph
- Agent selection guide by task type
- Version history

### `.qwen/skills/README.md`
- Complete skill catalog with categories
- Execution examples
- Validation criteria and thresholds

---

## Validation Scripts Created

| Script | Purpose | Status |
|--------|---------|--------|
| `scripts/validate_fresnel.py` | Fresnel equations validation | ✅ Complete |
| `scripts/validate_heterodyne.py` | Heterodyne kernel validation | ✅ Complete |
| `scripts/validate_iqumamba.py` | IQUMamba inference | TODO |
| `scripts/validate_tri_modal.py` | Tri-modal correlation | TODO |
| `scripts/validate_sph.py` | SPH parameters | TODO |
| `scripts/validate_mcp.py` | MCP server | TODO |

---

## Agent Trigger Patterns

### File-Based Triggers

| Agent | File Patterns |
|-------|---------------|
| dorothy-heterodyne-specialist | `domains/spectrum/dorothy/**/*.rs`, `**/heterodyne.wgsl` |
| tri-modal-defense-specialist | `domains/spectrum/shield/src/tri_modal/**/*.rs` |
| mmwave-fusion-specialist | `domains/spectrum/shield/src/hal/mmwave/**/*.rs` |
| crystal-ball-reconstruction | `domains/cognitive/crystal_ball/src/**/*.rs` |
| synesthesia-holographic-ui | `domains/interface/synesthesia/src/**/*.rs`, `**/hologram*.wgsl` |
| glinda-mcp-orchestrator | `domains/cognitive/glinda/src/**/*.rs`, `**/mcp/**/*.rs` |

### Content-Based Triggers

| Agent | Content Patterns |
|-------|------------------|
| dorothy-heterodyne-specialist | heterodyne, I/Q, complex_mix, Pluto, AD9363, PDM |
| tri-modal-defense-specialist | TriModalThreat, DefenseLayer, rf_bsdf, RIS, jamming |
| mmwave-fusion-specialist | mmWave, MR60BHA2, HLK-2410, FMCW, breathing, vibration |
| crystal-ball-reconstruction | RF_Vim, Mamba, bilinear, tustin, hermitian, Burn |
| synesthesia-holographic-ui | A2UI, slint_interpreter, hologram, MCP, zero_dom |
| glinda-mcp-orchestrator | mcp, JSON-RPC, tokio, mpsc, tool_registration |

---

## Integration with Existing Agents

The new agents complement existing agents:

| Existing Agent | New Agent | Collaboration |
|----------------|-----------|---------------|
| shield-rf-scientist | dorothy-heterodyne-specialist | RF signal processing |
| shield-rf-scientist | tri-modal-defense-specialist | RF-BSDF scattering |
| aether-fluid-specialist | crystal-ball-reconstruction | Neural material prediction |
| oz-render-architect | synesthesia-holographic-ui | WGPU texture sharing |
| train-state-space-ml | crystal-ball-reconstruction | IQUMamba training |

---

## Usage Examples

### Activate Agent for File Editing

```bash
# Edit heterodyne shader with Dorothy specialist
node .qwen/agents/activate.js dorothy-heterodyne-specialist
# Then edit assets/shaders/dorothy/heterodyne.wgsl

# Edit tri-modal defense with defense specialist
node .qwen/agents/activate.js tri-modal-defense-specialist
# Then edit domains/spectrum/shield/src/tri_modal/classifier/
```

### Run Skill Validation

```bash
# Validate Fresnel equations
python scripts/validate_fresnel.py \
  --input domains/physics/aether/src/shaders/rf_pbr.wgsl \
  --frequency 2400000000 \
  --material water

# Validate heterodyne kernel
python scripts/validate_heterodyne.py \
  --shader assets/shaders/dorothy/heterodyne.wgsl \
  --test_freq 1000000 \
  --shift 500000
```

---

## Next Steps

1. **Implement remaining validation scripts:**
   - `scripts/validate_iqumamba.py`
   - `scripts/validate_tri_modal.py`
   - `scripts/validate_sph.py`
   - `scripts/validate_mcp.py`

2. **Add agent activation CLI:**
   - `node .qwen/agents/list.js`
   - `node .qwen/agents/activate.js <agent>`

3. **Add skill runner CLI:**
   - `node .qwen/skills/run.js <skill> --input <path>`
   - `node .qwen/skills/run_all.js`

4. **Integrate with Qwen Code hooks:**
   - Auto-activate agents based on file patterns
   - Auto-run skills on file save

---

## Summary

**Created:**
- 6 new Qwen sub-agents (YAML format)
- 6 new Qwen skills (Markdown format)
- 2 validation scripts (Python)
- Updated README documentation

**Total Agents:** 14 (8 existing + 6 new)  
**Total Skills:** 16 (10 existing + 6 new)

All agents are grounded in actual codebase symbols and follow the existing agent format for consistency.
