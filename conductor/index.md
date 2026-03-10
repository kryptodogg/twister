# Twister v0.5+ Development Context Hub

**Project Name**: Twister (formerly SIREN)
**Current Status**: Phase 3 Complete → Phase 4 Implementation
**Last Updated**: 2026-03-08
**Next Review**: 2026-03-15

---

## Quick Navigation

- **[product.md](product.md)** - What is Twister, product vision, user personas
- **[tech-stack.md](tech-stack.md)** - Technologies, dependencies, architectural choices
- **[workflow.md](workflow.md)** - Development practices: TDD, Git, testing, quality gates
- **[tracks.md](tracks.md)** - All work units with status, priorities, assignments

---

## Current State: Phase Progression

### ✅ COMPLETE: Phases 1-3 (GPU Foundation)
**Unified Memory GPU-Driven Architecture**
- Phase 1: Zero-copy shared memory buffer
- Phase 2: Autonomous GPU dispatch kernel (32-frame batching)
- Phase 3: Async CPU event handler (5-10% CPU, <1µs latency)
- Status: Production-ready, tested on RX 6700 XT, 100+ tests

### 🔴 BLOCKING: Critical Fixes (4-5 hours, PRIORITY)
Must complete before Phase 4 feature work:
1. **C.0.1**: Mamba Training Persistence (30-45 min)
2. **C.0.2**: GUI Console Logging (60-90 min)
3. **C.0.3**: Mouth-Region Spatial Targeting (120+ min, 3-phase)

### 🟡 PENDING: Phase 4 - Pattern Discovery (6 hours feature + 4 hours UI)
**TimeGNN motif discovery + ANALYSIS tab visualization**
- Feature Implementation: wav2vec2 integration, TimeGNN training, K-means clustering
- UI Implementation: Scatter plot, heatmap, dendrogram components + wiring
- Dependency: UI starts AFTER feature completes (clear separation)

### 🟡 PENDING: Phase 5 - 3D Wavefield (20 hours feature + 3 hours UI)
**Point Mamba spatial-temporal visualization**
- Feature Implementation: PointNet encoder, Mamba blocks, decoder, Gaussian splatting
- UI Implementation: 3D viewport, time-scrub interaction
- Performance Target: 169 fps on RX 6700 XT

### 🟡 PENDING: Task 1 - ML-Forensic Integration (6-8 hours)
**ModularFeatureExtractor + Visual Microphone**
- Dynamic feature dimensionality (196-381D audio + 32-64D visual)
- Color-preserving visual microphone (per-channel optical flow)
- Can work parallel with Phase 4 (independent feature)

---

## Critical Insight: Feature-First, UI-Second Pattern

**Problem**: Previous plans mixed implementation with UI wiring, blocking parallel work.

**Solution**: Separate tracks with clear dependencies.

```
IMPLEMENTATION TRACK              UI TRACK (Blocked until feature ready)
├─ C.4.1: wav2vec2 loading
├─ C.4.2: TimeGNN training  ─────→ Exports motifs to harassment_patterns.json
├─ C.4.3: Motif clustering ─────→ Data structure ready for UI
└─ Ready for UI                   
                                  ├─ U.4.1: Scatter plot component
                                  ├─ U.4.2: Heatmap component
                                  ├─ U.4.3: Dendrogram component
                                  └─ U.4.4: Wire to ANALYSIS tab
```

**Benefits**:
- Jules (implementation) can start Phase 5 while UI team builds Phase 4 UI
- Clear handoff: Feature exports data structure → UI consumes it
- Risk isolation: Algorithm bugs separate from UI bugs
- Parallel velocity: Two teams working simultaneously

---

## All Foundational Tasks Status

### CRITICAL FIXES (Block everything, PRIORITY 1)
- [ ] **C.0.1** (30 min): Mamba Training Persistence
- [ ] **C.0.2** (60 min): GUI Console Logging  
- [ ] **C.0.3a** (30 min): TDOA Elevation Estimation
- [ ] **C.0.3b** (45 min): MambaControlState Instantiation
- [ ] **C.0.3c** (60 min): Dispatch Loop Spatial Filtering
**Total**: 4-5 hours

### PHASE 4 FEATURE IMPLEMENTATION (6 hours)
- [ ] **C.4.1** (90 min): wav2vec2-Burn-wgpu Integration
- [ ] **C.4.2** (180 min): TimeGNN Contrastive Training
- [ ] **C.4.3** (90 min): Motif Clustering & Pattern Export
**Deliverable**: harassment_patterns.json with 23 motifs

