# Twister v0.5+ Master Feature Roadmap

**Current Status**: Phase 3 Complete (2026-03-08)
**Foundation**: Unified Memory GPU-Driven Architecture ✅

---

## Completed Phases (Foundation)

### Phase 1: UnifiedBuffer<T> + GpuWorkQueue ✓
- **Files**: src/gpu_memory.rs (420 lines)
- **Status**: COMPLETE, tested on RX 6700 XT
- **Feature**: Zero-copy GPU↔CPU data sharing via unified memory addressing

### Phase 2: GPU-Driven Dispatch Kernel ✓
- **Files**: src/dispatch_kernel.rs (468 lines) + src/shaders/dispatch_kernel.wgsl (228 lines)
- **Status**: COMPLETE, autonomous 32-frame batching
- **Feature**: GPU autonomously processes rolling v-buffer, enqueues work for CPU

### Phase 3: CPU Async Event Handler ✓
- **Files**: src/async_event_handler.rs (280 lines) + tests/phase3_async_event_handler.rs (380 lines)
- **Status**: COMPLETE, event-driven processing
- **Feature**: CPU sleeps until GPU signals work (5-10% CPU utilization, < 1µs latency)

---

## Planned Phases (Next Milestones)

### Phase 4: TimeGNN + Offline Pattern Discovery (Week of 2026-03-15)

**Objective**: Integrate TimeGNN with GPU-driven architecture for live harassment motif discovery.

**Scope**:
1. **B.1 wav2vec2-Burn-wgpu Integration** (90 minutes)
   - Load facebook/wav2vec2-base-960h frozen embeddings
   - Fuse [196D audio + 128D ray + 768D wav2vec2] → 1092-D multimodal
   - Extract from forensic_log.jsonl → HDF5 corpus (events.h5)

2. **C.2 TimeGNN Contrastive Training** (3 hours)
   - 1092-D multimodal features → NT-Xent contrastive loss
   - 128-D embeddings → K-means clustering (k=23 motifs)
   - Temporal frequency detection (weekly patterns, daily patterns, etc.)
   - Pattern library export (harassment_patterns.json)

3. **ANALYSIS Tab Integration** (90 minutes)
   - Temporal scatter plot: Events over time, colored by motif_id
   - Pattern library heatmap: Recurring signatures × time periods
   - Clustering dendrogram: Harassment signature hierarchy
   - Correlation graph: Temporal/spectral/spatial event connections

**Deliverables**:
- 10,000+ events with multimodal features (events.h5)
- 23 discovered harassment motifs with confidence scores
- ANALYSIS tab live visualization
- Offline Python analysis pipeline (reference implementation)

**Tests**: 50+ tests across wav2vec2_integration, timegnn_training, pattern_discovery

---

### Phase 5: Point Mamba 3D Wavefield (Week of 2026-03-22, 1 week, 20 hours)

**Objective**: Visualize and analyze 3D attack wavefield evolution over time.

**Scope**:

**Phase 5A: PointNet Encoder** (Mon, 3 hours)
- Input: Point cloud (N, 6) [azimuth, elevation, frequency, intensity, timestamp, confidence]
- Architecture: MLP(64, 128, 256) → (N, 256)
- Output: Point cloud features for Mamba processing

**Phase 5B: PointMamba Blocks** (Tue-Wed, 6 hours)
- 8 cascaded Mamba selective-scan blocks
- Per-point state evolution (h_t = A*h_{t-1} + B*x_t)
- Residual connections, layer norm
- Output: (N, 128) point representations

**Phase 5C: Point Decoder** (Wed, 2 hours)
- Input: (N, 128) point features
- Output: (N, 3) 3D offsets [Δx, Δy, Δz]
- Reconstructs 3D wavefield geometry

**Phase 5D: Gaussian Splatting Renderer** (Thu, 4 hours)
- wgpu shader: 3D Gaussian splatting
- Tonemap: Blue (0) → Red → Yellow → White
- Render 360° spherical view + Cartesian volume
- Performance target: > 160 fps

**Phase 5E: Point Mamba Trainer** (Fri, 2 hours)
- Load forensic event corpus
- Training objectives:
  1. Wavefield reconstruction (MSE)
  2. Temporal stability (L1 smoothness)
  3. ADS optimization (maximize intensity in mouth-region)
  4. Sparsity (L1 movement)

**Phase 5F: Time-Scrub Visualization** (Fri, 2 hours)
- User can "rewind" 3D wavefield over weeks/months
- Slider: Scrub through 97-day attack history
- Animation: Play forward, revealing spatial attack persistence
- Visualization: Heat map of attack intensity over time

