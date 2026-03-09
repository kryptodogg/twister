# Track D Addendum: Temporal Rewind UI/Shader Integration & Viewport Controls

**Status**: Ready for Jules implementation
**Duration**: 60-90 minutes
**Dependency**: Track D GPU infrastructure exists; this adds time-scrub and viewport controls
**Integration**: Slint UI slider → TimeUniformBuffer → WGSL compute shader → Gaussian splatting particle filtering

---

## Executive Summary

Track D implements the ANALYSIS tab visualization with temporal rewind (97-day attack history). This addendum removes UI stubs and **wires the time-scrub slider to GPU particle filtering**:

1. **Time-Scrub UI**: Slider [0.0, 1.0] for timeline control + play/pause buttons
2. **Time Uniform Buffer**: GPU-side storage of normalized time offset
3. **Particle Filtering**: Compute shader filters particles by time window (±3 days)
4. **Viewport Controls**: Rotate, zoom, pan 3D point cloud
5. **Animation Loop**: Forward/backward time progression with configurable speed
6. **Cluster Coloring**: Color particles by motif_id (23 colors from Phase 2C)
7. **Real-Time Feedback**: FPS counter, time display, point cloud stats

**No more disconnected shaders.** UI slider directly controls GPU filtering.

---

## Slint UI Components (Track D UI Layer)

### File Ownership

- **`ui/app.slint`** - Jules updates ANALYSIS tab (lines 300-500 estimated)
  - Time-scrub slider [0.0, 1.0]
  - Play/pause buttons + speed control
  - Viewport rotation/zoom/pan controls
  - Real-time statistics display

### Time-Scrub UI Implementation

