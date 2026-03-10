# Parallel Feature Development: Twister V3 Architecture

**Goal**: Execute 7 parallel work tracks (A-G) with zero merge conflicts and clear integration points.

**Strategy**: Directory-based ownership (Track A owns `src/hardware_io/`, etc.) + Vertical slices (each track builds complete feature end-to-end) + Interface contracts (shared type definitions in separate, read-only files).

---

## File Ownership Map

### Track A: Device Orchestration (Hardware I/O)

**Owns** (exclusive):
```
src/hardware_io/
├── mod.rs
├── device_manager.rs          ← A.2 (created)
├── sdr_device.rs              ← A.1 (create safe_sdr_wrapper alias)
└── tests.rs

src/safe_sdr_wrapper.rs        ← A.1 (new file)
src/rtlsdr_ffi.rs              ← A.1 (extend, don't break existing)

ui/components/device_controls.slint  ← A.3 (UI definition)
src/ui/device_controls_controller.rs ← A.3 (new file, callbacks only)

examples/
├── test_radio_device_open.rs   ← A.1
├── test_device_manager_add_remove.rs ← A.2
└── test_slint_device_controls.rs    ← A.3
```

**Does NOT own**:
- `src/app_state/` (owned by no one, read-only interface)
- `src/main.rs` (coordination point, shared)

**Read-only imports**:
- `app_state::DirtyFlags` (interface in Track A.2, but defined elsewhere)

---

### Track B: Signal Ingestion (Audio → GPU)

**Owns** (exclusive):
```
src/dispatch.rs                   ← B.1 (new Tokio loop)

src/vbuffer.rs                    ← B.3 (extend existing, rolling history)

src/visualization/
├── stft_iq.wgsl                  ← B.2 (GPU FFT shader)
└── gpu_stft_dispatch.rs          ← B.2 (shader compilation + dispatch)

examples/
├── test_iq_dispatch_loop.rs      ← B.1
├── test_stft_shader.rs           ← B.2
└── test_vbuffer_rolling.rs       ← B.3
```

**Does NOT own**:
- `src/gpu.rs` (read-only, GPU device state)
- `src/gpu_memory.rs` (read-only, unified memory primitives)

**Read-only imports**:
- `gpu::Device` (interface for GPU dispatch)
- `vbuffer::IqVBuffer` (read-only struct definition)
- `gpu_memory::UnifiedBuffer<f32>` (for output buffer)

---

### Track C: Forensic Analysis & Pattern Discovery

**Owns** (exclusive):
```
src/ml/event_corpus.rs           ← C.1 (new file)
src/ml/timegnn.rs                ← C.2 (extend existing)
src/ml/pattern_discovery.rs       ← C.3 (new file)

@databases/
├── events.h5                     ← C.1 (output artifact)
└── harassment_patterns.json      ← C.2 (output artifact)

examples/
├── test_event_corpus_load.rs     ← C.1
├── test_timegnn_training.rs      ← C.2
└── test_motif_matching.rs        ← C.3
```

**Does NOT own**:
- `src/forensic_log.rs` (read-only, event emission)
- `src/ml/multimodal_fusion.rs` (read-only, feature extraction)

**Read-only imports**:
- `forensic_log::ForensicEvent` (interface struct)
- `multimodal_fusion::Embedding1092D` (type alias)

---

### Track D: Spatial Localization (Point Mamba)

**Owns** (exclusive):
```
src/ml/pointnet_encoder.rs        ← D.1 (new file)
src/ml/mamba_block.rs             ← D.2 (extend existing)
src/ml/point_decoder.rs           ← D.3 (new file)
src/ml/point_mamba.rs             ← D.1-D.4 orchestration

src/visualization/
├── point_mamba_visualizer.rs     ← D.4 (new renderer)
└── point_mamba_visualizer/       ← D.4 (shader directory)
    ├── gaussian_splatter.wgsl
    └── tonemap.wgsl

@models/
├── pointnet_pretrained.safetensors   ← D.1 (weights)
├── point_decoder.safetensors         ← D.3 (weights)
└── point_mamba_checkpoint.pt         ← D.2 (checkpoint)

examples/
├── test_pointnet_encoder.rs       ← D.1
├── test_mamba_block_inference.rs  ← D.2
├── test_point_decoder_output.rs   ← D.3
└── test_gaussian_splatting_render.rs ← D.4
```

