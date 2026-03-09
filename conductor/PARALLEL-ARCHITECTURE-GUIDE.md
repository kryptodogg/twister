# Parallel Architecture Guide: Removing Blockers

**Principle**: If Track X depends on Track Y output, they are NOT parallel. They must either:
1. Be split into independent parallel sub-tasks
2. Use interface contracts (stubs) so Y can proceed without X

---

## Current Problem: Track Dependencies

### ❌ PROBLEM: Track D has blockers

```
D.1 (TDOA Elevation) outputs → point cloud format
  ↓ (blocks)
D.4 (Temporal Rewind UI) needs point cloud format
```

**Result**: D.1 must finish before D.4 can meaningfully start → NOT parallel

### ❌ PROBLEM: Track I has blockers

```
I.1 (MediaPipe) outputs → skeleton [x, y, z, confidence]
  ↓ (blocks)
I.2 (Pose Materials) needs skeleton format
  ↓ (blocks)
I.3 (IMU Fusion) needs both
  ↓ (blocks)
I.4 (PointMamba) needs learned materials
```

**Result**: All sequential → NOT parallel

---

## SOLUTION: Interface Contracts (Stubs)

Each track defines the **input/output interface** at the START. Engineers can work in parallel by:
1. **Producer** (D.1) defines what it outputs: `struct SpatialPoint { azimuth, elevation, ... }`
2. **Consumer** (D.4) imports the interface and codes to that contract
3. **D.4 stubs the input** (hardcoded synthetic points) and proceeds independently
4. **Later**: D.1 implementation plugs in, replaces stub, tests pass

---

## Restructured Parallel Tasks

### ✅ WAVE 1: Truly Parallel (No Blockers)

```
Track A (Signal Ingestion)
├─ Outputs: EventNode stream
└─ No dependencies ✓

Track B (LFM2.5 Training)
├─ Inputs: EventNode (from A)
├─ Outputs: PatternNode
└─ Blocks: None (reads from A)

Track C (Audio Processing)
├─ Inputs: Raw audio (from A)
├─ Outputs: SpectralFrame
└─ Blocks: None (reads from A)

Track D.1 (TDOA Elevation)
├─ Inputs: Existing TDOA azimuth
├─ Outputs: SpatialPoint {azimuth, elevation, ...}
├─ Interface: Define `pub struct SpatialPoint` at module level
└─ Blocks: None (independent of D.4) ✓

Track D.4 (Temporal Rewind UI)
├─ Inputs: SpatialPoint (interface contract)
├─ Outputs: 3D visualization
├─ Stub input: Hardcode synthetic SpatialPoints for testing
├─ Interface: Import `SpatialPoint` from D.1 module (stub implementation provided)
└─ Blocks: None (works with stub, replaces with D.1 output later) ✓

Track I.1 (MediaPipe GPU)
├─ Inputs: Camera frames
├─ Outputs: PoseFrame {keypoints: [PoseKeypoint; 33]}
├─ Interface: Define `pub struct PoseFrame` at module level
└─ Blocks: None (independent of I.2) ✓

Track I.2 (Pose Materials)
├─ Inputs: PoseFrame (interface contract)
├─ Outputs: PointCloudWithMaterials
├─ Stub input: Hardcode synthetic PoseFrames for testing
├─ Interface: Import `PoseFrame` from I.1 module (stub provided)
└─ Blocks: None (works with stub, replaces with I.1 output later) ✓

Track Particle System
├─ Inputs: None (self-contained infrastructure)
├─ Outputs: ParticleSystem trait + implementations
└─ Blocks: None (pulled in by D.4, I.5, VI) ✓

Track E (Knowledge Graph)
├─ Inputs: Forensic logs (from A-D)
├─ Outputs: Graph queries
├─ Blocks: None (reads from logs, doesn't block log writes) ✓
```

**Total parallel tracks for Wave 1**: 9 truly independent teams
**Timeline**: All can work simultaneously; no waiting

---

## Implementation Pattern: Interface + Stub

### Example: Track D (TDOA Elevation + Temporal Rewind)

**Step 1: D.1 Defines Interface** (top of src/spatial/mod.rs)

```rust
// This interface is DEFINED NOW but IMPLEMENTED LATER
// D.1 implements; D.4 imports and stubs

pub struct SpatialPoint {
    pub azimuth_rad: f32,
    pub elevation_rad: f32,
    pub frequency_hz: f32,
    pub intensity: f32,
    pub timestamp_us: u64,
    pub confidence: f32,
}

pub trait SpatialEstimator {
    fn estimate(&mut self, device_amplitudes: &[f32]) -> SpatialPoint;
}
```

**Step 2: D.1 Engineer Implements**

```rust
// src/spatial/elevation_estimator.rs
pub struct ElevationEstimator { /* ... */ }

impl SpatialEstimator for ElevationEstimator {
    fn estimate(&mut self, device_amplitudes: &[f32]) -> SpatialPoint {
        // Compute elevation from energy ratio
        // ...
        SpatialPoint {
            azimuth_rad: azimuth,
            elevation_rad: elevation,
            /* ... */
        }
    }
}
```

**Step 3: D.4 Engineer Stubs the Interface**

