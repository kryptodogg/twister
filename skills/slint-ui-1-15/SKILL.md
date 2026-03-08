---
name: Slint UI 1.15.1+ (FemtoVG/WGPU)
description: >
  Mandatory reference when editing .slint files or Slint Rust integration code.
  Covers reactivity, rendering backends, property binding rules, image upload patterns,
  and wgpu compute shader integration for custom visualizations.
---

# Slint UI Skill (v1.15.1+)

Expert knowledge for the Slint UI framework with FemtoVG (wgpu) rendering backend.

## ⚠️ ALWAYS APPLY WHEN

- Editing **any `.slint` file**
- Writing Rust code that calls `slint::` APIs
- Creating custom visualizations (waterfall, spectrum, charts)
- Working with `SharedPixelBuffer`, `Image`, or pixel data
- Integrating wgpu compute shaders with Slint display

---

## 1. Rendering Backend: FemtoVG (WGPU)

Slint 1.15.1 with `renderer-femtovg` uses **wgpu** internally (not OpenGL).

| Backend Name | GPU API | Cargo Feature |
|---|---|---|
| `winit-femtovg-wgpu` | Metal, Direct3D, Vulkan via [wgpu.rs](http://wgpu.rs) | `renderer-femtovg` |
| `winit-skia-opengl` | OpenGL | `renderer-skia-opengl` |
| `winit-software` | CPU only | `renderer-software` |

```toml
# Cargo.toml — This project's configuration
[dependencies]
slint = { version = "1.15.1", features = ["renderer-femtovg", "backend-winit"] }

[build-dependencies]
slint-build = "1.15.1"
```

> **Key insight**: FemtoVG and your wgpu compute shaders both use wgpu, but on
> separate device instances. No resource conflicts. You can safely run multiple
> wgpu devices for compute (waterfall, ridge plot, etc.) alongside FemtoVG rendering.

### Environment Variables
- `SLINT_BACKEND=winit-femtovg-wgpu` — Force FemtoVG renderer
- `SLINT_FULLSCREEN=1` — Start fullscreen

---

## 2. Reactivity — Core Concepts

Slint uses **fine-grained reactivity**. Components update in place — they are
**never destroyed and recreated** (unlike React.js).

### Rules

1. **All properties are reactive by default.** No opt-in needed.
2. **Bindings are expressions**, not assignments. `x: other.value;` creates a
   live dependency — `x` updates whenever `other.value` changes.
3. **Imperative assignment breaks bindings.** `foo.bar = 42;` from Rust or
   `.slint` code breaks any prior binding expression. The property becomes static.
4. **Two-way bindings** (`<=>`) keep two properties in sync without breaking.
5. **Purity**: Binding expressions **must be pure** — no side effects.
   Use the `pure` keyword for callbacks/functions used in bindings.
6. **Lazy evaluation**: Dependencies are re-evaluated only when queried.

### Examples

```slint
// Reactive binding — auto-updates when ta.mouse-x changes
myRect := Rectangle {
    x: ta.mouse-x;
    background: ta.pressed ? orange : skyblue;
}

// Two-way binding — synced in both directions
input := TextInput {
    text <=> root.thing.name;
}

// Pure function — safe to use in bindings
pure function lengthToInt(n: length) -> int {
    return (n / 1px);
}
```

### Anti-Patterns

| ❌ Wrong | ✅ Correct |
|---|---|
| Setting a bound property from Rust imperatively | Use `<=>` or `changed` callback |
| Calling non-pure functions in bindings | Mark functions `pure` |
| Destroying/recreating components for state changes | Let Slint's reactivity handle updates |
| Using `useMemo`/`useCallback` patterns | Not needed — Slint tracks dependencies automatically |

---

## 3. Rust ↔ Slint Integration

### Setup
```rust
slint::include_modules!();
use slint::{VecModel, Color, Brush, Image, Rgba8Pixel, SharedPixelBuffer};
```

### Component Lifecycle
```rust
let ui = AppWindow::new()?;     // Create
let ui_weak = ui.as_weak();     // Weak ref for closures
ui.run()?;                       // Blocks until window closes
```

### Property Updates
```rust
// Direct set (breaks any .slint binding on this property!)
ui.set_my_value(42.0);

// From background thread (safe cross-thread)
ui_weak.upgrade_in_event_loop(move |ui| {
    ui.set_my_value(42.0);
});
```

### Timers (UI refresh loop)
```rust
let timer = slint::Timer::default();
timer.start(
    slint::TimerMode::Repeated,
    std::time::Duration::from_millis(16),  // ~60fps
    move || {
        let ui = ui_weak.unwrap();
        // Read shared state, update UI properties
        ui.set_spectrum_path(path.as_str().into());
    },
);
```

---

## 4. Image Upload Pattern (SharedPixelBuffer)

The standard pattern for uploading GPU-computed or CPU-rendered pixel data to Slint:

```rust
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

// Create buffer
let mut pixels = SharedPixelBuffer::<Rgba8Pixel>::new(width, height);
let dst = pixels.make_mut_slice();

// Fill from packed u32 RGBA (R in LSB)
for (i, px) in dst.iter_mut().enumerate() {
    let src = rgba_data[i];
    px.r = (src & 0xFF) as u8;
    px.g = ((src >> 8) & 0xFF) as u8;
    px.b = ((src >> 16) & 0xFF) as u8;
    px.a = 255;
}

// Upload to Slint
ui.set_my_image(Image::from_rgba8(pixels));
```

### In .slint
```slint
export component AppWindow {
    in property <image> my-image;

    Image {
        source: my-image;
        image-fit: fill;  // or contain, cover
    }
}
```

> **Performance**: FemtoVG bilinearly interpolates the image to fill the element.
> Render at a lower resolution (e.g. 640×420) and let FemtoVG upscale smoothly.

---

## 5. SVG Path Pattern (Spectrum Lines)

For line-based visualizations, generate SVG path command strings in Rust:

```rust
let mut path = String::with_capacity(6000);
path.push_str("M 0 100 ");  // Move to start
for (i, val) in spectrum.iter().enumerate() {
    let x = i as f32 * x_step;
    let y = 100.0 - val * 100.0;
    write!(path, "L {x:.1} {y:.1} ").unwrap();
}
path.push_str("L 256 100 Z");  // Close path

ui.set_spectrum_path(path.as_str().into());
```

```slint
Path {
    commands: spectrum-path;
    stroke: cyan;
    stroke-width: 1.5px;
}
```

---

## 6. WGPU Compute Shader Integration

For GPU-accelerated visualizations, use a **separate wgpu device** for compute,
render to a buffer, readback, and upload as `SharedPixelBuffer`:

```
┌──────────────┐    ┌───────────────┐    ┌──────────────────┐
│ wgpu Compute │───>│ Readback      │───>│ SharedPixelBuffer│
│ (own device) │    │ buffer→CPU    │    │ → Slint Image    │
└──────────────┘    └───────────────┘    └──────────────────┘
       ↑                                         │
  Storage buffers                          FemtoVG (wgpu)
  (spectrum data)                          renders to screen
```

### Pattern
1. Create a **separate** wgpu device for compute (not Slint's internal device)
2. Upload data via `queue.write_buffer()`
3. Dispatch compute shader
4. `copy_buffer_to_buffer()` → readback buffer
5. `slice.map_async()` → read pixels
6. Convert to `SharedPixelBuffer<Rgba8Pixel>` → `Image::from_rgba8()`

### Shader Output Format
Pack RGBA as `u32` with R in LSB:
```wgsl
fn pack_rgba(c: vec4<f32>) -> u32 {
    let r = u32(clamp(c.x * 255.0, 0.0, 255.0));
    let g = u32(clamp(c.y * 255.0, 0.0, 255.0));
    let b = u32(clamp(c.z * 255.0, 0.0, 255.0));
    let a = u32(clamp(c.w * 255.0, 0.0, 255.0));
    return r | (g << 8u) | (b << 16u) | (a << 24u);
}
```

---

## 7. Common Error Patterns

| Wrong | Correct |
|---|---|
| `Rgba8Pixel::new(r,g,b,a)` | Set fields: `px.r = r; px.g = g; ...` |
| `pixels.make_mut_bytes()` | `pixels.make_mut_slice()` returns `&mut [Rgba8Pixel]` |
| `slint::Model` (trait) | `slint::VecModel` (concrete) |
| `Color` from wrong module | `slint::Color` |
| `set_property(Rc::new(...))` | `set_property(VecModel::from(...).into())` |
| CPU rendering with tiny-skia in UI timer | May block FemtoVG; prefer wgpu compute on dispatch thread |
| Sharing Slint's wgpu device | Create separate device; FemtoVG owns its wgpu internally |

---

## 8. Project-Specific Conventions

- **UI placement**: Examples in `ui/examples/`, tests in `ui/tests/`
- **Renderer**: FemtoVG (wgpu) — target iOS/macOS responsiveness
- **`.cargo/config.toml`**: Contains Slint optimizations and runners
- **Style**: Native Slint style by default
- **build.rs**: `slint_build::compile("ui/app.slint")`

## References
- [Slint Reactivity](https://docs.slint.dev/latest/docs/slint/guide/language/concepts/reactivity/)
- [Reactivity vs React.js](https://docs.slint.dev/latest/docs/slint/guide/language/concepts/reactivity-vs-react/)
- [Winit Backend / Renderers](https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backend_winit/)
- [Slint Docs](https://slint.dev/docs)
