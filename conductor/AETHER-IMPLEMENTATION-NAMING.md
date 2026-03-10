# Aether Implementation Naming Guide

**Principle**: Specs use industry-standard reference names (Lumen, Niagara, Megalights) for architectural clarity. **Actual codebase uses "Aether" terminology.**

---

## Reference → Implementation Mapping

| Reference (Spec) | Implementation (Code) | Purpose |
|------------------|----------------------|---------|
| Lumen GI | `src/visualization/aether_indirect_lighting.rs` | Probe-based global illumination |
| Niagara Particles | `src/visualization/aether_particle_emitter.rs` | Particle emission/dynamics |
| Megalights Rendering | `src/visualization/aether_ray_tracer.rs` | Ray-traced lighting |
| Volumetric Lighting | `src/visualization/aether_volumetric_effects.rs` | God rays, heterodyne scattering |
| Gaussian Splatting | `src/visualization/aether_splatting.rs` | Tone mapping, color |

**Key rule**: No third-party names in the codebase. "Aether" is the unified brand.

---

## Struct & Function Naming

### ✅ DO

```rust
pub struct AetherIndirectLighting { ... }
pub struct AetherParticleEmitter { ... }
pub struct AetherRayTracer { ... }
pub fn render_aether_frame() { ... }
pub fn update_aether_probes() { ... }
pub fn emit_aether_particles() { ... }
```

### ❌ DON'T

```rust
pub struct LumenGI { ... }              // ← No third-party names
pub struct NiagaraEmitter { ... }       // ← No Unreal terminology
pub struct MegalightsRenderer { ... }   // ← No made-up brand names
pub fn render_with_lumen() { ... }      // ← Aether, not Lumen
pub fn niagara_emit() { ... }           // ← Aether, not Niagara
```

---

## Module Organization

```rust
// src/visualization/mod.rs
pub mod aether_physics;           // VI.1 (Chaos)
pub mod aether_particles;         // VI.2 (Niagara → Aether Particles)
pub mod aether_rendering;         // VI.3 (Megalights → Aether Rendering)
    pub mod aether_ray_tracer;
    pub mod aether_indirect_lighting;
    pub mod aether_volumetric_effects;
```

---

## Variable & Constant Naming

```rust
// ✅ Aether-branded
const AETHER_PROBE_GRID_SIZE: usize = 8;
const AETHER_PARTICLE_MAX_LIFETIME: f32 = 30.0;
const AETHER_RAY_MARCH_STEPS: u32 = 64;

let aether_field = solve_rf_field(...);
let aether_particles = emit_particles(...);
let aether_framebuffer = render_aether(...);

// ❌ Third-party names
const LUMEN_PROBE_SIZE: usize = 8;
const NIAGARA_LIFETIME: f32 = 30.0;
const MEGALIGHTS_STEPS: u32 = 64;
```

---

## Comments & Documentation

```rust
/// Aether indirect lighting via probe grid
pub struct AetherIndirectLighting { ... }

/// Emit Aether particles from RF energy field
pub fn emit_particles(...) { ... }

/// Render Aether visualization with ray tracing
pub fn render_aether(...) { ... }
```

---

## Cargo Feature Flags

```toml
[features]
aether = ["wgpu/ray-tracing", "wgpu/spirv"]  # NOT "lumen" or "niagara"
aether-debug = ["aether", "debug-asserts"]
aether-experimental = ["aether", "experimental-features"]
```

---

## Log Messages & Diagnostics

```rust
eprintln!("[Aether] Probe grid initialized");
eprintln!("[Aether] Ray tracer compiled");
eprintln!("[Aether] {} particles emitted", count);
eprintln!("[Aether] Volumetric effects rendered");

// NOT:
eprintln!("[Lumen] ...");
eprintln!("[Niagara] ...");
eprintln!("[Megalights] ...");
```

---

## Test Naming

```rust
#[cfg(test)]
mod aether_tests {
    #[test]
    fn test_aether_ray_tracing() { ... }

    #[test]
    fn test_aether_probe_grid() { ... }

    #[test]
    fn test_aether_particle_emission() { ... }
}

// NOT:
#[test]
fn test_lumen_gi() { ... }
```

---

## Critical: Build System

In `Cargo.toml`, ensure:
```toml
[package]
name = "twister"
version = "0.5.0"
# NOT "siren-aether" or "aether-lumen"

[features]
aether = ["wgpu"]  # Aether-branded feature flag
```

---

## Enforcement

**Pre-commit hook** (`.git/hooks/pre-commit`):

```bash
#!/bin/bash
# Check: No third-party names in Rust code
if grep -r "Lumen\|Niagara\|Megalights" src/ --include="*.rs" 2>/dev/null; then
    echo "❌ ERROR: Found third-party names in codebase"
    echo "   Use 'Aether' terminology instead"
    exit 1
fi

if grep -r "pub struct Lumen\|pub fn lumen_\|pub struct Niagara" src/ --include="*.rs" 2>/dev/null; then
    echo "❌ ERROR: Third-party struct/function names found"
    exit 1
fi

echo "✅ Aether naming convention verified"
exit 0
```

---

## Summary

| Layer | Reference | Aether Implementation |
|-------|-----------|----------------------|
| **Physics** (VI.1) | Chaos | `aether_physics` |
| **Particles** (VI.2) | Niagara | `aether_particles` |
| **Ray Tracing** (VI.3) | Megalights | `aether_rendering` |
| **Indirect Light** (VI.3) | Lumen | `aether_indirect_lighting` |
| **Volumetric** (VI.3) | Volumetric Effects | `aether_volumetric_effects` |

**Rule**: Every struct, function, module, and constant should feel like it belongs to **Aether**—the unified, original vision for RF-matter visualization.
