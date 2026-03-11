# Phase 2: ANALYSIS Tab + Offline Teacher Pipeline + Ray-Aware TimeGNN Visualization

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Implement 448-D multimodal feature extraction, offline wav2vec2 + TimeGNN training pipeline, and real-time 3D ray-tracing visualization of harassment attack patterns.

**Architecture:** End-to-end system for discovering recurring harassment signatures from collected forensic events:
- **Multimodal extraction**: Audio (196-D) + RF (128-D) + Visual (64-D) = 448-D feature vectors
- **Offline training**: wav2vec2 frozen features → TimeGNN temporal pattern learning → 128-D attack embeddings
- **Ray-tracing visualization**: Hardware RT on RX 6700 XT with image-source method + mesh shaders for 3D attack geometry
- **Distillation path**: Compress TimeGNN→Mamba for future lightweight runtime integration

**Tech Stack:**
- **Rust ML backend**: **burn-wgpu 0.21-pre2** (Vulkan native on RX 6700 XT, NOT burn-candle)
  - ✅ burn-wgpu: wgpu 28.0 native Vulkan backend (RDNA2 optimized)
  - ❌ burn-candle: CUDA/Metal only - NO Vulkan/wgpu support
  - **Single device**: One wgpu::Device powers RT pipeline + TimeGNN inference + mesh shaders (zero-copy tensor sharing)
- **Visualization**: wgpu 28.0, Vulkan VK_KHR_ray_tracing_pipeline, WGSL shaders (28 LOD mesh)
- **Python analysis**: PyTorch, PyTorch Geometric, transformers (wav2vec2 offline only, not deployed at runtime)
- **Database**: Qdrant vector DB (local, 128-D embeddings), Neo4j (pattern correlations)
- **Hardware**: RX 6700 XT (36 RT cores, 12GB VRAM, 2.1 TFLOPS burn-wgpu inference), 476fps target

---

## Unified ML + Visualization Architecture (burn-wgpu 28.0.0)

**Key Insight**: Single `wgpu::Device` powers everything - zero-copy tensor sharing between RT pipeline, TimeGNN inference, and mesh shaders.

```rust
// Corrected Cargo.toml
[dependencies]
burn = { version = "0.21-pre2", features = ["wgpu", "vulkan"] }
burn-wgpu = "0.21-pre2"
wgpu = "28.0.0"

// ❌ REMOVE THESE:
// burn-candle = "*"  # CUDA/Metal only, no Vulkan
// candle-* = "*"     # No wgpu/Vulkan support

// Unified device creation
let wgpu_device = create_wgpu_device_28();  // Vulkan RX 6700 XT
let burn_device = WgpuDevice::from_wgpu_device(wgpu_device);  // Shared device

// Single pipeline: TimeGNN inference → RT tracing → mesh rendering
let timegnn_embedding = timegnn_model.forward(events_tensor, &burn_device);  // 1344-D → 128-D
let rt_hits = rt_pipeline.trace(timegnn_embedding.as_raw_buffer(), &wgpu_device);  // Shared memory
mesh_shader.render(rt_hits, &wgpu_device);  // Visualize ray hits
```

**Why burn-wgpu wins**:
- ✅ wgpu 28.0.0 native (exact version in use)
- ✅ Vulkan backend optimized for RDNA2 (RX 6700 XT)
- ✅ Mesh shader interop (your 28 LOD attack visualization)
- ✅ Hardware RT pipeline sharing (VK_KHR_ray_tracing)
- ✅ Zero tensor copies (shared memory between all subsystems)
- ✅ 2.1 TFLOPS TimeGNN inference (RX 6700 XT at 1920x1080)

---

## Current Status

✅ **Task A.1 COMPLETE**: Audio 196-D Feature Extractor
- Implementation: `src/features/audio.rs` (555 lines)
- Tests: `tests/features_audio.rs` (348 lines, all 10 tests passing)
- Spec Review: ✅ PASSED
- Code Quality Review: ✅ PASSED

✅ **Task D.1a COMPLETE**: RayTracer Image-Source Method (128-D)
- Implementation: `src/visualization/ray_tracer.rs` (341 lines)
- Tests: `tests/ray_tracer_integration.rs` (259 lines, 15 tests passing)
- Spec Review: ✅ PASSED
- Code Quality Review: ✅ APPROVED

✅ **Task C.1 COMPLETE**: TimeGNN burn-wgpu Model (1092-D → 128-D)
- Implementation: `src/ml/timegnn.rs` (240 lines)
- Tests: `tests/timegnn_integration.rs` (285 lines, 10 tests passing)
- Spec Review: ✅ PASSED
- Architecture: burn-wgpu 0.21-pre2, native Vulkan, 128-D event embeddings

**Build Status**: ✅ Release build successful (0 errors, 132 pre-existing warnings)

---

## Implementation Order Decision

**Two viable paths after Task A.1 (Audio Features)**:

### Path 1: Complete Multimodal Features First (A2→A3→A4)
**Pros**:
- ✅ Complete feature extraction foundation (448-D multimodal ready)
- ✅ Can start corpus preparation (B1-B2) in parallel
- ✅ Feeds all downstream tasks
**Cons**:
- Longer before ML pipeline is visible
- RF + Visual features may be less critical for initial ray tracing demo

### Path 2: Begin RT Pipeline + TimeGNN burn-wgpu in Parallel (D1a + C1)
**Pros**:
- ✅ Start rendering ray-traced attacks immediately
- ✅ TimeGNN burn-wgpu framework ready for immediate integration
- ✅ Demonstrate 476fps target sooner
- ✅ Can prototype with just audio features while RF/Visual added later
**Cons**:
- Skip RF/Visual features temporarily
- May need placeholder features for testing

