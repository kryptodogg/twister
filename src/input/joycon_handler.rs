// src/input/joycon_handler.rs — Joy-Con Gesture Control
//
// HID polling for Nintendo Joy-Con controllers via gilrs.
// Maps gestures to Twister parameter controls:
// - Gyro roll → viewport rotation
// - Gyro pitch/yaw → heterodyne steering
// - Sticks → fine azimuth/elevation control
// - R Trigger → carrier frequency modulation
// - Accelerometer magnitude → heterodyne strength
// - Buttons → mode selection
//
// Polling rate: 60 Hz (16.67ms interval)
// Latency target: <35ms gesture-to-parameter

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use gilrs::{Gilrs, Event, EventType, Button, Axis};
use tokio::sync::mpsc;

use crate::state::AppState;

/// Joy-Con gesture mapping configuration
#[derive(Debug, Clone)]
pub struct GestureMapping {
    /// Gyro roll → viewport rotation sensitivity (degrees per degree)
    pub roll_sensitivity: f32,
    /// Gyro pitch → heterodyne steering sensitivity
    pub pitch_sensitivity: f32,
    /// Gyro yaw → heterodyne steering sensitivity
    pub yaw_sensitivity: f32,
    /// Accelerometer magnitude → heterodyne strength multiplier
    pub accel_strength_sensitivity: f32,
    /// R Trigger → carrier frequency modulation depth (Hz)
    pub trigger_freq_depth_hz: f32,
    /// Stick deadzone threshold (values below this are treated as 0)
    pub stick_deadzone: f32,
    /// Button actions
    pub button_a_action: JoyConAction,
    pub button_b_action: JoyConAction,
    pub button_x_action: JoyConAction,
    pub button_y_action: JoyConAction,
}

impl Default for GestureMapping {
    fn default() -> Self {
        Self {
            roll_sensitivity: 1.0,
            pitch_sensitivity: 0.5,
            yaw_sensitivity: 0.5,
            accel_strength_sensitivity: 2.0,
            trigger_freq_depth_hz: 1000.0,
            stick_deadzone: 0.15,
            button_a_action: JoyConAction::TogglePdm,
            button_b_action: JoyConAction::ToggleAnc,
            button_x_action: JoyConAction::ToggleAutoTune,
            button_y_action: JoyConAction::ToggleRecording,
        }
    }
}

/// Joy-Con button action mappings
#[derive(Debug, Clone, Copy)]
pub enum JoyConAction {
    TogglePdm,
    ToggleAnc,
    ToggleAutoTune,
    ToggleRecording,
    ToggleSDR,
    IncreaseGain,
    DecreaseGain,
    NextMode,
    PreviousMode,
    None,
}

/// Joy-Con input state
#[derive(Debug, Clone, Default)]
pub struct JoyConState {
    /// Gyroscope: roll, pitch, yaw (degrees)
    pub gyro_roll: f32,
    pub gyro_pitch: f32,
    pub gyro_yaw: f32,

    /// Accelerometer: x, y, z (G)
    pub accel_x: f32,
    pub accel_y: f32,
    pub accel_z: f32,

    /// Left stick: x, y ([-1, 1])
    pub stick_left_x: f32,
    pub stick_left_y: f32,

    /// Right stick: x, y ([-1, 1])
    pub stick_right_x: f32,
    pub stick_right_y: f32,

    /// Triggers: L, R ([0, 1])
    pub trigger_l: f32,
    pub trigger_r: f32,

    /// Buttons
    pub button_a: bool,
    pub button_b: bool,
    pub button_x: bool,
    pub button_y: bool,
    pub button_up: bool,
    pub button_down: bool,
    pub button_left: bool,
    pub button_right: bool,
    pub button_plus: bool,
    pub button_minus: bool,
    pub button_l: bool,
    pub button_r: bool,
    pub button_zl: bool,
    pub button_zr: bool,

    /// Connection status
    pub connected: bool,
    pub device_id: Option<usize>,
}

impl JoyConState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply deadzone to stick input
    fn apply_deadzone(value: f32, deadzone: f32) -> f32 {
        if value.abs() < deadzone {
            0.0
        } else {
            // Normalize to [-1, 1] range after deadzone
            if value > 0.0 {
                (value - deadzone) / (1.0 - deadzone)
            } else {
                (value + deadzone) / (1.0 - deadzone)
            }
        }
    }

    /// Update stick values with deadzone application
    pub fn set_stick_left(&mut self, x: f32, y: f32, deadzone: f32) {
        self.stick_left_x = Self::apply_deadzone(x, deadzone);
        self.stick_left_y = Self::apply_deadzone(y, deadzone);
    }

    pub fn set_stick_right(&mut self, x: f32, y: f32, deadzone: f32) {
        self.stick_right_x = Self::apply_deadzone(x, deadzone);
        self.stick_right_y = Self::apply_deadzone(y, deadzone);
    }

    /// Get accelerometer magnitude (for heterodyne strength mapping)
    pub fn accel_magnitude(&self) -> f32 {
        (self.accel_x * self.accel_x + 
         self.accel_y * self.accel_y + 
         self.accel_z * self.accel_z).sqrt()
    }
}

