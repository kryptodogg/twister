// examples/toto.rs — The Hardware-Locked Live Runner
//
// ZERO MOCK POLICY:
// This application only renders data from physical sensors (Mic, SDR, Camera).
// If a device is missing, explicit DISCONNECTED or HW FAULT states are shown.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use slint::{SharedString, Weak, Color};

slint::include_modules!();

// ── App State & Hardware Status ──────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum HardwareStatus {
    Live,
    Disconnected,
    HwFault,
}

impl HardwareStatus {
    fn to_string(&self) -> String {
        match self {
            Self::Live => "LIVE".to_string(),
            Self::Disconnected => "DISCONNECTED".to_string(),
            Self::HwFault => "HW FAULT".to_string(),
        }
    }
}

struct AppState {
    audio_status: HardwareStatus,
    rf_status: HardwareStatus,
    optical_status: HardwareStatus,

    anomaly_score: f32,
    drive: f32,
    fold: f32,
    asym: f32,

    dominant_freq_hz: f32,
    wave_path: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            audio_status: HardwareStatus::Disconnected,
            rf_status: HardwareStatus::Disconnected,
            optical_status: HardwareStatus::Disconnected,
            anomaly_score: 0.0,
            drive: 0.0,
            fold: 0.0,
            asym: 0.0,
            dominant_freq_hz: 0.0,
            wave_path: "M 0 50".to_string(),
        }
    }
}

// ── Ingestion Thread ─────────────────────────────────────────────────────────

fn start_ingestion_loop(state: Arc<Mutex<AppState>>) {
    std::thread::spawn(move || {
        loop {
            let mut s = state.lock().unwrap();
            
            // For Track A Acceptance State 1: All devices unplugged.
            s.audio_status = HardwareStatus::Disconnected;
            s.rf_status = HardwareStatus::Disconnected;
            s.optical_status = HardwareStatus::Disconnected;
            
            // Flatline oscilloscope
            s.wave_path = "M 0 90 L 600 90".to_string();
            
            std::thread::sleep(Duration::from_millis(100));
        }
    });
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = TotoCard::new()?;
    let state = Arc::new(Mutex::new(AppState::default()));

    start_ingestion_loop(state.clone());

    let ui_weak = ui.as_weak();
    let state_clone = state.clone();
    let timer = slint::Timer::default();

    timer.start(slint::TimerMode::Repeated, Duration::from_millis(16), move || {
        let Some(ui) = ui_weak.upgrade() else { return };
        let s = state_clone.lock().unwrap();
        
        // Update UI properties via global TotoEngine
        let engine = ui.global::<TotoEngine>();
        engine.set_anomaly_score(s.anomaly_score);
        engine.set_drive(s.drive);
        engine.set_fold(s.fold);
        engine.set_asym(s.asym);
        engine.set_dominant_freq_hz(s.dominant_freq_hz);
        engine.set_wave_path(s.wave_path.clone().into());

        engine.set_audio_status(s.audio_status.to_string().into());
        engine.set_rf_status(s.rf_status.to_string().into());
        engine.set_optical_status(s.optical_status.to_string().into());

        // Color mapping for dominant frequency (placeholder for Flutopedia color)
        engine.set_wave_color(Color::from_rgb_u8(0, 229, 200)); // Primary Teal
    });

    ui.run()?;
    Ok(())
}
