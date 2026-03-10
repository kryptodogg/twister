# Skill: check_rdna2_alignment

## Overview

Executes a custom Rust AST parser that explicitly scans `#[repr(C)]` structs to ensure they perfectly divide by 128 bytes for RDNA2/3 infinity cache optimization.

## Applicable Agents

- `oz-render-architect`
- `aether-fluid-specialist`
- `resonance-kinematics`

## Execution

```bash
# Run alignment check on directory
cargo run --manifest-path scripts/Cargo.toml --bin static_align_check -- --input <INPUT_DIR> --output <OUTPUT_JSON>

# Example
cargo run --manifest-path scripts/Cargo.toml --bin static_align_check -- --input crates/oz/src/ --output crates/oz/alignment_report.json
```

## Validation Criteria

### Pass Conditions
- All `#[repr(C)]` structs have sizes divisible by 128 bytes
- GPU-visible buffers are explicitly aligned with `#[align(128)]`
- No false positives for CPU-only structures

### Fail Conditions
- Any GPU-visible struct size not divisible by 128
- Missing `#[align(128)]` on storage buffer structs
- Struct padding insufficient for cache line alignment

## Detection Patterns

The AST parser detects GPU-visible structures by:
- Naming patterns: `Gpu*`, `*Buffer`, `*Uniform`, `*Vertex`
- Attribute patterns: `#[repr(C)]`, `#[repr(align(128))]`
- Usage patterns: Passed to WGPU `write_buffer()`, `set_bind_group()`

## Output Format

```json
{
  "file": "crates/oz/src/gpu_data.rs",
  "structs": [
    {
      "name": "GpuParticle",
      "size": 128,
      "alignment": 128,
      "status": "PASS",
      "gpu_visible": true
    },
    {
      "name": "AetherUniforms",
      "size": 320,
      "alignment": 16,
      "status": "FAIL",
      "gpu_visible": true,
      "suggestion": "Add 16 bytes padding to reach 336 bytes (128 * 3)"
    }
  ],
  "summary": {
    "total": 10,
    "passed": 9,
    "failed": 1
  }
}
```

## Timeout

Maximum execution time: 30 seconds

## Integration

This skill is called automatically by validation hooks after editing:
- `crates/oz/src/**/*.rs`
- `crates/aether/src/**/*.rs`
- `crates/resonance/src/**/*.rs`
- Any file containing `#[repr(C)]` structs

## Related Files

- `scripts/static_align_check.rs` - Main AST parser
- `scripts/Cargo.toml` - Rust script dependencies (syn, quote)

## RDNA2/3 Alignment Rules

1. **Storage Buffers**: Must be 128-byte aligned
2. **Uniform Buffers**: Must be 256-byte aligned (wgpu requirement)
3. **Vertex Buffers**: Must be 128-byte aligned for cache efficiency
4. **Index Buffers**: Must be 128-byte aligned

### Why 128 Bytes?

AMD RDNA2/3 architecture uses 128-byte cache lines in the Infinity Cache. Misaligned structures cause:
- Cache line splits (2x memory bandwidth)
- Reduced occupancy
- Lower throughput in compute shaders