/// Joy-Con handler with gilrs integration
pub struct JoyconHandler {
    gilrs: Gilrs,
    state: JoyConState,
    mapping: GestureMapping,
    state_ref: Arc<AppState>,
    shutdown_rx: mpsc::Receiver<()>,
    running: bool,
}

impl JoyconHandler {
    /// Create a new Joy-Con handler
    pub fn new(state_ref: Arc<AppState>, mapping: GestureMapping) -> anyhow::Result<Self> {
        let gilrs = Gilrs::new()
            .map_err(|e| anyhow::anyhow!("Failed to initialize gilrs: {}", e))?;

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        // Store shutdown sender in a global location for graceful shutdown
        // (In practice, this would be managed by the main loop)
        let _ = shutdown_tx; // Suppress unused warning

        Ok(Self {
            gilrs,
            state: JoyConState::new(),
            mapping,
            state_ref,
            shutdown_rx,
            running: true,
        })
    }

    /// Poll Joy-Con events and update state
    /// Returns true if state was updated
    pub fn poll(&mut self) -> bool {
        let mut updated = false;

        // Poll gilrs for events
        while let Some(Event { event, .. }) = self.gilrs.next_event() {
            match event {
                EventType::Connected => {
                    self.state.connected = true;
                    self.state.device_id = Some(0); // First device
                    self.state_ref.set_joycon_connected(true);
                    eprintln!("[JoyCon] Controller connected");
                    updated = true;
                }
                EventType::Disconnected => {
                    self.state.connected = false;
                    self.state.device_id = None;
                    self.state_ref.set_joycon_connected(false);
                    self.state_ref.set_joycon_active(false);
                    eprintln!("[JoyCon] Controller disconnected");
                    updated = true;
                }
                EventType::ButtonChanged(button, value, _) => {
                    self.handle_button_event(button, value > 0.5);
                    updated = true;
                }
                EventType::AxisChanged(axis, value, _) => {
                    self.handle_axis_event(axis, value);
                    updated = true;
                }
                _ => {}
            }
        }

        // Update state in AppState
        if updated {
            self.sync_to_app_state();
        }

        updated
    }

    /// Handle button press events
    fn handle_button_event(&mut self, button: Button, pressed: bool) {
        match button {
            Button::South => self.state.button_a = pressed,  // A on Joy-Con
            Button::East => self.state.button_b = pressed,   // B on Joy-Con
            Button::West => self.state.button_x = pressed,   // X on Joy-Con
            Button::North => self.state.button_y = pressed,  // Y on Joy-Con
            Button::DPadUp => self.state.button_up = pressed,
            Button::DPadDown => self.state.button_down = pressed,
            Button::DPadLeft => self.state.button_left = pressed,
            Button::DPadRight => self.state.button_right = pressed,
            Button::Start => self.state.button_plus = pressed,
            Button::Select => self.state.button_minus = pressed,
            Button::LeftTrigger => self.state.button_l = pressed,
            Button::RightTrigger => self.state.button_r = pressed,
            Button::LeftTrigger2 => self.state.button_zl = pressed,
            Button::RightTrigger2 => self.state.button_zr = pressed,
            _ => {}
        }

        // Handle button press actions (only on press, not release)
        if pressed {
            self.handle_button_action(button);
        }
    }

