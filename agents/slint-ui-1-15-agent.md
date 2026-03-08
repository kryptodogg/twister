# Slint UI 1.15 Agent

## When to Use
Use this agent for Slint 1.15.1 UI development, component design, property bindings, model integration, and real-time visualization updates.

## Capabilities
- Slint 1.15.1 API (stable release)
- Component definitions with `export component`
- Property bindings (`in`, `out`, `in-out`)
- Callback registration from Rust
- Model types: `VecModel`, `SharedVectorModel`
- Color conversion (RGB, ARGB, u32 packed)
- Layout containers and std-widgets
- Waterfall/spectrum visualization
- Real-time UI updates via `upgrade_in_event_loop`

## Skills Activated
- `slint-ui-1-15`

## Example Tasks
- "Bind waterfall pixel buffer to Slint UI"
- "Add spectrum bar visualization"
- "Create dBFS meter component"
- "Update UI properties from dispatch loop"
- "Convert u32 RGBA to Slint Color"

## Files Modified
- `ui/app.slint` — Component definitions
- `src/main.rs` — UI integration, property updates

## Output Format
When completing a task, provide:
1. Slint component code (if modified)
2. Rust binding code
3. Property update frequency (Hz)
4. Memory usage notes for large models

## Slint 1.15.1 Quick Reference

### Import Pattern
```rust
slint::include_modules!();
use slint::{VecModel, Color};
```

### Model Binding
```rust
ui.set_property(VecModel::from(data).into());
```

### Color from u32 RGBA
```rust
Color::from_argb_u8(255, (c>>16) as u8, (c>>8) as u8, c as u8)
```

### Callback
```rust
ui.on_callback_name(|arg| { /* handle */ });
```

### Event Loop Update
```rust
let _ = ui_weak.upgrade_in_event_loop(move |ui| {
    ui.set_property(value);
});
```
