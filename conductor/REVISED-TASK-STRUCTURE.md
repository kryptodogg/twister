# Revised Task Structure: Signal-Centric Organization

**Insight**: "A signal is a signal whether it's light or sound." Metadata specifies sampling dimensions (temporal resolution, spatial scale, geotagging). Extract maximum information from each modality at its native scale. Fuse intelligently.

**Author**: User (architectural guidance)
**Date**: 2026-03-08

---

## Core Principle

Audio @ 192 kHz and Video @ 240 fps are **dual sensors of the same phenomenon** (vibration/EM radiation), just at different temporal-spatial scales:

- **Audio**: 192 kHz temporal resolution, 0-96 kHz frequency range, point spatial (mic location)
- **Video 240fps**: 4.2 ms temporal resolution, 0-120 Hz motion range, pixel-grid spatial
- **Video 30fps**: 33 ms temporal resolution, 0-15 Hz only, motion blur couples color channels
- **Depth camera**: 3D spatial information audio cannot measure

**Different temporal scales = Different extraction algorithms (not same algorithm downsampled)**

---

## Revised Parallel Task Structure

### **Parallel Track A: Audio Signal Extraction** (Independent, can start immediately)

**T.A.1: Audio Feature Encoder**
- Extract 196-381D features from 192 kHz audio
- Optimization: Exploit high temporal resolution
- Output: Raw audio features ready for multimodal fusion
- Timeline: 1-2 weeks
- Blocks: Nothing (self-contained)

### **Parallel Track B: Video Signal Extraction** (Independent, different per fps)

**T.B.1: 240fps Video Feature Encoder**
- High temporal resolution algorithm (0-120 Hz motion observable)
- Per-channel RGB optical flow preservation
- Cross-channel coherence metrics
- Output: 32-64D features optimized for slow-motion
- Timeline: 1-2 weeks

**T.B.2: 30fps Video Feature Encoder**
- Different algorithm (blur-tolerant, handles variable frame timing)
- Can only resolve 0-15 Hz motion
- Motion blur AS FEATURE (reveals motion speed)
- Output: 16-24D features (less information, coarser)
- Timeline: 1 week

**T.B.3: Depth Camera Feature Encoder**
- 3D scene flow (not 2D projection)
- Surface normal changes, deformation detection
- Output: 18-28D 3D spatial features
- Timeline: 1 week
- Blocks: Nothing (conditional, optional)

### **Blocking Gate: Temporal Alignment & Spectral Fusion** (After A+B)

**T.C.1: Temporal Alignment Network**
- Synchronize 192 kHz audio to 30-240 Hz video framerate
- Cross-correlation lag estimation
- Resample audio intelligently to video temporal scale
- Output: Time-aligned feature pairs

**T.C.2: Spectral Fusion Network**
- Combine 0-96 kHz audio domain with 0-120 Hz video domain
- These are DIFFERENT frequency ranges (not overlapping perfectly)
- Learn cross-modal coherence weights
- Output: 128-D latent space (agnostic to input modalities)

**Timing**: Only after T.A.1 + T.B.1/B.2/B.3 complete
**Blocks**: Everything downstream (TimeGNN, Point Mamba visualization)

### **Phase 4: TimeGNN Pattern Discovery** (After C.1+C.2)

Uses full multimodal features (audio + RF + wav2vec2 + video)

### **Phase 5: Point Mamba 3D** (After Phase 4)

3D spatial-temporal visualization

### **Phase 4 UI + Phase 5 UI** (Parallel, doesn't wait for feature completion)

UI team builds components while implementation team works on features

---

## Why This Reorganization Matters

**Old Structure**:
- Task 1 → Phase 4 → Phase 5 (sequential)
- Assumes Task 1 is "prerequisite" to Pattern Discovery
- Unclear dependencies
- UI blocked waiting for features

**New Structure**:
- Tracks A+B run parallel (audio + video encoders independent)
- Gate C (fusion) only blocks Phase 4 (logical dependency)
- Tracks A, B, C can run at different speeds (no artificial waits)
- UI team can build Phase 4 components while Jules finishes Track C
- **Jules has full freedom to get architecture right** (no time pressure)

---

## Timeline: Jules-Paced Implementation

