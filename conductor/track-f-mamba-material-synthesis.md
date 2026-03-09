# Track F: Mamba Material Synthesis (RF-BSDF Lexicon)

**Domain**: ML → GPU Rendering Bridge
**Ownership**: Graphics ML Engineer (collaborates with Mamba trainer + GPU specialist)
**Duration**: 3-4 days
**Blocker on**: C.2 (Mamba training complete), VI.1 (mesh shader foundation)

---

## Overview

Track F translates Mamba-learned 64-D anomaly embeddings into RF-BSDF material properties in real-time. Your Material Lexicon defines the semantic mapping:

- **Base Lead**: Mundane signals (WiFi, Bluetooth) → Roughness 1.0, Metallic 0.0 (matte, recedes)
- **Polished Obsidian**: Encrypted signals (AES-256, drone telemetry) → Metallic 1.0, Roughness 0.0 (impenetrable mirror)
- **Philosopher's Gold**: Locked/active threats (heterodyne attacks) → Emission 1.0+, god rays (blinding volumetric light)

This bridges what Mamba *knows* (anomaly embeddings) to what users *see* (material appearance).

---

## Track F.1: Embedding → Material Translation (1.5d)

**Deliverables**:
- `src/ml/mamba_material_translator.rs` (250 lines)
- `src/ml/material_lexicon.rs` (200 lines)

**Key work**:
- Input: 64-D Mamba latent + anomaly_score (0-10 dB range)
- Classify: Low (0-1) → Lead, Medium (1-3) → Obsidian, High (3-10) → Gold
- Output: MambaMaterial {hardness, roughness, wetness, emission_intensity, albedo_hue, confidence}

---

## Track F.2: Real-Time Material Stream (1d)

**Deliverables**:
- `src/visualization/material_updater.rs` (200 lines)
- `src/visualization/physics_push_constants.rs` (150 lines)

**Key work**:
- Receive 64-D embeddings from Mamba thread (crossbeam channel)
- Pack into 256-byte PhysicsPushConstants (your exact byte layout)
- Vulkan push constants zero-latency upload
- 100 Hz update rate (matches dispatch loop)

---

## Track F.3: Material Lifecycle (1d)

**Deliverables**:
- `src/visualization/material_lifecycle.rs` (200 lines)

**Key work**:
- Birth: New anomaly → spawn material with initial confidence
- Tracking: Mamba refines → confidence increases → material sharpens
- Decay: No updates → confidence drops → fade to Lead
- Death: 30s timeout → recycle particle

---

## Track F.4: Anomaly→Emission Mapper (1d)

**Deliverables**:
- `src/visualization/anomaly_emission_mapper.rs` (150 lines)

**Key work**:
- 0-0.5 dB → no emission (Lead)
- 0.5-3.0 dB → obsidian reflection (no emission)
- 3.0-10.0 dB → gold emission (1.0-2.0x multiplier)
- >10.0 dB → extreme emission + volumetric god rays

Emission intensity is visual proxy for Mamba's confidence in threat.

---

## Integration Points

**Input from**: Mamba (C.2)
**Output to**: GPU push constants, Track VI (all modules), Cognee graph (E)
**Files owned**: `src/ml/mamba_material_*`, `src/visualization/material_*`, `src/visualization/anomaly_emission_*`

---

## Success Criteria

✅ Embedding → material translation < 1ms
✅ 256-byte push constants byte-aligned per your research
✅ 100 Hz update rate (10ms cycle)
✅ Material lifecycle smooth transitions
✅ All tests passing, 0 warnings
✅ Integration with VI.1-VI.5 verified

**Last Updated**: 2026-03-08
**Status**: Ready for assignment
