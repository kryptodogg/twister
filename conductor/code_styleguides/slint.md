# Slint Style Guide

## Purpose
Define Slint UI coding standards for Project Twister, focusing on 60Hz+ refresh rates, minimal CPU overhead, and GPU-accelerated rendering via FemtoVG.

## Core Principles
1. **Pull-Based Reactivity**: Let Slint timer pull state at 60Hz (no push from DSP thread)
2. **Zero Allocation in Timer Callback**: Pre-allocate all strings/buffers
3. **GPU-Accelerated Rendering**: Use FemtoVG backend for WGPU integration
4. **Responsive Layouts**: Use stretch factors, not fixed sizes (1080p → 4K scaling)

## Timer-Based UI Updates

### Correct Pattern (Pull-Based)
```rust
// main.rs
let ui_weak = ui.as_weak();
let state_ui = state.clone();
let timer = slint::Timer::default();

timer.start(slint::TimerMode::Repeated, Duration::from_millis(16), move || {
    if let Some(ui) = ui_weak.upgrade() {
        // Pull atomics (no locking, no allocation)
        ui.set_detected_freq(state_ui.get_detected_freq());
        ui.set_is_running(state_ui.running.load(Ordering::Relaxed));

        // Pre-allocated string buffers
        ui.set_anc_status(anc_status_string.clone());
    }
});
```

### Incorrect Pattern (Push-Based)
```rust
// ❌ BAD: DSP thread pushing to UI (causes queue flooding)
while running.load() {
    let frame = process_frame();

    // Floods Slint event queue → UI lockup
    let _ = slint::invoke_from_event_loop(move || {
        ui.set_detected_freq(frame.freq);
    });
}
```

## Component Standards

### Property Naming
```slint
// ✅ GOOD: kebab-case for properties
in-out property <float> detected-freq;
in-out property <string> anc-status;

// ❌ BAD: snake_case (not idiomatic Slint)
in-out property <float> detected_freq;
```

### Callback Naming
```slint
// ✅ GOOD: verb-first, imperative mood
callback set-mode(int);
callback toggle-running();
callback anc-calibrate();

// ❌ BAD: noun-first or passive
callback mode-change(int);
callback running-toggle();
```

## Layout Patterns

### Responsive Design (1080p → 4K)
```slint
// ✅ GOOD: Stretch factors for scaling
HorizontalBox {
    VerticalBox {
        horizontal-stretch: 1; // 25% width
        // Controls panel
    }
    VerticalBox {
        horizontal-stretch: 3; // 75% width
        // Visualizations (spectrum, waterfall)
    }
}

// ❌ BAD: Fixed widths (breaks on 4K)
HorizontalBox {
    Rectangle { width: 340px; } // Fixed width
    Rectangle { width: 1020px; }
}
```

### Glassmorphic Panels
```slint
component GlassPanel inherits Rectangle {
    background: rgba(255, 255, 255, 0.65);
    border-color: rgba(255, 255, 255, 0.9);
    border-width: 1px;
    border-radius: 12px;
    drop-shadow-blur: 16px;
    drop-shadow-color: rgba(0, 0, 0, 0.05);
    drop-shadow-offset-y: 4px;
}
```

## Performance Optimization

### SVG Paths for Waveforms
```slint
// ✅ GOOD: Single Path element (GPU-accelerated)
Path {
    width: 100%;
    height: 100%;
    stroke: Palette.phosphor;
    stroke-width: 2px;
    commands: waveform-path; // "M 0 50 L 100 50 ..."
}

// ❌ BAD: Many Rectangle elements (CPU-bound)
for bar[i] in spectrum-bars: Rectangle {
    // 256 rectangles = 256 draw calls
}
```

### Waterfall Rendering
```slint
// ✅ GOOD: SharedPixelBuffer (zero-copy from Rust)
in-out property <[color]> waterfall-pixels;

for pixel[i] in waterfall-pixels: Rectangle {
    x: mod(i, 128) * (parent.width / 128);
    y: (i / 128) * 1px;
    background: pixel;
}
```

## Color Palette Standards

