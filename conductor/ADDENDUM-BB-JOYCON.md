# Track B Addendum: Joycon Gesture Control (Wizard Wand Interface)

**Status**: Ready for Jules implementation
**Duration**: 90 minutes
**Dependency**: Track B parameter controls already exist; this adds joycon as gesture input device
**Integration**: Joycon hardware → Rust HID handler → AppState parameter mutations → real-time UI feedback

---

## Executive Summary

Track B provides parameter controls (mic selection, gain, frequency, camera resolution) via keyboard/UI. This addendum adds **Nintendo Joy-Con controllers** as intuitive gesture input, enabling "wizard wand" interaction for 3D wavefield manipulation.

**Core Metaphor**: Hold joycon like a wand. Each gesture directly maps to a parameter:

- **Twist wrist** (gyro roll) → Rotate 3D wavefield (viewport rotation)
- **Tilt forward** (gyro pitch) → Steer RF heterodyne elevation (vertical beam angle)
- **Turn side** (gyro yaw) → Steer RF heterodyne azimuth (horizontal beam angle)
- **Squeeze trigger** (analog 0-255) → Increase RF carrier power (synthesis strength)
- **Left stick** (X/Y) → Azimuth/elevation fine steering (when precision needed)
- **Right stick** (X/Y) → Zoom/pan 3D view (spatial navigation)
- **Shake controller** (accelerometer magnitude) → Heterodyne strength modulation (dynamic intensity)
- **Buttons** (A/B/X/Y) → Preset/mode selection (AutoTune, Manual, WaveShape, Recording)

**User Experience**: Spatial synthesis feels like conducting an orchestra—the wand is your baton, the 3D RF field responds to gesture.

---

## Hardware Capabilities

### Joy-Con Input Specifications

| Input | Range | Update Rate | Latency | Precision |
|-------|-------|-------------|---------|-----------|
| **Gyro (Roll/Pitch/Yaw)** | ±2000°/s | 200 Hz | < 5ms | ±1° |
| **Accelerometer (X/Y/Z)** | ±8G | 200 Hz | < 5ms | ±0.1G |
| **Analog Stick (L/R)** | ±32768 | 60 Hz | < 16ms | 4096 positions/axis |
| **Triggers (L/R)** | 0-255 | 60 Hz | < 16ms | 8-bit resolution |
| **Buttons** | Digital | 60 Hz | < 16ms | 13 buttons total |

### Platform Support

- **Windows 11+**: Native HID API (USB/Bluetooth)
- **macOS**: IOKit native support
- **Linux**: evdev/udev support

**Recommended Library**: `gilrs` (cross-platform gamepad abstraction) + `joycon-rs` (Joy-Con specific for gyro/accel)

---

## Slint UI Integration

### File Ownership

- **`ui/joycon_panel.slint`** (NEW) - Joycon status + gesture visualization
- **`src/input/joycon_handler.rs`** (NEW) - HID polling + event processing
- **`src/main.rs`** - Wire joycon callbacks to parameter mutations

### Joycon Status Panel (Add to Main UI)

