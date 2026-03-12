use slint::{ComponentHandle, SharedString};
use std::sync::Arc;
use crate::state::AppState;
use std::time::Duration;

slint::include_modules!();

pub fn setup_toto_app(ui: &TotoCard, state: Arc<AppState>) {
    let ui_weak = ui.as_weak();
    let timer = slint::Timer::default();
    let state_clone = state.clone();

    timer.start(slint::TimerMode::Repeated, Duration::from_millis(16), move || {
        let Some(ui) = ui_weak.upgrade() else { return };
        let engine = ui.global::<TotoEngine>();

        // ── LIVE FORENSIC BINDING ─────────────────────────────────────────────
        // In a full integration, these are updated from the Dispatch Loop results.
        engine.set_anomaly_score(state_clone.mamba_anomaly_score.load(std::sync::atomic::Ordering::Relaxed));
        engine.set_drive(state_clone.waveshape_drive.load(std::sync::atomic::Ordering::Relaxed));

        // 12-Channel BSS Visualization Update
        // Placeholder for real holographic path mapping
        engine.set_path_fs("M 0 90 L 600 90".into());
    });
}
