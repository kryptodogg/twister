# Track I: Pose Estimation & Material Correlation

**Domain**: Computer Vision + Machine Learning (GPU-accelerated pose + IMU fusion)
**Ownership**: Computer Vision Engineer (I.1) + ML Engineer (I.2, I.4) + Shared (I.3, I.5)
**Duration**: 5-7 days (parallel: CV and ML teams work independently on I.1 & I.2 simultaneously)
**Blocker on**: None (can start immediately; feeds Tracks VI & H)

---

## Overview

**Vision**: Transform invisible RF fields into **human-pose-aware materials** using privacy-preserving computer vision.

Traditional RF detection (Tracks A-D) answers: "Where is the attack coming from?" (spatial + frequency)

**Track I answers**: "How does the attack *respond to me*?" by correlating:
- **Camera pose** (33-point skeleton @ 30fps via MediaPipe/RTMPose)
- **Phone IMU** (accelerometer + gyroscope, already available in Track A)
- **PointMamba learning** (discover attack patterns that change based on human presence/motion)

**Privacy guarantee**: No video stored. Only:
- Pose keypoints (x, y, z, confidence for 33 joints)
- Derived materials (hardness, roughness, wetness based on body region)
- Mamba embeddings (learned correlations)

Even forensic investigators cannot reconstruct room layout—only RF wavefield's *interaction* with human presence.

**Why this matters**:
- **IMU+Vision fusion** provides spatial positioning that TDOA alone cannot achieve
- **Pose-dependent RF behavior** reveals attack sophistication (targeting specific body regions adaptively)
- **Material properties from pose** enable Niagara-style particle visualization where particles interact with human skeleton
- **PointMamba discovers patterns** like: "When user's arm raises, RF field shifts to mouth region" (active targeting)

---

## Track I.1: MediaPipe/RTMPose GPU Integration (2 days)

**Deliverables**:
- `src/computer_vision/pose_estimator.rs` (400 lines) — GPU-accelerated pose detection
- `src/computer_vision/mediapipe_wrapper.rs` (300 lines) — HuggingFace MediaPipe binding
- `examples/pose_demo.rs` (200 lines) — Real-time pose visualization
- `tests/pose_integration.rs` (250 lines, 15 tests)

**Ownership**: Computer Vision Engineer (exclusive)

**Key work**:

### Pose Estimator Architecture

```rust
// src/computer_vision/pose_estimator.rs

use burn::prelude::*;
use hf_hub::api::sync::Api;

/// 33-point pose from MediaPipe BlazePose
#[derive(Debug, Clone)]
pub struct PoseKeypoint {
    pub x: f32,                     // Normalized (0-1) image width
    pub y: f32,                     // Normalized (0-1) image height
    pub z: f32,                     // Depth (relative to hip)
    pub confidence: f32,            // 0.0-1.0
}

#[derive(Debug, Clone)]
pub struct PoseFrame {
    pub timestamp_us: u64,
    pub keypoints: [PoseKeypoint; 33],  // 33-point BlazePose
    pub visibility: [f32; 33],          // Visibility score per joint
}

pub struct PoseEstimator<B: Backend> {
    model: PoseModel<B>,
    input_size: (u32, u32),     // (width, height)
}

impl<B: Backend> PoseEstimator<B> {
    /// Load MediaPipe BlazePose from HuggingFace
    pub fn new(device: &B::Device) -> Result<Self, Box<dyn Error>> {
        let api = Api::new()?;
        let repo = api.model("mediapipe/blazepose-full".to_string());

        // Download model weights (if not cached)
        let model = PoseModel::load(&repo, device)?;

        eprintln!("[PoseEstimator] Loaded MediaPipe BlazePose");
        eprintln!("[PoseEstimator] Input size: 256×256, Output: 33 keypoints");

        Ok(Self {
            model,
            input_size: (256, 256),
        })
    }

    /// Infer pose from camera frame
    pub async fn estimate_pose(
        &self,
        frame: &[u8],           // RGB image data
        frame_width: u32,
        frame_height: u32,
        device: &B::Device,
    ) -> Result<PoseFrame, Box<dyn Error>> {
        // 1. Preprocess: Resize frame to 256×256, normalize to [-1, 1]
        let input_tensor = preprocess_frame(frame, frame_width, frame_height, device)?;

        // 2. Forward pass
        let output = self.model.forward(input_tensor);

        // 3. Postprocess: Extract 33 keypoints + visibility
        let pose = postprocess_output(&output, frame_width, frame_height)?;

        Ok(pose)
    }
}

fn preprocess_frame<B: Backend>(
    frame: &[u8],
    width: u32,
    height: u32,
    device: &B::Device,
) -> Result<Tensor<B, 4>, Box<dyn Error>> {
    // Resize RGB frame to 256×256
    // Normalize to [-1, 1]
    // Return as (1, 3, 256, 256) tensor
    unimplemented!("Image preprocessing with burn")
}

fn postprocess_output(
    output: &Tensor<impl Backend, 3>,
    frame_width: u32,
    frame_height: u32,
) -> Result<PoseFrame, Box<dyn Error>> {
    // Parse output: [33 keypoints × (x, y, z)] + [33 visibility scores]
    // Denormalize coordinates back to frame dimensions

    let keypoints = [PoseKeypoint {
        x: 0.0, y: 0.0, z: 0.0, confidence: 0.9
    }; 33];

    let visibility = [0.9; 33];

    Ok(PoseFrame {
        timestamp_us: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_micros() as u64,
        keypoints,
        visibility,
    })
}
```