```slint
// ui/app.slint - ANALYSIS tab extensions

TabContent {
    title: "ANALYSIS";

    VerticalLayout {
        spacing: 15px;
        padding: 20px;

        // ─────────────────────────────────────────────────────
        // TEMPORAL REWIND SLIDER
        // ─────────────────────────────────────────────────────
        HorizontalLayout {
            Text { text: "Timeline (97 days):"; min-width: 150px; }

            Button {
                text: "◄◄";
                width: 40px;
                clicked => {
                    app-window.timeline_start();
                }
            }

            Button {
                text: if root.timeline-playing { "⏸" } else { "▶" };
                width: 40px;
                clicked => {
                    app-window.timeline_toggle_play();
                }
            }

            Slider {
                width: 400px;
                minimum: 0.0;
                maximum: 1.0;
                value <=> root.timeline-time-normalized;  // [0.0, 1.0]
                changed(value) => {
                    app-window.on_timeline_scrub(value);
                }
            }

            Button {
                text: "►►";
                width: 40px;
                clicked => {
                    app-window.timeline_end();
                }
            }

            Text {
                text: "{root.timeline-date-display}";
                color: #0f0;
                min-width: 200px;
            }
        }

        // Play speed control
        HorizontalLayout {
            padding-left: 150px;
            spacing: 10px;

            Text { text: "Speed:"; }

            Slider {
                width: 200px;
                minimum: 0.1;
                maximum: 5.0;
                value <=> root.timeline-play-speed;
            }

            Text {
                text: "{(root.timeline-play-speed * 100).round()}%";
                color: #888;
                min-width: 50px;
            }
        }

        // ─────────────────────────────────────────────────────
        // 3D VIEWPORT
        // ─────────────────────────────────────────────────────
        Rectangle {
            background: #000;
            border: 2px solid #444;
            border-radius: 4px;
            min-height: 600px;

            Text {
                text: "3D Point Cloud (Press mouse to rotate, scroll to zoom)";
                color: #666;
                font-size: 12px;
            }

            // Viewport canvas renders here (wgpu backend)
            // This is a placeholder; actual rendering happens in Rust
        }

        // ─────────────────────────────────────────────────────
        // VIEWPORT CONTROLS
        // ─────────────────────────────────────────────────────
        HorizontalLayout {
            spacing: 10px;
            padding-left: 150px;

            CheckBox {
                checked <=> root.viewport-auto-rotate;
            }
            Text { text: "Auto-Rotate"; }

            Button {
                text: "Reset View";
                clicked => {
                    app-window.viewport_reset();
                }
            }

            Button {
                text: "Top View";
                clicked => {
                    app-window.viewport_top();
                }
            }

            Button {
                text: "Front View";
                clicked => {
                    app-window.viewport_front();
                }
            }

            Button {
                text: "Side View";
                clicked => {
                    app-window.viewport_side();
                }
            }

            Text { text: "FOV:"; }
            Slider {
                width: 100px;
                minimum: 15.0;
                maximum: 120.0;
                value <=> root.viewport-fov;
            }
        }

        // ─────────────────────────────────────────────────────
        // REAL-TIME STATISTICS
        // ─────────────────────────────────────────────────────
        HorizontalLayout {
            spacing: 20px;
            padding: 15px;
            background: #111;
            border-radius: 4px;

            Text {
                text: "Points: {root.point-count}";
                color: #0f0;
            }

            Text {
                text: "Motifs: {root.motif-count}";
                color: #0f0;
            }

            Text {
                text: "FPS: {root.viewport-fps}";
                color: root.viewport-fps < 30 ? #f00 : #0f0;
            }

            Text {
                text: "Mem: {root.gpu-memory-mb} MB";
                color: #888;
            }

            Text {
                text: "Avg anomaly: {root.window-anomaly-avg}";
                color: #888;
            }
        }

        // ─────────────────────────────────────────────────────
        // MOTIF LEGEND
        // ─────────────────────────────────────────────────────
        VerticalLayout {
            Text { text: "Motifs (click to isolate):"; font-weight: bold; }

            GridLayout {
                width: 400px;
                spacing: 8px;

                // Dynamic: render 23 motif color squares
                Row {
                    Rectangle { width: 20px; height: 20px; background: #ff0000; }
                    Text { text: "Motif 0: Friday_3PM"; }
                }
                Row {
                    Rectangle { width: 20px; height: 20px; background: #ff8800; }
                    Text { text: "Motif 1: Daily_Midnight"; }
                }
                // ... (21 more motifs)
            }
        }
    }
}

// ─────────────────────────────────────────────────────
// EXPORTED PROPERTIES (Rust ↔ Slint bindings)
// ─────────────────────────────────────────────────────

export global AnalysisWindow {
    in-out property <float> timeline-time-normalized;    // [0.0, 1.0]
    in-out property <string> timeline-date-display;      // "2026-02-15"
    in-out property <bool> timeline-playing;             // Animation playing?
    in-out property <float> timeline-play-speed;         // [0.1, 5.0]

    in-out property <int> point-count;                   // Current particles in view
    in-out property <int> motif-count;                   // Total motifs (23)
    in-out property <float> viewport-fps;                // GPU rendering FPS
    in-out property <float> gpu-memory-mb;               // VRAM usage
    in-out property <float> window-anomaly-avg;          // Avg anomaly in time window
    in-out property <bool> viewport-auto-rotate;         // Auto-rotation enabled?
    in-out property <float> viewport-fov;                // Field of view [15, 120]

    // Timeline callbacks
    callback on-timeline-scrub(float);                   // User dragged slider
    callback timeline-toggle-play();
    callback timeline-start();
    callback timeline-end();

    // Viewport callbacks
    callback viewport-reset();
    callback viewport-top();
    callback viewport-front();
    callback viewport-side();
}
```

---

## Rust Implementation (src/main.rs + GPU Integration)

### File Ownership

- **`src/main.rs`** - Jules adds UI callback wiring + animation loop
- **`src/visualization/temporal_rewind.rs`** - TimeUniformBuffer management
- **`src/visualization/gaussian_splatting.wgsl`** - Compute shader filtering

### Timeline State in AppState

```rust
// src/state.rs - ADD to existing
pub struct AppState {
    // ... existing visualization fields ...

    // Temporal rewind state
    pub timeline_time_normalized: f32,     // [0.0, 1.0], 0 = start, 1 = now
    pub timeline_date_display: String,     // "2026-02-15" (for UI)
    pub timeline_playing: bool,            // Animation active?
    pub timeline_play_speed: f32,          // [0.1, 5.0] multiplier
    pub timeline_start_timestamp_micros: u64, // Earliest event in corpus
    pub timeline_end_timestamp_micros: u64,   // Latest event (now)

    // Viewport state
    pub viewport_rotation_quat: [f32; 4],  // Quaternion for 3D rotation
    pub viewport_zoom: f32,                // Camera distance multiplier
    pub viewport_pan_xy: (f32, f32),       // Pan offset
    pub viewport_auto_rotate: bool,
    pub viewport_fov: f32,                 // Field of view in degrees

    // Statistics
    pub point_cloud_count: u32,            // Particles in current time window
    pub motif_count: u32,                  // Total motifs (23)
    pub viewport_fps: f32,                 // GPU rendering FPS
    pub gpu_memory_mb: f32,                // VRAM usage estimate
    pub window_anomaly_avg: f32,           // Average anomaly in visible window

    // GPU resources
    pub time_uniform_buffer: TimeUniformBuffer,
}
```

