# Dead Code Integration Plan

**Status**: PLANNING & IMPLEMENTATION
**Priority**: HIGH (Unified Memory Zero-Copy Pipeline)
**Hardware**: RX 6700 XT (12GB VRAM, PCIe 4.0, Unified Memory)
**Date**: 2026-03-08

---

## ⚠️ CRITICAL BLOCKER: Burn/Candle API Migration

The codebase is in a **partially migrated state** between Candle and Burn ML frameworks. This is blocking compilation.

### Current State:
- `src/mamba.rs` - Uses Burn (`burn::` imports)
- `src/training.rs` - Was using Candle (`candle_core::Device`), partially migrated
- `src/trainer.rs` - Was using Candle, partially migrated
- ML modules in `src/ml/` - Use Burn

### Errors (26 total):
```
error[E0107]: missing generics for struct `OnlineTrainer`
error[E0308]: mismatched types (LinearConfig, LayerNormConfig constructors)
error[E0061]: method takes wrong arguments (squeeze, unsqueeze, ln)
error[E0599]: no method named `unwrap_or_else` found
```

### Required Fix:
**Option A**: Complete burn-wgpu migration (recommended)
- Fix all burn API changes in mamba.rs, training.rs, trainer.rs
- Update LinearConfig, LayerNormConfig constructors
- Fix method signatures (squeeze, unsqueeze, ln)

**Option B**: Revert to Candle
- Roll back mamba.rs to candle version
- Keep training.rs, trainer.rs on candle

### Burn API Changes (0.21.0-pre.2):
- `LinearConfig::new(in, out)` → check actual signature
- `tensor.squeeze(dim)` → `tensor.squeeze()` (no args)
- `tensor.unsqueeze(dim)` → `tensor.unsqueeze()` (no args)
- `tensor.ln()` → different API
- `WgpuDevice::default()` → `WgpuDevice::DefaultDevice`

---

## Executive Summary

This plan catalogs all dead code (139 warnings) in the Twister codebase and provides phased integration using unified memory principles. Goal: wire prepared ML components into runtime pipeline for live detection.

---

## Part 1: Dead Code Inventory

### Category A: ML Pipeline Modules (Prepared but Not Wired)

| Module | File | Lines | Status | Integration Target |
|--------|------|-------|--------|-------------------|
| wav2vec2_loader | src/ml/wav2vec2_loader.rs | ~200 | Dead | Training pipeline |
| timegnn | src/ml/timegnn.rs | ~300 | Dead | Analysis tab |
| timegnn_trainer | src/ml/timegnn_trainer.rs | ~450 | Dead | Offline training |
| event_corpus | src/ml/event_corpus.rs | ~400 | Dead | HDF5 corpus gen |
| pattern_discovery | src/ml/pattern_discovery.rs | ~350 | Dead | Pattern clustering |
| pointnet_encoder | src/ml/pointnet_encoder.rs | ~200 | Dead | Phase 3 pipeline |
| point_mamba | src/ml/point_mamba.rs | ~200 | Dead | 3D wavefield |
| point_decoder | src/ml/point_decoder.rs | ~200 | Dead | 3D reconstruction |
| point_mamba_trainer | src/ml/point_mamba_trainer.rs | ~450 | Dead | Phase 3 training |
| gaussian_splatting | src/visualization/gaussian_splatting.rs | ~250 | Dead | 3D rendering |

### Category B: GPU Engine Dead Fields

| File | Field/Method | Type | Issue |
|------|--------------|------|-------|
| src/pdm.rs | readback_pcm | wgpu::Buffer | Never read back |
| src/pdm.rs | readback_pdm | wgpu::Buffer | Never read back |
| src/pdm.rs | readback_wide | wgpu::Buffer | Never read back |
| src/pdm.rs | readback_carry | wgpu::Buffer | Never read back |
| src/vbuffer.rs | push_const() | fn | Never called |
| src/vbuffer.rs | ready() | fn | Never called |
| src/vbuffer.rs | VBUF_WGSL_HELPERS | const | Never included |
| src/trainer.rs | last_save | field | Never read |

### Category C: Visualization Dead Code

| File | Issue |
|------|-------|
| src/visualization/ray_tracer.rs | Unused mut variables |
| src/visualization/rt_attack_viz.rs | Unused bind_group_layout, embeddings_buffer, output_texture |
| src/visualization/mesh_shaders.rs | Multiple unused fields |

---

## Part 2: Integration Priorities

### Priority 1: PDM Unified Memory (Quick Win)

