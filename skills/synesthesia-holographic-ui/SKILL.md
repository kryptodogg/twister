# Synesthesia Holographic UI Skill

A2UI → Slint conversion, hologram shaders, MCP integration, real-time
visualization, adaptive frequency labels, dBFS metering, spectrum display.

## Domain
- Slint UI framework (components, properties, callbacks)
- Waterfall visualization (log-scale frequency, colormap)
- Spectrum bars (GPU-computed FFT display)
- dBFS metering (AGC input, output normalization)
- TDOA beam indicator (azimuth arc, confidence)
- Adaptive frequency labels (Hz/kHz/MHz auto-format)
- Degraded mode indicators (DB connection status)

## Trigger Patterns
"Slint", "UI", "waterfall", "spectrum", "meter", "visualization",
"hologram", "display", "frontend", "app.slint"

## Available Functions
- `create_app_window()` — Slint window initialization
- `update_waterfall()` — RGBA pixel buffer push
- `update_spectrum()` — Bar graph update
- `update_meters()` — AGC/output dBFS displays
- `show_degraded_mode()` — DB status indicators
- `format_frequency()` — Adaptive Hz/kHz/MHz labels

## UI Components

### DbMeter
```slint
component DbMeter inherits Rectangle {
    in property <float> level_db;
    in property <color> bar_color;
    in property <float> floor_db: -60.0;
    in property <float> peak_db: 0.0;
    // Renders bar at normalized height
}
```

### ModeButton
```slint
component ModeButton inherits Rectangle {
    in property <string> label;
    in property <bool> active;
    in property <color> active_color;
    callback clicked;
}
```

## Color Palette
| Name | Hex | Usage |
|------|-----|-------|
| bg | #0a0a0f | Background |
| panel | #111118 | Panels |
| border | #222233 | Borders |
| phosphor | #00ff88 | Primary |
| amber | #ffaa00 | Warnings |
| cyan | #00ccff | Info |
| red | #ff3355 | Active/Stop |
| pink | #ff44aa | PDM indicator |

## Code Patterns

### Adaptive Frequency Formatting
```rust
fn format_freq(hz: f32) -> String {
    if hz >= 1_000_000.0 { format!("{:.3} MHz", hz / 1e6) }
    else if hz >= 1_000.0 { format!("{:.1} kHz", hz / 1e3) }
    else { format!("{:.1} Hz", hz) }
}
```

### Waterfall Pixel Update
```rust
ui.set_waterfall_pixels(
    Rc::new(VecModel::from(rgba_colors))
);
```
