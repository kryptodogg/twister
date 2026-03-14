use crate::types::Settings;
use serde_json::json;

#[tauri::command]
pub async fn get_settings() -> Settings {
    Settings {
        devices: json!({
            "rtl-sdr": { "sample_rate": "2.4M", "ppm": 0, "gain_mode": "auto", "youloop": false },
            "pluto-sdr": { "mode": "rx", "tx_power": 0, "rx_gain": 40, "sample_rate": "2M" },
            "c925e-audio": { "mode": "audio_rx", "channels": "both", "raw_mode": true },
            "telephone-coil": { "monitoring_60hz": true, "harmonic_depth": "5th" },
            "ov9281-dual": { "fps": 120, "mode": "stereo_depth", "focal_length": 3.6 },
            "pico-2": { "pps_pin": 4, "serial_port": "AUTO", "uwb_mode": "disabled" }
        }),
        gpio: vec![] // This will be merged with get_gpio_assignments in frontend if needed
    }
}

#[tauri::command]
pub async fn save_settings(settings: Settings) -> Result<(), String> {
    println!("Saving settings: {:?}", settings);
    Ok(())
}