```slint
// ui/joycon_panel.slint

TabContent {
    title: "JOYCON";

    VerticalLayout {
        spacing: 15px;
        padding: 20px;

        // ─────────────────────────────────────────────────────
        // CONNECTION STATUS
        // ─────────────────────────────────────────────────────
        HorizontalLayout {
            Text { text: "Joy-Con Status:"; min-width: 150px; font-weight: bold; }

            Rectangle {
                background: if root.joycon-connected { #0f0 } else { #f00 };
                width: 20px;
                height: 20px;
                border-radius: 50%;
            }

            Text {
                text: if root.joycon-connected { "Connected (L+R)" } else { "Disconnected" };
                color: if root.joycon-connected { #0f0 } else { #f00 };
                font-weight: bold;
            }

            Text {
                text: "Battery: {root.joycon-battery}%";
                color: if root.joycon-battery > 50 { #0f0 } else if root.joycon-battery > 20 { #ff0 } else { #f00 };
            }
        }

        // ─────────────────────────────────────────────────────
        // GYRO ORIENTATION (3D ATTITUDE INDICATOR)
        // ─────────────────────────────────────────────────────
        HorizontalLayout {
            spacing: 20px;

            VerticalLayout {
                Text { text: "Gyro Orientation:"; font-weight: bold; }

                HorizontalLayout {
                    spacing: 10px;

                    VerticalLayout {
                        width: 80px;
                        Text { text: "Roll:"; }
                        Text { text: "{root.gyro-roll.round()}°"; color: #0f0; }
                    }

                    VerticalLayout {
                        width: 80px;
                        Text { text: "Pitch:"; }
                        Text { text: "{root.gyro-pitch.round()}°"; color: #0f0; }
                    }

                    VerticalLayout {
                        width: 80px;
                        Text { text: "Yaw:"; }
                        Text { text: "{root.gyro-yaw.round()}°"; color: #0f0; }
                    }
                }
            }

            // Attitude indicator (compass-like visualization)
            Rectangle {
                background: #222;
                border: 2px solid #444;
                width: 150px;
                height: 150px;

                VerticalLayout {
                    alignment: center;
                    Text { text: "Gyro\nAttitude"; alignment: center; color: #666; }
                }
                // TODO: Render 3D gyro visualization (future enhancement)
            }
        }

        // ─────────────────────────────────────────────────────
        // ACCELEROMETER (FORCE/INTENSITY CONTROL)
        // ─────────────────────────────────────────────────────
        HorizontalLayout {
            spacing: 20px;

            VerticalLayout {
                Text { text: "Accelerometer (G):"; font-weight: bold; }

                HorizontalLayout {
                    spacing: 10px;

                    VerticalLayout {
                        width: 80px;
                        Text { text: "X:"; }
                        ProgressBar {
                            value: (root.accel-x + 8.0) / 16.0;
                        }
                        Text { text: "{root.accel-x.round(2)} G"; }
                    }

                    VerticalLayout {
                        width: 80px;
                        Text { text: "Y:"; }
                        ProgressBar {
                            value: (root.accel-y + 8.0) / 16.0;
                        }
                        Text { text: "{root.accel-y.round(2)} G"; }
                    }

                    VerticalLayout {
                        width: 80px;
                        Text { text: "Z:"; }
                        ProgressBar {
                            value: (root.accel-z + 8.0) / 16.0;
                        }
                        Text { text: "{root.accel-z.round(2)} G"; }
                    }
                }

                Text {
                    text: "Net Acceleration: {sqrt(root.accel-x * root.accel-x + root.accel-y * root.accel-y + root.accel-z * root.accel-z).round(2)} G";
                    color: #0f0;
                }
                Text { text: "(Shake = Heterodyne Strength)"; color: #888; font-size: 10px; }
            }
        }

        // ─────────────────────────────────────────────────────
        // ANALOG STICKS (STEERING CONTROL)
        // ─────────────────────────────────────────────────────
        HorizontalLayout {
            spacing: 20px;

            VerticalLayout {
                Text { text: "Left Stick (Azimuth/Elevation):"; font-weight: bold; }

                HorizontalLayout {
                    spacing: 10px;

                    VerticalLayout {
                        width: 100px;
                        Text { text: "X (Azimuth):"; }
                        Slider {
                            value: (root.stick-l-x + 1.0) / 2.0;
                            minimum: 0.0;
                            maximum: 1.0;
                        }
                        Text { text: "{(root.stick-l-x * 180.0).round()}°"; color: #0f0; }
                    }

                    VerticalLayout {
                        width: 100px;
                        Text { text: "Y (Elevation):"; }
                        Slider {
                            value: (root.stick-l-y + 1.0) / 2.0;
                            minimum: 0.0;
                            maximum: 1.0;
                        }
                        Text { text: "{(root.stick-l-y * 90.0).round()}°"; color: #0f0; }
                    }
                }

                // Stick visualization
                Rectangle {
                    background: #222;
                    border: 2px solid #444;
                    width: 120px;
                    height: 120px;

                    Circle {
                        background: #0f0;
                        width: 8px;
                        height: 8px;
                        x: 60px + (root.stick-l-x * 40px);
                        y: 60px - (root.stick-l-y * 40px);  // Y inverted for screen coords
                    }
                }
            }

            VerticalLayout {
                Text { text: "Right Stick (Zoom/Pan):"; font-weight: bold; }

                HorizontalLayout {
                    spacing: 10px;

                    VerticalLayout {
                        width: 100px;
                        Text { text: "X (Pan):"; }
                        Slider {
                            value: (root.stick-r-x + 1.0) / 2.0;
                            minimum: 0.0;
                            maximum: 1.0;
                        }
                        Text { text: "{(root.stick-r-x * 100).round()}%"; color: #0f0; }
                    }

                    VerticalLayout {
                        width: 100px;
                        Text { text: "Y (Zoom):"; }
                        Slider {
                            value: (root.stick-r-y + 1.0) / 2.0;
                            minimum: 0.0;
                            maximum: 1.0;
                        }
                        Text { text: "{(1.0 + root.stick-r-y).round(2)}x"; color: #0f0; }
                    }
                }

                // Stick visualization
                Rectangle {
                    background: #222;
                    border: 2px solid #444;
                    width: 120px;
                    height: 120px;

                    Circle {
                        background: #0f0;
                        width: 8px;
                        height: 8px;
                        x: 60px + (root.stick-r-x * 40px);
                        y: 60px - (root.stick-r-y * 40px);
                    }
                }
            }
        }

        // ─────────────────────────────────────────────────────
        // TRIGGERS (RF POWER CONTROL)
        // ─────────────────────────────────────────────────────
        HorizontalLayout {
            spacing: 20px;

            VerticalLayout {
                width: 200px;
                Text { text: "L Trigger (Reserved):"; }
                ProgressBar {
                    value: root.trigger-l / 255.0;
                }
                Text { text: "{(root.trigger-l / 255.0 * 100).round()}%"; color: #0f0; }
            }

            VerticalLayout {
                width: 200px;
                Text { text: "R Trigger (RF Power):"; }
                ProgressBar {
                    value: root.trigger-r / 255.0;
                }
                Text { text: "{(root.trigger-r / 255.0 * 100).round()}%"; color: #0f0; }
                Text { text: "(Squeeze = Increase Power)"; color: #888; font-size: 10px; }
            }

            VerticalLayout {
                width: 200px;
                Text { text: "Carrier Frequency:"; }
                Text { text: "{root.carrier-frequency-hz / 1e9}GHz"; color: #0f0; font-weight: bold; }
                Text { text: "(Derived from trigger)"; color: #888; font-size: 10px; }
            }
        }

        // ─────────────────────────────────────────────────────
        // BUTTONS (MODE SELECTION)
        // ─────────────────────────────────────────────────────
        HorizontalLayout {
            spacing: 10px;

            Button {
                text: "A: AutoTune";
                background: #0f0 if root.button-a-pressed else #333;
                clicked => { app-window.mode_autotune(); }
            }

            Button {
                text: "B: Manual";
                background: #f00 if root.button-b-pressed else #333;
                clicked => { app-window.mode_manual(); }
            }

            Button {
                text: "X: WaveShape";
                background: #00f if root.button-x-pressed else #333;
                clicked => { app-window.mode_waveshape(); }
            }

            Button {
                text: "Y: Record";
                background: #ff0 if root.button-y-pressed else #333;
                clicked => { app-window.mode_record(); }
            }
        }

        // ─────────────────────────────────────────────────────
        // GESTURE MAPPING REFERENCE
        // ─────────────────────────────────────────────────────
        Rectangle {
            background: #1a1a1a;
            border: 2px solid #444;
            height: 180px;

            VerticalLayout {
                padding: 10px;
                spacing: 5px;

                Text { text: "Gesture Mapping Reference:"; font-weight: bold; color: #0f0; }

                Text { text: "• Twist wrist (roll ±90°) → Rotate 3D wavefield"; }
                Text { text: "• Tilt forward (pitch ±90°) → Steer heterodyne elevation"; }
                Text { text: "• Turn side (yaw ±180°) → Steer heterodyne azimuth"; }
                Text { text: "• Squeeze R trigger (0-255) → Increase RF carrier power"; }
                Text { text: "• Left stick → Precise azimuth/elevation (when hand gets tired)"; }
                Text { text: "• Right stick → Zoom in/out and pan around RF field"; }
                Text { text: "• Shake controller → Modulate heterodyne strength dynamically"; }
                Text { text: "• Buttons A/B/X/Y → Switch between AutoTune, Manual, WaveShape, Record modes"; }
            }
        }
    }
}

// ─────────────────────────────────────────────────────
// EXPORTED PROPERTIES (Joycon ↔ Slint bindings)
// ─────────────────────────────────────────────────────

export global JoyconStatus {
    // Connection
    in-out property <bool> joycon-connected;
    in-out property <int> joycon-battery;

    // Gyro (degrees)
    in-out property <float> gyro-roll;
    in-out property <float> gyro-pitch;
    in-out property <float> gyro-yaw;

    // Accelerometer (G)
    in-out property <float> accel-x;
    in-out property <float> accel-y;
    in-out property <float> accel-z;

    // Analog sticks ([-1, 1])
    in-out property <float> stick-l-x;
    in-out property <float> stick-l-y;
    in-out property <float> stick-r-x;
    in-out property <float> stick-r-y;

    // Triggers (0-255)
    in-out property <int> trigger-l;
    in-out property <int> trigger-r;

    // Derived parameters
    in-out property <float> carrier-frequency-hz;
    in-out property <float> heterodyne-azimuth;   // In degrees
    in-out property <float> heterodyne-elevation; // In degrees
    in-out property <float> heterodyne-strength;  // [0, 1]
    in-out property <float> viewport-zoom;        // 1.0 = normal

    // Button states
    in-out property <bool> button-a-pressed;
    in-out property <bool> button-b-pressed;
    in-out property <bool> button-x-pressed;
    in-out property <bool> button-y-pressed;

    // Callbacks
    callback mode_autotune();
    callback mode_manual();
    callback mode_waveshape();
    callback mode_record();
}
```

