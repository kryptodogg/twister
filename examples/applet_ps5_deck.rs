use std::time::Duration;
use tokio::sync::watch;
use std::sync::{Arc, Mutex};
use hidapi::HidApi;
use slint::ComponentHandle;

slint::include_modules!();

#[cfg(target_os = "windows")]
fn apply_acrylic(window: &slint::Window) {
    use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE};

    if let Ok(RawWindowHandle::Win32(handle)) = window.raw_window_handle() {
        let hwnd = windows::Win32::Foundation::HWND(handle.hwnd as _);
        let backdrop_type: u32 = 3; // 3 = DWMSBT_TRANSIENTWINDOW (Acrylic), 2 = DWMSBT_MAINWINDOW (Mica)
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                &backdrop_type as *const _ as *const _,
                std::mem::size_of::<u32>() as u32,
            );
        }
    }
}

// 0x054C is Sony Vendor ID, 0x0CE6 is DualSense Product ID
const VENDOR_ID: u16 = 0x054C;
const PRODUCT_ID_DUALSENSE: u16 = 0x0CE6;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = PS5DeckApplet::new()?;

    #[cfg(target_os = "windows")]
    apply_acrylic(ui.window());

    let (signal_tx, signal_rx) = watch::channel(0.0f32);

    // Background task for generating mock "RF Sparkle" Signal
    tokio::spawn(async move {
        let mut phase = 0.0f32;
        let mut interval = tokio::time::interval(Duration::from_millis(10));

        loop {
            interval.tick().await;

            // 440Hz sine wave, sampled at 100Hz (dt = 0.01)
            phase += 440.0 * std::f32::consts::TAU * 0.01;
            if phase > std::f32::consts::TAU {
                phase -= std::f32::consts::TAU;
            }

            // Low-frequency random walk for modulation
            let modulation: f32 = rand::random::<f32>();

            // Output signal strength 0.0 - 1.0
            let signal_strength = (phase.sin() * 0.5 + 0.5) * modulation;
            let _ = signal_tx.send(signal_strength);
        }
    });

    let ui_handle = ui.as_weak();

    // Shared state to bridge UI events to HID thread
    let shared_haptic_state = Arc::new(Mutex::new((true, false))); // (rumble_enabled, test_active)

    let state_clone = shared_haptic_state.clone();
    ui.global::<PS5Status>().on_rumble_toggled(move |enabled| {
        if let Ok(mut state) = state_clone.lock() {
            state.0 = enabled;
        }
    });

    let state_clone2 = shared_haptic_state.clone();
    ui.global::<PS5Status>().on_test_toggled(move |active| {
        if let Ok(mut state) = state_clone2.lock() {
            state.1 = active;
        }
    });

    // Background task for DualSense Telemetry & Haptics using raw HID
    tokio::spawn(async move {
        match HidApi::new() {
            Ok(api) => {
                let device_info = api.device_list().find(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID_DUALSENSE);
                if let Some(info) = device_info {
                    if let Ok(device) = api.open_path(info.path()) {
                        let _ = device.set_blocking_mode(false);
                        let mut interval = tokio::time::interval(Duration::from_millis(10)); // 100Hz
                        let mut read_buf = [0u8; 64];

                        loop {
                            interval.tick().await;

                            // Try to read telemetry
                            let _ = device.read(&mut read_buf);

                            // Parse PS5 HID Report (Report ID 1 or 0x31)
                            let mut l2_force = 0.0;
                            let mut r2_force = 0.0;
                            let mut touch_x = 0;
                            let mut touch_y = 0;
                            let mut battery_str = "100%".to_string();

                            // Simplistic decoding based on known DualSense packet structures.
                            // Assuming USB connection report (ID 0x01)
                            if read_buf[0] == 0x01 {
                                l2_force = read_buf[8] as f32 / 255.0;
                                r2_force = read_buf[9] as f32 / 255.0;

                                battery_str = format!("{}%", (read_buf[53] & 0x0F) * 10);

                                touch_x = ((read_buf[34] as u16 & 0x0F) << 8) | read_buf[33] as u16;
                                touch_y = ((read_buf[35] as u16) << 4) | ((read_buf[34] as u16 & 0xF0) >> 4);
                            }

                            let signal_strength = *signal_rx.borrow();

                            let (rumble_enabled, test_active) = {
                                let state = shared_haptic_state.lock().unwrap();
                                *state
                            };

                            let mut intensity = 0u8;

                            if test_active {
                                intensity = 128;
                            } else if rumble_enabled {
                                intensity = (signal_strength * 255.0) as u8;
                            }

                            // Send Haptic Vibration Data
                            // DualSense Write Report (USB ID 0x02)
                            let mut write_buf = [0u8; 64];
                            write_buf[0] = 0x02; // Report ID
                            write_buf[1] = 0x01 | 0x02; // Flags to enable rumble
                            write_buf[3] = intensity; // Right Motor
                            write_buf[4] = intensity; // Left Motor

                            // Actually send the rumble (fire and forget for this mockup if error)
                            let _ = device.write(&write_buf);

                            let vib_intensity = intensity as f32 / 255.0;

                            // Update Slint UI
                            let _ = slint::invoke_from_event_loop({
                                let ui_handle = ui_handle.clone();
                                move || {
                                    if let Some(ui) = ui_handle.upgrade() {
                                        ui.global::<PS5Status>().set_touch_x(touch_x as f32);
                                        ui.global::<PS5Status>().set_touch_y(touch_y as f32);
                                        ui.global::<PS5Status>().set_left_trigger_force(l2_force);
                                        ui.global::<PS5Status>().set_right_trigger_force(r2_force);
                                        ui.global::<PS5Status>().set_left_vibration_intensity(vib_intensity);
                                        ui.global::<PS5Status>().set_right_vibration_intensity(vib_intensity);
                                        ui.global::<PS5Status>().set_battery(battery_str.into());
                                    }
                                }
                            });
                        }
                    }
                } else {
                    eprintln!("PS5 Controller not found. Starting UI in mock mode.");

                    let mut interval = tokio::time::interval(Duration::from_millis(10)); // 100Hz
                    loop {
                        interval.tick().await;
                        let signal_strength = *signal_rx.borrow();

                        let (rumble_enabled, test_active) = {
                            let state = shared_haptic_state.lock().unwrap();
                            *state
                        };

                        let mut intensity = 0u8;
                        if test_active {
                            intensity = 128;
                        } else if rumble_enabled {
                            intensity = (signal_strength * 255.0) as u8;
                        }
                        let vib_intensity = intensity as f32 / 255.0;

                        let _ = slint::invoke_from_event_loop({
                            let ui_handle = ui_handle.clone();
                            move || {
                                if let Some(ui) = ui_handle.upgrade() {
                                    ui.global::<PS5Status>().set_left_vibration_intensity(vib_intensity);
                                    ui.global::<PS5Status>().set_right_vibration_intensity(vib_intensity);
                                }
                            }
                        });
                    }
                }
            }
            Err(e) => eprintln!("Failed to initialize HID API: {}", e),
        }
    });

    ui.run()?;
    Ok(())
}
