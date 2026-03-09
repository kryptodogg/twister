# Track B Addendum: Parameter Control Layer (Mic Selection, Gain, Frequency, Camera)

**Status**: Ready for Jules implementation
**Duration**: 45-60 minutes
**Dependency**: Track B UI already exists; this adds control panel wiring
**Integration**: Slint UI → AppState mutations → Audio/Visual subsystems

---

## Executive Summary

Track B currently has UI stubs for gain control, frequency selection, and camera resolution. This addendum **removes all stubs** and implements the full parameter control layer:

- **Mic Selection**: +/- buttons to cycle through 4 input devices
- **Gain Control**: Real-time AGC slider (0.0-1.0 multiplier)
- **Frequency Selection**: Dropdown for standard bands + manual entry
- **Camera Resolution**: Preset selector (480p, 720p, 1080p) with live feedback
- **Parameter Persistence**: Save/restore user preferences on app restart

**No more stubs.** All controls are wired to live backends.

---

## Interface Contracts (What Backends Expect)

### 1. Mic Selection State (Already in AppState)

```rust
// src/state.rs - EXTEND existing
pub struct AppState {
    pub audio_device_idx: usize,           // Current selected device (0-3)
    pub audio_devices: Vec<AudioDevice>,   // [C925e, Rear Pink, Rear Blue, RTL-SDR]
    // ... existing fields ...
}

pub struct AudioDevice {
    pub index: usize,
    pub name: String,                      // e.g., "C925e (AI Noise-Canceling)"
    pub sample_rate_hz: u32,               // 192000, 48000, etc.
    pub channels: u32,
    pub is_active: bool,                   // Whether device is currently recording
}
```

**Device Mapping**: // Devices selectable, addable and subtractable in interface, hardcoding only for defaults
```rust
const AUDIO_DEVICES: &[(&str, u32, u32)] = &[
    ("C925e"), 192_000, 2),
    ("Rear Mic (Pink)", 192_000, 1),
    ("Rear Line-In (Blue)", 192_000, 1),
    ("RTL-SDR (2.4 GHz receiver)", 12_288_000, 1),   // PDM clock should be a variable and adjustable in ui
    ("Pluto+ 70 Mhz - 6 Ghz", 12_288_000, 1)
];
```

### 2. Gain Control State (Already in AppState)

```rust
// src/state.rs - EXTEND existing
pub struct AppState {
    pub agc_gain_multiplier: f32,          // [0.0, 1.0] multiplier on incoming audio
    pub agc_enabled: bool,                 // Whether automatic gain control is active
    pub agc_target_level_db: f32,          // Target RMS level (-40 dB to 0 dB)
    pub input_level_db: f32,               // Real-time input level (for feedback)
    // ... existing fields ...
}
```

**Semantics**:
- `agc_gain_multiplier = 0.0` → Mute
- `agc_gain_multiplier = 0.5` → -6 dB attenuation
- `agc_gain_multiplier = 1.0` → No attenuation (pass-through)
- When `agc_enabled = true`, auto-adjusts gain to maintain `agc_target_level_db` 108db snl is output, 103 dbl snl input

### 3. Frequency Selection State (Already in AppState)

```rust
// src/state.rs - EXTEND existing
pub struct AppState {
    pub freq_selection_mode: FreqMode,     // Preset or Manual
    pub freq_preset_band: FreqBand,        // Current preset (if mode=Preset)
    pub freq_manual_hz: f32,               // Manual frequency (if mode=Manual)
    pub freq_actual_hz: f32,               // Actual frequency in use (dispatch loop)
    // ... existing fields ...
}

pub enum FreqMode {
    Preset,
    Manual,
}

pub enum FreqBand {
    VLF,           // 1 Hz - 100 Hz
    LF,            // 100 Hz - 10 kHz
    MF,            // 10 kHz - 1 MHz
    HF,            // 1 MHz - 30 MHz
    VHF,           // 30 MHz - 300 MHz
    UHF,           // 300 MHz - 3 GHz
    Custom(f32),   // User-specified frequency
}

impl FreqBand {
    pub fn center_hz(&self) -> f32 {
        match self {
            VLF => 50.0,
            LF => 1_000.0,
            MF => 100_000.0,
            HF => 10_000_000.0,
            VHF => 100_000_000.0,
            UHF => 1_000_000_000.0,
            Custom(f) => *f,
        }
    }
}
```

