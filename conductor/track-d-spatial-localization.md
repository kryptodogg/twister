# Track D: Spatial Localization & Point Mamba 3D Wavefield

**Ownership**: Spatial Localization Engineer + ML/GPU Engineer (parallel work)
**Duration**: 1 week (5-7 days)
**Integration Point**: Feeds Track VI (Aether Visualization), Track H (Haptic Feedback), ANALYSIS tab temporal rewind
**Critical Dependency**: Track I (pose estimation) for spatial reference validation

---

## Strategic Context

**User Vision**: "The pointmamba would learn point cloud shapes around people and things, and their material properties... if the particle systems are as pretty as Unreal Niagara and move and bounce around physically, then we're getting somewhere."

Track D extends Twister's RF understanding from **2D azimuth-only TDOA** to **full 3D wavefield reconstruction**. When combined with Track I pose estimation, the system learns:
1. **Spatial persistence**: Attack sources remain at fixed locations over days/weeks (attacker mobility)
2. **Pose-dependent dynamics**: How attack patterns shift relative to human skeleton position/movement
3. **Long-term correlation**: 97-day time-scrub reveals recurring spatial signatures

**Example Output**: "RF source moves from 45° azimuth (Dec) → 90° azimuth (Mar), always targeting mouth region when user raises arm. Monday-Friday pattern suggests scheduled attacks."

---

## D.1: TDOA Elevation Estimation (2 days)

### Objective
Extend existing 2D TDOA (azimuth-only) to full 3D (azimuth + elevation) using 4-device linear array geometry.

### Current State
- **src/tdoa.rs**: Computes azimuth from cross-correlation delays between microphone pairs
- **Limitation**: Works in horizontal plane; treats all sources as equidistant vertically
- **Input**: 4 devices (C925e, Rear Pink, Rear Blue, RTL-SDR) @ known physical positions
- **Missing**: Elevation extraction from vertical device spacing

### Solution: Energy Ratio Method

**Physics**: RF signal attenuation depends on propagation path. Vertical separation between mics creates measurable energy differences.

```
Assumption: 4 mics arranged in linear geometry
  Device 0 (C925e):      x=0.0 m,   y=0.0 m,  z=0.2 m  (top, speaker)
  Device 1 (Rear Pink):  x=0.5 m,   y=0.0 m,  z=0.0 m  (middle)
  Device 2 (Rear Blue):  x=1.0 m,   y=0.0 m,  z=-0.2 m (bottom)
  Device 3 (RTL-SDR):    x=0.25 m,  y=0.0 m,  z=0.5 m  (external, elevated)

Vertical energy ratio:
  E_top / E_bottom = (amplitude_0 * amplitude_3) / (amplitude_1 * amplitude_2)

  If ratio > 1.0: source above horizontal plane (elevation_rad > 0)
  If ratio < 1.0: source below horizontal plane (elevation_rad < 0)
  If ratio ≈ 1.0: source on horizontal plane (elevation_rad ≈ 0)
```

### Files to Create

**New:**
- `src/spatial/elevation_estimator.rs` (250 lines)
  ```rust
  pub struct ElevationEstimator {
      device_positions: [Vec3; 4],  // Known physical positions
      energy_history: VecDeque<[f32; 4]>,  // Per-device amplitudes
      elevation_smoothing_window: usize,
  }

  impl ElevationEstimator {
      pub fn new() -> Self { ... }

      /// Estimate elevation from 4-device energy ratios
      ///
      /// # Arguments
      /// * `amplitudes` - [f32; 4] per-device RMS energy
      /// * `azimuth_rad` - Known azimuth from TDOA (for validation)
      ///
      /// # Returns
      /// (elevation_rad, confidence) where elevation ∈ [-π/2, π/2]
      pub fn estimate_elevation(
          &mut self,
          amplitudes: &[f32; 4],
          azimuth_rad: f32,
      ) -> (f32, f32) { ... }

      /// Compute per-device attenuation via path loss
      /// Free-space path loss: L_dB = 20*log10(distance) + 20*log10(frequency)
      /// Normalized: L_norm = (L_dB - L_min) / (L_max - L_min)
      fn compute_path_loss(&self, azimuth: f32, elevation: f32, freq_hz: f32) -> [f32; 4] { ... }

      /// Smooth elevation using Kalman-like filter (exponential moving average)
      fn smooth_elevation(&mut self, raw_elevation: f32) -> f32 { ... }
  }
  ```