**Does NOT own**:
- `src/visualization/gaussian_splatting.rs` (read-only, rendering primitives)
- `src/gpu.rs` (read-only, GPU device)

**Read-only imports**:
- `gaussian_splatting::GaussianSplat` (vertex type)
- `gpu::RenderPass` (rendering interface)

---

### Track E: Agentic UI (Dorothy + TypeScript Harness)

**Owns** (exclusive):
```
agent_harness/
├── src/
│   ├── index.ts                  ← E.1
│   ├── graph/
│   │   └── harassment_investigation.ts ← E.2
│   └── editor/
│       └── collab_editor.ts       ← E.3
├── tests/
│   ├── websocket_connection.ts    ← E.1
│   ├── workflow_execution.ts      ← E.2
│   └── editor_collab.ts           ← E.3
└── package.json

ui/generated/
└── *.slint                        ← E.3 (LLM-generated components)
```

**Does NOT own**:
- `src/ui/app_controller.rs` (read-only, Slint event dispatch)
- `src/main.rs` (shared coordination)

**Read-only imports**:
- `AppState` (via WebSocket messages, not code imports)
- `ForensicEvent` (JSON schema, not Rust types)

---

### Track F: Integration & Testing

**Owns** (exclusive):
```
examples/full_harassment_defense_demo.rs  ← F.1 (orchestration test)

tests/integration/
├── end_to_end.rs                        ← F.1
└── smoke_test.rs                        ← F.1
```

**Does NOT own**:
- Everything else is read-only for F.1

**Read-only imports**:
- All modules from A-E

---

### Track G: Documentation & Release

**Owns** (exclusive):
```
docs/
├── modular-design.md                    ← G.1
├── api-contracts.md                     ← G.1
└── integration-guide.md                 ← G.1

CLAUDE.md (version 0.5.0 → 0.6.0)        ← G.2

.github/
└── CHANGELOG.md                         ← G.2
```

**Does NOT own**:
- Any code files (documentation only)

---

## Shared Interface Contracts (Read-Only)

These files define the boundaries between tracks. **No track modifies these except as noted.**

### Contract 1: `src/app_state/mod.rs` (Dirty Flags Interface)

```rust
// Owner: Track A.2 (defines), all tracks (read-only)
pub struct DirtyFlags {
    pub device_list_dirty: AtomicBool,
    pub frequency_lock_dirty: AtomicBool,
    pub audio_features_dirty: AtomicBool,      // Set by Track B
    pub latent_embedding_dirty: AtomicBool,    // Set by Track C
    pub point_cloud_dirty: AtomicBool,         // Set by Track D
    pub synthesis_dirty: AtomicBool,
}
```

**Who sets what**:
- Track A.2: `device_list_dirty`, `frequency_lock_dirty`
- Track B.1: `audio_features_dirty`
- Track C.2: `latent_embedding_dirty`
- Track D.4: `point_cloud_dirty`

---

### Contract 2: `src/ml/types/embeddings.rs` (Feature Dimensions)

```rust
// Owner: Track B.2 + Track C.1 (read-only)
pub const AUDIO_DIM: usize = 196;          // Track B.2 output
pub const VISUAL_DIM: usize = 128;
pub const WAV2VEC2_DIM: usize = 768;       // Track C.1 input
pub const MULTIMODAL_DIM: usize = 1092;    // Track C.1 definition

pub type AudioFeatures = [f32; AUDIO_DIM];
pub type VisualFeatures = [f32; VISUAL_DIM];
pub type Embedding1092D = [f32; MULTIMODAL_DIM];
pub type PointMambaLatent = [f32; 128];    // Track D.2 output
```