---

## Rust Implementation

### File Ownership

- **`src/input/joycon_handler.rs`** (NEW) - HID polling + event processing
- **`src/main.rs`** - Joycon spawn task + parameter mutations

### Joycon Handler

```rust
// src/input/joycon_handler.rs

use gilrs::{Gilrs, GamepadId, Button, Axis};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

pub struct JoyconHandler {
    pub gilrs: Gilrs,
    pub joycon_id: Option<GamepadId>,
    pub state: Arc<Mutex<JoyconState>>,
}

pub struct JoyconState {
    pub connected: bool,
    pub battery: u8,

    // Gyro (degrees, rate-integrated)
    pub gyro_roll: f32,
    pub gyro_pitch: f32,
    pub gyro_yaw: f32,

    // Accelerometer (G)
    pub accel_x: f32,
    pub accel_y: f32,
    pub accel_z: f32,

    // Analog sticks ([-1, 1])
    pub stick_l_x: f32,
    pub stick_l_y: f32,
    pub stick_r_x: f32,
    pub stick_r_y: f32,

    // Triggers (0-255)
    pub trigger_l: u8,
    pub trigger_r: u8,

    // Buttons
    pub buttons: HashMap<Button, bool>,

    // Derived
    pub carrier_frequency_hz: f32,
    pub heterodyne_azimuth: f32,
    pub heterodyne_elevation: f32,
    pub heterodyne_strength: f32,
}

impl JoyconHandler {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let gilrs = Gilrs::new()?;

        let joycon_id = gilrs.gamepads()
            .find(|(_, gp)| {
                let name = gp.name().to_lowercase();
                name.contains("joy-con") || name.contains("joycon")
            })
            .map(|(id, _)| id);

        if joycon_id.is_none() {
            eprintln!("[Joycon] No Joy-Con detected. Continuing without gamepad input.");
        }

        Ok(JoyconHandler {
            gilrs,
            joycon_id,
            state: Arc::new(Mutex::new(JoyconState {
                connected: joycon_id.is_some(),
                battery: 100,
                gyro_roll: 0.0,
                gyro_pitch: 0.0,
                gyro_yaw: 0.0,
                accel_x: 0.0,
                accel_y: 0.0,
                accel_z: 9.81,  // At rest, Z = gravity
                stick_l_x: 0.0,
                stick_l_y: 0.0,
                stick_r_x: 0.0,
                stick_r_y: 0.0,
                trigger_l: 0,
                trigger_r: 0,
                buttons: HashMap::new(),
                carrier_frequency_hz: 2.4e9,
                heterodyne_azimuth: 0.0,
                heterodyne_elevation: 0.0,
                heterodyne_strength: 0.5,
            })),
        })
    }

    pub async fn poll_events(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut state = self.state.lock().await;

        // Poll gamepad events
        while let Some(event) = self.gilrs.next_event() {
            match event {
                gilrs::ev::Event::ButtonPressed(button, gp_id) if Some(gp_id) == self.joycon_id => {
                    state.buttons.insert(button, true);
                }
                gilrs::ev::Event::ButtonReleased(button, gp_id) if Some(gp_id) == self.joycon_id => {
                    state.buttons.insert(button, false);
                }
                gilrs::ev::Event::AxisChanged(axis, value, gp_id) if Some(gp_id) == self.joycon_id => {
                    match axis {
                        Axis::LeftStickX => state.stick_l_x = value,
                        Axis::LeftStickY => state.stick_l_y = -value,  // Invert Y
                        Axis::RightStickX => state.stick_r_x = value,
                        Axis::RightStickY => state.stick_r_y = -value,
                        Axis::LT => state.trigger_l = ((value + 1.0) * 127.5) as u8,
                        Axis::RT => state.trigger_r = ((value + 1.0) * 127.5) as u8,
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // **Note**: Gyro/accel require native HID; gilrs doesn't expose them
        // TODO: Integrate joycon-rs for gyro/accel support

        Ok(())
    }

    /// Derive parameter values from joycon input
    pub async fn update_parameters(&self, app_state: &Arc<Mutex<AppState>>) {
        let jc_state = self.state.lock().await;
        let mut st = app_state.lock().await;

        // Gyro → Viewport rotation (3D wavefield)
        // roll ±2000°/s → keep accumulated rotation in quaternion
        st.viewport_rotation_pitch = jc_state.gyro_pitch.clamp(-90.0, 90.0);
        st.viewport_rotation_yaw = jc_state.gyro_yaw.clamp(-180.0, 180.0);

        // Left stick → Azimuth/elevation steering (precise control)
        st.heterodyne_azimuth = jc_state.stick_l_x * 180.0;      // [-180, 180]
        st.heterodyne_elevation = jc_state.stick_l_y * 90.0;     // [-90, 90]

        // Right stick → Zoom
        st.viewport_zoom = 1.0 + (jc_state.stick_r_y * 0.5);     // [0.5, 1.5]x

        // R Trigger → Carrier frequency modulation
        let trigger_normalized = jc_state.trigger_r as f32 / 255.0;
        st.carrier_frequency_hz = 2.4e9 + (trigger_normalized * 1.0e8);  // [2.4GHz, 2.5GHz]

        // Accelerometer magnitude → Heterodyne strength
        let accel_magnitude = (
            jc_state.accel_x.powi(2) +
            jc_state.accel_y.powi(2) +
            (jc_state.accel_z - 9.81).powi(2)  // Remove gravity
        ).sqrt();
        st.heterodyne_strength = (accel_magnitude / 16.0).clamp(0.0, 1.0);

        // Buttons → Mode selection
        if *jc_state.buttons.get(&Button::North).unwrap_or(&false) {
            st.detection_mode = DetectionMode::AutoTune;
        } else if *jc_state.buttons.get(&Button::East).unwrap_or(&false) {
            st.detection_mode = DetectionMode::Manual;
        } else if *jc_state.buttons.get(&Button::South).unwrap_or(&false) {
            st.detection_mode = DetectionMode::WaveShape;
        }
    }
}
```