**✅ DECIDED: Path 2 (TimeGNN + RT First)**

Implementation sequence:
1. ✅ Task A.1: Audio 196-D Feature Extractor (COMPLETE)
2. **→ Task D.1a: RayTracer Image-Source Method** (next - generates 128-D ray features)
3. **→ Task C.1: TimeGNN burn-wgpu Architecture** (uses 1344-D: 196 audio + 768 wav2vec2 + 128 ray)
4. **→ Task D.1b: RtAttackViz Hardware Ray Tracing** (renders TimeGNN output as 3D attack geometry)
5. **→ Task D.1c: Mesh Shaders (28 LOD)** (smooth 476fps visualization)
6. **→ Task D.2: ANALYSIS Tab UI** (4 visualization panels)
7. *Parallel*: Tasks A.2, A.3, A.4 (RF/Visual features) added as they're completed

---

## Task A.2: RF Features 128-D

**TDD approach**: Write failing test for 128-D RF feature dimension, verify fail, implement minimal extractor with magnitude+phase+heterodyne components, verify pass, commit.

---

## Task A.3: Visual Features 64-D

**TDD approach**: Extract optical flow from 8×8 grid (64 patches), test 64-D dimension, implement minimal visual feature extractor, commit.

---

## Task A.4: Multimodal Concatenation (448-D)

**TDD approach**: Test concatenation of Audio (196-D) + RF (128-D) + Visual (64-D), implement concatenate_features(), verify 448-D output.

---

## Task B.1: wav2vec2 Frozen Model Integration

**Python offline pipeline**: Load transformers Wav2Vec2Model, extract 768-D embeddings per 100ms frame, return embeddings for all forensic events.

---

## Task B.2: Event Corpus Structure (HDF5)

**Python corpus preparation**: Read forensic_log.jsonl, extract multimodal + wav2vec2 embeddings, write HDF5 with datasets for training.

---

## Task C.1: TimeGNN Graph Neural Network

**Architecture**: GraphAttention layers (1216-D → 512 → 256 → 128-D) with contrastive learning objective.

---

## Task C.2: TimeGNN Training Loop

**Training**: Contrastive loss on harassment signature similarity, temporal proximity edges, RF co-occurrence edges.

---

## Task D.1: Ray-Aware Visualization

**NEW: Ray-Tracing Integration** (from user's three architectural messages):

### D.1a: RayTracer Image-Source Method (128-D)
- Simulate room impulse response, extract ray features (direct path, reflections, diffuse spread)
- 128-D ray feature vector feeds into TimeGNN training

### D.1b: RtAttackViz Hardware Ray Tracing
- Vulkan VK_KHR_ray_tracing_pipeline on RX 6700 XT
- WGSL compute shader for ray tracing attack geometry
- Heat map visualization of attack intensity (blue→red→white)
- 476fps target on 1920x1080

### D.1c: Mesh Shader Visualization (28 LOD)
- Adaptive mesh LOD from 2048px coverage (level 0) to 64px (level 27)
- Smooth transitions for silky 476fps rendering
- Task + mesh + fragment shaders in WGSL

---

## Task D.2: ANALYSIS Tab UI (4 panels)

- Temporal scatter plot: Events over time, colored by pattern cluster
- Pattern library heatmap: Recurring signatures ranked by frequency
- Clustering dendrogram: Harassment signature hierarchy
- Correlation graph: Event connections (temporal + spectral + spatial)

---

## Task E.1: Offline Training Pipeline

**E2E workflow**: prepare_event_corpus.py → wav2vec2_extractor.py → timegnn_trainer.py → event_visualizer.py

---

## Integration: Ray-Aware TimeGNN

**Key insight from user**: TimeGNN + RayTracer create **ray-informed attack visualization** by training on:
- Multimodal features (audio 196-D + RF 128-D + visual 64-D)
- wav2vec2 embeddings (768-D)
- **Ray features (128-D)** ← NEW: Image-source method + room acoustics
- Total input: **1344-D**
- Output: **128-D event embeddings optimized for both pattern discovery AND ray-based visualization**

This allows TimeGNN to learn that certain harassment signatures have characteristic **room impulse response signatures** (direct path, early reflections, diffuse field) that correlate with attack intensity.

---

## Roadmap: Post-Phase-2 (After App Complete)

### Pico 2 RTC Integration (@twister_rtc)
- **Purpose**: Swappable hardware RTC for forensic timestamping independence
- **Use case**: If PDM mic integration happens, RTC provides independent timebase (PDM may not support arbitrary RX like current TX)
- **Technical insight**: PDM + arbitrary waveform TX up to Nyquist (PC speed) → Mamba must be waveform-agnostic (detect shape/pattern regardless of carrier: 1 Hz to 95 GHz)
  - Audio 196-D features already frequency-normalized → ready for this
  - RTC adds court-admissible timestamps separate from system clock
- **Implementation**: USB HID gadget mode on RP2350, dual timestamps in forensic logs
- **Timeline**: Post-Phase-2, parallel with other enhancements

---

## Success Criteria

✅ **Phase 2 Complete**:
- All Tasks A-E passing tests
- Ray-tracing rendering at 476fps target
- TimeGNN training converges (contrastive loss < 0.5)
- ANALYSIS tab displays 4 interactive visualizations
- End-to-end pipeline functional (corpus → training → visualization)

**Next**: D.1b RtAttackViz implementation (Vulkan RT + WGSL heat map visualization)