---

### Contract 3: `src/ml/types/forensic_events.rs` (Event Schema)

```rust
// Owner: Track C.3 (emitter), Track E.2 (consumer), read-only
pub struct ForensicEvent {
    pub timestamp: u64,
    pub motif_id: u32,
    pub confidence: f32,
    pub spatial_location: Option<Point3D>,
}

pub struct Point3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
```

**Producer**: Track C.3 (calls forensic_log::emit())
**Consumer**: Track E.2 (reads from WebSocket messages)

---

### Contract 4: `src/visualization/types/point_mamba_output.rs` (Renderer Input)

```rust
// Owner: Track D.3 (producer), Track D.4 (consumer), read-only
pub struct PointMambaOutput {
    pub point_cloud: Vec<Point3D>,
    pub intensities: Vec<f32>,
    pub confidences: Vec<f32>,
    pub timestamp: u64,
}
```

**Producer**: Track D.3 (point decoder)
**Consumer**: Track D.4 (Gaussian splatting renderer)

---

## Integration Strategy: Vertical Slices + Shared Contracts

Each track (A-E) implements a **complete feature end-to-end**:

```
Track A: User clicks "Add Device" → RTL-SDR opens → UI shows status (complete slice)
Track B: IQ samples arrive → GPU STFT → spectral history updated (complete slice)
Track C: Spectral history → TimeGNN → harassment patterns detected (complete slice)
Track D: Detection event → Point Mamba → 3D visualization (complete slice)
Track E: Harassment event → Dorothy → report generated (complete slice)
```

**Integration happens via shared interfaces**:
- Track A → Track B: Device registry passed to dispatch loop
- Track B → Track C: Audio features in dirty flags
- Track C → Track D: Detection events via ForensicEvent schema
- Track D → Track E: PointMambaOutput + ForensicEvent via WebSocket messages

**No file overlap** = No merge conflicts.

---

## Conflict Avoidance Rules

### ✅ DO