### Main Loop Integration

```rust
// src/main.rs - Spawn joycon polling task

let mut joycon_handler = match JoyconHandler::new() {
    Ok(handler) => handler,
    Err(e) => {
        eprintln!("[Joycon] Initialization failed: {}. Continuing without gamepad.", e);
        // Create dummy handler that doesn't crash on poll()
        JoyconHandler::new_dummy()
    }
};

// Spawn joycon polling task
tokio::spawn({
    let state = state.clone();
    async move {
        let joycon = joycon_handler;

        loop {
            // Poll joycon events (non-blocking)
            if let Err(e) = joycon.poll_events().await {
                eprintln!("[Joycon] Poll error: {}", e);
            }

            // Update app state from joycon input
            joycon.update_parameters(&state).await;

            // Update Slint UI globals (every 16ms = 60 Hz)
            let jc_st = joycon.state.lock().await;
            let ui_jc = ui.global::<JoyconStatus>();

            ui_jc.set_gyro_roll(jc_st.gyro_roll);
            ui_jc.set_gyro_pitch(jc_st.gyro_pitch);
            ui_jc.set_gyro_yaw(jc_st.gyro_yaw);
            ui_jc.set_accel_x(jc_st.accel_x);
            ui_jc.set_accel_y(jc_st.accel_y);
            ui_jc.set_accel_z(jc_st.accel_z);
            ui_jc.set_stick_l_x(jc_st.stick_l_x);
            ui_jc.set_stick_l_y(jc_st.stick_l_y);
            ui_jc.set_stick_r_x(jc_st.stick_r_x);
            ui_jc.set_stick_r_y(jc_st.stick_r_y);
            ui_jc.set_trigger_l(jc_st.trigger_l as i32);
            ui_jc.set_trigger_r(jc_st.trigger_r as i32);
            ui_jc.set_carrier_frequency_hz(jc_st.carrier_frequency_hz);

            tokio::time::sleep(Duration::from_millis(16)).await;  // 60 Hz
        }
    }
});
```

