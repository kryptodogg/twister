pub mod types;
pub mod commands;

use tauri::Manager;
use window_vibrancy::apply_mica;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::devices::get_device_states,
            commands::devices::pop_out_device,
            commands::gpio::get_gpio_assignments,
            commands::gpio::save_gpio_assignments,
            commands::gpio::export_gpio_config,
            commands::controllers::list_controllers,
            commands::controllers::get_controller_state,
            commands::controllers::test_rumble,
            commands::controllers::test_haptic,
            commands::controllers::set_lightbar_color,
            commands::settings::get_settings,
            commands::settings::save_settings,
        ])
        .setup(|app| {
            let window = app.get_webview_window("brick-road").unwrap();

            #[cfg(target_os = "windows")]
            apply_mica(&window, Some(true)).expect("apply_mica failed");

            window.show().unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
