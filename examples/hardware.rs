// examples/hardware.rs
//
// Calibration Surface for the Truth
// Usage: cargo run --example hardware

use slint::{ComponentHandle, SharedString, Image};
use twister::hardware::HardwareRegistry;
use twister::utils::latency::QpcTimer;
use twister::ml::PoseEstimator;
use twister::dispatch::generate_density_sparkle;
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
    engine.set_cmos_wired(false);

    // Setup Pose Estimator
    let _pose_estimator = Arc::new(PoseEstimator::<burn_ndarray::NdArray<f32>>::new(
        burn::backend::ndarray::NdArrayDevice::Cpu
    ));

    // Wire Forensic Snapshot
    let timer_clone = timer.clone();
    engine.on_take_snapshot(move || {
        let ts = timer_clone.now_us();
        let path = format!("assets/snapshot_{}.csv", ts);
        let _ = std::fs::create_dir_all("assets");
        println!("[Forensic] Snapshot triggered: {}", path);
        if let Ok(mut f) = std::fs::File::create(&path) {
            use std::io::Write;
            let _ = writeln!(f, "timestamp_us,source,intensity,x,y,z");
            // Real session data would be dumped here
        }
    });

    engine.on_toggle_pose(|enabled| {
        println!("[GPU] Pose Estimation: {}", if enabled { "ACTIVE" } else { "OFF" });
    });

    // Real-Time UI Bridge (Simulation of live hardware)
    let ui_weak = ui.as_weak();
    let timer_ui = slint::Timer::default();
    timer_ui.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(33), move || {
        if let Some(ui) = ui_weak.upgrade() {
            let engine = ui.global::<HardwareEngine>();
            // If data was flowing, generate_density_sparkle would be called here
            engine.set_density_sparkle_path(generate_density_sparkle(&[]).into());
        }
    });

    ui.run()?;
    Ok(())
}