---

## Gesture Mapping Table

| Gesture | Joycon Input | Parameter Updated | Range | Effect |
|---------|-------|----------|-------|--------|
| **Twist wrist right** | Gyro roll +90° | viewport_rotation_roll | -180°…+180° | 3D wavefield rotates clockwise |
| **Twist wrist left** | Gyro roll -90° | viewport_rotation_roll | -180°…+180° | 3D wavefield rotates counter-clockwise |
| **Tilt forward** | Gyro pitch +90° | heterodyne_elevation | -90°…+90° | RF beam steers upward |
| **Tilt backward** | Gyro pitch -90° | heterodyne_elevation | -90°…+90° | RF beam steers downward |
| **Turn right** | Gyro yaw +180° | heterodyne_azimuth | -180°…+180° | RF beam pans right |
| **Turn left** | Gyro yaw -180° | heterodyne_azimuth | -180°…+180° | RF beam pans left |
| **Left stick → up** | Stick Y +1 | heterodyne_elevation | -90°…+90° | Precise elevation steering |
| **Left stick → down** | Stick Y -1 | heterodyne_elevation | -90°…+90° | Precise elevation steering |
| **Left stick → right** | Stick X +1 | heterodyne_azimuth | -180°…+180° | Precise azimuth steering |
| **Left stick → left** | Stick X -1 | heterodyne_azimuth | -180°…+180° | Precise azimuth steering |
| **Right stick → up** | Stick Y +1 | viewport_zoom | 0.5x…1.5x | Zoom in on RF field |
| **Right stick → down** | Stick Y -1 | viewport_zoom | 0.5x…1.5x | Zoom out from RF field |
| **Right stick → right** | Stick X +1 | (reserved for future) | - | Pan right (future) |
| **Right stick → left** | Stick X -1 | (reserved for future) | - | Pan left (future) |
| **Squeeze R trigger** | Trigger 0→255 | carrier_frequency_hz | 2.4GHz…2.5GHz | Increase RF synthesis power |
| **Release R trigger** | Trigger 255→0 | carrier_frequency_hz | 2.4GHz…2.4GHz | Reduce RF synthesis power |
| **Shake controller** | Accel magnitude | heterodyne_strength | [0, 1] | Dynamic heterodyne modulation |
| **Press A** | Button A | detection_mode | - | Switch to AutoTune |
| **Press B** | Button B | detection_mode | - | Switch to Manual |
| **Press X** | Button X | detection_mode | - | Switch to WaveShape |
| **Press Y** | Button Y | detection_mode | - | Switch to Record |