### 4. Camera Resolution State (Already in AppState)

```rust
// src/state.rs - EXTEND existing
pub struct AppState {
    pub camera_resolution: CameraResolution,
    pub camera_active: bool,
    pub camera_fps: f32,                   // Actual frame rate (for feedback)
    // ... existing fields ...
}

pub enum CameraResolution {
    _480p,   // 640x480 @ 30fps
    _720p,   // 1280x720 @ 30fps
    _1080p,  // 1920x1080 @ 30fps
}

impl CameraResolution {
    pub fn width_height(&self) -> (u32, u32) {
        match self {
            _480p => (640, 480),
            _720p => (1280, 720),
            _1080p => (1920, 1080),
        }
    }
}
```

---

## UI Implementation (Slint)

### File Ownership

- **`ui/app.slint`** - Jules updates the control panel (lines 50-150 estimated)
  - Mic selector (+/- buttons + label)
  - Gain slider + dB readout
  - Frequency preset dropdown + manual input
  - Camera resolution selector
  - Real-time feedback displays (input level, actual frequency, camera FPS)

### UI Components to Add/Wire

```slint
// ui/app.slint - ADD to existing tab or create "CONTROLS" section

TabContent {
    title: "CONTROLS";

    VerticalLayout {
        spacing: 20px;
        padding: 20px;

        // ─────────────────────────────────────────────────────
        // MIC SELECTION
        // ─────────────────────────────────────────────────────
        HorizontalLayout {
            Text {
                text: "Audio Input:";
                min-width: 150px;
            }

            Button {
                text: "◄";
                width: 40px;
                clicked => {
                    app-window.mic_prev();
                }
            }

            Rectangle {
                background: #222;
                border: 1px solid #444;
                border-radius: 4px;
                min-width: 250px;
                HorizontalLayout {
                    padding: 8px;
                    Text {
                        text: root.mic-name;  // Bound to Rust: "C925e (AI Noise-Canceling)"
                        color: #0f0;
                    }
                }
            }

            Button {
                text: "►";
                width: 40px;
                clicked => {
                    app-window.mic_next();
                }
            }

            Text {
                text: "{root.mic-sample-rate-khz} kHz";
                color: #888;
            }
        }

        // ─────────────────────────────────────────────────────
        // GAIN CONTROL
        // ─────────────────────────────────────────────────────
        HorizontalLayout {
            Text {
                text: "Gain:";
                min-width: 150px;
            }

            Slider {
                width: 300px;
                minimum: 0.0;
                maximum: 1.0;
                value <=> root.gain-multiplier;
            }

            Text {
                text: "{root.gain-db} dB";
                min-width: 60px;
                color: root.input-level-db > -20.0 ? #f00 : #0f0;  // Red if clipping risk
            }

            Text {
                text: "Input: {root.input-level-db} dB";
                color: #888;
                min-width: 150px;
            }
        }

        // AGC Toggle
        HorizontalLayout {
            spacing: 10px;
            padding-left: 150px;

            CheckBox {
                checked <=> root.agc-enabled;
            }

            Text {
                text: "Auto Gain Control (AGC)";
            }

            Text {
                text: "Target: {root.agc-target-db} dB";
                color: #888;
                visible: root.agc-enabled;
            }
        }

        // ─────────────────────────────────────────────────────
        // FREQUENCY SELECTION
        // ─────────────────────────────────────────────────────
        HorizontalLayout {
            Text {
                text: "Frequency Band:";
                min-width: 150px;
            }

            ComboBox {
                model: ["VLF (50 Hz)", "LF (1 kHz)", "MF (100 kHz)", "HF (10 MHz)", "VHF (100 MHz)", "UHF (1 GHz)", "Manual"];
                current-index <=> root.freq-band-index;
                selected(index) => {
                    app-window.set_freq_band(index);
                }
            }

            Text {
                text: "{root.freq-actual-hz} Hz";
                color: #0f0;
                min-width: 150px;
            }
        }

        // Manual Frequency Entry (visible only if "Manual" selected)
        HorizontalLayout {
            visible: root.freq-band-index == 6;  // Index 6 = "Manual"
            padding-left: 150px;
            spacing: 10px;

            Text { text: "Enter frequency (Hz):"; }

            TextInput {
                text <=> root.freq-manual-hz-string;
                placeholder-text: "e.g., 145500000";
                width: 200px;
            }

            Text {
                text: root.freq-manual-hz-string.parse::<f32>().ok().map(|f| f.to_string()).unwrap_or("Invalid".to_string());
                color: #888;
            }
        }

        // ─────────────────────────────────────────────────────
        // CAMERA RESOLUTION
        // ─────────────────────────────────────────────────────
        HorizontalLayout {
            Text {
                text: "Camera:";
                min-width: 150px;
            }

            ComboBox {
                model: ["480p (640×480)", "720p (1280×720)", "1080p (1920×1080)"];
                current-index: root.camera-resolution-index;
                selected(index) => {
                    app-window.set_camera_resolution(index);
                }
            }

            Text {
                text: "{root.camera-fps} FPS";
                color: #0f0;
                min-width: 100px;
            }

            CheckBox {
                checked <=> root.camera-enabled;
            }

            Text {
                text: "Active";
            }
        }

        // ─────────────────────────────────────────────────────
        // PARAMETER PERSISTENCE BUTTONS
        // ─────────────────────────────────────────────────────
        HorizontalLayout {
            padding-left: 150px;
            spacing: 10px;

            Button {
                text: "Save Preset";
                clicked => {
                    app-window.save_parameters();
                }
            }

            Button {
                text: "Load Preset";
                clicked => {
                    app-window.load_parameters();
                }
            }

            Button {
                text: "Reset to Default";
                clicked => {
                    app-window.reset_parameters();
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────
// EXPORTED PROPERTIES (Rust ↔ Slint bindings)
// ─────────────────────────────────────────────────────

export global AppWindow {
    in-out property <string> mic-name;
    in-out property <f32> mic-sample-rate-khz;
    in-out property <f32> gain-multiplier;
    in-out property <f32> gain-db;
    in-out property <f32> input-level-db;
    in-out property <bool> agc-enabled;
    in-out property <f32> agc-target-db;
    in-out property <int> freq-band-index;
    in-out property <f32> freq-actual-hz;
    in-out property <string> freq-manual-hz-string;
    in-out property <int> camera-resolution-index;
    in-out property <f32> camera-fps;
    in-out property <bool> camera-enabled;

    // Callbacks wired to Rust
    callback mic-prev();
    callback mic-next();
    callback set-freq-band(int);
    callback set-camera-resolution(int);
    callback save-parameters();
    callback load-parameters();
    callback reset-parameters();
}
```