### UI Callback Wiring

```rust
// src/main.rs - Wire Slint callbacks to Rust state mutations

let analysis_ui = ui.global::<AnalysisWindow>();
let state_clone = state.clone();

// Timeline scrub (user dragged slider)
analysis_ui.on_on_timeline_scrub(move |time_normalized| {
    let state = state_clone.clone();
    tokio::spawn(async move {
        let mut st = state.lock().await;
        st.timeline_time_normalized = time_normalized.clamp(0.0, 1.0);
        st.timeline_playing = false;  // Stop animation when user scrubs

        // Update GPU time uniform
        st.time_uniform_buffer.update(&st.render_context.queue, time_normalized);

        // Compute display date
        let elapsed_days = time_normalized * 97.0;  // 97-day corpus
        let display_date = compute_date_from_days_offset(
            st.timeline_start_timestamp_micros,
            elapsed_days
        );
        st.timeline_date_display = display_date;

        eprintln!("[D.4] Timeline scrubbed to {:.2}% ({:?})", time_normalized * 100.0, display_date);
    });
});

// Play/pause toggle
analysis_ui.on_timeline_toggle_play(move || {
    let state = state_clone.clone();
    tokio::spawn(async move {
        let mut st = state.lock().await;
        st.timeline_playing = !st.timeline_playing;
        eprintln!("[D.4] Timeline play: {}", st.timeline_playing);
    });
});

// Jump to start
analysis_ui.on_timeline_start(move || {
    let state = state_clone.clone();
    tokio::spawn(async move {
        let mut st = state.lock().await;
        st.timeline_time_normalized = 0.0;
        st.time_uniform_buffer.update(&st.render_context.queue, 0.0);
        st.timeline_date_display = format_date(st.timeline_start_timestamp_micros);
    });
});

// Jump to end
analysis_ui.on_timeline_end(move || {
    let state = state_clone.clone();
    tokio::spawn(async move {
        let mut st = state.lock().await;
        st.timeline_time_normalized = 1.0;
        st.time_uniform_buffer.update(&st.render_context.queue, 1.0);
        st.timeline_date_display = format_date(st.timeline_end_timestamp_micros);
    });
});

// Viewport controls
analysis_ui.on_viewport_reset(move || {
    let state = state_clone.clone();
    tokio::spawn(async move {
        let mut st = state.lock().await;
        st.viewport_rotation_quat = [0.0, 0.0, 0.0, 1.0];  // Identity quaternion
        st.viewport_zoom = 1.0;
        st.viewport_pan_xy = (0.0, 0.0);
    });
});

// View presets
analysis_ui.on_viewport_top(move || {
    let state = state_clone.clone();
    tokio::spawn(async move {
        let mut st = state.lock().await;
        // Quaternion for 90° rotation around X axis (looking down)
        st.viewport_rotation_quat = [1.0, 0.0, 0.0, 0.0].normalize();
    });
});

analysis_ui.on_viewport_front(move || {
    let state = state_clone.clone();
    tokio::spawn(async move {
        let mut st = state.lock().await;
        st.viewport_rotation_quat = [0.0, 0.0, 0.0, 1.0];  // Identity
    });
});

analysis_ui.on_viewport_side(move || {
    let state = state_clone.clone();
    tokio::spawn(async move {
        let mut st = state.lock().await;
        // Quaternion for 90° rotation around Y axis (side view)
        st.viewport_rotation_quat = [0.0, 1.0, 0.0, 0.0].normalize();
    });
});
```

### Animation Loop (in main render loop)