---

## Latency Budget

| Stage | Time | Cumulative |
|-------|------|-----------|
| Joycon HID poll (60 Hz) | < 5ms | 5ms |
| Update parameters | < 1ms | 6ms |
| Update AppState | < 1ms | 7ms |
| GPU wavefield rotation | < 10ms | 17ms |
| Slint UI update | < 3ms | 20ms |
| Frame render | < 12ms | 32ms |

**Total**: ~32ms end-to-end (responsive 30 fps when hardware is busy, 60+ fps normally)

---

## Implementation Checklist (for Jules)

### Phase 1: Slint UI (20 min)
- [ ] Create ui/joycon_panel.slint
- [ ] Add JoyconStatus global with all properties
- [ ] Add gesture mapping reference documentation
- [ ] Verify Slint compiles

### Phase 2: Joycon Handler (25 min)
- [ ] Create src/input/joycon_handler.rs
- [ ] Implement JoyconState struct
- [ ] Wire gilrs for polling (sticks, buttons, triggers)
- [ ] Placeholder for native gyro/accel (pending joycon-rs)
- [ ] Tests: Event polling, state accumulation

### Phase 3: Parameter Mapping (15 min)
- [ ] Implement update_parameters() method
- [ ] Map gyro → viewport rotation (pitch/yaw)
- [ ] Map sticks → heterodyne azimuth/elevation
- [ ] Map triggers → carrier frequency
- [ ] Map accel magnitude → heterodyne strength
- [ ] Map buttons → mode selection