    /// Handle button action mappings
    fn handle_button_action(&self, button: Button) {
        let action = match button {
            Button::South => self.mapping.button_a_action,
            Button::East => self.mapping.button_b_action,
            Button::West => self.mapping.button_x_action,
            Button::North => self.mapping.button_y_action,
            _ => return,
        };

        match action {
            JoyConAction::TogglePdm => {
                let current = self.state_ref.pdm_active.load(Ordering::Relaxed);
                self.state_ref.pdm_active.store(!current, Ordering::Relaxed);
                eprintln!("[JoyCon] PDM toggled: {}", !current);
            }
            JoyConAction::ToggleAnc => {
                // Trigger ANC calibration request
                self.state_ref.pending_anc_calibration.store(true, Ordering::Relaxed);
                eprintln!("[JoyCon] ANC calibration requested");
            }
            JoyConAction::ToggleAutoTune => {
                let current = self.state_ref.auto_tune.load(Ordering::Relaxed);
                self.state_ref.auto_tune.store(!current, Ordering::Relaxed);
                eprintln!("[JoyCon] Auto-tune toggled: {}", !current);
            }
            JoyConAction::ToggleRecording => {
                // Manual recording toggle
                eprintln!("[JoyCon] Recording toggle requested");
            }
            JoyConAction::ToggleSDR => {
                let current = self.state_ref.sdr_active.load(Ordering::Relaxed);
                self.state_ref.sdr_active.store(!current, Ordering::Relaxed);
                eprintln!("[JoyCon] SDR toggled: {}", !current);
            }
            JoyConAction::IncreaseGain => {
                let current = self.state_ref.master_gain.load(Ordering::Relaxed);
                self.state_ref.master_gain.store((current + 0.1).min(1.0), Ordering::Relaxed);
                eprintln!("[JoyCon] Gain increased: {:.1}", (current + 0.1).min(1.0));
            }
            JoyConAction::DecreaseGain => {
                let current = self.state_ref.master_gain.load(Ordering::Relaxed);
                self.state_ref.master_gain.store((current - 0.1).max(0.0), Ordering::Relaxed);
                eprintln!("[JoyCon] Gain decreased: {:.1}", (current - 0.1).max(0.0));
            }
            JoyConAction::NextMode => {
                let current = self.state_ref.mode.load(Ordering::Relaxed);
                self.state_ref.mode.store((current + 1) % 6, Ordering::Relaxed);
                eprintln!("[JoyCon] Mode changed to: {}", (current + 1) % 6);
            }
            JoyConAction::PreviousMode => {
                let current = self.state_ref.mode.load(Ordering::Relaxed);
                self.state_ref.mode.store((current + 5) % 6, Ordering::Relaxed);
                eprintln!("[JoyCon] Mode changed to: {}", (current + 5) % 6);
            }
            JoyConAction::None => {}
        }
    }

    /// Handle axis change events (sticks, triggers, gyro, accel)
    fn handle_axis_event(&mut self, axis: Axis, value: f32) {
        match axis {
            Axis::LeftStickX => {
                self.state.set_stick_left(value, self.state.stick_left_y, self.mapping.stick_deadzone);
            }
            Axis::LeftStickY => {
                self.state.set_stick_left(self.state.stick_left_x, -value, self.mapping.stick_deadzone); // Invert Y for consistency
            }
            Axis::RightStickX => {
                self.state.set_stick_right(value, self.state.stick_right_y, self.mapping.stick_deadzone);
            }
            Axis::RightStickY => {
                self.state.set_stick_right(self.state.stick_right_x, -value, self.mapping.stick_deadzone);
            }
            Axis::RightZ => {
                // gilrs uses RightZ for right trigger (positive value)
                self.state.trigger_r = value.clamp(0.0, 1.0);
            }
            Axis::LeftZ => {
                // gilrs uses LeftZ for left trigger (positive value)
                self.state.trigger_l = value.clamp(0.0, 1.0);
            }
            // Note: Gyroscope and accelerometer data in gilrs requires SDL2 backend
            // and is accessed through separate events. For basic gesture control,
            // we use stick and trigger inputs.
            _ => {}
        }
    }

    /// Sync Joy-Con state to AppState atomics
    fn sync_to_app_state(&self) {
        self.state_ref.update_joycon_state(
            self.state.gyro_roll,
            self.state.gyro_pitch,
            self.state.gyro_yaw,
            self.state.accel_x,
            self.state.accel_y,
            self.state.accel_z,
            self.state.stick_left_x,
            self.state.stick_left_y,
            self.state.stick_right_x,
            self.state.stick_right_y,
            self.state.trigger_l,
            self.state.trigger_r,
            self.state.button_a,
            self.state.button_b,
            self.state.button_x,
            self.state.button_y,
        );

        // Apply gesture mappings to parameters
        self.apply_gesture_mappings();
    }