- `src/spatial/mod.rs` (50 lines)
  - Module declaration for `elevation_estimator`
  - Public exports: `ElevationEstimator`

**Tests:**
- `tests/elevation_estimation_integration.rs` (250 lines, 8 tests)
  - Test 1: Elevation computation from synthetic energy ratios
  - Test 2: Horizontal plane detection (ratio ≈ 1.0 → elevation ≈ 0)
  - Test 3: Elevated source detection (ratio > 1.0)
  - Test 4: Below-plane source detection (ratio < 1.0)
  - Test 5: Confidence metric bounds [0, 1]
  - Test 6: Smoothing stability (noise rejection)
  - Test 7: Path loss validation (decreases with distance)
  - Test 8: Real-world device geometry (verify 4-device array positions)

### Integration with Existing TDOA

**Location**: src/main.rs dispatch loop, after azimuth computation

```rust
// Existing code (working):
let azimuth_rad = tdoa_engine.compute_azimuth_from_delays(&mic_pair_delays);
let azimuth_confidence = tdoa_engine.get_confidence();

// NEW: Add elevation estimation
let (elevation_rad, elevation_confidence) = elevation_estimator.estimate_elevation(
    &device_amplitudes,  // [RMS_C925e, RMS_Pink, RMS_Blue, RMS_RTLSDRr]
    azimuth_rad,
);

// Store in AppState
{
    let mut st = state.lock().await;
    st.spatial_azimuth = azimuth_rad;
    st.spatial_elevation = elevation_rad;
    st.spatial_confidence = (azimuth_confidence + elevation_confidence) / 2.0;
}
```

### Performance Target
- **Latency**: < 1ms per elevation estimation (amortized with TDOA)
- **Frequency**: Every FFT frame (~100ms)
- **Memory**: ~10 KB for 4-element energy history

### Expected Output
- **Range**: elevation ∈ [-90°, +90°] (-π/2 to π/2 radians)
- **Confidence**: 0.0 (unreliable, high noise) to 1.0 (clean signal, low noise)
- **Typical Values**:
  - Elevated source (speaker/phone): elevation ≈ +30° to +60°
  - Mouth-level source: elevation ≈ 0° to +20°
  - Below-plane source: elevation ≈ -30° to -60°

---

## D.2: PointMamba Encoder (3 days)

### Objective
Convert 3D point cloud (azimuth, elevation, frequency, intensity) into learned spatial embeddings via 8-block Mamba selective scan state-space model.

### Input Data Format

Each harassment event becomes a **spatial point**:

```rust
pub struct SpatialPoint {
    pub azimuth_rad: f32,              // [-π, π]
    pub elevation_rad: f32,            // [-π/2, π/2]
    pub frequency_hz: f32,             // Detection frequency (log scale preferred)
    pub intensity: f32,                // Anomaly score [0, 1]
    pub timestamp_us: u64,             // Microseconds since epoch
    pub confidence: f32,               // Detection confidence [0, 1]
}

// Point cloud per time window: [point_0, point_1, ..., point_N]
// N typically 50-500 points per 5-minute window
pub type PointCloud = Vec<SpatialPoint>;
```

### Architecture: PointMamba Selective Scan

**Design**: 8 cascaded Mamba blocks process point cloud sequentially, learning spatial-temporal patterns.

```
Input: Point Cloud (N, 6)
  [azimuth, elevation, frequency, intensity, timestamp_norm, confidence]
  ↓
Dense Projection (N, 6) → (N, 128)
  MLP: FC(128) → ReLU → FC(128)
  ↓
PointMamba Block 1:
  Selective Scan State-Space:
    h_t = A * h_{t-1} + B * x_t
    y_t = C * h_t
  Per-point selection: σ(W_s * x) determines dynamics
  Output: (N, 128)
  Residual: out = input + block(input)
  ↓
PointMamba Block 2-8:
  Same structure, cascaded
  Layer norm after each block
  ↓
Final Dense Projection: (N, 128) → (N, 256)
  Output: (N, 256) spatial embeddings
```