```rust
// src/main.rs - In UI timer callback (every 16ms for 60 FPS)

let mut st = state.lock().await;

// Timeline animation
if st.timeline_playing {
    let dt = 0.016;  // Frame time (60 FPS)
    let speed_multiplier = st.timeline_play_speed;
    let increment = (dt / 97.0) * speed_multiplier / 86400.0;  // Normalized per-frame increment

    st.timeline_time_normalized = (st.timeline_time_normalized + increment).min(1.0);

    // Update GPU uniform
    st.time_uniform_buffer.update(&st.render_context.queue, st.timeline_time_normalized);

    // Update display date
    let elapsed_days = st.timeline_time_normalized * 97.0;
    st.timeline_date_display = compute_date_from_days_offset(
        st.timeline_start_timestamp_micros,
        elapsed_days
    );

    // Stop at end
    if st.timeline_time_normalized >= 1.0 {
        st.timeline_playing = false;
    }
}

// Auto-rotate viewport
if st.viewport_auto_rotate {
    let rotation_speed = 0.01;  // Radians per frame
    let (x, y, z, w) = quat_from_axis_angle([0.0, 1.0, 0.0], rotation_speed);
    st.viewport_rotation_quat = quat_multiply(st.viewport_rotation_quat, [x, y, z, w]);
}

// Update UI globals
ui_analysis.set_timeline_time_normalized(st.timeline_time_normalized);
ui_analysis.set_timeline_date_display(st.timeline_date_display.clone());
ui_analysis.set_timeline_playing(st.timeline_playing);
ui_analysis.set_point_cloud_count(st.point_cloud_count);
ui_analysis.set_viewport_fps(st.viewport_fps);
ui_analysis.set_gpu_memory_mb(st.gpu_memory_mb);
ui_analysis.set_window_anomaly_avg(st.window_anomaly_avg);
```

---

## GPU Integration: TimeUniformBuffer & Shader Filtering

### TimeUniformBuffer Definition (src/visualization/temporal_rewind.rs)

```rust
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TimeUniformData {
    pub time_offset_normalized: f32,  // [0.0, 1.0]
    pub _padding: [u32; 3],           // 16-byte alignment
}

pub struct TimeUniformBuffer {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl TimeUniformBuffer {
    pub fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let data = TimeUniformData {
            time_offset_normalized: 1.0,
            _padding: [0; 3],
        };

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Time Uniform Buffer"),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(
            "time_bind_group",
            layout,
            &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        );

        Self { buffer, bind_group }
    }

    pub fn update(&self, queue: &wgpu::Queue, time_normalized: f32) {
        let data = TimeUniformData {
            time_offset_normalized: time_normalized.clamp(0.0, 1.0),
            _padding: [0; 3],
        };
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[data]));
    }
}
```

### WGSL Compute Shader (src/visualization/gaussian_splatting.wgsl)