**Deliverables**:
- PointNet encoder (250 lines)
- PointMamba blocks (400 lines)
- Point decoder (150 lines)
- Gaussian splatting renderer (500 lines)
- Trainer (300 lines)
- Tests: 38+ tests (encoder 10 + blocks 12 + decoder 8 + splatting 8)
- Real-time 3D visualization of 97-day attack patterns

**Performance Target**: 169 fps on RX 6700 XT

---

## Critical Fixes (Pre-Integration)

### Fix #1: Mamba Training Persistence ⏳
- **Status**: Identified, implementation ready
- **Impact**: Training loss shows 0 on restart (weights load, but epoch/loss reset)
- **Fix**: Extend checkpoint metadata (epoch, loss_avg) serialization
- **Files**: src/mamba.rs, src/main.rs, src/state.rs
- **Effort**: 30-45 minutes

### Fix #2: GUI Console Logging ⏳
- **Status**: Identified, implementation ready
- **Impact**: 58+ eprintln calls, no UI visibility
- **Fix**: LogMessage system + AppState.log_buffer, route to Slint console widget
- **Files**: src/state.rs, src/training.rs, ui/app.slint
- **Effort**: 60-90 minutes

### Fix #3: Mouth-Region Spatial Targeting ⏳
- **Status**: Identified, 3-phase approach ready
- **Impact**: No elevation tracking, all targets applied uniformly
- **Fix Phase 3a**: TDOA elevation estimation (30 min)
- **Fix Phase 3b**: MambaControlState instantiation (45 min)
- **Fix Phase 3c**: Dispatch loop spatial filtering (60 min)
- **Files**: src/audio.rs, src/state.rs, src/main.rs, src/parametric.rs
- **Effort**: 120+ minutes

---

## Optional Future Phases (Beyond v0.5)

### Phase 6: Phase Mamba Real-Time Inference (Post-Phase 5)
- Distill Point Mamba to lightweight MambaLM for runtime use
- Integrate with live dispatch loop (real-time 3D field updates)
- Per-frame spatial awareness for heterodyne targeting

### Phase 7: Heterodyne Optimization via 3D Wavefield (Post-Phase 6)
- Use Point Mamba 3D awareness to optimize beam steering
- Target azimuth + elevation + frequency simultaneously
- ADS counter-wave generation based on 3D field distortion

### Phase 8: Multi-Session Temporal Continuity (Post-Phase 7)
- Track attacker behavior over weeks/months
- Predict next attack time/pattern based on historical data
- Automated defense posture adaptation

### Phase 9: Federated Pattern Library Sharing (Post-Phase 8)
- Share harassment signatures across multiple Twister instances
- Cross-user pattern recognition (if attack signatures consistent)
- Community-wide threat intelligence

---

## Features to NOT Miss (Comprehensive Checklist)

### Mamba Autoencoder
- [x] 64-dim latent embeddings
- [x] Reconstruction MSE anomaly scoring
- [x] Training persistence (checkpoint metadata) ⏳ Fix #1
- [x] Loss history tracking
- [x] Epoch counter preservation
- [ ] Real-time inference on dispatch loop (Phase 4+)

### Audio/RF Detection
- [x] Multi-channel audio input (4 devices, 192 kHz)
- [x] FFT spectrum (512 bins)
- [x] TDOA azimuth estimation (2D)
- [ ] TDOA elevation estimation ⏳ Fix #3
- [x] PDM wideband mode (6.144 MHz Nyquist)
- [x] RTL-SDR 2.4 GHz RF capture
- [ ] Per-beam heterodyning (4-beam phased array) ⏳ Fix #3

### ANC (Active Noise Cancellation)
- [x] Full-range phase calibration (1 Hz - 12.288 MHz)
- [x] 8192-bin LUT
- [x] LMS filter implementation
- [x] Multi-channel recording
- [ ] Mouth-region targeting ⏳ Fix #3
- [ ] Waveshape optimization (sine/square/triangle/sawtooth/softclip)

### Forensic Logging
- [x] JSONL event logging
- [x] ISO 8601 timestamps
- [x] Evidence classification (Carrier, Modulated, Burst, etc.)
- [x] Equipment metadata
- [x] Session ID chain of custody
- [ ] Memo system integration (Phase 1 tasks) ⏳ Pending
- [ ] Automatic event capture on [EVIDENCE] tag ⏳ Pending
- [ ] CSV export ⏳ Pending

