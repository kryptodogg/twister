# Stub-to-Implementation Cascade Pattern

**Principle**: Stubs are **temporary**. By the time consuming tracks finish, stubs must be replaced with real implementations.

---

## Timeline: Stub Lifecycle

### Week 1: Stubs Deployed (Wave 1 Ships)

```
D.4 Temporal Rewind UI
├─ Imports: SpatialPoint interface (from D.1 module)
├─ Stubs with: StubbedSpatialEstimator
└─ Tests pass: ✅ "Renders 100 synthetic RF points"

I.2 Pose Materials
├─ Imports: PoseFrame interface (from I.1 module)
├─ Stubs with: StubbedPoseEstimator
└─ Tests pass: ✅ "Converts synthetic poses to materials"
```

### Week 2: Real Implementations Arrive (Wave 1 Mid-Development)

```
D.1 TDOA Elevation Estimator
├─ Implements: SpatialPoint interface
├─ Real elevation computation
└─ Integration: D.4 swaps stub for D.1 output
   BEFORE: synthetic 45° azimuth, +30° elevation (stub)
   AFTER: real azimuth/elevation from 4-device array

I.1 MediaPipe GPU Integration
├─ Implements: PoseFrame interface
├─ Real 33-point skeleton from camera
└─ Integration: I.2 swaps stub for I.1 output
   BEFORE: synthetic "standing" pose (stub)
   AFTER: real human skeleton tracking
```

**Critical**: By end of Week 2, all stubs replaced with real implementations.

### Week 3: Integration Complete

```
D.4 + D.1 ✅ Integrated
├─ D.4 no longer uses stub
├─ Receives real SpatialPoints from D.1
└─ Tests pass with real data

I.2 + I.1 ✅ Integrated
├─ I.2 no longer uses stub
├─ Receives real PoseFrames from I.1
└─ Tests pass with real skeleton data
```

---

## What if Real Implementation Doesn't Arrive?

### Scenario: D.1 Implementation Delayed

