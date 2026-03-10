# SKILL: Slint UI + Material Design 3 — Project Synesthesia Design Language

**For use by any coding agent (Jules, Claude, Gemini, etc.)**
**Ground truth for Figma components and Slint code generation**
**Last Updated**: 2026-03-10

---

## Purpose and Scope

This document defines the complete design language for Project Synesthesia applets. Every applet (Toto, Dorothy, Lion, Oz, Kansas, etc.) uses this language. When you write Slint code for this project, every color value, font size, spacing unit, and component structure must come from this document — not from intuition.

**Conflict resolution**: When this document conflicts with a track spec, this document wins for visual decisions. Track specs win for data model and behavior decisions.

---

## 1. Design Philosophy

Project Synesthesia applets are **forensic instruments, not dashboards**. The visual metaphor is an oscilloscope or spectrum analyzer: high information density, zero decoration for its own sake, every pixel communicates signal state.

The Material Design 3 system is used because its tonal color palette and elevation model map naturally to the physical concept of signal strength — higher-confidence signals "surface" visually, lower-confidence signals "recede."

**Project departures from standard MD3**:
1. **Dark-only palette**: No light mode. The dark surface scheme is the only mode.
2. **Chromatic frequency mapping**: The 12-tone octave color system overrides MD3's three-color (primary/secondary/tertiary) scheme for data visualization. MD3 colors apply only to UI chrome (buttons, chips, toggles, borders). Data (particles, spectra, waveforms) always use the harmonic palette.

---

## 2. Color System

### 2.1 MD3 Tonal Roles (UI Chrome Only)

These token names match the MD3 specification and Figma's Material Theme plugin. All values are for the dark scheme. There is no light scheme.

| Token | Hex | Usage |
|-------|-----|-------|
| **Primary** | `#00E5C8` | Teal. Active labels, headers, key borders. |
| **On-Primary** | `#003730` | Text on primary-colored surfaces. |
| **Primary Container** | `#004D43` | Elevated card border when active. |
| **On-Primary-Container** | `#6FFCE8` | Text inside primary containers. |
| **Secondary** | `#A855F7` | Violet. Loss curves, anomaly warnings. |
| **On-Secondary** | `#2A004F` | Text on secondary surfaces. |
| **Secondary Container** | `#3D0070` | Secondary card backgrounds. |
| **On-Secondary-Container** | `#D9AAFF` | Text inside secondary containers. |
| **Tertiary** | `#F59E0B` | Amber/Gold. Threat tier Gold indicator. |
| **On-Tertiary** | `#3D2400` | Text on tertiary surfaces. |
| **Tertiary Container** | `#5A3600` | Gold-tier card backgrounds. |
| **On-Tertiary-Container** | `#FFDDB0` | Text inside tertiary containers. |
| **Error** | `#FF5449` | Sensor offline, parse error, NaN detected. |
| **On-Error** | `#690005` | Text on error surfaces. |
| **Error Container** | `#93000A` | Error card backgrounds. |
| **On-Error-Container** | `#FFDAD6` | Text inside error containers. |
| **Surface-Dim** | `#0F111A` | Darkest background. Window root fill. |
| **Surface** | `#14161F` | Default card background. |
| **Surface-Bright** | `#1A1D2B` | Elevated card background (z=1). |
| **Surface-Container-Low** | `#161820` | Subtle grouping areas. |
| **Surface-Container** | `#1A1D2B` | Standard container fill. |
| **Surface-Container-High** | `#22253A` | High-emphasis container. |
| **Surface-Container-Highest** | `#292C3F` | Highest-emphasis container (modal, tooltip). |
| **On-Surface** | `#E1E3F0` | Primary text on any surface. |
| **On-Surface-Variant** | `#9CA3AF` | Secondary text, sublabels, placeholders. |
| **Outline** | `rgba(255,255,255, 12%)` | Subtle borders. |
| **Outline-Variant** | `rgba(255,255,255, 6%)` | Dividers, separators. |
| **Scrim** | `rgba(0, 0, 0, 60%)` | Modal backdrop. |
| **Inverse-Surface** | `#E1E3F0` | Not used (no light mode). |
| **Shadow** | `rgba(0, 0, 0, 40%)` | Drop shadows. |

### 2.2 Harmonic Palette (Data Visualization Only)

The 12-tone octave mapping from Emerald City. These are used **ONLY** for:
- Particle colors in canvas elements
- Frequency spectrum bar fills
- Cluster blob colors in BSS visualization
- Waveform stroke colors keyed to frequency

They are **NEVER** used for UI chrome (buttons, labels, borders, icons).

