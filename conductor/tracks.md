# Project Tracks

This file tracks all major tracks for the project. Each track represents a complete work unit: **Beginning → Middle → End** with clear boundaries and no code conflicts.

---

## Navigation

- **Existing Tracks**:
  - [~] **Track: Circular GPU Buffer & Phase State Persistence & Desperado Integration**
    *Link: [./tracks/circular_gpu_buffer_20260305/](./tracks/circular_gpu_buffer_20260305/)*

---

## V3 Architecture Tracks (Device Manager + Dorothy)

Tracks for the modular, Brain/Brawn architecture with explicit device control and agentic UI.

### Track A: Device Orchestration (Hardware I/O)

**Goal**: Safe FFI wrappers + device registry with explicit user control (no auto-detect)

#### A.1: FFI Wrapper Consolidation
- **Beginning**: `rtlsdr_ffi.rs` exists (feature-gated)
- **Middle**: Create `src/safe_sdr_wrapper.rs` (`RadioDevice` enum, unsafe FFI isolated)
- **End**: `cargo build --features pluto-plus` compiles cleanly
- **Tests**: `examples/test_radio_device_open.rs`
- **Status**: [ ] Not started
- **Owner**: —

#### A.2: Device Manager Registry
- **Beginning**: (stub) No device coordination
- **Middle**: `src/hardware_io/device_manager.rs` (add/remove/tune lifecycle)
- **End**: Device list tracked, dirty flags propagate to UI
- **Tests**: `examples/test_device_manager_add_remove.rs`
- **Status**: [✓] Created (device_manager.rs written)
- **Owner**: Claude

#### A.3: Slint ↔ Device Manager Wiring
- **Beginning**: `ui/components/device_controls.slint` exists (no callbacks)
- **Middle**: `src/ui/app_controller.rs` (button clicks → DeviceManager calls)
- **End**: Click "+ Add RTL-SDR" → device opens → UI shows "Ready"
- **Tests**: `examples/test_slint_device_controls.rs`
- **Status**: [ ] Not started
- **Owner**: —
- **Blocker on**: A.2

#### A.4: Zero-Copy DMA Gateway (The Brick Road)
- **Beginning**: `src/vbuffer.rs::IqVBuffer` exists (bare shell, no staging buffer)
- **Middle**: `src/hardware_io/dma_vbuffer.rs` (host staging → GPU VRAM, rolling circular buffer, zero allocation)
- **End**: Raw `[u8]` IQ samples from RadioDevice → CPU-mapped staging buffer → DMA copy to GPU (no f32 conversion on CPU)
- **Tests**: `examples/test_dma_ingestion.rs`
- **Status**: [ ] Not started
- **Owner**: —
- **Blocker on**: None (parallel-safe, but B.1 depends on this)

---

### Track B: Signal Ingestion (Audio → GPU)

**Goal**: Zero-copy IQ bytes from RTL-SDR/Pluto+ → GPU STFT → rolling spectral history

#### B.1: IQ Sample Stream from Devices
- **Beginning**: `vbuffer.rs::IqVBuffer` exists (bare shell)
- **Middle**: `src/dispatch.rs` (Tokio loop: device.read_sync() → IqVBuffer)
- **End**: RTL-SDR → `[u8; 2]` samples → GPU (zero f32 conversion)
- **Tests**: `examples/test_iq_dispatch_loop.rs`
- **Status**: [ ] Not started
- **Owner**: —
- **Blocker on**: A.2

#### B.2: STFT (GPU FFT on IQ Data)
- **Beginning**: `src/visualization/stft_iq.wgsl` (stub)
- **Middle**: WGSL Radix-2 FFT ([2048] → [512] bins)
- **End**: Raw IQ → GPU FFT → spectral magnitude
- **Tests**: `examples/test_stft_shader.rs`
- **Status**: [ ] Not started
- **Owner**: —

#### B.3: V-Buffer Versioning
- **Beginning**: `vbuffer.rs::GpuVBuffer` (push_frame stub)
- **Middle**: Rolling history with `version % DEPTH` indexing
- **End**: 512-frame context window (10.7s spectral history)
- **Tests**: `examples/test_vbuffer_rolling.rs`
- **Status**: [ ] Not started
- **Owner**: —
- **Blocker on**: B.2

---

### Track C: Forensic Analysis & Pattern Discovery

