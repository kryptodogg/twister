# Qwen Agent System for Project Oz

This directory contains specialized agent configurations for Qwen Code to use when working on Project Oz.

## Agent Architecture

Agents are context-specific configurations that:
1. **Restrict file access** to relevant crates/domains
2. **Inject domain knowledge** (rules, patterns, constraints)
3. **Enable specialized skills** for validation and testing
4. **Define triggers** for automatic activation

## Available Agents

| Agent | Crate/Domain | Primary Focus |
|-------|--------------|---------------|
| `oz-render-architect` | oz/ | Hybrid clustered forward rendering, mesh shaders |
| `aether-particle-specialist` | aether/ | Particle swarm management, GPU radix sort, SoA layouts |
| `resonance-physics-mathematician` | resonance/ | SPH solvers, SDF collision math, exact Fresnel |
| `shield-security-gate` | shield/ | Physical trust boundary, YubiKey auth |
| `dorothy-sdr-specialist` | dorothy/ | Pluto+ SDR, WGPU heterodyne, Super-Nyquist folding |
| `tri-modal-defense-specialist` | shield/tri_modal/ | RF/acoustic/optical fusion, 4-layer defense |
| `mmwave-fusion-specialist` | shield/hal/mmwave/ | 60GHz FMCW radar integration |
| `crystal-ball-persistence` | crystal_ball/ | HDF5 forensic recording and replay |
| `synesthesia-holographic-ui` | synesthesia/ | Zero-DOM UI, A2UI→Slint translation |
| `trinity-mcp-orchestrator` | trinity/ | MCP server, SQLite state, workspace governance |
| `brain-intelligence-hub` | brain/ | Burn 0.21 IQUMamba-1D, Forensic RAG |
| `siren-extreme-dsp` | siren/ | 192kHz PCM, 600Hz haptic encoding |

## Agent Communication Graph

```
                    ┌─────────────────────┐
                    │  glinda-orchestrator │
                    │   (MCP Server)       │
                    └──────────┬──────────┘
                               │
         ┌─────────────────────┼─────────────────────┐
         │                     │                     │
         ▼                     ▼                     ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  oz-render      │  │  aether-fluid   │  │  synesthesia    │
│  architect      │  │  specialist     │  │  holographic-ui │
└────────┬────────┘  └────────┬────────┘  └────────┬────────┘
         │                    │                     │
         │              ┌─────┴─────┐               │
         │              │           │               │
         ▼              ▼           ▼               ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  resonance      │  │  shield-rf      │  │  crystal-ball   │
│  kinematics     │  │  scientist      │  │  reconstruction │
└─────────────────┘  └────────┬────────┘  └─────────────────┘
                              │
                    ┌─────────┴─────────┐
                    │                   │
                    ▼                   ▼
         ┌─────────────────┐  ┌─────────────────┐
         │  dorothy        │  │  tri-modal      │
         │  heterodyne     │  │  defense        │
         │  specialist     │  │  specialist     │
         └─────────────────┘  └────────┬────────┘
                                      │
                                      ▼
                             ┌─────────────────┐
                             │  mmwave         │
                             │  fusion         │
                             │  specialist     │
                             └─────────────────┘
```

## Activation

Agents are activated automatically based on:
- **File patterns** - Editing files in specific crates
- **Content patterns** - Mentioning domain-specific terms
- **Explicit request** - User asks for specific agent

## Skills

Skills are executable validation tools:

| Skill | Purpose | Agents |
|-------|---------|--------|
| `check_rdna2_alignment` | 128-byte GPU struct alignment | oz, aether, resonance, dorothy |
| `validate_dsp_python` | DSP filter verification with SciPy | shield, siren, cipher |
| `validate_rf_bsdf_fresnel` | Exact Fresnel equations validation | shield-rf, tri-modal, crystal-ball |
| `verify_iqumamba_1d_inference` | RF-Vim inference pipeline | crystal-ball, train, tri-modal |
| `validate_heterodyne_kernel` | WGPU complex I/Q mixing | dorothy, shield-rf |
| `verify_tri_modal_correlation` | Multi-modal threat fusion | tri-modal, mmwave, shield-rf |
| `validate_sph_parameters` | SPH fluid simulation | aether, physics-mathematician |
| `validate_mcp_server` | JSON-RPC 2.0 MCP compliance | glinda, synesthesia |
| `super_nyquist_reconstruction` | Aliased signal recovery | dorothy, shield-rf |
| `run_hitl_sandbox` | Human-in-the-loop validation | synesthesia, siren |

## Usage

```bash
# List available agents
node .qwen/agents/list.js

# Activate specific agent
node .qwen/agents/activate.js oz-render-architect

# Run skill validation
node .qwen/skills/run.js check_rdna2_alignment --input oz/src/gpu_data.rs

# Run all skills for current agent
node .qwen/skills/run_all.js
```

## File Format

Agents are defined in YAML format:

```yaml
name: "Agent Name"
description: "What this agent does"
restricted_paths:
  - "crate/**/*"
  - "docs/**/*"
forbidden_paths:
  - "other-crate/**/*"
rules:
  - id: "rule_id"
    description: "Rule description"
    severity: "error|warning|info"
    pattern: "regex pattern"
reference_bundles:
  - path: "docs/reference.md"
    access: "read-only"
triggers:
  file_patterns: ["crate/src/**/*.rs"]
  content_patterns: ["keyword1", "keyword2"]
skills: ["skill1", "skill2"]
```

## Agent Selection Guide

### When editing GPU/WGSL code:
- **Particle systems** → `aether-fluid-specialist` + `rdna2-compute-specialist`
- **Render pipeline** → `oz-render-architect`
- **Heterodyne/mixing** → `dorothy-heterodyne-specialist`
- **Hologram UI** → `synesthesia-holographic-ui`

### When editing RF/signal processing:
- **Fresnel/BSDF** → `shield-rf-scientist`
- **Heterodyne/SDR** → `dorothy-heterodyne-specialist`
- **Neural reconstruction** → `crystal-ball-reconstruction`
- **Tri-modal fusion** → `tri-modal-defense-specialist`

### When editing ML/training:
- **IQUMamba/RF-Vim** → `crystal-ball-reconstruction` + `train-state-space-ml`
- **Burn WGPU backend** → `train-state-space-ml`

### When editing audio/haptics:
- **192kHz audio** → `toto-hardware-hal` + `siren-extreme-dsp`
- **Haptic encoding** → `resonance-kinematics` + `siren-extreme-dsp`

### When editing orchestration:
- **MCP server** → `glinda-mcp-orchestrator`
- **Cross-crate flows** → `glinda-mcp-orchestrator`

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-02-21 | Initial agent system |
| 1.1.0 | 2026-02-22 | Added dorothy, tri-modal, mmwave, crystal-ball, synesthesia, glinda agents |
| 1.1.0 | 2026-02-22 | Added RF-BSDF, IQUMamba, heterodyne, tri-modal, SPH, MCP skills |