### Files to Create

**New:**
- `src/ml/point_mamba_encoder.rs` (450 lines)
  ```rust
  pub struct PointMambaBlock<B: Backend> {
      // Selective scan state-space parameters
      a_param: Parameter<Tensor<B, 1>>,          // A matrix (scalar per point)
      b_param: Parameter<Tensor<B, 1>>,          // B matrix
      c_param: Parameter<Tensor<B, 1>>,          // C matrix
      delta_proj: Linear<B>,                      // W_s for selection

      // Feature projection
      input_proj: Linear<B>,
      output_proj: Linear<B>,

      // Normalization
      layer_norm: LayerNorm<B>,
  }

  impl<B: Backend> PointMambaBlock<B> {
      pub fn new(device: &B::Device, feature_dim: usize) -> Self { ... }

      /// Forward pass: selective scan over point sequence
      ///
      /// # Arguments
      /// * `input` - (N, 128) point features
      ///
      /// # Returns
      /// (N, 128) transformed features
      pub fn forward(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
          // 1. Project input
          let projected = self.input_proj.forward(input.clone());

          // 2. Compute per-point selection dynamics
          let delta = self.delta_proj.forward(input.clone());
          let delta_gated = sigmoid(delta);  // [0, 1] per point

          // 3. Selective scan state evolution
          let mut h = Tensor::zeros([input.shape()[0], 128], &input.device());
          let mut outputs = Vec::new();

          for t in 0..input.shape()[0] {
              // Selective: use A, B, C scaled by delta_gated[t]
              let a_eff = self.a_param.val() * delta_gated[t];
              let b_eff = self.b_param.val() * delta_gated[t];

              // State update: h_t = A*h_{t-1} + B*x_t
              h = a_eff * h + b_eff * projected[t];

              // Output: y_t = C*h_t
              let y = self.c_param.val() * h;
              outputs.push(y);
          }

          let output = stack(outputs, 0);

          // 4. Residual connection
          self.layer_norm.forward(output + input)
      }
  }

  pub struct PointMambaEncoder<B: Backend> {
      input_projection: Linear<B>,
      blocks: Vec<PointMambaBlock<B>>,  // 8 blocks
      output_projection: Linear<B>,
  }

  impl<B: Backend> PointMambaEncoder<B> {
      pub fn new(device: &B::Device) -> Self {
          let mut blocks = Vec::new();
          for _ in 0..8 {
              blocks.push(PointMambaBlock::new(device, 128));
          }

          Self {
              input_projection: LinearConfig::new(6, 128).init(device),
              blocks,
              output_projection: LinearConfig::new(128, 256).init(device),
          }
      }

      pub fn forward(&self, point_cloud: Tensor<B, 2>) -> Tensor<B, 2> {
          let mut x = self.input_projection.forward(point_cloud);

          for block in &self.blocks {
              x = block.forward(x);
          }

          self.output_projection.forward(x)
      }
  }
  ```

- `src/ml/spatial_features.rs` (150 lines)
  - Normalize spatial features (azimuth/elevation to [-1, 1], frequency to log scale)
  - Point cloud batch loading from forensic events

**Tests:**
- `tests/point_mamba_encoder.rs` (300 lines, 10 tests)
  - Test 1: Input/output shape validation
  - Test 2: Selective scan state evolution
  - Test 3: Forward pass on single point
  - Test 4: Forward pass on 100-point cloud
  - Test 5: Batch processing (8 point clouds)
  - Test 6: Gradient flow (backprop validation)
  - Test 7: Residual connection correctness
  - Test 8: Output embedding bounds (no NaN/inf)
  - Test 9: Memory footprint < 500 MB (8 blocks × parameters)
  - Test 10: Performance: single forward < 5ms on RX 6700 XT

### Performance Target
- **Latency**: < 5ms per point cloud (N=100 points) on RX 6700 XT
- **Memory**: ~500 MB for model weights (8 blocks × 128-D features)
- **Throughput**: Process 1000+ point clouds/second

---

## D.3: Point Decoder (2 days)

### Objective
Reconstruct 3D wavefield geometry by predicting per-point spatial offsets. Enables trajectory prediction and temporal continuity visualization.