| ID | Note | Hue° | Hex | Physical Association |
|----|------|------|-----|----------------------|
| 0 | C | 0° | `#FF1A1A` | AC mains (50/60 Hz), ELF |
| 1 | C# | 30° | `#FF6600` | Low-frequency audio artifacts |
| 2 | D | 60° | `#FFAA00` | Mid audio, ultrasound sub-harmonics |
| 3 | D# | 90° | `#E5E500` | Upper audio, sonar |
| 4 | E | 150° | `#66E620` | VHF lower edge |
| 5 | F | 270° | `#9910FF` | Violet anchor (F4=349Hz, 2.4GHz) |
| 6 | F# | 280° | `#6600FF` | WiFi 2.4 GHz upper, ISM band |
| 7 | G | 200° | `#0099FF` | Bluetooth, 2.4–2.5 GHz |
| 8 | G# | 180° | `#00E5CC` | Teal: 85 kHz target band, ultrasound |
| 9 | A | 120° | `#1ADD4D` | Green: 5 GHz WiFi, GPS |
| 10 | A# | 50° | `#B3F21A` | Yellow-teal: mmWave lower edge |
| 11 | B | 330° | `#F21AB3` | Magenta: High RF, 24 GHz radar |

### 2.3 Threat Tier Visual States

These are the Lead/Obsidian/Gold material states. They affect the entire applet's color temperature, not just one element.

| Tier | Anomaly Range | Text Color | Glow Color | Surface Tint |
|------|---------------|-----------|-----------|-------------|
| **Lead** | 0.0 – 1.0 | `#6C7280` | none | none |
| **Obsidian** | 1.0 – 3.0 | `#9CA3AF` | `rgba(156,163,175,0.2)` | none |
| **Gold** | 3.0+ | `#F59E0B` | `rgba(245,158,11,0.35)` | `rgba(245,158,11,0.04)` |

The glow is implemented as a drop-shadow filter or glowing border on the anomaly score display only — not the entire window. Gold tier also adds a very subtle warm tint to Surface.

---

## 3. Typography

### 3.1 Typeface

- **Primary face**: Inter (variable font, weights 300–700)
- **Fallback chain**: "Inter", "Segoe UI Variable", "Ubuntu", "Roboto", sans-serif
- **Monospace face** (values, telemetry): "JetBrains Mono", "Cascadia Code", "Consolas", monospace

Do not use any other typefaces. Do not embed font files unless Inter is already in the repository's `assets/fonts/` directory.

### 3.2 Type Scale

These map to MD3's named roles. Use the Slint `font-size` and `font-weight` properties.

| Role | Size | Weight | Letter-spacing | Usage |
|------|------|--------|-----------------|-------|
| Display Large | 57px | 300 | -0.25px | Not used in applets |
| Display Medium | 45px | 400 | 0 | Not used in applets |
| Display Small | 36px | 400 | 0 | Not used in applets |
| Headline Large | 32px | 400 | 0 | Major applet titles |
| Headline Medium | 28px | 400 | 0 | Section headers |
| Headline Small | 24px | 400 | 0 | Sub-section headers |
| **Title Large** | **22px** | **500** | **0** | **Card titles, modal headers** |
| **Title Medium** | **16px** | **500** | **0.15px** | **Panel labels** |
| **Title Small** | **14px** | **500** | **0.1px** | **Sub-panel labels** |
| **Body Large** | **16px** | **400** | **0.5px** | **Primary body text** |
| **Body Medium** | **14px** | **400** | **0.25px** | **Standard body text** |
| **Body Small** | **12px** | **400** | **0.4px** | **Supporting text, captions** |
| **Label Large** | **14px** | **500** | **0.1px** | **Button text** |
| **Label Medium** | **12px** | **500** | **0.5px** | **Chip text, badge text** |
| **Label Small** | **11px** | **500** | **0.5px** | **Status bar, footnotes** |
| **Mono Value** | **24px** | **300** | **0** | **Numeric readouts (anomaly score)** |
| **Mono Label** | **11px** | **500** | **2px uppercase** | **Column headers, unit labels** |

---

## 4. Spacing and Layout Grid

MD3 uses a 4px base unit. All spacing values must be multiples of 4.

| Token | Value | Usage |
|-------|-------|-------|
| `spacing-1` | 4px | Tight internal padding (icon to label) |
| `spacing-2` | 8px | Component internal padding |
| `spacing-3` | 12px | Small gap between related elements |
| `spacing-4` | 16px | Standard card padding, section padding |
| `spacing-5` | 20px | Medium gap between sections |
| `spacing-6` | 24px | Large section margin |
| `spacing-8` | 32px | Major section separation |
| `spacing-12` | 48px | Window-level padding (top/side margins) |

### 4.1 Applet Layout Structure

Every applet uses this three-zone vertical layout:

```
┌──────────────────────────────────────────────────────────────────┐
│  HEADER ZONE  (height: 72px, padding: 16px)                      │
│  Left: Icon + Title (Title Large, On-Surface)                    │
│  Right: Key metrics + primary toggle                             │
├──────────────────────────────────────────────────────────────────┤
│  MAIN CANVAS ZONE  (flexible height, min: 200px)                 │
│  Contains the primary data visualization.                        │
│  Background: Surface-Dim (#0F111A)                               │
│  Border: 1px Outline (rgba white 12%), radius: 8px               │
├──────────────────────────────────────────────────────────────────┤
│  METRIC STRIP  (height: 120px)                                   │
│  3–4 metric cards in a horizontal row.                           │
│  Each card: Surface-Container background, 8px radius             │
├──────────────────────────────────────────────────────────────────┤
│  STATUS BAR  (height: 24px, padding: 8px)                        │
│  Label Small, On-Surface-Variant color. Inference status text.   │
└──────────────────────────────────────────────────────────────────┘
```

**Window root properties**:

```slint
Window {
    background: transparent;   // Compositor handles blur behind
    min-width:  860px;
    min-height: 560px;
}

// Root background rectangle (not the Window itself)
Rectangle {
    background: #0F111A;   // Surface-Dim
    // If compositor blur is active, use:
    // background: rgba(15, 17, 26, 0.88);
    // The 12% alpha lets the blur show through.
    width:  parent.width;
    height: parent.height;
    border-radius: 8px;    // Rounded corners on the floating window
}
```

---

## 5. Elevation Model

MD3 elevation is expressed as a tonal overlay (surface tint) rather than shadows. In the dark scheme, higher elevation = lighter surface tint.

| Level | Surface Color | Usage |
|-------|---------------|-------|
| 0 | Surface-Dim `#0F111A` | Window root, canvas backgrounds |
| 1 | Surface `#14161F` | Default cards, panels |
| 2 | Surface-Bright `#1A1D2B` | Elevated cards, popovers |
| 3 | Container-High `#22253A` | Active/focused card state |
| 4 | Container-Highest `#292C3F` | Dialogs, modals, tooltips |

**Drop shadows** (used sparingly for floating elements only):
- **Level 2+**: `box-shadow: 0 2px 8px rgba(0,0,0,0.4)`
- **Level 3+**: `box-shadow: 0 4px 16px rgba(0,0,0,0.5), 0 1px 4px rgba(0,0,0,0.3)`

---

## 6. Agent Instructions

When you are a coding agent generating Slint UI for this project:

1. **Import tokens.slint** at the top of every `.slint` file. Use `Colors.primary` instead of `#00E5C8`. Use `Spacing.md` instead of `16px`. **No magic numbers.**

2. Every numeric value (anomaly score, loss, frequency) uses:
   - `font-family: "JetBrains Mono", "Cascadia Code", monospace`
   - `font-weight: 300` (the mono-value role)

3. Every label above a value uses:
   - `font-weight: 700`
   - `letter-spacing: 1.5px`
   - `Colors.on-surface-variant`
   - ALL CAPS

4. **Canvas backgrounds are always `#0A0C14`** (darker than surface-dim). This ensures colored particles have maximum contrast.

5. **Never use `Colors.secondary` (`#A855F7` violet) for UI chrome** unless it is specifically the Mamba/inference visualization border. The violet color is reserved for "this is what Mamba computed." Teal (`Colors.primary`) is for "this is live hardware signal."

6. **The harmonic palette colors** (`Colors.harm-0` through `Colors.harm-11`) are used **ONLY** for data points, never for UI chrome. If you find yourself using `Colors.harm-5` for a button label, you are violating this rule.

7. **All property animations**:
   - `duration: 150ms; easing: ease-in-out;` for interactive state changes (hover, active, checked)
   - `duration: 400ms;` for semantic state changes (threat tier transitions)

8. **The window root background is always `transparent`**. The opaque surface is provided by a child Rectangle with `background: Colors.surface-dim`. This separation enables platform-specific compositor blur without changing the component structure.

---

## 7. Component Reference

Use Material Slint UI third-party library components. Map these from the official Material Design 3 component set:

- **Card** → Metric display cards
- **Button** → Toggle, action buttons
- **Switch** → Binary toggles
- **Icon Button** → Floating action buttons, panel toggles
- **Chip** → Tag displays
- **Text Field** → Input controls (rare in forensic applets)
- **Slider** → Range selectors
- **Checkbox** → Multiple selections
- **Radio Button** → Single selections

**Custom component additions** (not in standard MD3):
- **CanvasPanel**: Labeled visualization container
- **Sparkline**: Scrolling line chart for loss/anomaly curves
- **FrequencyBar**: 260px logarithmic frequency spectrum bar

---

## 8. Window Transparency — Platform Architecture

See IMPLEMENTATION section below for Rust code patterns.