**Goal**: Wire unified memory CPU←GPU reads for real-time PDM monitoring

**Files**: src/pdm.rs

**Current Issue**:
- PDM engine allocates separate readback buffers
- Data must copy from GPU VRAM → CPU RAM via PCIe
- Readback buffers never used (dead code)

**Solution**:
- Use gpu_memory.rs GpuRingBuffer with unified memory
- CPU can read PDM results directly without copy
- Enable real-time PDM visualization

**Impact**: Real-time PDM without PCIe copies

---

### Priority 2: V-Buffer Shader Helpers

**Goal**: Integrate WGSL helpers into GPU synthesis pipeline

**Files**: src/vbuffer.rs, src/synthesis.rs

**Current Issue**:
- VBUF_WGSL_HELPERS constant defined but never included
- push_const() and ready() methods never called
- Spectrum visualization computed but not fully utilized

**Solution**:
- Include VBUF_WGSL_HELPERS in synthesis WGSL
- Wire push_const() to shader dispatch
- Enable V-buffer spectrum display

**Impact**: Full spectrum visualization with zero-copy

---

### Priority 3: TimeGNN Live Embeddings

**Goal**: Connect TimeGNN to dispatch loop for live pattern detection

**Files**: src/ml/timegnn.rs, src/main.rs

**Current Issue**:
- TimeGNN model defined but never instantiated
- timegnn_trainer.rs is offline-only
- No live pattern detection in dispatch loop

**Solution**:
- Add TimeGNN inference to dispatch loop
- Process bispectrum events through embedding
- Update analysis tab with pattern matches

**Impact**: Real-time harassment pattern detection

---

### Priority 4: Point Mamba 3D Pipeline

**Goal**: Wire PointNet→PointMamba→PointDecoder for 3D wavefield

**Files**: src/ml/point_*.rs, src/main.rs

**Current Issue**:
- All point_mamba modules defined but not integrated
- gaussian_splatting renderer ready but not fed data
- No 3D wavefield visualization

**Solution**:
- Create point cloud from TDOA events
- Process through PointNet → PointMamba → Decoder
- Render 3D reconstruction with Gaussian splatting

**Impact**: Live 3D attack source visualization

---

## Part 3: Implementation Steps

### Step 1: PDM Unified Memory Integration

**Time**: 30 minutes

**Files Modified**:
- src/pdm.rs

**Actions**:
1. Replace readback buffers with unified memory approach
2. Use gpu_memory.rs GpuRingBuffer
3. Wire PDM output to state for UI display

```rust
// Target: src/pdm.rs
// Replace separate readback buffers with unified memory
```

---

### Step 2: V-Buffer Shader Integration

**Time**: 20 minutes

**Files Modified**:
- src/vbuffer.rs
- src/synthesis.rs (or wherever shaders are built)

**Actions**:
1. Include VBUF_WGSL_HELPERS in synthesis shader source
2. Wire push_const() call to shader dispatch
3. Connect spectrum output to UI

---

### Step 3: TimeGNN Runtime Integration

**Time**: 60 minutes

**Files Modified**:
- src/main.rs
- src/state.rs (add TimeGNN state fields)

**Actions**:
1. Add TimeGnnModel to AppState
2. Initialize in main after GPU setup
3. Add inference call in dispatch loop
4. Wire pattern output to analysis tab

---

### Step 4: Point Mamba Pipeline

**Time**: 90 minutes

**Files Modified**:
- src/main.rs
- src/state.rs
- src/visualization/gaussian_splatting.rs

**Actions**:
1. Add PointMamba components to AppState
2. Create point cloud from TDOA azimuth/elevation events
3. Process through PointNet → PointMamba → Decoder
4. Render with GaussianSplatter to texture
5. Display in ANALYSIS tab

---

## Part 4: Printf/Logging Cleanup

**Remaining**: ~159 println!/eprintln! calls

**Status**: Partially converted to state.log()

**Remaining Work**:
- Convert key debug prints to state.log()
- Focus on: audio init, SDR tuning, training progress
- Keep: test output, startup diagnostics

---

## Success Criteria

- [ ] PDM unified memory - real-time PDM display works
- [ ] V-Buffer shaders - full spectrum visualization
- [ ] TimeGNN - live pattern detection in ANALYSIS tab
- [ ] Point Mamba - 3D wavefield rendering
- [ ] Dead code warnings reduced from 139 to <50

---

## Notes

- Unified memory: RX 6700 XT supports zero-copy CPU↔GPU
- All ML modules use burn-wgpu backend
- Integration follows existing async patterns in main.rs
