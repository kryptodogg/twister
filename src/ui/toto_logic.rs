use slint::ComponentHandle;
use std::sync::Arc;
use crate::state::AppState;
use std::time::Duration;

slint::include_modules!();

pub fn setup_toto_app(ui: &TotoCard, state: Arc<AppState>) {
    let ui_weak = ui.as_weak();
    let timer = slint::Timer::default();
    let _state_clone = state.clone();

    timer.start(slint::TimerMode::Repeated, Duration::from_millis(16), move || {
        let Some(ui) = ui_weak.upgrade() else { return };

        let engine = ui.global::<TotoEngine>();
        // Zero-Mock: Hard disconnected state until physical hardware is wired
        engine.set_audio_status("DISCONNECTED".into());
        engine.set_rf_status("DISCONNECTED".into());
        engine.set_optical_status("DISCONNECTED".into());
    });
}
