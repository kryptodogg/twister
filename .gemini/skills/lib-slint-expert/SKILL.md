---
name: lib-slint-expert
description: Comprehensive Slint GUI development expert. Covers Rust integration (v1.15.1), component design, layout primitives, and performance optimization for WGPU/RDNA2. Incorporates Apple Style Guide and React.js design specialties.
---

# Slint GUI Expert (Cyclone Edition)

This skill provides expert guidance for building high-performance, declarative UIs with Slint and Rust, optimized for the Cyclone project's RDNA2/WGPU stack.

## 🎯 Primary Sources of Truth
1.  **Comprehensive Manual:** `docs/stack/slint/slint-agent-manual.md`
2.  **Specialized Reference Index:** `docs/stack/slint/reference/`
3.  **Official Rust API:** [https://docs.slint.dev/latest/docs/rust/slint/](https://docs.slint.dev/latest/docs/rust/slint/)

## 🛠️ Core Engineering Principles

### 1. Declarative "Homeowner" Philosophy
- **Describe Relationships:** Use `property: expression;` to link elements. Do not "do" things in callbacks that can be described with bindings.
- **Fine-Grained Reactivity:** Treat properties like spreadsheet formulas. Bindings are lazy and pure.
- **Preserve Bindings:** Avoid imperative assignments (`property = value;`) in callbacks as they destroy reactivity. Use state-based logic instead: `x: override ? 10px : calculated_x;`.

### 2. Rust Integration (The Bridge)
- **`slint::include_modules!()` Requirement:** Every `.slint` file **must** be listed in `build.rs` via `slint_build::compile()`. Call the macro without arguments in Rust.
- **Kebab to Snake:** Slint `my-property` becomes Rust `get_my_property()` and `set_my_property()`.
- **Memory Safety:** Use `ui.as_weak()` when passing UI handles into closures to prevent circular reference leaks.
- **Thread Safety:** Use `slint::invoke_from_event_loop` to update the UI from background worker threads.

## 🎨 UI Design Guidelines (Apple & React Style)

### a. Consistency & Terminology (Apple Style)
- **Capitalization:** Use **Sentence-style capitalization** for most UI text (e.g., "Add to library"). Use **Title-style** only for headers or when matching specific platform conventions.
- **Verbs:** Use `click` for mouse-driven desktop interactions and `tap` for touch/mobile. Use `press` for physical buttons (e.g., Digital Crown).
- **Controls:** A checkbox is "selected" or "unselected," not "checked." Use `Switch` for immediate on/off actions; use `CheckBox` for list selections.
- **Clarity:** Avoid jargon. Instead of "mount a disk," use "open the disk image."

### b. Hierarchy & Layout
- **Box-First Design:** Rely on `VerticalBox` and `HorizontalBox` for structured, hierarchical layouts. Use `StyleMetrics` (`layout-spacing`, `layout-padding`) to maintain consistent, platform-native gaps.
- **Responsive Sizing:** Prefer `preferred-width/height` over fixed dimensions to allow for fluid window resizing.

### c. Feedback & Accessibility
- **Interactivity:** Always provide visual cues for `hover`, `pressed`, and `focused` states. Use the `animate` keyword for smooth transitions.
- **Accessibility (a11y):** Proactively define `accessible-role`, `accessible-label`, and `accessible-value`. Use the "Made with Slint" (`AboutSlint`) badge for attribution.

### d. Component-Driven Design (React Style)
- **Modularity:** Break complex UIs into small, reusable components.
- **Prop-Driven Configuration:** Use `in` properties to configure components, keeping them generic.
- **Single Source of Truth:** Manage heavy state in the Rust backend (`AppState`) and expose it via `in-out` properties.

## 🚀 Performance Optimization (WGPU/RDNA2)
- **VRAM is Primary:** Minimize CPU-to-GPU data transfers. Use `bytemuck` for zero-copy writes where possible.
- **Rendering Hints:** Use `cache-rendering-hint: true;` for complex, static UI elements to batch draw calls into textures.
- **Phase Continuity:** Advance phase accumulators on the CPU after GPU dispatches to prevent audio glitches.

## 🔍 Live Debugging & Introspection
- **Proactive Logging:** Use `debug("Label: " + property)` to verify values. messages appear in the console during execution.
- **UI Interaction:** Correlate user actions with `debug()` output to verify event flows.
- **Compiled Logic:** Infer component hierarchy from the `.slint` structure to remove guesswork during development.

## 📚 Specialized References
- [Primitive Types](docs/stack/slint/reference/primitive-types.md)
- [Layouts (Grid, Horizontal, Vertical)](docs/stack/slint/reference/horizontallayout.md)
- [Interactive Widgets](docs/stack/slint/reference/button.md)
- [Window & Dialog Management](docs/stack/slint/reference/window.md)