### 33-Point Skeleton Definition

```rust
/// MediaPipe BlazePose 33-point skeleton topology
pub struct SkeletonTopology;

impl SkeletonTopology {
    pub const NOSE: usize = 0;
    pub const LEFT_EYE_INNER: usize = 1;
    pub const LEFT_EYE: usize = 2;
    pub const LEFT_EYE_OUTER: usize = 3;
    pub const RIGHT_EYE_INNER: usize = 4;
    pub const RIGHT_EYE: usize = 5;
    pub const RIGHT_EYE_OUTER: usize = 6;

    // ... (28 more points: ears, mouth, shoulders, elbows, wrists, hips, knees, ankles)

    pub const LEFT_ANKLE: usize = 27;
    pub const RIGHT_ANKLE: usize = 32;

    /// All connections: (from, to) for skeleton rendering
    pub fn connections() -> Vec<(usize, usize)> {
        vec![
            (0, 1), (1, 2), (2, 3),           // Right eye
            (0, 4), (4, 5), (5, 6),           // Left eye
            (9, 10), (10, 12),                // Left shoulder → elbow → wrist
            (13, 14), (14, 16),               // Right shoulder → elbow → wrist
            (11, 23), (12, 24),               // Shoulders → hips
            (23, 25), (25, 27),               // Left hip → knee → ankle
            (24, 26), (26, 28),               // Right hip → knee → ankle
            // ... more connections
        ]
    }
}
```

### Performance Target

```
RTMPose (GPU-accelerated variant):
- Input: 640×480 camera frame @ 30 fps
- Output: 33 keypoints @ 30 fps latency < 33ms
- Memory: < 500MB GPU VRAM
- Accuracy: 95%+ confidence on typical indoor environments
```

---

## Track I.2: Pose → Point Cloud Materials (1.5 days)

**Deliverables**:
- `src/ml/pose_materials.rs` (350 lines) — Map pose skeleton to 3D points + RF-BSDF materials
- `src/ml/body_region_classifier.rs` (200 lines) — Classify which body region is targeted
- `tests/pose_materials_integration.rs` (200 lines, 12 tests)

**Ownership**: ML Engineer (exclusive)

**Key work**:

### Pose to Point Cloud + Materials