**Goal**: TimeGNN extracts harassment signatures from multimodal data

#### C.1: Event Corpus Preparation
- **Beginning**: `src/ml/event_corpus.rs` (stub)
- **Middle**: Load forensic log → extract [196D audio + 128D visual + 768D wav2vec2] → HDF5
- **End**: Ready-to-train corpus in `@databases/events.h5`
- **Tests**: `examples/test_event_corpus_load.rs`
- **Status**: [ ] Not started
- **Owner**: —

#### C.2: TimeGNN Pattern Discovery
- **Beginning**: `src/ml/timegnn_trainer.rs` (stub)
- **Middle**: Train TimeGNN on corpus → K=23 motifs → JSON output
- **End**: Harassment signatures in `@databases/harassment_patterns.json`
- **Tests**: `examples/test_timegnn_training.rs`
- **Status**: [ ] Not started
- **Owner**: —
- **Blocker on**: C.1

#### C.3: Motif Matching & Reporting
- **Beginning**: `src/applications/harassment_defense.rs` (stub)
- **Middle**: Load patterns → match embeddings → emit ForensicEvent
- **End**: Real-time detection + evidence logging
- **Tests**: `examples/test_motif_matching.rs`
- **Status**: [ ] Not started
- **Owner**: —
- **Blocker on**: C.2

---

### Track D: Spatial Localization (Point Mamba)

**Goal**: 3D attack source localization via Mamba + Gaussian splatting

#### D.1: Point Cloud → Mamba Encoder
- **Beginning**: `src/ml/pointnet_encoder.rs` (stub)
- **Middle**: PointNet (N,6) → (N,256) + Burn backend
- **End**: Point cloud → semantic embeddings
- **Tests**: `examples/test_pointnet_encoder.rs`
- **Status**: [ ] Not started
- **Owner**: —

#### D.2: Selective Scan Mamba Blocks
- **Beginning**: `src/ml/mamba_block.rs` (stub)
- **Middle**: S6 state-space (8 cascaded blocks)
- **End**: Spatial reasoning layer
- **Tests**: `examples/test_mamba_block_inference.rs`
- **Status**: [ ] Not started
- **Owner**: —
- **Blocker on**: D.1

#### D.3: 3D Reconstruction Decoder
- **Beginning**: `src/ml/point_decoder.rs` (stub)
- **Middle**: Mamba latent (N,128) → positions (N,3) + intensity
- **End**: Reconstructed 3D point cloud
- **Tests**: `examples/test_point_decoder_output.rs`
- **Status**: [ ] Not started
- **Owner**: —
- **Blocker on**: D.2

#### D.4: GPU Gaussian Splatting Renderer
- **Beginning**: `src/visualization/gaussian_splatting.rs` (Wave64 optimized)
- **Middle**: Accept PointMambaOutput → render Gaussian splats @ 169 FPS
- **End**: Real-time 3D attack visualization
- **Tests**: `examples/test_gaussian_splatting_render.rs`
- **Status**: [ ] Not started
- **Owner**: —
- **Blocker on**: D.3

---

### Track E: Agentic UI (Dorothy + TypeScript Harness)

**Goal**: Rust ↔ TS bridge for LLM-driven report generation

#### E.1: Dorothy Initialization
- **Beginning**: No TypeScript harness exists
- **Middle**: `agent_harness/src/index.ts` (Express + WebSocket server)
- **End**: Message routing operational
- **Tests**: `agent_harness/tests/websocket_connection.ts`
- **Status**: [ ] Not started
- **Owner**: —

#### E.2: LangGraph Workflow
- **Beginning**: Dorothy stub
- **Middle**: `agent_harness/src/graph/harassment_investigation.ts` (StateGraph)
- **End**: Event → investigation workflow
- **Tests**: `agent_harness/tests/workflow_execution.ts`
- **Status**: [ ] Not started
- **Owner**: —
- **Blocker on**: E.1

#### E.3: Collaborative Report Editor
- **Beginning**: Stub
- **Middle**: `agent_harness/src/editor/collab_editor.ts` (CopilotKit + CRDT)
- **End**: User + Dorothy co-author reports in real-time
- **Tests**: `agent_harness/tests/editor_collab.ts`
- **Status**: [ ] Not started
- **Owner**: —
- **Blocker on**: E.2

---

### Track F: Integration & Testing

**Goal**: Full pipeline smoke test