```rust
// src/visualization/temporal_rewind_state.rs
use crate::spatial::{SpatialPoint, SpatialEstimator};

// Stub implementation (works immediately, replaced later)
pub struct StubbedSpatialEstimator;

impl SpatialEstimator for StubbedSpatialEstimator {
    fn estimate(&mut self, _device_amplitudes: &[f32]) -> SpatialPoint {
        // Return hardcoded synthetic points for testing
        SpatialPoint {
            azimuth_rad: std::f32::consts::PI / 4.0,  // 45 degrees
            elevation_rad: std::f32::consts::PI / 6.0, // 30 degrees
            frequency_hz: 2.4e9,
            intensity: 0.8,
            timestamp_us: current_timestamp(),
            confidence: 0.9,
        }
    }
}

pub struct TemporalRewindState {
    spatial_estimator: Box<dyn SpatialEstimator>,  // Can be stub or real
    // ...
}

impl TemporalRewindState {
    pub fn new_with_stub() -> Self {
        Self {
            spatial_estimator: Box::new(StubbedSpatialEstimator),
            // ...
        }
    }
}
```

**Step 4: D.4 Tests with Stub**

```bash
# D.4 engineer tests independently
cargo test temporal_rewind_state --lib -- --nocapture
# Expected: Tests pass with stubbed points
# Output: "Temporal rewind UI renders 100 synthetic points at varying azimuths"
```

**Step 5: Later, D.1 & D.4 Integration**

```rust
// src/main.rs dispatch loop - LATER, once D.1 is done
let elevation_estimator = ElevationEstimator::new();
let mut rewind_state = TemporalRewindState::new();
rewind_state.spatial_estimator = Box::new(elevation_estimator);  // Plug in real D.1

// D.4 tests still pass; just with real data now instead of stubs
```

---

## Revised Track Structure for Parallel Work

### ✅ Wave 1: Send Now (All Parallel, No Blockers)

```
1. Track A (Signal Ingestion)
2. Track B (LFM2.5 Training)
3. Track C (Audio Processing)
4. Track D.1 (TDOA Elevation - DEFINE INTERFACE)
5. Track D.4 (Temporal Rewind - USE STUB)
6. Track I.1 (MediaPipe - DEFINE INTERFACE)
7. Track I.2 (Pose Materials - USE STUB)
8. Track: Particle System Infrastructure
9. Track E (Knowledge Graph)
```

**Interface Contracts to Define (done now)**:
- `SpatialPoint` (D.1 outputs, D.4 consumes)
- `PoseFrame` (I.1 outputs, I.2 consumes)
- `PointCloudWithMaterials` (I.2 outputs, I.4 consumes)
- `ParticleSystem` trait (Particle System, used by D.4/I.5/VI)

**Stub Implementations to Provide** (D.4, I.2 engineers can start day 1):
- `StubbedSpatialEstimator` (returns hardcoded SpatialPoints)
- `StubbedPoseEstimator` (returns hardcoded PoseFrames)

---

### ⏳ Wave 2: Sequential Blocks (After Wave 1)

```
D.2 + D.3 (PointMamba Encoder → Decoder)
  Dependency: D.1 interface stabilized (reads SpatialPoint format)
  Start: After D.1 interface review confirmed

I.3 + I.4 (IMU Fusion → PointMamba Learning)
  Dependency: I.1 interface stabilized (reads PoseFrame format)
  Start: After I.1 interface review confirmed
```

---

### ❌ Wave 3: Blocked (Wait for Wave 2)

```
Track VI (Aether Visualization)
  Depends: D.3, D.4, I.2, Particle System (all Wave 1/2)

Track H (Haptic Feedback)
  Depends: D, I outputs (Wave 2)

Track G (Dorothy Orchestrator)
  Depends: Widget Framework understanding (read first), E interface
```

---

## Critical Success Factor: Interface Definition

**Before Wave 1 engineers start**, must have:

1. ✅ `SpatialPoint` interface defined (in D.1 module)
2. ✅ `PoseFrame` interface defined (in I.1 module)
3. ✅ `PointCloudWithMaterials` interface defined (in I.2 module)
4. ✅ `ParticleSystem` interface defined (in Particle System module)
5. ✅ `PatternNode` interface defined (in B module)
6. ✅ `SpectralFrame` interface defined (in C module)
7. ✅ Stub implementations provided for consumers

**Then**: Each engineer implements independently, tests with stubs, integrates later.

---

## Verification: No Blockers

**For each Wave 1 track, verify**:
- ❓ Does this track DEPEND on another Wave 1 track?
  - If YES: Use interface contract + stub → NOT blocked
  - If NO: Truly independent → ✅ Parallel

**D.1 vs D.4 check**:
- D.4 needs SpatialPoint? YES
- Does D.4 use interface contract? YES
- Does D.4 provide stub? YES
- Result: NOT blocked ✅

**I.1 vs I.2 check**:
- I.2 needs PoseFrame? YES
- Does I.2 use interface contract? YES
- Does I.2 provide stub? YES
- Result: NOT blocked ✅

---

## Summary: Parallel Architecture

| Track | Interface | Stub | Dependency | Parallel? |
|-------|-----------|------|-----------|-----------|
| A | - | - | None | ✅ |
| B | PatternNode | - | A (read) | ✅ |
| C | SpectralFrame | - | A (read) | ✅ |
| D.1 | SpatialPoint | - | None | ✅ |
| D.4 | - | StubbedSpatialPoint | D.1 (interface) | ✅ |
| I.1 | PoseFrame | - | None | ✅ |
| I.2 | PointCloudWithMaterials | StubbedPoseFrame | I.1 (interface) | ✅ |
| Particles | ParticleSystem | - | None | ✅ |
| E | - | - | A-D (read logs) | ✅ |

**All Wave 1 tracks are parallel** ✓ No blockers

---

## Action Items Before Wave 1 Ships

1. ✅ Define interface contracts in each module (D.1, I.1, B, C, Particles)
2. ✅ Provide stub implementations (D.4, I.2 engineers can start day 1)
3. ✅ Document expected data shapes (for engineers to code against)
4. ✅ Create sample/test data (show what real SpatialPoint looks like)
5. ✅ Verify: Zero cross-track blocking dependencies