```rust
// src/ml/pose_materials.rs

pub struct PointWithMaterial {
    pub point: Point3D,           // 3D coordinate in room space
    pub material: MaterialProps,  // RF-BSDF hardness/roughness/wetness
    pub body_region: BodyRegion, // Which part of body
    pub confidence: f32,
}

pub enum BodyRegion {
    Head,
    Mouth,
    LeftEye,
    RightEye,
    Chest,
    LeftArm,
    RightArm,
    LeftHand,
    RightHand,
    Abdomen,
    LeftLeg,
    RightLeg,
    LeftFoot,
    RightFoot,
}

pub fn pose_to_material_points(
    pose: &PoseFrame,
    previous_pose: Option<&PoseFrame>,
) -> Vec<PointWithMaterial> {
    let mut materials = Vec::new();

    for (joint_idx, keypoint) in pose.keypoints.iter().enumerate() {
        if keypoint.confidence < 0.3 {
            continue;  // Skip low-confidence joints
        }

        // 1. Determine body region
        let region = classify_body_region(joint_idx);

        // 2. Compute material properties
        let material = compute_material_from_pose(
            keypoint,
            region,
            previous_pose,
            pose.keypoints,
        );

        // 3. Convert normalized pose to 3D room coordinates
        // Assume camera is at origin, depth from MediaPipe z coordinate
        let point = Point3D {
            x: keypoint.x * 5.0,          // Scale to ~5m room width
            y: (1.0 - keypoint.y) * 3.0,  // Flip Y (image coords), scale to ~3m height
            z: keypoint.z * 2.0,           // Depth ~2m from camera
        };

        materials.push(PointWithMaterial {
            point,
            material,
            body_region: region,
            confidence: keypoint.confidence,
        });
    }

    materials
}

pub fn compute_material_from_pose(
    keypoint: &PoseKeypoint,
    region: BodyRegion,
    previous_pose: Option<&PoseFrame>,
    all_keypoints: [PoseKeypoint; 33],
) -> MaterialProps {
    // Material properties based on:
    // 1. Body region (mouth = softer/wetter, limbs = harder)
    // 2. Motion velocity (moving joints = rougher)
    // 3. Pose stability (shaking = noisier material)

    let (base_hardness, base_roughness, base_wetness) = match region {
        BodyRegion::Mouth => (0.3, 0.4, 0.8),   // Soft, wet (biological)
        BodyRegion::Head => (0.5, 0.3, 0.2),    // Medium, dry
        BodyRegion::Chest => (0.4, 0.2, 0.3),   // Soft, slightly moist
        BodyRegion::LeftArm | BodyRegion::RightArm => (0.6, 0.4, 0.1),  // Hard, dry
        BodyRegion::LeftHand | BodyRegion::RightHand => (0.7, 0.5, 0.2), // Hard hands
        BodyRegion::LeftLeg | BodyRegion::RightLeg => (0.5, 0.3, 0.1),   // Medium
        BodyRegion::LeftFoot | BodyRegion::RightFoot => (0.8, 0.6, 0.0), // Hard feet
        _ => (0.5, 0.3, 0.2),
    };

    // Motion adjustment: faster motion → rougher (noisier)
    let motion_velocity = if let Some(prev) = previous_pose {
        let prev_keypoint = &prev.keypoints[region_to_keypoint_idx(region)];
        ((keypoint.x - prev_keypoint.x).powi(2) + (keypoint.y - prev_keypoint.y).powi(2)).sqrt()
    } else {
        0.0
    };

    let roughness_adjustment = (motion_velocity * 5.0).min(1.0);  // Cap at 1.0

    MaterialProps {
        hardness: base_hardness.min(1.0),
        roughness: (base_roughness + roughness_adjustment * 0.3).min(1.0),
        wetness: base_wetness,
        emission_intensity: if motion_velocity > 0.1 { 0.5 } else { 0.0 },
    }
}

fn classify_body_region(joint_idx: usize) -> BodyRegion {
    match joint_idx {
        0 => BodyRegion::Head,           // Nose
        1..=3 => BodyRegion::LeftEye,    // Left eye points
        4..=6 => BodyRegion::RightEye,   // Right eye points
        9 | 10 => BodyRegion::LeftArm,
        11 | 12 => BodyRegion::RightArm,
        13 | 15 => BodyRegion::LeftHand,
        14 | 16 => BodyRegion::RightHand,
        23 => BodyRegion::LeftLeg,
        24 => BodyRegion::RightLeg,
        25 | 27 => BodyRegion::LeftFoot,
        26 | 28 => BodyRegion::RightFoot,
        _ => BodyRegion::Chest,
    }
}
```

---

## Track I.3: IMU + Pose Fusion (1.5 days)

**Deliverables**:
- `src/fusion/imu_pose_fusion.rs` (300 lines) — Synchronize phone IMU with pose
- `src/fusion/spatial_validator.rs` (200 lines) — Validate pose + IMU coherence
- `tests/fusion_tests.rs` (150 lines, 10 tests)

**Ownership**: Shared (interface contract, both CV and ML teams use)