#### F.1: End-to-End Harassment Defense Demo
- **Beginning**: All tracks complete
- **Middle**: `examples/full_harassment_defense_demo.rs` (Device → STFT → Pattern match → Report)
- **End**: Full pipeline demonstrable in 30 seconds
- **Tests**: Must pass before shipping
- **Status**: [ ] Not started
- **Owner**: —
- **Blocker on**: A.3, B.3, C.3, D.4, E.2

---

### Track H: Human Input Devices (HID)

**Goal**: Unified gamepad input (Joy-Con, DualSense 6-axis IMU) with explicit Slint binding

#### H.1: Safe HID Wrapper
- **Beginning**: No HID support exists
- **Middle**: `src/safe_hid_wrapper.rs` (Joy-Con L/R, DualSense via hidapi or gilrs)
- **End**: `HidDeviceType { JoyConL, JoyConR, DualSense }` with 6-axis IMU (gyro, accel) polling
- **Tests**: `examples/test_hid_device_open.rs`
- **Status**: [ ] Not started
- **Owner**: —

#### H.2: HID Device Manager Extension
- **Beginning**: DeviceManager only handles RF devices
- **Middle**: Extend DeviceManager to `add_hid_device()`, `remove_hid_device()`, high-polling event loop (60+ Hz for IMU)
- **End**: HID devices tracked separately from RF, no interference
- **Tests**: `examples/test_hid_device_manager.rs`
- **Status**: [ ] Not started
- **Owner**: —
- **Blocker on**: H.1

#### H.3: Slint HID Binding
- **Beginning**: `ui/components/device_controls.slint` has no gamepad section
- **Middle**: Add "+ Connect Joy-Con" / "+ Connect DualSense" buttons, display 6-axis data (gyro X/Y/Z, accel X/Y/Z)
- **End**: Live gamepad motion display in UI
- **Tests**: `examples/test_slint_hid_controls.rs`
- **Status**: [ ] Not started
- **Owner**: —
- **Blocker on**: H.2

---

### Track G: Documentation & Release

**Goal**: Ship with clean API documentation and versioning

#### G.1: API Documentation
- **Beginning**: Code written, no docs
- **Middle**: `cargo doc` + `docs/modular-design.md`
- **End**: New developers onboard in 1 hour
- **Status**: [ ] Not started
- **Owner**: —
- **Blocker on**: All other tracks

#### G.2: Release Versioning
- **Beginning**: `version = "0.5.0"`
- **Middle**: Tag commits, update CLAUDE.md, publish to GitHub Releases
- **End**: Shipping cadence clear
- **Status**: [ ] Not started
- **Owner**: —
- **Blocker on**: F.1

---

## Dependency Graph

```
PARALLEL START (no blockers):
  A.1, A.4, B.2, C.1, D.1, E.1, H.1

RF I/O Track (A):
  A.1 → A.2 [complete] → A.3 + A.4 (parallel) → B.1 (IQ dispatch depends on A.4)

HID Track (H, independent):
  H.1 → H.2 → H.3

Signal Pipeline (B):
  B.2 → B.3 (depends on B.2)
  A.4 + B.1 + B.3 → F.1 (integration test)

Forensic (C):
  C.1 → C.2 → C.3

Spatial (D):
  D.1 → D.2 → D.3 → D.4

Dorothy (E):
  E.1 → E.2 → E.3

Integration (F):
  A.3 + A.4 + B.3 + C.3 + D.4 + E.2 + H.3 (all complete) → F.1

Release (G):
  F.1 → G.1 + G.2
```

---

## Ground Rules (No Code Traffic)

✅ **DO**:
- Modify **only your track's files**
- Add new modules (don't rename existing ones)
- Write examples that test your feature in isolation
- Mark dirty flags when state changes

❌ **DON'T**:
- Edit files outside your track
- Rename public APIs
- Remove features from other tracks
- Commit without passing examples

---

## Assignment Template

```markdown
# [TRACK X.Y]: [Task Name]

**Status**: [ ] Not started / [🔄] In progress / [✓] Complete
**Owner**: [Person/Agent]

## Implementation Checklist
- [ ] Implement Feature A in [file]
- [ ] Add tests in examples/test_*.rs
- [ ] cargo build --release passes
- [ ] Example runs cleanly

## To claim this track:
Edit this section with your name and start date.
```