```wgsl
// Particle filtering by time window

@group(0) @binding(0) var<uniform> time_uniform: TimeUniform;
@group(0) @binding(1) var<storage, read> particles: array<Particle>;
@group(0) @binding(2) var<storage, read_write> output: array<atomic<u32>>;

struct TimeUniform {
    time_offset_normalized: f32,
}

struct Particle {
    position_xyz: vec3<f32>,
    intensity: f32,
    timestamp_norm: f32,  // Normalized [0.0, 1.0] within corpus
    motif_id: u32,
    confidence: f32,
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3u) {
    let particle_idx = global_id.x;
    if (particle_idx >= arrayLength(&particles)) { return; }

    let particle = particles[particle_idx];

    // Filter by time window: ±3 days around scrubbed time
    let window_size = 3.0 / 97.0;  // 3 days in 97-day corpus
    let time_distance = abs(particle.timestamp_norm - time_uniform.time_offset_normalized);

    if (time_distance > window_size) {
        return;  // Particle outside visible window, skip
    }

    // Particle is visible, accumulate statistics
    atomicAdd(&output[0u], 1u);  // Point count
}

@compute @workgroup_size(256)
fn gaussian_splat(
    @builtin(global_invocation_id) global_id: vec3u
) {
    let particle_idx = global_id.x;
    if (particle_idx >= arrayLength(&particles)) { return; }

    let particle = particles[particle_idx];

    // Filter by time window
    let window_size = 3.0 / 97.0;
    let time_distance = abs(particle.timestamp_norm - time_uniform.time_offset_normalized);
    if (time_distance > window_size) { return; }

    // Render Gaussian splat with time-based fade
    // Closer to edge of time window → lower opacity
    let fade = 1.0 - (time_distance / window_size);

    // Color by motif_id (23 distinct colors)
    let color = motif_color(particle.motif_id);

    // Gaussian: G(x,y,z) = intensity * exp(-0.5 * ||p - (x,y,z)||² / σ²)
    let sigma = 0.1;
    // ... render to framebuffer ...
}

fn motif_color(motif_id: u32) -> vec3<f32> {
    // 23 distinct colors via HSL rotation
    let hue = f32(motif_id % 23u) / 23.0;
    return hsv_to_rgb(hue, 1.0, 1.0);
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> vec3<f32> {
    let c = v * s;
    let h_prime = (h * 6.0) % 6.0;
    let x = c * (1.0 - abs((h_prime % 2.0) - 1.0));

    var rgb: vec3<f32>;
    if (h_prime < 1.0) { rgb = vec3<f32>(c, x, 0.0); }
    else if (h_prime < 2.0) { rgb = vec3<f32>(x, c, 0.0); }
    else if (h_prime < 3.0) { rgb = vec3<f32>(0.0, c, x); }
    else if (h_prime < 4.0) { rgb = vec3<f32>(0.0, x, c); }
    else if (h_prime < 5.0) { rgb = vec3<f32>(x, 0.0, c); }
    else { rgb = vec3<f32>(c, 0.0, x); }

    let m = v - c;
    return rgb + vec3<f32>(m);
}
```

---

## Mouse & Input Handling

### File Ownership

- **`src/main.rs`** - Add mouse/keyboard input callbacks

### Mouse Controls

```rust
// In Slint callback or winit event loop

ui.on_mouse_move(move |(x, y)| {
    if mouse_button_pressed {
        let state = state_clone.clone();
        tokio::spawn(async move {
            let mut st = state.lock().await;

            // Convert mouse delta to rotation
            let dx = (x - last_mouse_x) * 0.01;
            let dy = (y - last_mouse_y) * 0.01;

            // Update quaternion
            let quat_x = quat_from_axis_angle([1.0, 0.0, 0.0], -dy);
            let quat_y = quat_from_axis_angle([0.0, 1.0, 0.0], -dx);
            st.viewport_rotation_quat = quat_multiply(
                quat_multiply(st.viewport_rotation_quat, quat_x),
                quat_y
            );
        });
    }
});

ui.on_mouse_scroll(move |delta| {
    let state = state_clone.clone();
    tokio::spawn(async move {
        let mut st = state.lock().await;
        st.viewport_zoom *= 1.0 + (delta * 0.1);  // Zoom in/out
        st.viewport_zoom = st.viewport_zoom.clamp(0.1, 10.0);
    });
});
```

---

## Generation Protection Constraints

### ✅ DO

- **Time-normalized coordinates**: All timestamps converted to [0.0, 1.0] range
- **3-day time window**: Fixed window size for consistent history visibility
- **Smooth animation**: Play speed controlled by user, no hard-coded rates
- **GPU-side filtering**: Particle filtering in compute shader (fast)
- **Real-time feedback**: FPS counter shows performance impact
- **Motif coloring**: 23 distinct colors via HSL rotation for pattern identification

### ❌ DON'T

- **Unbounded time range**: Always normalize to corpus time span (97 days)
- **Blocking viewport updates**: All rotation/zoom async
- **CPU-side particle filtering**: Must happen on GPU (100M+ particles possible)
- **Hardcoded window size**: 3-day window should be configurable if needed
- **Clipping at time boundaries**: Fade effect (not harsh cutoff) as time window edge approached

---

## Implementation Checklist (for Jules)

### Phase 1: UI Components (15 min)
- [ ] Add time-scrub slider to ANALYSIS tab
- [ ] Add play/pause buttons
- [ ] Add timeline date display
- [ ] Add viewport controls (rotate, zoom, pan)
- [ ] Add statistics display (point count, FPS, memory)
- [ ] Verify Slint compiles