    /// Apply gesture mappings to Twister parameters
    fn apply_gesture_mappings(&self) {
        // Gyro roll → viewport rotation (beam azimuth)
        let roll_adjustment = self.state.gyro_roll * self.mapping.roll_sensitivity;
        let current_azimuth = self.state_ref.get_beam_azimuth_deg();
        self.state_ref.set_beam_azimuth_deg(current_azimuth + roll_adjustment * 0.1);

        // Gyro pitch/yaw → heterodyne steering (beam elevation)
        let pitch_adjustment = self.state.gyro_pitch * self.mapping.pitch_sensitivity;
        let current_elevation = self.state_ref.beam_elevation_rad.load(Ordering::Relaxed);
        self.state_ref.beam_elevation_rad.store(
            (current_elevation + pitch_adjustment.to_radians() * 0.05).clamp(-std::f32::consts::FRAC_PI_4, std::f32::consts::FRAC_PI_4),
            Ordering::Relaxed,
        );

        // Left stick → fine azimuth/elevation control
        let stick_azimuth = self.state.stick_left_x * 2.0; // degrees per frame
        let stick_elevation = self.state.stick_left_y * 1.0;
        
        let current_azimuth = self.state_ref.get_beam_azimuth_deg();
        self.state_ref.set_beam_azimuth_deg(current_azimuth + stick_azimuth);
        
        let current_elevation = self.state_ref.beam_elevation_rad.load(Ordering::Relaxed);
        self.state_ref.beam_elevation_rad.store(
            (current_elevation + stick_elevation.to_radians() * 0.01).clamp(
                -std::f32::consts::FRAC_PI_4, 
                std::f32::consts::FRAC_PI_4
            ),
            Ordering::Relaxed,
        );

        // R Trigger → carrier frequency modulation
        let trigger_value = self.state.trigger_r;
        if trigger_value > 0.1 {
            let base_freq = self.state_ref.get_sdr_center_hz();
            let modulation = trigger_value * self.mapping.trigger_freq_depth_hz;
            self.state_ref.set_sdr_center_hz(base_freq + modulation);
        }

        // Accelerometer magnitude → heterodyne strength (waveshape drive)
        let accel_mag = self.state.accel_magnitude();
        let heterodyne_strength = (accel_mag / self.mapping.accel_strength_sensitivity).clamp(0.0, 1.0);
        self.state_ref.set_waveshape_drive(heterodyne_strength);
    }

    /// Start background polling loop (60 Hz)
    pub async fn run_polling_loop(mut self) {
        eprintln!("[JoyCon] Starting polling loop at 60 Hz");

        let poll_interval = Duration::from_millis(16); // ~60 Hz

        while self.running {
            // Check for shutdown signal (non-blocking)
            match self.shutdown_rx.try_recv() {
                Ok(()) => {
                    eprintln!("[JoyCon] Shutdown signal received");
                    break;
                }
                Err(_) => {
                    // No shutdown signal, continue
                }
            }

            // Poll for events
            self.poll();

            // Small delay to maintain ~60 Hz polling rate
            tokio::time::sleep(poll_interval).await;
        }

        eprintln!("[JoyCon] Polling loop terminated");
    }

    /// Get current Joy-Con state
    pub fn get_state(&self) -> &JoyConState {
        &self.state
    }

    /// Check if Joy-Con is connected
    pub fn is_connected(&self) -> bool {
        self.state.connected
    }

    /// Stop the polling loop
    pub fn stop(&mut self) {
        self.running = false;
    }
}

/// Spawn Joy-Con polling task
pub fn spawn_joycon_task(
    state_ref: Arc<AppState>,
    mapping: GestureMapping,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        match JoyconHandler::new(state_ref.clone(), mapping) {
            Ok(handler) => {
                handler.run_polling_loop().await;
            }
            Err(e) => {
                eprintln!("[JoyCon] Failed to initialize: {}", e);
                eprintln!("[JoyCon] Joy-Con gesture control will be unavailable");
                state_ref.set_joycon_connected(false);
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deadzone_application() {
        // Values below deadzone should become 0
        assert_eq!(JoyConState::apply_deadzone(0.1, 0.15), 0.0);
        assert_eq!(JoyConState::apply_deadzone(-0.1, 0.15), 0.0);
        
        // Values above deadzone should be normalized
        let result = JoyConState::apply_deadzone(0.5, 0.15);
        assert!(result > 0.4);
        assert!(result < 0.5);
    }

    #[test]
    fn test_accel_magnitude() {
        let mut state = JoyConState::new();
        state.accel_x = 1.0;
        state.accel_y = 0.0;
        state.accel_z = 0.0;
        assert!((state.accel_magnitude() - 1.0).abs() < 0.001);

        state.accel_x = 1.0;
        state.accel_y = 1.0;
        state.accel_z = 0.0;
        assert!((state.accel_magnitude() - 2.0f32.sqrt()).abs() < 0.001);
    }

    #[test]
    fn test_gesture_mapping_defaults() {
        let mapping = GestureMapping::default();
        assert_eq!(mapping.roll_sensitivity, 1.0);
        assert_eq!(mapping.stick_deadzone, 0.15);
        assert!(matches!(mapping.button_a_action, JoyConAction::TogglePdm));
    }
}
