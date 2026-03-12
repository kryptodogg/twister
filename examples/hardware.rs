// examples/hardware.rs
// Minimal runner for the Hardware Configuration Applet

use twister::ui::hardware_logic::setup_hardware_app;
use twister::utils::latency::QpcTimer;
use std::sync::Arc;
use slint::ComponentHandle;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let timer = Arc::new(QpcTimer::new());
    let ui = HardwareApp::new()?;

    setup_hardware_app(&ui, timer)?;

    ui.run()?;
    Ok(())
}
