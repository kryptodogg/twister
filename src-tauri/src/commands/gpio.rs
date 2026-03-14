use crate::types::{GpioPin, GpioDirection, GpioPull, GpioActiveState};

#[tauri::command]
pub async fn get_gpio_assignments() -> Vec<GpioPin> {
    vec![
        GpioPin { pin: 4, function: "PPS from Pico 2".into(), direction: GpioDirection::In, pull: GpioPull::Down, connected_to: "Pico 2 GPIO 0".into(), active_state: GpioActiveState::High },
        GpioPin { pin: 17, function: "IR LED bank A".into(), direction: GpioDirection::Out, pull: GpioPull::None, connected_to: "IR LED bank A".into(), active_state: GpioActiveState::High },
        GpioPin { pin: 18, function: "IR LED bank B".into(), direction: GpioDirection::Out, pull: GpioPull::None, connected_to: "IR LED bank B".into(), active_state: GpioActiveState::High },
        GpioPin { pin: 22, function: "IR receiver data".into(), direction: GpioDirection::In, pull: GpioPull::Up, connected_to: "IR Receiver".into(), active_state: GpioActiveState::High },
        GpioPin { pin: 23, function: "UWB TX trigger".into(), direction: GpioDirection::Out, pull: GpioPull::None, connected_to: "Pico 2 GPIO 1".into(), active_state: GpioActiveState::High },
        GpioPin { pin: 24, function: "UWB RX data".into(), direction: GpioDirection::In, pull: GpioPull::Down, connected_to: "Pico 2 GPIO 2".into(), active_state: GpioActiveState::High },
        GpioPin { pin: 2, function: "I2C SDA OV9281".into(), direction: GpioDirection::Alt, pull: GpioPull::None, connected_to: "OV9281".into(), active_state: GpioActiveState::High },
        GpioPin { pin: 3, function: "I2C SCL OV9281".into(), direction: GpioDirection::Alt, pull: GpioPull::None, connected_to: "OV9281".into(), active_state: GpioActiveState::High },
        GpioPin { pin: 10, function: "SPI MOSI MEMS PDM".into(), direction: GpioDirection::Alt, pull: GpioPull::None, connected_to: "MEMS".into(), active_state: GpioActiveState::High },
        GpioPin { pin: 11, function: "SPI CLK MEMS PDM".into(), direction: GpioDirection::Alt, pull: GpioPull::None, connected_to: "MEMS".into(), active_state: GpioActiveState::High },
    ]
}

#[tauri::command]
pub async fn save_gpio_assignments(pins: Vec<GpioPin>) -> Result<(), String> {
    // In a real app, this would write to a file
    println!("Saving GPIO assignments: {:?}", pins);
    Ok(())
}

#[tauri::command]
pub async fn export_gpio_config(path: String) -> Result<(), String> {
    println!("Exporting GPIO config to: {}", path);
    Ok(())
}