### Phase 2: State Extensions (10 min)
- [ ] Add timeline_* fields to AppState
- [ ] Add viewport_* fields to AppState
- [ ] Add statistics fields to AppState
- [ ] Add TimeUniformBuffer to AppState
- [ ] Initialize with 97-day corpus time range

### Phase 3: Callback Wiring (20 min)
- [ ] Wire on_timeline_scrub() callback
- [ ] Wire timeline_toggle_play() callback
- [ ] Wire timeline_start() / timeline_end()
- [ ] Wire viewport_reset() / preset views
- [ ] Tests: Callbacks execute, state updates correctly

### Phase 4: Animation Loop (15 min)
- [ ] Implement timeline animation in UI timer
- [ ] Update time uniform buffer per frame
- [ ] Update display date calculation
- [ ] Implement auto-rotate if enabled
- [ ] Update Slint globals (FPS, point count, etc.)

### Phase 5: GPU Integration (20 min)
- [ ] Create/update TimeUniformBuffer
- [ ] Bind time uniform in compute shader
- [ ] Implement particle time-window filtering
- [ ] Implement motif-based color mapping (HSV)
- [ ] Tests: Compute shader filters correctly, colors distinct

### Phase 6: Mouse Input (10 min)
- [ ] Wire mouse move → viewport rotation
- [ ] Wire mouse scroll → viewport zoom
- [ ] Verify quaternion rotation smooth and intuitive
- [ ] Tests: Rotation/zoom feel responsive

### Phase 7: Integration Testing (10 min)
- [ ] Cargo build → 0 errors
- [ ] Cargo run → ANALYSIS tab appears
- [ ] Drag time slider → particles scroll through time
- [ ] Click play → timeline animates forward
- [ ] Rotate view with mouse → 3D rotation smooth (60+ FPS)
- [ ] Verify color distinct per motif

---

## Total Duration

| Task | Time |
|------|------|
| Phase 1: UI components | 15 min |
| Phase 2: State extensions | 10 min |
| Phase 3: Callback wiring | 20 min |
| Phase 4: Animation loop | 15 min |
| Phase 5: GPU integration | 20 min |
| Phase 6: Mouse input | 10 min |
| Phase 7: Testing | 10 min |
| **Total** | **100 min** |

*Estimated 60-90 min with concurrent work*

---

## Verification & Success Criteria

✅ **Time-scrub fully live**:
- Slider moves from 0.0 → 1.0 (start → now)
- Particles filter dynamically as user scrubs
- Display date updates correctly

✅ **Animation loop smooth**:
- Play button starts forward time progression
- Pause button stops animation
- Speed slider adjusts animation rate
- Auto-rotate spins 3D view smoothly

✅ **Viewport controls responsive**:
- Mouse drag rotates 3D view
- Scroll wheel zooms in/out
- View presets (Top/Front/Side) work correctly
- Reset view returns to default camera

✅ **Real-time feedback accurate**:
- FPS counter shows GPU performance (target 60+ FPS)
- Point count reflects time window size (~20k-50k particles per window)
- Anomaly average shown for visible window

✅ **Motif coloring distinct**:
- 23 motifs each have unique, distinct color
- Colors don't clip/overflow
- HSV rotation produces perceptually uniform hues

---

## Notes for Jules

This addendum connects the GPU infrastructure (point cloud, Gaussian splatting) with the UI timeline control. The key design decision is to filter particles on the GPU (in compute shader) rather than CPU—this allows seamless handling of 100M+ particle histories without stalling.

**Key insight**: The 3-day time window is fixed by design. At 100 frames per second (97 days / ~8.4M frames), a 3-day window shows ~259k frames of history. This balances temporal detail (see minute-to-minute changes) with spatial density (particles don't become too sparse).

Time normalization is critical: all timestamps converted to [0.0, 1.0] range at corpus creation time. This decouples the timeline UI from absolute clock time—the slider always represents "fraction of total history" regardless of whether the corpus is 1 day or 1 year.

---

## Future Enhancements (Post-DD)

- **Time window size control**: Slider to adjust ±N days visible at once
- **Cluster isolation**: Click motif in legend to show only that cluster
- **Time markers**: Event annotations on timeline (e.g., "Attack Pattern X Detected")
- **Statistics overlay**: Real-time heatmap of anomaly intensity over time
- **Spherical projection**: Option to project 3D cloud onto sphere for 360° view