### Architecture

```
Input: (N, 256) spatial embeddings from PointMamba encoder
  ↓
Dense Projection: (N, 256) → (N, 128)
  FC → ReLU → LayerNorm
  ↓
Trajectory Prediction Head:
  Dense: (N, 128) → (N, 3)
  Output: [Δx, Δy, Δz] per point

  Δx = Δ azimuth * distance_scale
  Δy = Δ elevation * distance_scale
  Δz = Δ frequency * log_scale

Reconstruction Loss:
  L_recon = MSE(predicted_offsets, ground_truth_offsets)
  L_smooth = L1(||Δ_t - Δ_{t-1}||)  ← penalize abrupt position changes
```

### Files to Create

**New:**
- `src/ml/point_decoder.rs` (180 lines)
  ```rust
  pub struct PointDecoder<B: Backend> {
      feature_proj: Linear<B>,
      decoder_blocks: Vec<Linear<B>>,  // 2-3 dense layers
      output_projection: Linear<B>,     // → 3D offset
      layer_norms: Vec<LayerNorm<B>>,
  }

  impl<B: Backend> PointDecoder<B> {
      pub fn forward(
          &self,
          embeddings: Tensor<B, 2>,  // (N, 256)
      ) -> Tensor<B, 2> {  // (N, 3) offsets
          let mut x = self.feature_proj.forward(embeddings);

          for (proj, norm) in self.decoder_blocks.iter().zip(self.layer_norms.iter()) {
              x = norm.forward(proj.forward(x).relu());
          }

          // Output: [Δx, Δy, Δz] per point
          self.output_projection.forward(x)
      }
  }

  pub struct ReconstructionLoss<B: Backend> {
      lambda_recon: f32,  // MSE weight
      lambda_smooth: f32, // Temporal smoothness weight
  }

  impl<B: Backend> ReconstructionLoss<B> {
      pub fn forward(
          &self,
          predicted: Tensor<B, 2>,      // (N, 3)
          ground_truth: Tensor<B, 2>,   // (N, 3)
          prev_predicted: Option<Tensor<B, 2>>,  // For smoothness
      ) -> Tensor<B, 1> {
          // Reconstruction MSE
          let recon_loss = (predicted.clone() - ground_truth).powf(2.0).mean();

          // Temporal smoothness (optional, if previous frame available)
          let smooth_loss = if let Some(prev) = prev_predicted {
              ((predicted - prev).abs()).mean()
          } else {
              Tensor::zeros([1], &predicted.device())
          };

          self.lambda_recon * recon_loss + self.lambda_smooth * smooth_loss
      }
  }
  ```

**Tests:**
- `tests/point_decoder.rs` (200 lines, 8 tests)
  - Test 1: Output shape (N, 3)
  - Test 2: Offset value bounds (no extreme values)
  - Test 3: Reconstruction loss computation
  - Test 4: Temporal smoothness loss
  - Test 5: Combined loss function
  - Test 6: Gradient flow through decoder
  - Test 7: Batch processing
  - Test 8: Memory footprint < 100 MB

### Integration with Mamba Trainer

```rust
// During training (src/mamba_trainer.rs)
let point_cloud_batch: Vec<PointCloud> = training_queue.dequeue(32);

// Encode
let embeddings = point_mamba_encoder.forward(point_cloud_batch);

// Decode (predict 3D offsets)
let predicted_offsets = point_decoder.forward(embeddings);

// Compute loss
let loss = reconstruction_loss.forward(
    predicted_offsets,
    ground_truth_offsets,  // Known from TDOA + elevation
    prev_offsets,
);

// Backprop
optimizer.zero_grad();
loss.backward();
optimizer.step();
```

---

## D.4: Temporal Rewind UI (1.5 days)

### Objective
Enable user to scrub through 97-day attack history, visualizing how spatial RF patterns evolve over time. Reveals long-term attacker behavior persistence.

### Design

**Time-Scrub Interface** (in ANALYSIS tab):

