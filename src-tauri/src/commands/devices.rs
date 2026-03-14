use crate::types::{DeviceState, DeviceStatus};
use tauri::WebviewWindowBuilder;

#[tauri::command]
pub async fn get_device_states() -> Vec<DeviceState> {
    vec![
        DeviceState { id: "rtl-sdr".into(), status: DeviceStatus::Connected, last_seen_ms: Some(123) },
        DeviceState { id: "pluto-sdr".into(), status: DeviceStatus::Connected, last_seen_ms: Some(123) },
        DeviceState { id: "c925e-audio".into(), status: DeviceStatus::Connected, last_seen_ms: Some(123) },
        DeviceState { id: "telephone-coil".into(), status: DeviceStatus::Connected, last_seen_ms: Some(123) },
        DeviceState { id: "ov9281-dual".into(), status: DeviceStatus::Connected, last_seen_ms: Some(123) },
        DeviceState { id: "c925e-video".into(), status: DeviceStatus::Connected, last_seen_ms: Some(123) },
        DeviceState { id: "ir-emitter-array".into(), status: DeviceStatus::Unwired, last_seen_ms: None },
        DeviceState { id: "mems-microphones".into(), status: DeviceStatus::Unwired, last_seen_ms: None },
        DeviceState { id: "pico-2".into(), status: DeviceStatus::Connected, last_seen_ms: Some(123) },
    ]
}

#[tauri::command]
pub async fn pop_out_device(app: tauri::AppHandle, device_id: String) -> Result<(), String> {
    let window = WebviewWindowBuilder::new(
        &app,
        format!("device-{}", device_id),
        tauri::WebviewUrl::App(format!("device.html?id={}", device_id).into()),
    )
    .transparent(true)
    .decorations(false)
    .inner_size(480.0, 640.0)
    .build()
    .map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    window_vibrancy::apply_mica(&window, Some(true)).ok();

    window.show().map_err(|e| e.to_string())?;
    Ok(())
}