**Week 1: Signal Extraction (Parallel A+B)**
- Monday-Friday: Audio encoder (Track A) + both video encoders (Tracks B.1, B.2)
- Can overlap work (3 independent code streams)
- Deliverable: Audio features extracted, Video features extracted (separately)

**Week 2: Alignment & Fusion (Gate C)**
- Temporal alignment network (cross-modal sync)
- Spectral fusion (combine 96 kHz + 120 Hz domains)
- Integration tests (verify alignment working)
- Deliverable: 128-D latent embeddings from multimodal input

**Week 2-3: Phase 4 Feature (TimeGNN)**
- Load multimodal corpus
- Train TimeGNN with contrastive loss
- Discover 23 harassment motifs
- Deliverable: harassment_patterns.json
- **Parallel**: UI team builds ANALYSIS tab components

**Week 3-4: Phase 5 Feature (Point Mamba)**
- PointNet encoder → PointMamba blocks → decoder
- Gaussian splatting renderer
- Trainer integration
- Deliverable: 3D wavefield at 169 fps
- **Parallel**: UI team builds 3D viewport + time-scrub

**Critical Fixes** (Anytime, lowest priority):
- C.0.1: Training persistence (30 min)
- C.0.2: GUI console (60 min)
- C.0.3: Mouth-region targeting (120 min)
- Can happen Week 1 morning or interleaved

---

## Remote vs Local Verification

**Can work remote** (test on CPU/sample data):
- Audio feature extraction (test on 1000 samples)
- Video feature extraction (test on 10 video clips)
- Temporal alignment (cross-correlation logic)
- Spectral fusion (loss computation)
- Burn tensor operations (test on NdArray backend)

**Verify locally on RX 6700 XT** (GPU-specific):
- Point Mamba inference (> 160 fps requirement)
- Gaussian splatting shader (169 fps target)
- GPU memory (12GB unified memory budget)
- Real-time performance (if <10ms is critical)

**Result**: 80% remote work, 20% local RX 6700 XT verification for GPU components

---

## Feature-First, UI-Second: Updated Dependencies

```
Track A (Audio)     Track B (Video 240fps)     Track B (Video 30fps)     Track B (Depth)
     └─────────────────────┬──────────────────────────┬──────────────────────┘
                           │
                    Gate C: Temporal Alignment + Fusion
                           │
              ┌────────────┴─────────────────┐
              │                              │
      Phase 4: TimeGNN          Phase 4 UI (Parallel)
      └─────────┬────────────────────────────────────┐
                │                                    │
        Phase 4 Motifs                      ANALYSIS Tab Live
        (23 discovered)                    (Scatter, Heatmap, Dendrogram)
                │
         Phase 5: Point Mamba      Phase 5 UI (Parallel)
                │                  (3D Viewport, Time-Scrub)
                │
       3D Wavefield Ready
       (169 fps visualization)
```

Clear blocking gates, parallel work streams.

---

## Architecture Principle (Summarized)

"Different sample rates = Different physics. 240 fps video has different information than 30 fps video, not just 'better resolution.' Extract maximum information at each modality's native temporal scale. Fuse in 128-D latent space where they meet."

---

## Updated Conductor Artifacts

- **index.md**: Updated to reflect signal-centric organization
- **product.md**: No change (still focuses on harassment detection)
- **tech-stack.md**: No change (still Burn, wgpu, Slint)
- **workflow.md**: No change (still TDD, Git practices)
- **tracks.md**: REBUILD with A+B+C gate structure (in progress)
- **SIGNAL-ARCHITECTURE.md**: This document (new, explains physics + algorithms)

---

## For Jules: Implications

1. **You have full freedom on timing.** Tracks A, B, C have clear endpoints. No artificial rush.
2. **Architecture matters more than speed.** Get temporal alignment + spectral fusion RIGHT.
3. **Different fps = different algorithms.** Don't try to make 30fps look like 240fps.
4. **Test locally only for GPU.** CPU testing for feature extraction is fine.
5. **UI doesn't block you.** While you build Phase 4/5, UI team builds UI independently.

Take the time. Get it right. "A signal is a signal."

---

**Next**: Rebuild tracks.md with Tracks A+B+C gate structure
