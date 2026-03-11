# Slint Waveshaper Metrics UI Integration Guide

## Overview

The Slint metrics UI provides real-time visualization of Mamba neural inference, displaying:
- Live threat metrics (anomaly score with color-coded threat levels)
- Waveform oscilloscope visualization
- 128D latent embedding activity (Drive, Foldback, Asymmetry)
- Neural Auto-Steer toggle (AI vs manual control)
- Parameter sliders for waveshaping control
- Frame and sample counters

## Architecture

```
Tokio Dispatch Loop (100 Hz)
    ├─ Audio processing (C925e @ 32kHz)
    ├─ RF processing (SDR @ 6.144MHz)
    ├─ Mamba inference
    └─ Update Arc<Mutex<MetricsState>>
            ↓
Slint UI (60 FPS render)
    ├─ Timer polls metrics
    ├─ Updates reactive state
    └─ Renders live visualization
```

## Integration Steps

### 1. Add Slint Component to Main App

In `ui/app.slint`:

```slint
import { WaveshaperMetrics } from "waveshaper_metrics.slint";

export component AppWindow {
    // ... existing UI ...

    Tab {
        title: "Metrics";
        WaveshaperMetrics {
            // Metrics will be passed via binding
        }
    }
}
```

### 2. Create Metrics Module in Rust

Already provided in `src/ui/waveshaper_metrics.rs`:

```rust
use crate::ui::waveshaper_metrics::{MetricsState, setup_waveshaper_ui};
```

### 3. Wire Dispatch Loop to Metrics

In `src/main.rs`:

```rust
// Create shared metrics state
let metrics_state = Arc::new(Mutex::new(MetricsState::default()));
let metrics_clone = metrics_state.clone();

// Spawn dispatch loop
tokio::spawn(async move {
    let mut tick = interval(Duration::from_millis(10)); // 100 Hz

    loop {
        tick.tick().await;

        // ... perform signal processing ...

        // Update metrics
        let current_metrics = MetricsState {
            anomaly_score: /* from Mamba */,
            drive: /* from latent projection */,
            foldback: /* from latent projection */,
            asymmetry: /* from latent projection */,
            frame_index: frame_count as i32,
            total_samples: sample_count as i32,
            is_connected: true,
            auto_steer: true, // Or from UI state
        };

        if let Ok(mut guard) = metrics_clone.lock().await {
            *guard = current_metrics;
        }
    }
});
```

### 4. Create UI Timer for 60 FPS Updates

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

let timer = slint::Timer::default();
let metrics_state_clone = metrics_state.clone();

timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(16), move || {
    // Read metrics (non-blocking read in Slint context)
    let metrics = metrics_state_clone.clone();

    // Update UI bindings
    if let Some(ui_handle) = ui_weak.upgrade() {
        // Use tokio::runtime to get current metrics
        // Note: In Slint callbacks, you may need to use spawn_blocking or similar
    }
});
```

## Metric Calculation Examples

### Anomaly Score (from Mamba)
```rust
// From src/mamba.rs forward pass
let reconstruction_error = /* MSE of autoencoder */;
let anomaly_score = (reconstruction_error / max_loss).clamp(0.0, 1.0);
```

### Latent Projection (Mamba → Waveshaper Parameters)
```rust
// From 128D latent embedding
let latent: Vec<f32> = mamba.forward(&input);

// Pool to 3 parameters
let drive = latent[0..32].iter().sum::<f32>() / 32.0; // [0..31]
let foldback = latent[32..64].iter().sum::<f32>() / 32.0; // [32..63]
let asymmetry = latent[64..96].mean() * 2.0 - 1.0; // [64..95] → [-1, 1]
```

### Frame Index Tracking
```rust
let frame_index = dispatch_frame_counter;
let total_samples = dispatch_frame_counter * samples_per_frame;
```

## UI State Synchronization

The Slint component defines a `MetricsState` struct that maps directly to the Rust type:

```slint
export struct MetricsState {
    anomaly_score: float,      // 0.0-1.0
    drive: float,              // 0.0-1.0
    foldback: float,           // 0.0-1.0
    asymmetry: float,          // -1.0-1.0
    frame_index: int,
    total_samples: int,
    is_connected: bool,
    auto_steer: bool,
}
```

### Reactive Updates

Color changes are automatic via Slint's computed properties:

```slint
property <bool> is_attack: metrics.anomaly_score > 0.5;
property <color> threat_color: is_attack ? #dc2626ff : #16a34aff;
```

When `metrics.anomaly_score` changes, the color instantly updates.

## Performance Considerations

### Lock Contention
- **Dispatch Loop**: Holds lock for ~1ms per 10ms cycle (10% utilization)
- **UI Timer**: Holds lock for ~2ms per 16ms cycle (12.5% utilization)
- **Total**: ~22% lock overhead (acceptable for 100 Hz metrics)

### Memory Usage
- `MetricsState`: 32 bytes (4×f32 + 2×i32 + 2×bool)
- `Arc<Mutex<>>`: ~64 bytes
- Total: ~100 bytes per active metrics stream

### Frame Drops
- Slint timer runs at 60 FPS (16ms interval)
- Dispatch loop at 100 Hz (10ms interval)
- 60 FPS fully supported (6 dispatch frames per UI frame)

## Customization

### Add Latent Activity Visualization

To display actual latent embeddings instead of pooled values:

```slint
// In waveshaper_metrics.slint
export struct WaveshapeEngine {
    // ... existing ...
    in-out property <[float]> latent-embedding: [];  // 128 values
}