**Key insight**: IMU alone can't localize in room (just acceleration). Pose alone is 2D camera projection. **Together**: IMU velocity correlates with joint motion, enabling absolute 3D positioning.

```rust
// src/fusion/imu_pose_fusion.rs

pub struct ImuPoseFusion {
    /// Current pose in room coordinates
    pub pose_position: Vec<Point3D>,

    /// Phone accelerometer (m/s²)
    pub imu_acceleration: [f32; 3],

    /// Phone gyroscope (rad/s)
    pub imu_rotation: [f32; 3],

    /// Fusion confidence (how well IMU and pose agree)
    pub fusion_confidence: f32,
}

pub fn fuse_imu_and_pose(
    pose: &[PoseKeypoint; 33],
    imu_accel: [f32; 3],
    imu_gyro: [f32; 3],
    previous_fusion: Option<&ImuPoseFusion>,
) -> ImuPoseFusion {
    // 1. Integrate IMU acceleration to get velocity estimate
    // 2. Compare against pose-derived velocity (from keypoint motion)
    // 3. If they correlate, pose positioning is validated
    // 4. Use IMU to refine absolute position in room

    unimplemented!("IMU-Pose Kalman filter fusion")
}
```

**File Ownership Rules**:
- `src/fusion/imu_pose_fusion.rs` — Owned by: Shared (both teams read-only)
- `src/fusion/fusion_types.rs` — Interface contract (owned by: Project lead, read-only for both)

Both CV and ML engineers call functions from this module but don't modify it.

---

## Track I.4: PointMamba Material Learning (1.5 days)

**Deliverables**:
- `src/ml/pose_mamba_trainer.rs` (300 lines) — Train Mamba on pose-correlated features
- `src/ml/pose_pattern_discovery.rs` (200 lines) — Discover attack patterns that adapt to pose
- `tests/pose_mamba_training.rs` (150 lines, 10 tests)

**Ownership**: ML Engineer (exclusive)

**Key work**:

### Discovering Pose-Dependent Attack Patterns

The PointMamba model from Track D learns on point clouds. Now we extend it:

```rust
// src/ml/pose_mamba_trainer.rs

pub struct PoseMambaTrainer {
    base_mamba: PointMamba,  // From Track D
}

impl PoseMambaTrainer {
    /// Train Mamba to correlate RF attacks with human pose
    pub async fn train_on_pose_correlated_events(
        &mut self,
        events: &[PoseCorrelatedEvent],
    ) -> Result<(), Box<dyn Error>> {
        // Input: Events with (point_cloud, pose_frame, attack_pattern)
        // Learn: How does Mamba embedding change as pose changes?

        for epoch in 0..50 {
            let mut loss_sum = 0.0;

            for event in events {
                // 1. Extract point cloud from this event
                let mut point_cloud = event.spatial_points.clone();

                // 2. Augment with pose materials
                for point in &mut point_cloud {
                    if let Some(material) = event.body_region_materials.get(&point.region) {
                        point.material = material.clone();
                    }
                }

                // 3. Forward through Mamba
                let embedding = self.base_mamba.forward(&point_cloud)?;

                // 4. Predict: "Will the attack shift to mouth region if user tilts head?"
                let pose_delta = compute_pose_perturbation(&event.pose);
                let embedding_with_perturbed_pose = self.base_mamba.forward_with_pose_delta(
                    &point_cloud,
                    &pose_delta,
                )?;

                // 5. Loss: embeddings should diverge predictably with pose changes
                let contrastive_loss = contrastive_loss(&embedding, &embedding_with_perturbed_pose);
                loss_sum += contrastive_loss;
            }

            eprintln!("[PoseMamba] Epoch {}: loss = {:.4}", epoch, loss_sum / events.len() as f32);
        }

        Ok(())
    }
}

pub struct PoseCorrelatedEvent {
    pub spatial_points: Vec<Point3D>,
    pub pose_frame: PoseFrame,
    pub body_region_materials: HashMap<BodyRegion, MaterialProps>,
    pub attack_pattern_id: usize,
    pub timestamp: u64,
}
```

**Discovery Output**:

