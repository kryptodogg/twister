use crate::types::{ControllerInfo, ControllerState};

#[tauri::command]
pub async fn list_controllers() -> Vec<ControllerInfo> {
    vec![
        ControllerInfo { id: "joycon-l".into(), name: "Joy-Con (L)".into(), connection_type: "Bluetooth".into() },
        ControllerInfo { id: "joycon-r".into(), name: "Joy-Con (R)".into(), connection_type: "Bluetooth".into() },
        ControllerInfo { id: "dualsense".into(), name: "DualSense Wireless Controller".into(), connection_type: "USB".into() },
    ]
}

#[tauri::command]
pub async fn get_controller_state(id: String) -> ControllerState {
    ControllerState {
        battery_level: 0.75,
        accelerometer: [0.1, 9.8, 0.2],
        gyroscope: [0.0, 0.0, 0.0],
        buttons: vec![],
        sticks: vec![[0.0, 0.0]],
    }
}

#[tauri::command]
pub async fn test_rumble(id: String, side: String, intensity: f32) -> Result<(), String> {
    println!("Testing rumble on {}: {} side, intensity {}", id, side, intensity);
    Ok(())
}

#[tauri::command]
pub async fn test_haptic(id: String, side: String, intensity: f32) -> Result<(), String> {
    println!("Testing haptic on {}: {} side, intensity {}", id, side, intensity);
    Ok(())
}

#[tauri::command]
pub async fn set_lightbar_color(id: String, r: u8, g: u8, b: u8) -> Result<(), String> {
    println!("Setting lightbar color on {} to R:{} G:{} B:{}", id, r, g, b);
    Ok(())
}