### Phase 4: Main Loop Wiring (15 min)
- [ ] Spawn joycon polling task in main.rs
- [ ] Update AppState from joycon every frame
- [ ] Update Slint UI globals every 16ms
- [ ] Verify latency < 35ms (with headroom)
- [ ] Tests: End-to-end gesture response

### Phase 5: Native HID Support (20 min)
- [ ] Add joycon-rs to Cargo.toml
- [ ] Integrate gyro/accel polling into JoyconHandler
- [ ] Test with real Joy-Con hardware (if available)
- [ ] Tests: Gyro orientation, accelerometer magnitude

### Phase 6: Testing & Polish (15 min)
- [ ] Cargo build → 0 errors
- [ ] Connect real Joy-Cons or mock input
- [ ] Test each gesture → verify parameter update
- [ ] Test mode selection (A/B/X/Y buttons)
- [ ] Monitor framerate (should stay 60+ fps with joycon active)
- [ ] Tests: Full workflow, no crashes on disconnect

---

## Total Duration

| Task | Time |
|------|------|
| Phase 1: Slint UI | 20 min |
| Phase 2: Joycon handler | 25 min |
| Phase 3: Parameter mapping | 15 min |
| Phase 4: Main loop wiring | 15 min |
| Phase 5: Native HID (gyro/accel) | 20 min |
| Phase 6: Testing + polish | 15 min |
| **Total** | **110 min** |