```json
{
  "pattern_7_pose_dependency": {
    "base_pattern": "Friday 3 PM heterodyne",
    "spatial_shift_with_pose": {
      "user_head_tilts_left": {
        "attack_shifts_to": "mouth region",
        "confidence": 0.89,
        "delay_ms": 250
      },
      "user_arm_raises": {
        "attack_follows_to": "raised arm azimuth",
        "confidence": 0.76,
        "delay_ms": 180
      }
    }
  }
}
```

---

## Track I.5: Physics Particle System (1.5 days)

**Deliverables**:
- `src/physics/particle_system.rs` (400 lines) — Material-aware particle collisions
- `src/visualization/particle_renderer.wgsl` (300 lines) — GPU particle rendering
- `examples/particle_demo.rs` (150 lines)
- `tests/particle_physics.rs` (150 lines, 10 tests)

**Ownership**: Shared (CV and ML teams use, but physics module is self-contained)

**Key concept**: Particles bounce differently off different materials (Niagara-style):
- **Hardness** → particle energy retention (hard = bouncy, soft = absorb)
- **Roughness** → surface friction (rough = slow down, smooth = slide)
- **Wetness** → drag coefficient (wet = sticky, dry = slippery)

```rust
// src/physics/particle_system.rs

pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub lifetime_ms: f32,
    pub material: MaterialProps,  // Hardness, roughness, wetness
}

pub fn simulate_particle_collision(
    particle: &mut Particle,
    surface_material: &MaterialProps,
    surface_normal: Vec3,
    dt: f32,
) {
    // Collision response based on both particle and surface materials

    let relative_hardness = particle.material.hardness * surface_material.hardness;
    let relative_roughness = (particle.material.roughness + surface_material.roughness) / 2.0;
    let relative_wetness = (particle.material.wetness + surface_material.wetness) / 2.0;

    // Bounce coefficient: hard materials bounce more
    let bounce_coefficient = relative_hardness;

    // Friction: rough surfaces slow down particles
    let friction = relative_roughness * 0.5;

    // Drag: wet surfaces create drag
    let drag = relative_wetness * 0.3;

    // Reflect velocity
    let incident = particle.velocity;
    let reflected = incident - 2.0 * incident.dot(surface_normal) * surface_normal;

    // Apply material interactions
    particle.velocity = reflected * bounce_coefficient;
    particle.velocity *= (1.0 - friction * dt);
    particle.velocity *= (1.0 - drag);

    // Update position
    particle.position += particle.velocity * dt;
    particle.lifetime_ms -= dt * 1000.0;
}
```

---

## Integration Points

| Track | Integration |
|-------|-------------|
| **A** | Provides IMU (accel + gyro) input |
| **D** | Point cloud + spatial locations (PointMamba learns pose-correlated patterns) |
| **VI** | Aether Wavefield gains pose-awareness (particles interact with human skeleton) |
| **H** | Haptic responses to pose-dependent targeting (haptics intensify when arm is raised, etc.) |
| **C** | Pattern discovery includes pose-correlation analysis |

---

## Success Criteria

✅ **I.1**: MediaPipe/RTMPose running @ 30 fps with <33ms latency
✅ **I.2**: 33 keypoints → point cloud materials, body region classification working
✅ **I.3**: IMU + pose fusion confident (correlation > 0.8) on test sequences
✅ **I.4**: PointMamba discovers pose-dependent patterns (e.g., "attack shifts when arm raises")
✅ **I.5**: Particles collide realistically with human skeleton (hard bounce, soft absorb)
✅ **No video stored** — Only pose keypoints + materials (privacy-preserving)
✅ **All tests passing**: 75+ unit + integration tests

---

## File Ownership

| File/Directory | Owner | Permissions |
|---|---|---|
| `src/computer_vision/` | CV Engineer | Exclusive write |
| `src/ml/pose_materials.rs` | ML Engineer | Exclusive write |
| `src/ml/pose_mamba_trainer.rs` | ML Engineer | Exclusive write |
| `src/fusion/imu_pose_fusion.rs` | Shared (read-only) | Both read |
| `src/physics/particle_system.rs` | Shared (read-only) | Both read |
| `src/fusion/fusion_types.rs` | Project Lead (interface) | Both read |

**Conflict avoidance**: No shared write files. Interface contracts only.

---

**Last Updated**: 2026-03-09
**Status**: Ready for parallel assignment (CV team starts I.1, ML team starts I.2 simultaneously)