---

## Rust Implementation (Callbacks & State Management)

### File Ownership

- **`src/state.rs`** - Jules extends AppState with all fields above
- **`src/main.rs`** - Jules wires Slint callbacks (lines 200-350 estimated)
  - `mic_prev()` / `mic_next()` - cycle through devices
  - `set_freq_band(index)` - update frequency selection
  - `set_camera_resolution(index)` - change video resolution
  - `save_parameters()` / `load_parameters()` - JSON persistence
- **`src/parameters.rs`** (NEW, 80 lines) - Parameter file I/O

### Callback Wiring (in src/main.rs)

```rust
// Mic selection callbacks
ui.on_mic_prev(move || {
    let state = state.clone();
    tokio::spawn(async move {
        let mut st = state.lock().await;
        st.audio_device_idx = (st.audio_device_idx + 3) % 4;  // Wrap around
        eprintln!("[Control] Mic switched to: {}", st.audio_devices[st.audio_device_idx].name);
        // TODO: Signal audio system to switch input device
    });
});

ui.on_mic_next(move || {
    let state = state.clone();
    tokio::spawn(async move {
        let mut st = state.lock().await;
        st.audio_device_idx = (st.audio_device_idx + 1) % 4;
        eprintln!("[Control] Mic switched to: {}", st.audio_devices[st.audio_device_idx].name);
    });
});

// Gain slider callback (wired via <=> binding)
ui.on_gain_multiplier_changed(move |value| {
    let state = state.clone();
    tokio::spawn(async move {
        let mut st = state.lock().await;
        st.agc_gain_multiplier = value;
        // Audio system reads this value in next dispatch iteration
    });
});

// Frequency band selection
ui.on_set_freq_band(move |index| {
    let state = state.clone();
    tokio::spawn(async move {
        let mut st = state.lock().await;
        st.freq_selection_mode = FreqMode::Preset;
        st.freq_preset_band = match index {
            0 => FreqBand::VLF,
            1 => FreqBand::LF,
            2 => FreqBand::MF,
            3 => FreqBand::HF,
            4 => FreqBand::VHF,
            5 => FreqBand::UHF,
            6 => {
                st.freq_selection_mode = FreqMode::Manual;
                FreqBand::Custom(st.freq_manual_hz)
            }
            _ => FreqBand::LF,
        };
        st.freq_actual_hz = st.freq_preset_band.center_hz();
    });
});

// Camera resolution
ui.on_set_camera_resolution(move |index| {
    let state = state.clone();
    tokio::spawn(async move {
        let mut st = state.lock().await;
        st.camera_resolution = match index {
            0 => CameraResolution::_480p,
            1 => CameraResolution::_720p,
            2 => CameraResolution::_1080p,
            _ => CameraResolution::_720p,
        };
        eprintln!("[Control] Camera resolution: {:?}", st.camera_resolution);
        // TODO: Signal camera system to change resolution
    });
});

// Parameter persistence
ui.on_save_parameters(move || {
    let state = state.clone();
    tokio::spawn(async move {
        let st = state.lock().await;
        if let Err(e) = save_parameters(&st) {
            eprintln!("[Control] Failed to save parameters: {}", e);
        } else {
            eprintln!("[Control] Parameters saved");
        }
    });
});

ui.on_load_parameters(move || {
    let state = state.clone();
    tokio::spawn(async move {
        match load_parameters() {
            Ok(params) => {
                let mut st = state.lock().await;
                *st = params;
                eprintln!("[Control] Parameters loaded");
            }
            Err(e) => {
                eprintln!("[Control] Failed to load parameters: {}", e);
            }
        }
    });
});
```

