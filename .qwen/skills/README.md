# Qwen Skills for Project Oz

Skills are executable validation tools that agents can use to verify their work before committing.

## Available Skills

| Skill | Purpose | Agents | Execution Time |
|-------|---------|--------|----------------|
| `check_rdna2_alignment` | 128-byte GPU struct alignment | oz, aether, resonance, dorothy | 30s |
| `validate_dsp_python` | DSP filter verification with SciPy | shield, siren, cipher | 30s |
| `validate_rf_bsdf_fresnel` | Exact Fresnel equations (NO Schlick) | shield-rf, tri-modal, crystal-ball | 30s |
| `verify_iqumamba_1d_inference` | RF-Vim inference pipeline validation | crystal-ball, train, tri-modal | 60s |
| `validate_heterodyne_kernel` | WGPU complex I/Q mixing | dorothy, shield-rf | 30s |
| `verify_tri_modal_correlation` | Multi-modal threat fusion | tri-modal, mmwave, shield-rf | 60s |
| `validate_sph_parameters` | SPH fluid simulation | aether, physics | 60s |
| `validate_mcp_server` | JSON-RPC 2.0 MCP compliance | glinda, synesthesia | 30s |
| `super_nyquist_reconstruction` | Aliased signal recovery techniques | dorothy, shield-rf | 30s |
| `run_hitl_sandbox` | Human-in-the-loop validation | synesthesia, siren | Manual |
| `validate_dsp_python` | DSP filter verification | shield, siren, cipher | 30s |
| `coprime_sampling_moire_reconstruction` | Coprime sampling for periodic signals | dorothy, shield-rf | 30s |
| `multiple_multirate_samplers` | Multi-rate signal reconstruction | dorothy, shield-rf | 30s |
| `sparse_compressive_methods` | Compressed sensing recovery | crystal-ball, train | 30s |
| `structured_beyond_nyquist_recovery` | Known frequency reconstruction | dorothy, shield-rf | 30s |
| `repeated_aliased_events` | Repeated measurement reconstruction | dorothy, shield-rf | 30s |
| `integrate_mmwave_sensor` | XIAO MR60BHA2 HAL integration | mmwave-fusion, tri-modal, shield-rf | 60s |

## Usage

```bash
# Run single skill
node .qwen/skills/run.js <skill_name> --input <INPUT_PATH>

# Example: Check RDNA2 alignment
node .qwen/skills/run.js check_rdna2_alignment --input domains/physics/aether/src/gpu_data.rs

# Example: Validate Fresnel equations
python scripts/validate_fresnel.py --input domains/physics/aether/src/shaders/rf_pbr.wgsl --frequency 2400000000

# Example: Validate IQUMamba inference
python scripts/validate_iqumamba.py --model models/rf_vim.onnx --input test_data/iq_samples.npy

# Run all skills for current agent
node .qwen/skills/run_all.js

# Run skill with custom parameters
python scripts/validate_heterodyne.py --shader assets/shaders/dorothy/heterodyne.wgsl --test_freq 1000000 --shift 500000
```

## Skill Categories

### GPU/Compute Skills

| Skill | Validates | Target |
|-------|-----------|--------|
| `check_rdna2_alignment` | 128-byte struct alignment | GPU buffers, uniforms |
| `validate_heterodyne_kernel` | Complex I/Q mixing | Dorothy heterodyne |
| `validate_sph_parameters` | SPH kernel functions | Aether fluid simulation |

### RF/Signal Processing Skills

| Skill | Validates | Target |
|-------|-----------|--------|
| `validate_rf_bsdf_fresnel` | Fresnel equations | RF-BSDF scattering |
| `validate_heterodyne_kernel` | Frequency shifting | SDR heterodyne |
| `super_nyquist_reconstruction` | Aliased signal recovery | Super-Nyquist mode |
| `coprime_sampling_moire_reconstruction` | Coprime sampling | Periodic signals |
| `multiple_multirate_samplers` | Multi-rate reconstruction | Heterogeneous SDRs |
| `sparse_compressive_methods` | Compressed sensing | Sparse signals |
| `structured_beyond_nyquist_recovery` | Known frequency recovery | Structured signals |
| `repeated_aliased_events` | Repeated measurement | Aliased events |