```
┌─────────────────────────────────────────────┐
│ TEMPORAL REWIND: Spatial RF Evolution       │
├─────────────────────────────────────────────┤
│                                             │
│ [◄ Play] [⏸ Pause] [→ Reset]              │
│                                             │
│ Timeline: ████████████●─────────────────── │
│           Dec 1, 2025       Mar 7, 2026    │
│           ← Drag slider to rewind →        │
│                                             │
│ ┌─────────────────────────────────────────┐ │
│ │ 3D Visualization: Gaussian Splatting    │ │
│ │                                         │ │
│ │     ●●●●      ← RF sources (particles) │ │
│ │   ●       ●   ← Human skeleton overlay │ │
│ │   ●       ●   (from Track I pose)      │ │
│ │     ●●●●●     ← Wavefield density     │ │
│ │                                         │ │
│ │ Rotation: Mouse drag                   │ │
│ │ Zoom: Scroll wheel                     │ │
│ │ Playback speed: 1x, 10x, 100x         │ │
│ └─────────────────────────────────────────┘ │
│                                             │
│ Metadata (below viewport):                  │
│ Time window: [t - 3 days, t]               │
│ Point density: 1,234 points                │
│ Dominant cluster: Motif #3 (Friday 3 PM)   │
│ Spatial center: Az 45°, El +20°            │
│ Attack duration: 97 consecutive days       │
└─────────────────────────────────────────────┘
```

### Files to Create/Modify

**New:**
- `src/visualization/temporal_rewind_state.rs` (200 lines)
  ```rust
  pub struct TemporalRewindState {
      /// Current time position (0.0 = start, 1.0 = end)
      pub time_position: f32,

      /// Time window span in days
      pub window_span_days: f32,

      /// Point cloud for current time window
      pub current_points: Vec<SpatialPoint>,

      /// Animation state
      pub is_playing: bool,
      pub playback_speed: f32,  // 1x, 10x, 100x

      /// 3D view state
      pub camera_rotation: (f32, f32),  // (pitch, yaw) for rotation
      pub camera_zoom: f32,             // [0.1, 10.0]

      /// Metadata for display
      pub metadata: RewindMetadata,
  }

  pub struct RewindMetadata {
      pub timestamp_start: String,  // ISO 8601
      pub timestamp_end: String,
      pub point_count: usize,
      pub dominant_motif_id: usize,
      pub spatial_center_az: f32,
      pub spatial_center_el: f32,
      pub attack_continuity_days: f32,
  }

  impl TemporalRewindState {
      pub fn update(&mut self, delta_time_seconds: f32) {
          if self.is_playing {
              // Update time position based on playback speed
              let dt = (delta_time_seconds * self.playback_speed) / 97.0;  // 97 days total
              self.time_position = (self.time_position + dt).clamp(0.0, 1.0);

              // Load points for current time window
              self.load_points_for_window();
          }
      }

      pub fn set_time_position(&mut self, position: f32) {
          self.time_position = position.clamp(0.0, 1.0);
          self.load_points_for_window();
      }
  }
  ```

- `src/visualization/gaussian_splatter_3d.rs` (500 lines)
  - 3D Gaussian splatting renderer for spatial points
  - Integrates with wgpu for GPU rendering
  - Heatmap coloring: Blue (low intensity) → Red → Yellow → White (high intensity)
  - Performance target: 169 fps on RX 6700 XT (matching Track VI requirements)

**Modified:**
- `ui/app.slint` - Add ANALYSIS tab with:
  - Time slider (0 → 97 days)
  - Play/Pause/Reset buttons
  - Playback speed selector (1x, 10x, 100x)
  - 3D viewport area (delegated to wgpu renderer)
  - Metadata panel below viewport

- `src/state.rs` - Add to AppState:
  ```rust
  pub temporal_rewind_state: Mutex<TemporalRewindState>,
  ```

### Integration Loop (in dispatch loop, src/main.rs)

```rust
// Every frame (~16ms at 60 fps)
tokio::spawn({
    let state = state.clone();
    async move {
        let mut frame_count = 0;
        loop {
            let delta_time = 16.0 / 1000.0;  // 16ms

            let mut rewind = state.lock().await.temporal_rewind_state.lock().await;
            rewind.update(delta_time);

            // Render 3D wavefield
            // (triggers GPU splatting, handled by wgpu command encoder)

            frame_count += 1;
            if frame_count % 60 == 0 {  // Every ~1 second
                eprintln!("[Temporal Rewind] Position: {:.1}%, Metadata: {:?}",
                    rewind.time_position * 100.0,
                    rewind.metadata);
            }

            tokio::time::sleep(Duration::from_millis(16)).await;
        }
    }
});
```