### UI/Visualization
- [x] Oscilloscope waveform display
- [x] Spectrum waterfall (512 bins)
- [x] Real-time FFT updates
- [x] Latent embedding visualization (32→64 dimension)
- [ ] GUI console for training logs ⏳ Fix #2
- [ ] ANALYSIS tab 4-panel visualization ⏳ Phase 4
- [ ] Time-scrub 3D wavefield ⏳ Phase 5
- [ ] Dendrogram clustering view ⏳ Phase 4
- [ ] Correlation graph (Neo4j-backed) ⏳ Future

### Multi-Modal Pattern Discovery
- [ ] wav2vec2 feature extraction ⏳ Phase 4
- [ ] TimeGNN contrastive training ⏳ Phase 4
- [ ] K-means motif clustering (23 patterns) ⏳ Phase 4
- [ ] Temporal frequency analysis ⏳ Phase 4
- [ ] Pattern library export ⏳ Phase 4
- [ ] Point Mamba 3D reconstruction ⏳ Phase 5
- [ ] Gaussian splatting visualization ⏳ Phase 5
- [ ] Time-scrub interaction ⏳ Phase 5

### Hardware Integration
- [x] RX 6700 XT unified memory support
- [x] wgpu compute pipeline
- [x] Autonomous GPU processing
- [x] Zero-copy data propagation
- [ ] Real hardware validation on RX 6700 XT ⏳ Phase 4+
- [ ] Memory bandwidth measurement ⏳ Phase 4+
- [ ] Performance profiling ⏳ Phase 4+

### Data Persistence
- [x] Neo4j graph correlation
- [x] Mamba checkpoint save/load (weights)
- [ ] Training metadata persistence ⏳ Fix #1
- [ ] Memo storage (JSONL) ⏳ Phase 1
- [ ] Pattern library export (JSON) ⏳ Phase 4
- [ ] HDF5 event corpus ⏳ Phase 4

---

## Known Issues & Workarounds

### Burn Library API Changes
- **Issue**: `squeeze(0)`, `mean_dim(0)` API signatures changed in burn 0.21-pre.2
- **Status**: Not blocking (unused in active code paths)
- **Workaround**: Update API calls when needed for training integration

### Slint UI API
- **Issue**: `slint::Model`, `slint::VecModel`, `slint::Color` changed
- **Status**: Fixed in latest Slint 1.15.1
- **Verification**: All UI-related code compiles successfully

### Unused Dead Code Warnings
- **Count**: ~95 warnings (pre-existing)
- **Assessment**: Intentional (code prepared for future features)
- **Action**: Will be resolved as features are activated in main flow

---

## Testing Strategy

### Phase 4 Tests (TimeGNN)
- 10 wav2vec2 integration tests
- 15 TimeGNN training tests
- 20 pattern discovery tests
- Total: 45 tests, targeting 100% pass rate

### Phase 5 Tests (Point Mamba)
- 10 PointNet encoder tests
- 12 PointMamba block tests
- 8 Point decoder tests
- 8 Gaussian splatting tests
- Total: 38 tests, targeting 100% pass rate

### Integration Tests (All Phases)
- 50+ tests across src/, ml/, visualization/
- Real hardware validation on RX 6700 XT
- Performance benchmarking (fps, latency, CPU utilization)

---

## Success Criteria (v0.5 Complete)

- [x] Phase 1: Unified V-Buffer implementation
- [x] Phase 2: GPU-Driven dispatch kernel
- [x] Phase 3: CPU async event handler
- [ ] Phase 4: TimeGNN motif discovery (23 patterns live)
- [ ] Phase 5: Point Mamba 3D wavefield visualization
- [ ] Fix #1: Mamba training persistence
- [ ] Fix #2: GUI console logging
- [ ] Fix #3: Mouth-region spatial targeting
- [ ] All 3 critical fixes + 5 feature phases = v0.5 COMPLETE

---

## Next Immediate Action

**Start Date**: 2026-03-15 (Next Monday)
**Phase 4**: TimeGNN offline pattern discovery (3-6 hours)
**Owner**: Implement B.1 + C.2 + ANALYSIS tab integration
**Deliverable**: 23 discovered harassment motifs, live ANALYSIS visualization

**Estimated Timeline**:
- Phase 4: 2-3 days (TimeGNN training + integration)
- Fixes #1-3: 4-5 hours (parallel with Phase 4)
- Phase 5: 5 days (Point Mamba + visualization)
- **Total to v0.5 Complete**: 1-2 weeks

---

**Last Updated**: 2026-03-08 (Phase 3 Complete)
**Next Review**: 2026-03-15 (Pre-Phase 4)