**Week 2 Status**:
- ❌ D.1 TDOA Elevation not ready (implementation delayed)
- ✅ D.4 Temporal Rewind still working with stub
- ❌ But D.4 tests only valid with stubs (can't verify real integration)

**Action**: Create **Follow-Up Track AA** (not an addendum)

```
Track AA: D.1 → D.4 Integration Completion
├─ Owner: Different engineer (not D.1 or D.4)
├─ Task: Implement D.1, swap stub in D.4, verify integration tests
├─ Duration: 1-2 days
└─ Status: SCHEDULED POST-D.1 COMPLETION
```

**Why separate track, not addendum?**
- Addendum = small fix to existing track (before merge)
- Follow-up track = new work that depends on external completion
- Avoids blocking original developers; can assign to anyone

### Scenario: I.1 MediaPipe Implementation Delayed

**Week 2 Status**:
- ❌ I.1 MediaPipe not ready (GPU integration complex)
- ✅ I.2 Pose Materials still working with stub
- ❌ I.2 tests only valid with stubs

**Action**: Create **Follow-Up Track BB**

```
Track BB: I.1 → I.2 Integration Completion
├─ Owner: Different engineer (or CV engineer after I.1 done)
├─ Task: Complete I.1 MediaPipe, swap stub in I.2, verify integration tests
├─ Duration: 1-2 days
└─ Status: SCHEDULED POST-I.1 COMPLETION
```

---

## Follow-Up Track Naming Convention

### Primary Tracks (Existing)
- A, B, C, D, E (first wave)
- I (pose estimation)
- Particle System (infrastructure)
- VI, H, G (final wave)

### Follow-Up Tracks (If Needed)

**Replacement Tracks** (stubs → real implementations):
- **AA**: D.1 → D.4 integration (if D.1 delayed)
- **BB**: I.1 → I.2 integration (if I.1 delayed)
- **CC**: D.2 → D.3 integration (if sequential bottleneck)
- **DD**: I.3 → I.4 integration (if sequential bottleneck)

**Enhancement Tracks** (post-completion):
- **EE**: Track E enrichment (semantic layer if Cognee added)
- **FF**: Track VI performance optimization
- **GG**: Track H haptic pattern tuning

**New Features** (discovered during development):
- **HH**, **II**, **JJ**, etc. (as needed)

### Naming Rule
- Double letters (AA, BB, CC) for **follow-up/completion** work
- Sequential (HH, II, JJ) for **new features** discovered

---

## Stub-to-Implementation Verification Checklist

### When D.1 Implementation Arrives

**Verification Steps**:
```
☐ D.1 exports SpatialPoint struct (matches interface)
☐ D.1 SpatialPoint fields match stub expectations
  - azimuth_rad ∈ [-π, π]
  - elevation_rad ∈ [-π/2, π/2]
  - confidence ∈ [0, 1]
  - timestamps microsecond-precision
☐ D.1 tests pass (elevation estimation works)
☐ D.4 can import D.1 implementation
☐ D.4 replaces StubbedSpatialEstimator with real ElevationEstimator
☐ D.4 tests still pass (now with real data)
☐ D.4 + D.1 integration tests created and passing
```

### When I.1 Implementation Arrives

**Verification Steps**:
```
☐ I.1 exports PoseFrame struct (matches interface)
☐ I.1 PoseFrame fields match stub expectations
  - keypoints: [PoseKeypoint; 33]
  - keypoints[i].confidence ∈ [0, 1]
  - timestamps microsecond-precision
☐ I.1 tests pass (MediaPipe inference works)
☐ I.1 achieves < 50ms inference latency per frame
☐ I.2 can import I.1 implementation
☐ I.2 replaces StubbedPoseEstimator with real PoseEstimator
☐ I.2 tests still pass (now with real poses)
☐ I.2 + I.1 integration tests created and passing
```

---

## Decision Tree: Stub or Follow-Up Track?

```
Is implementation delayed?
├─ NO → Proceed normally (no follow-up track needed)
└─ YES → Can consumer proceed with stub?
    ├─ YES (test quality acceptable with stub) → Create Follow-Up Track (AA, BB)
    │   └─ Schedule after implementation arrives
    └─ NO (consumer blocked, can't verify) → Escalate (not typical)
```

---

## Example: D.1 → D.4 If Delayed

### Timeline: D.1 Implementation Delayed by 3 Days

**Week 1, Day 5 (Friday)**:
- D.1 engineer: "TDOA elevation computation 80% done, need 3 more days"
- Status: D.1 will arrive Monday of Week 2

**Week 2, Monday Morning**:
- D.4 engineer: "I've finished temporal rewind UI with stubs, tests passing"
- Manager: "Perfect. You're unblocked. Create Track AA for integration once D.1 arrives"
- D.4 engineer: "Submitting D.4 for review; will integrate D.1 output as Track AA"

**Week 2, Wednesday**:
- D.1 engineer: "TDOA elevation done, exported SpatialPoint interface"
- D.4 engineer: "Creating Track AA to swap stub for real D.1 output"

**Track AA Tasks**:
```
1. Import D.1 ElevationEstimator (5 min)
2. Remove StubbedSpatialEstimator from D.4 (5 min)
3. Wire D.1 instance into dispatch loop (15 min)
4. Verify D.4 tests still pass (20 min)
5. Create D.1 + D.4 integration tests (20 min)
→ Total: 1-2 hours (1/4 day)
```

**Result**:
- D.4 unblocked for Week 1
- No bottleneck when D.1 arrives
- Clean separation: D.4 development vs D.1→D.4 integration
- Anyone (including D.1 engineer) can do Track AA

---

## When to Convert Stub to Implementation

### Immediate Integration (Same Day)
```
D.1 ready by 5pm Monday
D.4 engineer available to integrate
→ Integrate immediately, no Track AA needed
```

### Delayed Integration (Track AA)
```
D.1 ready Friday (D.4 engineer on vacation)
Different engineer available Monday
→ Create Track AA, assign to available engineer
```

### Critical Path Integration
```
D.1 ready but D.4 already integrated with stub
Tests already passing with stub data
→ Still create Track AA (swap stub for real data)
→ Verify no regression with real implementation
```

---

## Benefits of This Pattern

### For D.4 Engineer
- ✅ Unblocked (doesn't wait for D.1)
- ✅ Ships real product (D.4 stub) on time
- ✅ Can test and optimize independently

### For Manager
- ✅ D.4 delivery guaranteed (stub works)
- ✅ No team blocking (interface isolates teams)
- ✅ Follow-up track (AA) is straightforward integration work

### For Project
- ✅ Features ship incrementally (D.4 works even with stub)
- ✅ Real implementations plug in cleanly (interface contract)
- ✅ No cascading delays (D.1 lateness doesn't block anything)

---

## Critical: Stub Quality

**For stub to actually unblock work, it must be high-quality:**

### Bad Stub (Won't Unblock)
```rust
// ❌ This doesn't unblock D.4
pub fn stub_spatial_point() -> SpatialPoint {
    SpatialPoint::default()  // All zeros
}
// D.4 can't test anything meaningful with zeros
```

### Good Stub (Unblocks D.4)
```rust
// ✅ This unblocks D.4
pub fn stub_spatial_point() -> SpatialPoint {
    SpatialPoint {
        azimuth_rad: std::f32::consts::PI / 4.0,      // 45°
        elevation_rad: std::f32::consts::PI / 6.0,     // 30°
        frequency_hz: 2.4e9,                           // 2.4 GHz
        intensity: 0.8,
        timestamp_us: current_timestamp_us(),
        confidence: 0.9,
    }
}
// D.4 can fully test temporal rewind with synthetic points
// Tests verify: rendering, time-scrubbing, persistence
```

---

## Pre-Implementation Checklist

Before Wave 1 ships, verify stubs are high-quality:

```
D.4 Stub:
☐ StubbedSpatialEstimator returns realistic synthetic SpatialPoints
☐ Varies azimuth/elevation (not same value every time)
☐ Includes confidence scores
☐ D.4 tests verify rendering, interaction, temporal logic
☐ D.4 can ship with stub (users see working temporal rewind)

I.2 Stub:
☐ StubbedPoseEstimator returns realistic 33-point skeletons
☐ Simulates realistic pose variations (standing, arm raised, etc.)
☐ Includes confidence per keypoint
☐ I.2 tests verify material assignment, motion modulation
☐ I.2 can ship with stub (users see working pose materials)
```

---

## Summary: Stub Lifecycle

1. **Week 1**: Stubs deployed, unblock consumers, tests passing
2. **Week 2**: Real implementations arrive, schedule Track AA/BB for integration
3. **Day 1 of Track AA**: Swap stub for real implementation (1-2 hours)
4. **Integration tests**: Verify real data works with consumer
5. **Ship**: Consumer now uses real implementation, stub removed

**Net result**: No blockers, incremental delivery, clean integration.