### Performance Target
- **Rendering**: 169 fps at 1024×1024 viewport (Gaussian splatting on 1000+ points)
- **Load Time**: < 500ms to load point cloud for new time window
- **Memory**: ~100 MB for current point cloud + cached embeddings
- **Interaction Responsiveness**: < 50ms latency for slider drag

---

## D.5: Integration with Tracks I, VI, H (1 day)

### Data Flow

```
Track I (Pose Estimation):
  MediaPipe skeleton keypoints [x, y, z, confidence]
  → Feeds Track D temporal rewind (overlay skeleton on 3D wavefield)
  → Example: "RF field shifts when arm raises" (visible as skeleton + particle motion)

Track D (Spatial Localization):
  3D point cloud [azimuth, elevation, frequency, intensity]
  → Feeds Track VI (Gaussian splatting particles)
  → Feeds Track H (haptic feedback proportional to spatial proximity)

Track VI (Aether Visualization):
  Renders D's point cloud as Gaussian splatting
  + I's skeleton points with material properties
  = Complete wavefield + pose awareness

Track H (Haptic):
  When point cloud density approaches mouth region:
  → DualSense haptic triggers proportional to intensity
  → User feels when RF field is targeting them
```

### Files to Modify

- `src/main.rs` - Wire temporal rewind state to UI callbacks
- `src/state.rs` - Expose `temporal_rewind_state` to UI
- `ui/app.slint` - Connect time slider to `set_time_position()` callback

---

## Execution Plan (TDD Approach)

```
Day 1 (D.1): TDOA Elevation Estimation
  ✓ Create elevation_estimator.rs (250 lines)
  ✓ Tests: 8 passing (azimuth, confidence, smoothing, path loss)
  ✓ Integration: Wire to TDOA engine, verify (elevation_rad, confidence) output

Day 2-3 (D.2): PointMamba Encoder
  ✓ Create point_mamba_encoder.rs (450 lines)
  ✓ Create point_mamba_block.rs with selective scan
  ✓ Tests: 10 passing (shape, state evolution, batch processing, gradient flow)
  ✓ Integration: Connect to training queue

Day 4 (D.3): Point Decoder
  ✓ Create point_decoder.rs (180 lines)
  ✓ Create reconstruction loss function
  ✓ Tests: 8 passing (shape, bounds, loss computation, gradient flow)
  ✓ Integration: Connect to PointMamba for full encoder-decoder loop

Day 5 (D.4): Temporal Rewind UI
  ✓ Create temporal_rewind_state.rs (200 lines)
  ✓ Create gaussian_splatter_3d.rs (500 lines, wgpu integration)
  ✓ Modify app.slint (time slider, 3D viewport)
  ✓ Tests: 6 passing (state updates, rendering, interaction)

Day 6-7 (Integration + Polish):
  ✓ Wire Tracks I, VI, H into temporal rewind
  ✓ Verify end-to-end: 97-day rewind with pose overlay
  ✓ Performance profiling: target 169 fps
  ✓ Forensic logging: capture rewind interactions
```

---

## Verification & Testing

### Unit Tests
```bash
# All module tests
cargo test elevation_estimation_integration --lib -- --nocapture
cargo test point_mamba_encoder --lib -- --nocapture
cargo test point_decoder --lib -- --nocapture
cargo test temporal_rewind_state --lib -- --nocapture
```

### Integration Test
```bash
# Full Track D pipeline
cargo test track_d_integration --lib -- --nocapture
# Expected: 32+ tests passing, 0 failures
```