### ML/AI Skills

| Skill | Validates | Target |
|-------|-----------|--------|
| `verify_iqumamba_1d_inference` | RF-Vim inference | Crystal Ball reconstruction |
| `validate_dsp_python` | DSP filters | Shield/siren processing |

### System Integration Skills

| Skill | Validates | Target |
|-------|-----------|--------|
| `verify_tri_modal_correlation` | Multi-modal fusion | Tri-modal defense |
| `validate_mcp_server` | JSON-RPC 2.0 | Glinda MCP server |
| `run_hitl_sandbox` | Human validation | UI/haptic systems |

## Skill Output Format

All skills produce JSON output with consistent structure:

```json
{
  "skill_name": "skill_identifier",
  "input": "path/to/input",
  "tests": [
    {
      "name": "test_name",
      "expected": "expected_value",
      "computed": "computed_value",
      "error": 0.01,
      "status": "PASS|FAIL"
    }
  ],
  "summary": {
    "total": 10,
    "passed": 9,
    "failed": 1,
    "max_error": 0.05
  }
}
```

## Integration with Agents

Skills are automatically invoked by agents based on file patterns:

| Agent | Auto-Invoked Skills |
|-------|---------------------|
| `oz-render-architect` | `check_rdna2_alignment` |
| `aether-fluid-specialist` | `check_rdna2_alignment`, `validate_sph_parameters` |
| `dorothy-heterodyne-specialist` | `validate_heterodyne_kernel`, `super_nyquist_reconstruction` |
| `shield-rf-scientist` | `validate_rf_bsdf_fresnel`, `validate_dsp_python` |
| `crystal-ball-reconstruction` | `verify_iqumamba_1d_inference`, `validate_rf_bsdf_fresnel` |
| `tri-modal-defense-specialist` | `verify_tri_modal_correlation`, `validate_rf_bsdf_fresnel` |
| `glinda-mcp-orchestrator` | `validate_mcp_server` |
| `synesthesia-holographic-ui` | `validate_mcp_server`, `run_hitl_sandbox` |

## Validation Criteria

### Pass Thresholds

| Skill | Pass Criteria |
|-------|---------------|
| `check_rdna2_alignment` | All GPU structs divisible by 128 bytes |
| `validate_rf_bsdf_fresnel` | < 0.1% error vs analytical solution |
| `verify_iqumamba_1d_inference` | ≥ 10 dB SI-SDR improvement, < 1 ms latency |
| `validate_heterodyne_kernel` | < 0.01 Hz frequency error |
| `verify_tri_modal_correlation` | ≥ 0.8 correlation confidence |
| `validate_sph_parameters` | Density error < 1%, > 30 FPS at 1M particles |
| `validate_mcp_server` | < 10 ms response time, JSON-RPC 2.0 compliant |

### Fail Conditions

Skills fail when:
- Error exceeds threshold
- Required patterns not found
- Forbidden patterns detected
- Performance targets not met
- Schema validation fails

## Adding New Skills

1. Create skill file in `.qwen/skills/` with `.md` or `.skill` extension
2. Define execution command and validation criteria
3. Specify applicable agents
4. Document output format and thresholds
5. Update this README

## Skill Development Guidelines

1. **Deterministic**: Same input → same output
2. **Fast**: < 60 seconds execution time
3. **Clear errors**: Explain what failed and why
4. **Actionable**: Suggest fixes when possible
5. **JSON output**: Machine-readable results

## Related Files

- `.qwen/agents/README.md` - Agent documentation
- `scripts/` - Skill implementation scripts
- `conductor/tracks/` - Implementation plans with validation requirements

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-02-21 | Initial skills: check_rdna2_alignment, validate_dsp_python, super_nyquist_reconstruction |
| 1.1.0 | 2026-02-22 | Added: validate_rf_bsdf_fresnel, verify_iqumamba_1d_inference, validate_heterodyne_kernel, verify_tri_modal_correlation, validate_sph_parameters, validate_mcp_server |