*Estimated 90 min with concurrent work (Phases 2-4 can overlap)*

---

## Verification & Success Criteria

✅ **Joycon detected and connected**:
- UI shows "Connected (L+R)"
- Battery percentage updates correctly
- No crashes if joycon unplugs mid-session

✅ **All inputs functional**:
- Gyro outputs in real-time (UI display)
- Sticks move slider visualizations
- Triggers show progress bars (0-255)
- Buttons highlight when pressed (A/B/X/Y)

✅ **Gesture mapping responsive**:
- Twist wrist → 3D wavefield rotates smoothly (<32ms latency)
- Tilt forward → RF beam elevation changes in real-time
- Left stick → Precise azimuth/elevation steering works
- Right stick → Zoom in/out responsive
- R trigger squeeze → Carrier frequency modulates
- Accelerometer shake → Heterodyne strength responds
- All gestures latency < 35ms (target 32ms)

✅ **Mode selection works**:
- Button A → AutoTune mode active
- Button B → Manual mode active
- Button X → WaveShape mode active
- Button Y → Record mode active
- Mode changes reflected in UI immediately

✅ **Integration complete**:
- AppState updates from joycon input
- Slint UI reflects all joycon state in real-time
- No framerate drops (60+ fps maintained)
- Graceful fallback if joycon disconnected (no crashes)

---

## Notes for Jules

**Why the "wizard wand" metaphor?** Gesture-based 3D manipulation is intuitive—users already understand wand/baton physics from conducting orchestras, lightsaber games, or painting with motion controllers. Gyro = natural extension of wrist/hand gesture.

**Latency critical?** Yes. Sub-50ms response is essential for immersion. Any delay > 100ms breaks the illusion that the wand controls the wavefield. Target is 32ms, achieved by 60 Hz joycon polling + non-blocking state updates.

**Fallback if disconnected?** Joycon detection is graceful—if USB unplugs or Bluetooth drops, the handler enters a safe "no-op" state. Keyboard/UI controls still work. On reconnect, handler polls events again. No crashes.

**Future enhancement:** IMU fusion (gyro integral + accel zero-crossing) can improve orientation stability vs. raw gyro alone. This is Phase 5 (native HID) work.

**Gyro persistence issue:** Current implementation doesn't persist gyro orientation across frames (would need IMU integration). For now, gyro drift is acceptable since user is actively holding and moving the wand. Future: add complementary filter (gyro + accel + magnetometer) for static orientation estimation.

---

## Dependencies

Add to Cargo.toml:
```toml
gilrs = "0.11"          # Gamepad abstraction (cross-platform)
joycon-rs = "0.1"       # Joy-Con specific (gyro/accel, optional for Phase 5)
tokio = { version = "1", features = ["full"] }
```

---

## Related Addendums

- **ADDENDUM-FF (Material Framework)**: Joycon can be combined with material editor to place points while holding the wand
- **ADDENDUM-BB (Parameter Control)**: These joycon gestures complement keyboard/UI parameter controls—either input method works
- **ADDENDUM-DD (Temporal Rewind)**: Joycon right stick can be wired to timeline scrubbing (future enhancement)