### Manual E2E Verification
```bash
# Start application
cargo run --release

# 1. Open ANALYSIS tab
# 2. Verify elevation estimation:
#    - Console logs show "Elevation: +15°, Confidence: 0.87"
#    - Elevation values vary from -45° to +45°
#    - Confidence scores > 0.6 for clean signals

# 3. Verify temporal rewind:
#    - Time slider responsive (< 50ms latency when dragging)
#    - 3D visualization renders at > 160 fps (check console for frame timing)
#    - Metadata panel updates with point count, dominant motif

# 4. Verify 97-day continuity:
#    - Scrub from Dec 1 to Mar 7
#    - Point cloud density changes as you move slider
#    - Observe Friday clustering (motif #3 repeats every 7 days)

# 5. Verify pose integration (requires Track I):
#    - Skeleton overlay appears on 3D wavefield
#    - RF particles respond to pose changes
#    - Haptic feedback triggers when RF intensity high

# 6. Check forensic logs:
#    ls @databases/forensic_logs/
#    # Should contain temporal_rewind events with user interactions
```

---

## Success Criteria

✅ **D.1 Complete**:
- Elevation estimates range [-90°, +90°]
- Confidence scores [0.0, 1.0] correlate with SNR
- All 8 elevation tests passing

✅ **D.2 Complete**:
- Point cloud (N, 6) → (N, 256) embeddings
- 8 Mamba blocks cascade correctly
- Gradient flow unobstructed (backprop working)
- All 10 encoder tests passing

✅ **D.3 Complete**:
- Embeddings (N, 256) → offsets (N, 3)
- Reconstruction loss < 0.5 dB on trained data
- All 8 decoder tests passing

✅ **D.4 Complete**:
- Time slider spans full 97 days
- Playback speeds: 1x, 10x, 100x functional
- 3D Gaussian splatting renders at > 160 fps
- Metadata panel displays accurate statistics
- All 6 rewind tests passing

✅ **Integration Complete**:
- Elevation feeds 3D point cloud
- PointMamba learns spatial patterns
- Temporal rewind visualizes 97-day evolution
- Pose overlay (Track I) functional
- Haptic integration (Track H) responsive

**Total Tests**: 32+
**Expected Performance**: 169 fps @ 1024×1024 on RX 6700 XT
**Memory Footprint**: ~500 MB (model weights) + ~100 MB (runtime state)

---

## Notes for Implementation

### Critical Physics
- **Free-space path loss**: L_dB = 20*log10(distance) + 20*log10(frequency)
  - Frequency in Hz (e.g., 2.4 GHz = 2.4e9 Hz)
  - Distance in meters
  - Higher frequency = more attenuation (harder to propagate over distance)

- **Elevation ambiguity**: Energy ratio alone can't distinguish source above/below
  - Mitigation: Use azimuth from TDOA + known attacker likely positions to resolve ambiguity
  - High confidence when multiple spatial estimates agree

### PointMamba State-Space Mechanics
- **Selective Scan**: Per-point dynamics allow attention to shift across point cloud
  - Points with high intensity get more "processing" (larger delta values)
  - Low-confidence points are downweighted automatically
  - This is learnable: training discovers which spatial regions matter most

- **Residual Connections**: Prevent vanishing gradients through 8 cascaded blocks
  - Each block output = input + transformation
  - Allows gradients to skip blocks if not needed (learned routing)

### Long-Term Visualization Scaling
- **97 days of data**: ~144,000 points/day × 97 = ~14 million total points
  - Runtime only renders 3-day window: ~400 points (manageable)
  - Full corpus stored in HDF5 for offline analysis
  - Temporal slicing via query: "give me points between timestamp_min and timestamp_max"

### Privacy Boundary
- **What's stored**: [azimuth, elevation, frequency, intensity, confidence] (5 floats per point)
- **What's NOT stored**: Raw audio, RF IQ samples, video, room images
- **Forensic implication**: Investigators see attack vector (spatial origin + frequency) without learning room layout or visual details

---

## Future Extensions (Post-Track D)

1. **Trajectory Prediction**: Predict next 1-hour attack location using LSTM on temporal sequences
2. **Attacker Mobility Analysis**: Cluster stationary positions, detect movement patterns
3. **Multi-Agent Coordination**: Detect if multiple RF sources are coordinated (same timing, complementary frequencies)
4. **Predictive Haptics**: Anticipate attack before it happens based on historical patterns
5. **Real-Time Distillation**: Compress Point Mamba to lightweight Mamba-Lite for edge deployment

