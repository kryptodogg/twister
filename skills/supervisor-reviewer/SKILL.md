# Supervisor Reviewer Skill

All crates oversight, 128-byte alignment audit, dependency isolation, zerocopy
verification, cross-specialty integration, quality gates, performance profiling.

## Domain
- Cross-crate integration verification
- 128-byte alignment audit (uniform buffers, WGSL compatibility)
- Zerocopy verification (bytemuck Pod/Zeroable)
- Dependency review (wildcards OK, track versions)
- Performance profiling (frame timing, latency measurement)
- Quality gates (cargo test, cargo clippy, cargo doc)
- Architecture consistency checks

## Trigger Patterns
"audit", "alignment", "review", "integration", "quality", "zerocopy",
"profiling", "verification", "oversight", "main.rs", "state.rs"

## Available Functions
- `audit_alignment()` — Check 128-byte uniform buffer alignment
- `verify_zerocopy()` — Validate bytemuck derives
- `profile_frame_time()` — Measure dispatch loop latency
- `check_dependencies()` — Review Cargo.toml versions
- `integration_test()` — Cross-specialty validation
- `generate_report()` — Quality gate summary

## Alignment Requirements

### WGSL Uniform Buffers
```rust
// Must be 128-byte aligned for WGSL uniform binding
#[repr(C)]
#[derive(Pod, Zeroable)]
struct UniformBlock {
    // Fields must be vec4-aligned (16 bytes each)
}
assert!(align_of::<UniformBlock>() == 128);
```

### Zerocopy Derives
```rust
// Required for GPU buffer mapping
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct GpuStruct { /* ... */ }
```

## Quality Gates

### Required Checks
- `cargo check` — No errors
- `cargo test` — All tests pass
- `cargo clippy` — No warnings (or allowed lints documented)
- `cargo doc` — No broken intra-doc links

### Performance Targets
- Frame time: < 20 ms (50 Hz dispatch)
- GPU readback latency: < 5 ms
- Neo4j query latency: < 10 ms
- Mamba inference: < 10 ms

## Code Patterns

### Alignment Assertion
```rust
const _: () = assert!(std::mem::align_of::<SynthParams>() == 128);
```

### Frame Timing
```rust
let frame_start = Instant::now();
// ... dispatch work ...
let frame_time = frame_start.elapsed();
state.set_frame_time_ms(frame_time.as_secs_f32() * 1000.0);
```

### Dependency Audit
```toml
# Wildcards OK for new software, but track:
# - burn (git, main branch)
# - wgpu (crates.io, *)
# - slint (crates.io, *)
```