// In Rust:
metrics_state.latent_activity = Some(mamba.latent.clone());
```

### Add Waveform Data Streaming

To render actual oscilloscope waveform:

```rust
// Store waveform snapshot
let waveform: Vec<f32> = current_audio_buffer
    .iter()
    .step_by(buffer_size / 100)  // Downsample to 100 points
    .copied()
    .collect();

// Would need to extend MetricsState or create WaveformState
```

### Connect Manual Parameter Control

To allow users to adjust Drive/Foldback/Asymmetry via sliders:

```rust
// In main.rs, bind slider changes to waveshaper
if let Some(ui) = app_window.upgrade() {
    // Listen for parameter changes from UI
    ui.on_drive_changed(|new_drive| {
        // Update waveshaper with new drive value
    });
}
```

## Testing

### Mock Metrics Generator

For testing without backend:

```rust
fn generate_test_metrics(frame: i32) -> MetricsState {
    let time = (frame as f32) / 100.0;
    let is_attack = (time * 0.2).sin() > 0.0;

    MetricsState {
        anomaly_score: if is_attack {
            0.7 + (time * 5.0).cos() * 0.2
        } else {
            0.1 + rand::random::<f32>() * 0.05
        },
        drive: if is_attack { 0.8 } else { 0.0 },
        foldback: if is_attack { 0.6 } else { 0.0 },
        asymmetry: 0.5 + (time * 1.5).sin() * 0.3,
        frame_index: frame,
        total_samples: (frame * 224) as i32,
        is_connected: true,
        auto_steer: true,
    }
}
```

### Unit Tests

```rust
#[tokio::test]
async fn test_metrics_update_speed() {
    let metrics = Arc::new(Mutex::new(MetricsState::default()));
    let start = Instant::now();

    for i in 0..10_000 {
        let mut guard = metrics.lock().await;
        guard.anomaly_score = (i as f32) / 10_000.0;
    }

    let elapsed = start.elapsed();
    println!("10k updates in {:?}", elapsed);
    assert!(elapsed < Duration::from_millis(100));
}
```

## Troubleshooting

### UI Not Updating
1. Verify dispatch loop is writing to `metrics_state`
2. Check Slint timer is running every 16ms
3. Confirm `MetricsState` struct matches Slint definition

### Flickering/Jitter
1. Reduce lock contention (avoid nested locks)
2. Ensure dispatch loop cycle time is consistent
3. Use atomic operations for simple value updates

### Performance Degradation
1. Profile with `perf` or similar (check lock wait times)
2. Verify UI timer not being blocked by other tasks
3. Consider reducing update frequency if needed

## Migration Path (React → Slint)

If you have the React metrics applet:

1. **Slint is native**: No WebSocket overhead
2. **Direct state binding**: No JSON serialization
3. **Better performance**: Native Rust + compiled Slint
4. **Easier integration**: Access to async Rust ecosystem

To migrate:
- Keep React for web dashboard
- Use Slint for native desktop app
- Both can read from same `Arc<Mutex<MetricsState>>`

## Future Enhancements

- [ ] Waveform oscilloscope with actual audio rendering
- [ ] Chart timeline (Slint doesn't have charting, but can use Canvas)
- [ ] Network latency indicator
- [ ] Recording/playback of metrics history
- [ ] Export metrics to CSV/JSON
- [ ] Multi-threaded metrics aggregation