### Global Palette
```slint
global Palette {
    out property <color> bg:          #f0f2f5;
    out property <color> panel:       rgba(255, 255, 255, 0.65);
    out property <color> border:      rgba(255, 255, 255, 0.9);
    out property <color> text_hi:     #1e1e24;
    out property <color> text_lo:     #6a6a80;
    out property <color> phosphor:    #007aff;
    out property <color> amber:       #ff9500;
    out property <color> red:         #ff3b30;
    out property <color> green:       #34c759;
}
```

### Usage Pattern
```slint
Text {
    text: "DETECTED";
    color: Palette.text_lo; // Consistent theming
    font-size: 11px;
}
```

## Animation Standards

### Smooth Transitions (60ms budget)
```slint
Rectangle {
    background: active ? Palette.phosphor : Palette.border;

    // Smooth color transition
    animate background { duration: 60ms; }
    animate width { duration: 40ms; } // Meter animation
}
```

### Compass Needle Animation
```slint
Rectangle {
    // Needle rotation via trigonometry (no transform support)
    property <float> rad: (beam-azimuth-deg - 90) * 3.14159 / 180.0;
    x: 28px + 16px * cos(rad * 1rad) - 1px; // Convert to angle type
    y: 28px + 16px * sin(rad * 1rad) - 10px;

    animate x, y, background { duration: 120ms; }
}
```

## Accessibility

### Font Sizes
```slint
// Minimum 10px for readability
Label { font-size: 11px; } // Body text
Value { font-size: 13px; font-weight: 700; } // Emphasized
Text { font-size: 32px; font-weight: 800; } // Large display
```

### Color Contrast
```slint
// ✅ GOOD: High contrast (WCAG AA compliant)
Text {
    text: "ACTIVE";
    color: Palette.red; // #ff3b30 on #f0f2f5 = 4.5:1
}

// ❌ BAD: Low contrast
Text {
    text: "status";
    color: Palette.text_lo; // #6a6a80 on #141418 = 2.1:1
}
```

## Testing Standards

### Visual Regression
```rust
#[test]
fn test_ui_properties() {
    let ui = AppWindow::new().unwrap();

    // Set properties
    ui.set_detected_freq(440.0);
    ui.set_is_running(true);

    // Verify bindings
    assert_relative_eq!(ui.get_detected_freq(), 440.0);
    assert!(ui.get_is_running());
}
```

### Performance Testing
```rust
#[test]
fn test_timer_callback_latency() {
    let start = Instant::now();

    // Simulate 60 timer callbacks (1 second at 60Hz)
    for _ in 0..60 {
        timer_callback();
    }

    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_millis(100),
        "Timer callback took {:?} (budget: 100ms for 60 calls)", elapsed);
}
```

## Common Patterns

### DbMeter Component
```slint
component DbMeter inherits Rectangle {
    in property <float> level_db;
    in property <color> bar_color: Palette.phosphor;
    in property <float> floor_db: -60.0;
    in property <float> peak_db: 0.0;

    height: 12px;
    background: rgba(0, 0, 0, 0.05);
    border-radius: 6px;
    clip: true;

    property <float> frac: clamp((level_db - floor_db) / (peak_db - floor_db), 0.0, 1.0);

    Rectangle {
        width: parent.width * frac;
        height: parent.height;
        background: bar_color;
        animate width { duration: 50ms; }
    }
}
```

### ModeButton Component
```slint
component ModeButton inherits Rectangle {
    in property <string> label;
    in property <bool> active;
    in property <color> active_color: Palette.phosphor;
    callback clicked;

    width: 64px;
    height: 28px;
    border-radius: 6px;
    background: active ? active_color.with-alpha(0.15) : rgba(255, 255, 255, 0.8);
    border-width: active ? 2px : 1px;
    border-color: active ? active_color : rgba(0, 0, 0, 0.05);

    Text {
        text: label;
        color: active ? active_color : Palette.text_lo;
        font-size: 11px;
        font-weight: active ? 700 : 500;
        horizontal-alignment: center;
        vertical-alignment: center;
    }

    TouchArea { clicked => { root.clicked(); } }
}
```

## References
- [Slint Documentation](https://slint.dev/docs/)
- [Slint Language Reference](https://slint.dev/docs/slint)
- [FemtoVG Renderer](https://docs.rs/femtovg/latest/femtovg/)