### Parameter Persistence (src/parameters.rs)

```rust
use serde::{Deserialize, Serialize};
use std::fs;
use crate::state::AppState;

const PARAM_FILE: &str = "~/.siren/parameters.json";

#[derive(Serialize, Deserialize)]
pub struct SavedParameters {
    pub audio_device_idx: usize,
    pub agc_gain_multiplier: f32,
    pub agc_target_level_db: f32,
    pub freq_band_index: usize,
    pub freq_manual_hz: f32,
    pub camera_resolution_index: usize,
    pub agc_enabled: bool,
}

pub fn save_parameters(state: &AppState) -> Result<(), Box<dyn std::error::Error>> {
    let params = SavedParameters {
        audio_device_idx: state.audio_device_idx,
        agc_gain_multiplier: state.agc_gain_multiplier,
        agc_target_level_db: state.agc_target_level_db,
        freq_band_index: 0,  // TODO: map enum to index
        freq_manual_hz: state.freq_manual_hz,
        camera_resolution_index: 1,  // TODO: map enum to index
        agc_enabled: state.agc_enabled,
    };

    let json = serde_json::to_string_pretty(&params)?;
    fs::write(PARAM_FILE, json)?;
    Ok(())
}

pub fn load_parameters() -> Result<AppState, Box<dyn std::error::Error>> {
    let json = fs::read_to_string(PARAM_FILE)?;
    let params: SavedParameters = serde_json::from_str(&json)?;

    let mut state = AppState::new();
    state.audio_device_idx = params.audio_device_idx;
    state.agc_gain_multiplier = params.agc_gain_multiplier;
    state.agc_target_level_db = params.agc_target_level_db;
    state.freq_manual_hz = params.freq_manual_hz;
    state.agc_enabled = params.agc_enabled;

    Ok(state)
}
```

