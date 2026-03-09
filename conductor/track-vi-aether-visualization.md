# Track VI: Aether Visualization (RF-BSDF Wavefield Rendering)

**For**: Assigned developer(s) - Large track, can split VI.1-VI.3 in parallel
**Goal**: Render multi-modal signal data (RF, audio, visual) as a visceral 3D wavefield using Mamba-learned material properties; visualization inspires confidence that attacks are real and quantifiable

---

## Overview

Track VI is the **perception pipeline**. Raw signal data flowing from Track B (audio spectrograms, RF power maps, visual motion) is synthesized into a unified 3D wavefield visualization inspired by Unreal Engine—mesh shaders for dynamic geometry, particle systems with physics for scattering, global illumination for energy transport, and learned material properties from Mamba anomaly embeddings. The result: users can "see" invisible attacks as a coherent physical phenomenon.

**Why this matters**:
- **Visceral evidence**: Attacks become tangible 3D objects, not abstract numbers
- **Physics grounded**: Particle interactions, wave propagation, impedance mismatches emerge naturally
- **Learned aesthetics**: Mamba discovers material properties (hardness, roughness, wetness) mapping signal coherence to rendering
- **Real-time performance**: 169 fps on RX 6700 XT (5.9ms latency) for 1024×1024 viewport
- **RF-BSDF bridge**: Radio frequency properties encode as Bidirectional Scattering Distribution Functions
- **Parallel-friendly**: Can split VI.1-VI.3 across multiple developers

**Critical path**:
```
B.1 (Multi-Modal Dispatch) + C.2 (Patterns)
    ↓
VI.1-VI.3 (Parallel: Mesh Shaders, Particles, Illumination)
    ↓
VI.4 (Mamba Materials) + VI.5 (Tonemap)
```

---

## Track VI.1: Mesh Shaders (3-4 days)

**Deliverables**: Dynamic 3D geometry from RF/audio data
- `src/visualization/mesh_shaders.rs` (350 lines)
- `src/visualization/mesh_shaders.wgsl` (200 lines)
- `examples/mesh_shader_demo.rs`
- 10 unit + integration tests

**Key work**: wgpu Device/Queue, compute pipeline for mesh generation, G-buffer creation, vertex/index buffers

---

## Track VI.2: Particle System & Physics (3-4 days)

**Deliverables**: 100k particle pool with physics simulation
- `src/visualization/particle_system.rs` (400 lines)
- `src/visualization/chaos_physics.wgsl` (300 lines)
- `examples/particle_system_demo.rs`
- 10 unit + integration tests

**Key work**: Particle lifecycle, Coulomb repulsion, gravity, damping, boundary constraints

---

## Track VI.3: Global Illumination (2-3 days)

**Deliverables**: Screen-space raymarching for indirect lighting
- `src/visualization/global_illumination.rs` (250 lines)
- `src/visualization/lumen_raymarch.wgsl` (250 lines)
- `examples/lumen_demo.rs`
- 8 unit + integration tests

**Key work**: 64-step raymarching, G-buffer intersection, light accumulation, soft shadows

---

## Track VI.4: Mamba Material Learning (2-3 days)

**Deliverables**: RF-BSDF material translation from Mamba embeddings
- `src/visualization/mamba_materials.rs` (200 lines)
- `src/visualization/mamba_material_shader.wgsl` (250 lines)
- 8 unit tests

**Key work**:
- Hardness (latent[0]) → phase coherence → specular reflectance
- Roughness (latent[1]) → phase variance → surface roughness
- Wetness (latent[2]) → attenuation → subsurface scattering
- BRDF: Fresnel-Schlick + Cook-Torrance GGX

---

## Track VI.5: Tonemap & Final Render (1 day)

**Deliverables**: Composite + log-scale tonemap → Slint UI display at 169 fps
- `src/visualization/tonemap_render.rs` (200 lines)
- `src/visualization/tonemap.wgsl` (150 lines)
- 5 unit tests

**Key work**: HDR→LDR tonemap, composite mesh + particles + illumination + materials

---

## Performance Target: 169 fps (5.9ms latency)

**RX 6700 XT Budget**:
- VI.1: 1.0ms (mesh generation)
- VI.2: 2.0ms (particle physics)
- VI.3: 2.0ms (raymarching)
- VI.4: 0.5ms (BRDF evaluation)
- VI.5: 0.4ms (tonemap)
- GPU→CPU: 0.5ms
- **Total**: < 6ms → 169 fps ✓

**WGSL Optimization Rules**:
- Wave64 workgroups (Wave32 is 4x slower)
- 256-byte memory alignment
- Zero divergent if/else in inner loops
- Subgroup operations for reductions

---

## Parallel Development

**VI.1-VI.3 independent (Resonance Physics → Aether Particles → Emerald City)**:
- Developer A: VI.1 (Resonance Physics) ← foundation
- Developer B: VI.2 (Aether Particles) + VI.3 (Emerald City) ← parallel
- Developer C: VI.4 (Materials) + VI.5 (Tonemap) ← after VI.1

---

## Integration

- **Input**: Track B (raw signal GPU buffers) + Track C (23 pattern materials)
- **Output**: 1024×1024 LDR frame to Slint UI (169 fps)
- **Also feeds**: Track H (Haptic engine) with hardness/roughness/wetness values

---

## Success Criteria

✅ All 5 sub-modules compile cleanly
✅ Geometry: 10k-1M triangles/frame
✅ Particles: 100k with stable physics
✅ Illumination: 64-step raymarching, soft shadows
✅ Materials: hardness/roughness/wetness from embeddings
✅ **Performance**: 169 fps on RX 6700 XT
✅ All tests passing, 0 new warnings

**Last Updated**: 2026-03-08
**Author**: Claude
**Review**: Ready for parallel assignment