### 8.1 Slint Component Side (All Platforms)

```slint
export component AppletWindow inherits Window {
    background: transparent;   // ← Tells rendering backend not to paint

    Rectangle {
        background: #0F111A;   // ← Solid fallback (dark)
        // When compositor blur confirmed active, switch to:
        // background: rgba(15, 17, 26, 0.88);
        width: 100%;
        height: 100%;
    }
}
```

### 8.2 Windows 11 — DWM Acrylic Blur (Rust Implementation)

**Cargo.toml dependencies**:
```toml
windows-sys = { version = "0.48", features = ["Win32_Graphics_Dwm", "Win32_Foundation"] }
raw-window-handle = "0.5"
```

**Enable acrylic blur before event loop**:
```rust
#[cfg(target_os = "windows")]
fn enable_acrylic_blur(window: &slint::Window) {
    use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    if let Ok(handle) = window.window_handle() {
        if let RawWindowHandle::Win32(h) = handle.as_raw() {
            // DWMSBT_TRANSIENTWINDOW = 3 (Acrylic, for utility windows)
            // DWMSBT_MAINWINDOW = 2 (Mica, for main app windows)
            let backdrop_type: u32 = 3;
            unsafe {
                DwmSetWindowAttribute(
                    h.hwnd.get() as _,
                    DWMWA_SYSTEMBACKDROP_TYPE,
                    &backdrop_type as *const _ as *const _,
                    std::mem::size_of::<u32>() as u32,
                );
            }
        }
    }
}

// In main():
let window = AppletWindow::new()?;
enable_acrylic_blur(&window);
window.run()?;
```

### 8.3 Linux — KWin Wayland/X11

**Wayland (KWin)**: `background: transparent` is sufficient. KWin applies blur automatically if "Blur" is enabled in System Settings → Desktop Effects.

**X11 (KWin)**: Request blur property via x11rb crate (optional for advanced transparency).

**GNOME (Mutter)**: No per-window blur support. Transparent background will appear unblurred. Solid fallback (`#0F111A`) ensures readability.

### 8.4 Runtime Detection

```rust
/// Returns true if the compositor supports alpha-blended windows.
pub fn compositor_supports_transparency() -> bool {
    #[cfg(target_os = "windows")]
    { true }  // Windows 11 always supports Acrylic

    #[cfg(target_os = "linux")]
    {
        std::env::var("WAYLAND_DISPLAY").is_ok()
            || std::env::var("DESKTOP_SESSION")
                .map(|s| s.contains("plasma") || s.contains("kde"))
                .unwrap_or(false)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    { false }
}
```

---

## 9. Animation Patterns

### 9.1 Slint Property Animation (Simple Transitions)

```slint
Text {
    color: root.threat-tier == 0 ? #6C7280
         : root.threat-tier == 1 ? #9CA3AF
         : #F59E0B;
    animate color { duration: 400ms; easing: ease-in-out; }
}
```

### 9.2 Rust Timer → Slint Property (Complex Animation)

For pulse effects, scrolling sparklines, and particle movement:

```rust
let window_weak = window.as_weak();
let timer = slint::Timer::default();
timer.start(
    slint::TimerMode::Repeated,
    std::time::Duration::from_millis(16),  // 60 Hz
    move || {
        if let Some(w) = window_weak.upgrade() {
            let t = w.get_animation_tick() + 0.016_f32;
            w.set_animation_tick(t);
        }
    }
);
```

In `.slint`:
```slint
in property <float> animation-tick;

Rectangle {
    opacity: 0.6 + 0.4 * abs(sin(animation-tick * π * 0.5));
}
```

### 9.3 Particle Canvas Refresh Pattern

Particle canvas data is updated from Rust at 30 Hz. Always replace the full `ModelRc::new(VecModel)` atomically — never mutate individual items.

---

## 10. Figma Component Mapping

When building in Figma, use the **Material Theme Builder plugin** to generate the tonal palette, then override with project tokens from §2.1.

**Seed colors for Material Theme Builder**:
- Primary seed: `#00E5C8` (teal)
- Secondary seed: `#A855F7` (violet)
- Tertiary seed: `#F59E0B` (amber)
- Neutral seed: `#1A1D2B` (dark slate)
- Error seed: `#FF5449`

Select "Dark scheme only."

**Auto-layout settings for applet root frame**:
- Direction: Vertical
- Gap: 8px
- Padding: 16px all sides
- Fill: `#0F111A` (or 88% opacity for transparent window mockup)
- Corner radius: 8px
- Min-width: 860px, Min-height: 560px

---

## 11. Canonical Tokens File

Reference implementation: `ui/tokens.slint` — imported by all applets.

See next document: `tokens.slint` template.

---

**End of SKILL-SLINT-MD3.md**
