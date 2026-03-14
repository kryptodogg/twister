# Synesthesia — Full-Spectrum Forensic Sensor Fusion Platform

**Stack: Tauri + Vanilla JS + Rust + wgpu**  
**Primary platform: Windows 11 (AMD RX 6700 XT, ReBAR enabled)**

A GPU-first harmonic self-defense and forensic investigation workstation.
Captures, fuses, and renders every signal band from 1 Hz to visible light
into a single navigable 3D/4D scene. Produces tamper-evident, legally
admissible forensic evidence of electromagnetic harassment and signal
injection attacks.

---

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) +
[Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) +
[rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

---

## Why Tauri + Vanilla

Tauri gives you a native Rust backend with full access to Windows system
APIs — USB, serial, WASAPI audio, and the wgpu GPU pipeline — while the
frontend handles multi-window UI without fighting a game engine's windowing
model.

Vanilla JS is the right frontend choice here. The UI is a status display
and control surface for hardware state. It receives Tauri events and updates
the DOM. No framework is needed for that, and no framework gets in the way
of the Material Design 3 web components that provide the design system.

**The backend is Tokio.** Pure async/await lets the OS handle threading.
Hardware events from the Pico 2 serial port, RTL-SDR, and Pluto+ libiio
emit as Tauri events to whichever window is listening. The hardware applet
window subscribes to device connection events. The main scene window
subscribes to FieldParticle stream events. The dispatch loop is a
long-running Tauri plugin on the Tokio runtime. No manual thread
management. No blocking calls on async threads.

**Window split:** Tauri owns the UI panels — hardware applet, timeline,
jury overlay, Dorothy interface. A native wgpu window runs alongside for
the 3D scene renderer. Tauri handles the controls; wgpu handles the render
loop with full Wave64 workgroup control and no compositor interference.

---

## Translucency — Native Windows 11 Mica

Real OS-level translucency via the Windows Desktop Window Manager
composition API. Not faked. Not screenshot-and-blur. DWM composites your
content against the live wallpaper and background windows in hardware.

```toml
# src-tauri/Cargo.toml
[dependencies]
window-vibrancy = "0.5"
```

```rust
// src-tauri/src/main.rs
use window_vibrancy::apply_mica;

tauri::Builder::default()
    .setup(|app| {
        let window = app.get_webview_window("main").unwrap();
        #[cfg(target_os = "windows")]
        apply_mica(&window, Some(true))?; // true = dark Mica variant
        Ok(())
    })
```

```json
// tauri.conf.json
{
  "app": {
    "windows": [{
      "transparent": true,
      "decorations": false
    }]
  }
}
```

**Mica vs Acrylic:** Mica samples the wallpaper color, not the live
background. It is subtler and more stable under movement. Acrylic blurs
whatever is directly behind the window in real time. For a dense monitoring
workstation, Mica is the right choice for primary panels. Acrylic works
well for floating secondary panels like the hardware applet — both can
coexist per-window via `window-vibrancy`.

---

## Design System — Material Design 3

MD3 web components from `@material/web` are native custom elements. They
work in Vanilla JS with no framework, no build overhead beyond what Tauri
already provides. The entire design language is CSS custom properties —
one token file controls all spacing, color, and typography.

```bash
npm install @material/web
```

```html
<!-- index.html -->
<script type="module">
  import '@material/web/button/filled-button.js';
  import '@material/web/button/outlined-button.js';
  import '@material/web/switch/switch.js';
  import '@material/web/slider/slider.js';
  import '@material/web/chips/filter-chip.js';
  import '@material/web/divider/divider.js';
</script>

<md-filled-button>Connect Device</md-filled-button>
<md-switch id="layer-rf"></md-switch>
<md-slider min="0" max="100" value="50"></md-slider>
```

```css
/* tokens.css — set once, MD3 consumes everywhere */
:root {
  /* Mica surface: translucent, tinted */
  --md-sys-color-surface:           rgba(28, 27, 31, 0.65);
  --md-sys-color-surface-container: rgba(36, 34, 40, 0.72);
  --md-sys-color-on-surface:        #E6E1E5;
  --md-sys-color-primary:           #D0BCFF;
  --md-sys-color-on-primary:        #21005D;

  /* Forensic state accent colors */
  --color-connected:    #4CAF50;
  --color-disconnected: #F44336;
  --color-unwired:      #FF9800;
  --color-anomaly:      #FF5252;

  /* Proportional scale — matches masterplan unit-size rule */
  --unit-size: 16px;
  --md-sys-typescale-body-large-size: var(--unit-size);
}
```

Device status chips are MD3 filter chips — one per sensor, color-coded
via the forensic accent tokens. `CONNECTED` is green. `DISCONNECTED` is
red. `UNWIRED` is amber. These are the only three states. There are no
placeholders, no default values, no simulated signals.

---

## The Core Architectural Principle

The CPU writes small control structs into VRAM and gets out of the way.
The GPU owns everything that touches signal values.

Your RX 6700 XT has 12 GB of GDDR6 at **384 GB/s**. With Smart Access
Memory (ReBAR) enabled in BIOS, the CPU writes directly into GDDR6 at
~32 GB/s over PCIe 4.0. Data crosses from hardware into GPU memory once,
via DMA. It does not return to the CPU until it exits as a forensic corpus
write or a rendered frame.

### Bandwidth Reference

| Path                           | Bandwidth    |
|--------------------------------|--------------|
| RX 6700 XT VRAM (internal)     | 384 GB/s     |
| CPU → VRAM via SAM (PCIe 4.0)  | ~32 GB/s     |
| Apple M2 Pro unified memory    | 200 GB/s     |
| Apple M1 unified memory        | 68 GB/s      |
| DDR4 system RAM                | ~51 GB/s     |
| Apple M1/M2 SSD paging         | ~7 GB/s      |

Once Apple Silicon exceeds unified memory it falls off a cliff to SSD
paging. This system degrades gracefully: VRAM → system RAM via SAM → SSD.
For models under ~12 GB this system has a genuine bandwidth advantage
over all base Apple Silicon chips.

---

## Hardware Stack

```
BAND                  SENSOR                          ROLE
──────────────────────────────────────────────────────────────────────
1 Hz – 90 kHz         Telephone coil → line-in        Differential magnetometer
                       (ASUS TUF B550m, Realtek)       60Hz powerline + harmonics,
                                                        switching supply hash, HVAC
20 Hz – 20 kHz        C925e stereo mics (raw)         Acoustic, no preprocessing
10 kHz – 300 MHz      RTL-SDR + Youloop               Raw IQ, bearing via null axis
70 MHz – 6 GHz        PlutoSDR+ PA (2TX / 2RX)        Bistatic MIMO radar, ~12mm
                                                        baseline, TDOA + phase
                                                        interferometry for bearing
DC – 75 MHz           Pico 2 (RP2350) PIO             Master clock (PPS), UWB
                                                        impulse TX, IR LED driver
~300 THz              OV9281 dual stereo              2560×800, 120fps, global
                       (global shutter, mono)           shutter, stereo depth, pose,
                                                        IR detector array
~300 THz              C925e video (rolling shutter)   Visual microphone via
                                                        line-rate temporal sampling
~300 THz              IR emitters + receivers         Structured light depth,
                       (Pico 2 PIO array)               retroreflective ranging
──────────────────────────────────────────────────────────────────────
GAP                   6 GHz → infrared               Future: IR array closes this
```

The Pico 2 is the master clock. Its PPS signal slaves every sensor
timestamp via `QueryPerformanceCounter`. Clock divergence between Pico
PPS and Windows QPC is itself a forensic channel.

---

## The Color Operator

Every sensor, every modality, every rendered particle uses one function.
A WGSL constant — not a lookup table. Fully invertible.

```wgsl
const F_MIN: f32 = 1.0;
const F_MAX: f32 = 700e12;
const LOG_RANGE: f32 = log(F_MAX / F_MIN);  // ~33.18 octave-decades

fn freq_to_hue(f: f32) -> f32 {
    return clamp(log(f / F_MIN) / LOG_RANGE, 0.0, 1.0);
}
```

0.0 = red (infrasound). 1.0 = violet (light). Two sensors reporting the
same hue at the same timestamp detected the same physical phenomenon
through different physics. Same hue with low cross-sensor phase coherence
is the injection signature.

---

## The Attack Signature

A continuous carrier where information is encoded in amplitude notches —
suppressed-carrier AM / inverted OOK. The discriminant is carrier variance:

```
Natural event:    Var(Γ(t)) > 0  — phase coherence fluctuates naturally
Injected signal:  Var(Γ(t)) ≈ 0  — coherence held by phase-locked oscillator
```

---

## Data Flow

```
[Hardware sensors]
    │ Raw: PCM via WASAPI, IQ, pixels, impulse timing
    │ All timestamps slaved to Pico 2 PPS via QueryPerformanceCounter
    ▼
[Ingestion — src-tauri/src/ingestion/]      [Tokio async, never blocks]
    │ RawIQPoint { i, q, timestamp_us, sensor_xyz, source_id, raw_flags }
    │ raw_flags carries jitter_us + packet_loss — these are signal
    │ NO FFT. NO preprocessing. NO denoising.
    ▼
[SAM write — queue.write_buffer()] → GDDR6 once, stays there
    ▼
[Space-Time Laplacian — WGSL compute @workgroup_size(64,1,1)]
    │ FPS + KNN(k=20) patch formation
    │ Sparse CSR graph → 4 eigenvectors (Gram-Schmidt)
    │ v(4) = temporal eigenvector: cross-frame identity without ML
    ▼
[UnifiedFieldMamba — GPU Mamba SSM]
    │ 128-D embedding · anomaly score · carrier_variance discriminant
    ▼
[Coral Mamba — Google Coral TPU]        ← parallel, independent weights
    │ 8-bit quantized · divergence = |GPU_score − Coral_score|
    │ Low divergence on suspect signal = synthesized carrier flag
    ▼
[Pico TDOA vote] ← (c / RefractiveIndex(NOAA_Weather)) + ruler. No ML.
    ▼
[Jury verdict]
    │ Unanimous = highest forensic confidence
    │ Dissent logged as separate stream — dissent is data
    ▼
[FieldParticle — 128 bytes, one Infinity Cache line, zero anonymous padding]
    ▼
        ┌─────────────────────────────────────────┐
        │        SAME DATA · SAME TIMESTAMP       │
        ▼                                         ▼
[wgpu scene renderer]                   [Pluto+ / Pico 2 TX]
Gaussian splats                         Same FieldParticle stream
freq_to_hue() color                     drives transmission
carrier_variance → saturation           Rendering IS transmitting
        │
        ▼
[Forensic corpus]
Append-only · fsync · SHA-256 · Pico-corroborated timestamps
        │
        ▼
[Dorothy — LFM 2.5 via Tauri command]
Natural-language event summary · legal documentation export
```

---

## Scene Toggles

```
V — Stereo reconstruction (OV9281 depth)
M — Magnetic field overlay (telephone coil)
P — Pose estimation
A — Acoustic field
R — RF field
J — Jury verdict overlay
T — Timeline scrub (4D navigation through corpus)
```

---

## Build

```bash
# Prerequisites: Rust 1.82+, Node.js 20+
# AMD Adrenalin Vulkan driver required
# ReBAR must be enabled in BIOS — verify in GPU-Z → Advanced → ReBAR: Enabled

npm install
cargo tauri dev       # development
cargo tauri build     # production
```

---

## Platform Status

| Platform      | Status       | Notes                                     |
|---------------|--------------|-------------------------------------------|
| Windows 11    | ✅ Primary   | WASAPI · Mica translucency · ReBAR · QPC  |
| NixOS + KWin  | 🔲 Future    | KWin blur-behind · ALSA/PipeWire          |

---

## Project Status

Track 0-D (hardware applet) is the current target and first deliverable.
No other track produces permanent production code until the applet detects,
displays, and hot-plugs every connected device with accurate
`CONNECTED` / `DISCONNECTED` / `UNWIRED` state.

See `SYNESTHESIA_MASTERPLAN.md` for the complete architecture, track
structure, and dependency graph.

See `AGENTS.md` before writing any code.