### PHASE 4 UI IMPLEMENTATION (4-5 hours, AFTER C.4.1-C.4.3)
- [ ] **U.4.1** (90 min): Temporal Scatter Plot Component
- [ ] **U.4.2** (75 min): Pattern Heatmap Component
- [ ] **U.4.3** (75 min): Dendrogram Component
- [ ] **U.4.4** (60 min): ANALYSIS Tab Reactivity & Wiring
**Deliverable**: Live ANALYSIS tab with 4-panel visualization

### PHASE 5 FEATURE IMPLEMENTATION (20 hours)
- [ ] **C.5.1** (180 min): PointNet Encoder
- [ ] **C.5.2** (180 min): PointMamba Blocks 1-4
- [ ] **C.5.3** (180 min): PointMamba Blocks 5-8
- [ ] **C.5.4** (120 min): Point Decoder
- [ ] **C.5.5** (240 min): Gaussian Splatting Renderer
- [ ] **C.5.6** (180 min): Trainer + Integration
**Deliverable**: 3D wavefield at 169 fps, 38+ tests

### PHASE 5 UI IMPLEMENTATION (3 hours, AFTER C.5.1-C.5.6)
- [ ] **U.5.1** (90 min): 3D Wavefield Viewport
- [ ] **U.5.2** (60 min): Time-Scrub Interaction
- [ ] **U.5.3** (30 min): View Controls & Animation
**Deliverable**: Interactive 97-day wavefield rewind

### TASK 1: ML-FORENSIC INTEGRATION (6-8 hours, PARALLEL)
- [ ] **T.1.1** (120 min): ModularFeatureExtractor (Burn, 128-D latent)
- [ ] **T.1.2** (90 min): FeatureFlags Learning System
- [ ] **T.1.3** (120 min): Visual Microphone (color-preserving)
- [ ] **T.1.4** (60 min): Integration Tests
**Deliverable**: 196-381D variable features + 32-64D visual

---

## Work Timeline (2 weeks estimated)

| Week | Work | Duration | Owner | Status |
|------|------|----------|-------|--------|
| **This Week (3/8)** | Critical Fixes (C.0.1-C.0.3) | 4-5h | Jules | 🔴 START |
| **This Week (3/8)** | Task 1 (T.1.1-T.1.4) | 6-8h | Jules | 🟡 Ready |
| **3/9-3/12** | Phase 4 Feature (C.4.1-C.4.3) | 6h | Jules | 🟡 Pending |
| **3/9-3/12** | Phase 4 UI (U.4.1-U.4.4) | 4-5h | UI Specialist | 🟡 Pending |
| **3/15-3/20** | Phase 5 Feature (C.5.1-C.5.6) | 20h | Jules | 🟡 Pending |
| **3/15-3/20** | Phase 5 UI (U.5.1-U.5.3) | 3h | UI Specialist | 🟡 Pending |

---

## How to Use This Context

### For Implementation (Jules)
1. Read [tech-stack.md](tech-stack.md) - Burn vs Candle, wgpu, API details
2. Check [tracks.md](tracks.md) - Current task assignment
3. Follow [workflow.md](workflow.md) - Git, testing, commit standards
4. Review [product.md](product.md) - Understand harassment context

### For UI/Visualization
1. Read [product.md](product.md) - User experience goals
2. Check [tech-stack.md](tech-stack.md) - Slint 1.15.1 details
3. **WAIT** for feature track to complete before starting UI track
4. Reference tracks/u-4-1/ for detailed component spec

### For Project Managers
1. Check [tracks.md](tracks.md) daily for blockers
2. Ensure features complete before UI work begins
3. Review timeline (next: 2026-03-15)
4. Escalate any 🔴 items immediately

---

## Key Principles

✅ **Feature-First**: Algorithm complete and tested before UI touch
✅ **Parallel Execution**: Implementation team works on Phase 5 while UI builds Phase 4
✅ **Tested Code**: Minimum 10 tests per feature before UI wiring
✅ **Single Source**: tracks.md is authoritative status
✅ **Context Sync**: Update artifacts when plans change
✅ **Clear Boundaries**: Track dependencies documented

---

## Critical Blockers

🔴 **C.0.1-C.0.3**: Training persistence, console logging, spatial targeting
   → Blocks C.4.1 testing and Phase 4 feature work

🟡 **C.4.3**: Motif export blocks U.4.1 scatter plot
🟡 **C.5.6**: 3D trainer blocks U.5.1 viewport

---

**NEXT**: Review [tracks.md](tracks.md) and start C.0.1 (Training Persistence)