1. **Modify only your assigned files** (Track A modifies only `src/hardware_io/*`, etc.)
2. **Import read-only contracts** (e.g., Track B imports `DirtyFlags` but doesn't modify `app_state/mod.rs`)
3. **Request interface changes** (If Track B needs a new dirty flag, ask in a PR comment; Track A.2 owner applies it)
4. **Write isolated examples** (Each track's example tests only that track's functionality)
5. **Use dirty flags for synchronization** (Don't call other tracks' functions directly)

### ❌ DON'T

1. **Modify files outside your track** (Track C never touches `src/hardware_io/`)
2. **Rename public APIs** in shared contracts (breaking change for all other tracks)
3. **Directly call other tracks' private functions** (use interfaces instead)
4. **Commit to `main` without passing your track's example tests**
5. **Assume initialization order** (use lazy initialization, Arc<Mutex<>>)

---

## Branch Management Strategy

**Single branch strategy** (no sub-branches):
- All tracks work on `feature/v3-architecture` simultaneously
- File ownership prevents conflicts
- Smaller, more frequent commits = easier rebases

**When conflicts arise**:
1. Identify which track owns the conflicted file
2. That track's owner resolves the conflict (by rebasing their changes)
3. Non-owner reopens their work without the conflicted file

**Merge to main**: Only after F.1 (integration test) passes

---

## Implementation Timeline

```
Week 1:
  [ ] A.1: FFI wrapper (3 days)
  [ ] B.2: STFT shader (3 days)
  [ ] C.1: Event corpus (2 days)
  [ ] D.1: PointNet encoder (3 days)
  [ ] E.1: Dorothy init (2 days)

Week 2:
  [ ] A.2: Device manager (2 days) [blocks A.3, B.1]
  [ ] B.1: IQ dispatch (2 days) [blocked by A.2]
  [ ] B.3: V-Buffer (2 days) [blocked by B.2]
  [ ] C.2: TimeGNN (3 days) [blocked by C.1]
  [ ] D.2: Mamba blocks (3 days) [blocked by D.1]

Week 3:
  [ ] A.3: Slint wiring (2 days) [blocked by A.2]
  [ ] C.3: Motif matching (2 days) [blocked by C.2]
  [ ] D.3: Decoder (2 days) [blocked by D.2]
  [ ] E.2: LangGraph (3 days) [blocked by E.1]

Week 4:
  [ ] D.4: Gaussian splatting (2 days) [blocked by D.3]
  [ ] E.3: Collab editor (3 days) [blocked by E.2]

Week 5:
  [ ] F.1: Integration test (3 days) [blocked by A.3, B.3, C.3, D.4, E.2]

Week 6:
  [ ] G.1: API docs (3 days) [blocked by F.1]
  [ ] G.2: Release versioning (2 days) [blocked by F.1]
```

---

## Checklist for Track Owners

When you claim a track:

- [ ] Copy the "Track Assignment Template" from `tracks.md`
- [ ] List the specific files you will create/modify
- [ ] Review the "File Ownership Map" above (don't overlap)
- [ ] Import only read-only contracts (check Contract 1-4)
- [ ] Write an example that tests **only your track** in isolation
- [ ] Run `cargo build --release` and verify no new warnings
- [ ] Create a PR with title: `[TRACK X.Y] Feature Name`
- [ ] Request review from the team/lead
- [ ] After merge, update `tracks.md` status to `[✓] Complete`
- [ ] Check if your completion unblocks dependent tracks

---

## Current Assignments

| Track | Owner | Status | Begin Date | Est. Completion |
|-------|-------|--------|------------|-----------------|
| A.1 | — | [ ] | — | — |
| A.2 | Claude | [✓] | 2026-03-08 | ✓ |
| A.3 | — | [ ] | — | — |
| B.1 | — | [ ] | — | — |
| B.2 | — | [ ] | — | — |
| B.3 | — | [ ] | — | — |
| C.1 | — | [ ] | — | — |
| C.2 | — | [ ] | — | — |
| C.3 | — | [ ] | — | — |
| D.1 | — | [ ] | — | — |
| D.2 | — | [ ] | — | — |
| D.3 | — | [ ] | — | — |
| D.4 | — | [ ] | — | — |
| E.1 | — | [ ] | — | — |
| E.2 | — | [ ] | — | — |
| E.3 | — | [ ] | — | — |
| F.1 | — | [ ] | — | — |
| G.1 | — | [ ] | — | — |
| G.2 | — | [ ] | — | — |

---

## How to Claim a Track

1. Find an unassigned track with **no blockers**
2. Comment in this file: `# CLAIMING: [TRACK X.Y] - [Your Name]`
3. Create a feature branch: `feature/track-x-y`
4. Implement the "Middle" section from `tracks.md`
5. Run tests, commit, open PR
6. After approval, mark `[✓]` in this table

Example:

```markdown
# CLAIMING: TRACK A.1 - Jules

**Status**: [🔄] In progress
**Branch**: feature/track-a-1-ffi-wrapper
**PR**: #42 (pending review)
```

---

## Q&A

**Q: Can Track B.1 start before A.2 finishes?**
A: No—B.1 depends on `DeviceManager` from A.2. But B.2 (STFT shader) can run in parallel; it doesn't depend on A.

**Q: What if I need to modify a file outside my track?**
A: Request it in the PR review comment. The file's owner will make the change sequentially to avoid conflicts.

**Q: How do I test my track without other tracks completing?**
A: Write an example that mocks the dependencies. E.g., Track B.1 example mocks `DeviceManager::read_sync()` to inject fake IQ samples.

**Q: What if Contract 2 (embeddings.rs) needs a new dimension?**
A: Submit a PR to `src/ml/types/embeddings.rs` with the justification. All affected tracks must acknowledge the change.

---

**Last Updated**: 2026-03-08
**Architecture Version**: V3 (Modular, Brain/Brawn, Explicit Device Control)