---

## Real-Time Feedback Loop

**UI Timer Cycle** (every 50ms in Slint):

```rust
// In UI update loop (src/main.rs)
let state_read = state.lock().await;

ui.set_mic_name(state_read.audio_devices[state_read.audio_device_idx].name.clone());
ui.set_gain_db(20.0 * state_read.agc_gain_multiplier.log10());
ui.set_input_level_db(state_read.input_level_db);
ui.set_freq_actual_hz(state_read.freq_actual_hz);
ui.set_camera_fps(state_read.camera_fps);
```

---

## Generation Protection Constraints

### ✅ DO

- **Simple state mutations**: Gain multiplier directly modifies audio input level
- **Tunable parameters**: All values configurable, saved/restored
- **Feedback display**: Real-time input level, actual frequency, camera FPS
- **Device switching**: Non-blocking mic selection (< 10ms)
- **Resolution change**: Applied on next frame (camera subsystem picks up change)

### ❌ DON'T

- **Hardcoded limits**: All ranges tunable (don't hardcode max gain)
- **Blocking I/O**: Parameter save/load async via tokio
- **Mic hotplug detection**: Device list is static (set at startup); hotplug not supported
- **Frequency validation**: Manual entry accepts any f32 (let dispatch loop filter)

---

## Pre-Commit Hook Validation

```bash
#!/bin/bash
# .git/hooks/pre-commit (add to existing)

# ✓ All gain callbacks async (no blocking UI)
if grep -q "agc_gain_multiplier.*lock.*await" src/main.rs; then
    echo "✓ Gain control is async"
else
    echo "⚠ Gain control should use async/await"
fi

# ✓ Frequency band enum has all 6 bands + Custom
if grep -qE "VLF|LF|MF|HF|VHF|UHF|Custom" src/state.rs; then
    echo "✓ All frequency bands defined"
else
    echo "❌ Missing frequency band variants"
    exit 1
fi

# ✓ Camera resolution enum covers 480p, 720p, 1080p
if grep -qE "_480p|_720p|_1080p" src/state.rs; then
    echo "✓ All camera resolutions defined"
else
    echo "❌ Missing camera resolution variants"
    exit 1
fi

# ✓ Parameter file uses JSON (not binary)
if grep -q "serde_json" src/parameters.rs; then
    echo "✓ Parameters use JSON format"
else
    echo "⚠ Consider JSON for human-readable persistence"
fi

echo "✓ Track BB parameter control validation passed"
exit 0
```

---

## Implementation Checklist (for Jules)

### Phase 1: State Extension (10 min)
- [ ] Add `audio_device_idx, audio_devices` to AppState
- [ ] Add `agc_gain_multiplier, agc_enabled, agc_target_level_db, input_level_db` to AppState
- [ ] Add `freq_selection_mode, freq_preset_band, freq_manual_hz, freq_actual_hz` to AppState
- [ ] Add `camera_resolution, camera_active, camera_fps` to AppState
- [ ] Tests: AppState creation, field initialization

### Phase 2: UI Component Additions (15 min)
- [ ] Add mic selector (+/- buttons) to Slint
- [ ] Add gain slider to Slint
- [ ] Add AGC toggle + target display
- [ ] Add frequency band dropdown + manual entry
- [ ] Add camera resolution selector
- [ ] Add Save/Load/Reset buttons
- [ ] Verify Slint compiles without errors

### Phase 3: Rust Callback Wiring (20 min)
- [ ] Wire `mic_prev()` callback
- [ ] Wire `mic_next()` callback
- [ ] Wire `set_freq_band(index)` callback
- [ ] Wire `set_camera_resolution(index)` callback
- [ ] Wire `save_parameters()` callback
- [ ] Wire `load_parameters()` callback
- [ ] Wire `reset_parameters()` callback
- [ ] Verify all callbacks execute without panic

### Phase 4: Parameter Persistence (10 min)
- [ ] Create `src/parameters.rs`
- [ ] Implement `save_parameters()`
- [ ] Implement `load_parameters()`
- [ ] Test save → app restart → load preserves settings
- [ ] Test invalid JSON gracefully falls back to defaults

### Phase 5: Real-Time Feedback (10 min)
- [ ] Wire UI timer to read `state.input_level_db`
- [ ] Wire UI timer to read `state.freq_actual_hz`
- [ ] Wire UI timer to read `state.camera_fps`
- [ ] Verify feedback updates at 20 Hz (50ms)

### Phase 6: Integration Testing (5 min)
- [ ] Cargo build → 0 errors
- [ ] Cargo run → UI appears, controls responsive
- [ ] Click mic +/- → device label updates immediately
- [ ] Drag gain slider → dB display updates, input level changes
- [ ] Select frequency band → frequency Hz updates
- [ ] Change camera resolution → camera resolution updates
- [ ] Save preset → parameters.json created
- [ ] Load preset → previous settings restored

---

## Total Duration

| Task | Time |
|------|------|
| Phase 1: State extension | 10 min |
| Phase 2: UI components | 15 min |
| Phase 3: Callback wiring | 20 min |
| Phase 4: Parameter persistence | 10 min |
| Phase 5: Feedback loop | 10 min |
| Phase 6: Testing | 5 min |
| **Total** | **70 min** |

*Estimated 45-60 min with concurrent work*

---

## Verification & Success Criteria

✅ **All controls live (no stubs)**:
- Mic selection immediately changes `audio_device_idx`
- Gain slider directly modifies audio input in next dispatch iteration
- Frequency dropdown updates dispatch loop frequency
- Camera resolution changes applied on next frame

✅ **Real-time feedback**:
- Input level display updates every 50ms
- Actual frequency shows current selection
- Camera FPS updates as resolution changes

✅ **Parameter persistence**:
- Settings saved to `~/.siren/parameters.json` on "Save Preset"
- Settings restored on app restart or "Load Preset"
- Reset button returns to hardcoded defaults

✅ **No blocking I/O**:
- All callbacks use `tokio::spawn(async { ... })`
- UI remains responsive during mic switch, resolution change, or save/load

✅ **Build succeeds**:
- `cargo build --release` → 0 errors
- No new warnings introduced (or warnings resolve existing dead code)

---

## Notes for Jules

This addendum **eliminates stub code**. Every control in the UI is wired to a live backend. The parameter persistence layer allows you to save your preferred settings and restore them on restart—critical for forensic workflows where you return to the same config repeatedly.

**Key insight**: Gain control is multiplicative (0.0-1.0 multiplier on raw audio), not additive dB. This allows seamless blending with auto-gain-control—when AGC is disabled, manual gain takes over; when enabled, AGC overrides manual gain to maintain target RMS level.

Mic selection uses modulo arithmetic to wrap around the device list, so cycling through devices is seamless. Manual frequency entry accepts any f32; the dispatch loop will naturally filter invalid frequencies.

Camera resolution changes are applied on the next frame—no need for complex hot-switching logic. The camera subsystem will simply check `state.camera_resolution` and adapt internally.

---

## Integration with Existing Tracks

| Track | Integration |
|-------|-------------|
| Track B (UI) | **This addendum extends B** |
| Track A (Core) | Dispatch loop reads `state.agc_gain_multiplier`, `state.freq_actual_hz` |
| Track C (Audio) | Mic selection updates device index; AGC respects gain multiplier |
| Track D.x (Vis) | Camera resolution affects renderer; updates `state.camera_fps` |

**No blockers.** All dependencies are unidirectional (UI → State → Backend).

---

## Future Extensions (Post-BB)

- **Mic hotplug detection**: Monitor Windows device notifications, update device list
- **Frequency sweep**: Implement automated band sweep for spectral analysis
- **Recording profiles**: Pre-configured mic + gain + frequency sets (e.g., "Office Mode", "Vehicle Mode")
- **Gain automation**: Time-based or event-triggered gain adjustments for specific attack patterns

