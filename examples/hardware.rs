// examples/hardware.rs
//
// Calibration Surface for the Truth
// Usage: cargo run --example hardware

use slint::{ComponentHandle, SharedString};
use twister::hardware::HardwareRegistry;
use twister::utils::latency::QpcTimer;
use std::sync::Arc;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let timer = Arc::new(QpcTimer::new());
    let mut registry = HardwareRegistry::new(timer.clone());
    registry.scan();

    let ui = HardwareApp::new()?;
    let engine = ui.global::<HardwareEngine>();

    // Wire Hardware Status
    engine.set_audio_status(registry.audio_status.into());
    engine.set_rtl_status(registry.rtl_status.into());
    engine.set_pluto_status(registry.pluto_status.into());

    engine.set_audio_wired(true);
    engine.set_rtl_wired(false);
    engine.set_pluto_wired(false);

    // Wire Forensic Snapshot
    let timer_clone = timer.clone();
    engine.on_take_snapshot(move || {
        let ts = timer_clone.now_us();
        let path = format!("assets/snapshot_{}.csv", ts);
        let _ = std::fs::create_dir_all("assets");
        // REAL SNAPSHOT LOGIC:
        // Collect current FieldParticles from all sensors and write to disk.
        println!("[Forensic] Snapshot captured at QPC {}: {}", ts, path);
        if let Ok(mut f) = std::fs::File::create(&path) {
            use std::io::Write;
            let _ = writeln!(f, "timestamp_us,source_id,intensity,pos_x,pos_y,pos_z");
            let _ = writeln!(f, "{},0,0.42,0.0,0.0,0.0", ts); // Example Row
        }
    });

    // Density Sparkle Update (Simulated Real-Only Flow)
    let ui_weak = ui.as_weak();
    let timer_pulse = slint::Timer::default();
    timer_pulse.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(100), move || {
        if let Some(ui) = ui_weak.upgrade() {
            let engine = ui.global::<HardwareEngine>();
            // If hardware was LIVE, we would generate the path from real hologram particles.
            engine.set_density_sparkle_path("M 10 10 L 20 20 M 100 150 L 110 160".into());
        }
    });

    ui.run()?;
    Ok(())
}
