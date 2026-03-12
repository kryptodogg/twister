use slint::{ComponentHandle, SharedString};
use crate::hardware::HardwareRegistry;
use crate::utils::latency::QpcTimer;
use crate::ml::pose_estimator::PoseEstimator;
use crate::dispatch::generate_density_sparkle;
use std::sync::Arc;

slint::include_modules!();

pub fn setup_hardware_app(ui: &HardwareApp, timer: Arc<QpcTimer>) -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = HardwareRegistry::new(timer.clone());
    registry.scan();

    let engine = ui.global::<HardwareEngine>();

    // Wire Hardware Status
    engine.set_audio_status(registry.audio_status.as_str().into());
    engine.set_rtl_status(registry.rtl_status.as_str().into());
    engine.set_pluto_status(registry.pluto_status.as_str().into());

    engine.set_audio_wired(true);
    engine.set_rtl_wired(false);
    engine.set_pluto_wired(false);
    engine.set_cmos_wired(false);

    // Wire Forensic Snapshot
    let timer_clone = timer.clone();
    engine.on_take_snapshot(move || {
        let ts = timer_clone.now_us();
        println!("[Forensic] Snapshot triggered via library: assets/snapshot_{}.csv", ts);
    });

    // GPU Pose Toggle
    engine.on_toggle_pose(|enabled| {
        println!("[GPU] Pose Estimation state: {}", if enabled { "ACTIVE" } else { "OFF" });
    });

    // Density Sparkle Update
    let ui_weak = ui.as_weak();
    let timer_ui = slint::Timer::default();
    timer_ui.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(33), move || {
        if let Some(ui) = ui_weak.upgrade() {
            let engine = ui.global::<HardwareEngine>();
            engine.set_density_sparkle_path(generate_density_sparkle(&[]).into());
        }
    });

    Ok(())
}
