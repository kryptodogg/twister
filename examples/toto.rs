// examples/toto.rs
// Toto HUD Applet (mock stream)
//
// This is a fast UI demo: it cycles dominant frequency to show Emerald City
// octave-fold color transitions, while animating wave/loss paths and
// drive/fold/asym projection bars.

slint::include_modules!();

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use twister::ui::{enable_acrylic_blur, get_resonant_color};

fn build_wave_path(phase: f32, freq_step: usize) -> String {
    // Viewbox: 0..100 x 0..100
    let n = 100usize;
    let (cycles, amp) = match freq_step {
        0 => (2.0_f32, 35.0_f32), // 60 Hz: slow, large
        1 => (4.0_f32, 25.0_f32), // 100 kHz (folded): medium
        _ => (6.0_f32, 18.0_f32), // 2.4 GHz (folded): fast, tighter
    };

    let mut out = String::with_capacity(n * 16);
    out.push_str("M 0 50");

    for i in 0..n {
        let t = if n <= 1 { 0.0 } else { i as f32 / (n - 1) as f32 };
        let x = 100.0 * t;
        let y = 50.0 - (phase * 1.2 + t * cycles * std::f32::consts::TAU).sin() * amp;
        out.push_str(&format!(" L {:.2} {:.2}", x, y));
    }

    out
}

fn build_series_path(values: &VecDeque<f32>) -> String {
    let n = 100usize;
    if values.is_empty() {
        return "M 0 100 L 100 100".to_string();
    }

    let min_v = values
        .iter()
        .cloned()
        .filter(|v| v.is_finite())
        .fold(f32::INFINITY, f32::min);
    let max_v = values
        .iter()
        .cloned()
        .filter(|v| v.is_finite())
        .fold(f32::NEG_INFINITY, f32::max);

    let (min_v, max_v) = if min_v.is_finite() && max_v.is_finite() && max_v > min_v {
        (min_v, max_v)
    } else {
        (0.0, 1.0)
    };

    let mut out = String::with_capacity(n * 16);
    out.push_str("M 0 100");

    for i in 0..n {
        let t = if n <= 1 { 0.0 } else { i as f32 / (n - 1) as f32 };
        let idx = ((values.len() - 1) as f32 * t).round() as usize;
        let v = values[idx];
        let vn = ((v - min_v) / (max_v - min_v)).clamp(0.0, 1.0);
        let x = 100.0 * t;
        let y = 100.0 - vn * 100.0;
        out.push_str(&format!(" L {:.2} {:.2}", x, y));
    }

    out
}

struct WidgetState {
    anomaly_score: f32,
    dominant_freq_hz: f32,
    wave_path: String,
    learning_loss: f32,
    loss_path: String,
    drive: f32,
    fold: f32,
    asym: f32,
    animation_tick: f32,
}

struct MockStream {
    phase: f32,
    freq_step: usize, // 0=60Hz, 1=100kHz, 2=2.4GHz
    last_freq_change: Instant,
    loss_hist: VecDeque<f32>,
}

impl MockStream {
    fn new() -> Self {
        Self {
            phase: 0.0,
            freq_step: 2,
            last_freq_change: Instant::now(),
            loss_hist: VecDeque::with_capacity(64),
        }
    }

    fn tick(&mut self, dt: Duration) -> WidgetState {
        self.phase += dt.as_secs_f32();

        if self.last_freq_change.elapsed() > Duration::from_secs(2) {
            self.freq_step = (self.freq_step + 1) % 3;
            self.last_freq_change = Instant::now();
        }

        let dominant_freq_hz = match self.freq_step {
            0 => 60.0,
            1 => 100_000.0,
            _ => 2_400_000_000.0,
        };

        let anomaly = 0.150 + 0.030 * (self.phase * 0.9).sin().abs();
        let loss = 0.040 + 0.015 * (self.phase * 0.7).cos().abs();

        // Projection bars: smooth drift within 0..1.
        let drive = (0.25 + 0.15 * (self.phase * 0.3).sin()).clamp(0.0, 1.0);
        let fold = (0.70 + 0.20 * (self.phase * 0.2).cos()).clamp(0.0, 1.0);
        let asym = (0.15 + 0.10 * (self.phase * 0.5).sin().abs()).clamp(0.0, 1.0);

        self.loss_hist.push_back(loss);
        while self.loss_hist.len() > 64 {
            self.loss_hist.pop_front();
        }

        WidgetState {
            anomaly_score: anomaly,
            dominant_freq_hz,
            wave_path: build_wave_path(self.phase, self.freq_step),
            learning_loss: loss,
            loss_path: build_series_path(&self.loss_hist),
            drive,
            fold,
            asym,
            animation_tick: self.phase,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = std::sync::mpsc::sync_channel::<WidgetState>(1);
    let rx = Arc::new(Mutex::new(rx));

    std::thread::spawn(move || {
        let mut stream = MockStream::new();
        loop {
            std::thread::sleep(Duration::from_millis(33));
            let state = stream.tick(Duration::from_millis(33));
            let _ = tx.try_send(state);
        }
    });

    twister::ui::register_default_fonts();

    let window = TotoHudApplet::new()?;
    window.set_unit_size(384.0);
    window.set_dvr_recording(true);
    window.set_dvr_buffer_days(97);

    enable_acrylic_blur(window.window());

    let window_weak = window.as_weak();
    let rx_clone = rx.clone();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            let Some(w) = window_weak.upgrade() else { return };
            if let Ok(state) = rx_clone.lock().unwrap().try_recv() {
                let color = get_resonant_color(state.dominant_freq_hz as f64);
                w.set_anomaly_score(state.anomaly_score);
                w.set_auto_steer(true);
                w.set_dominant_freq_hz(state.dominant_freq_hz);
                w.set_wave_path(state.wave_path.into());
                w.set_learning_loss(state.learning_loss);
                w.set_loss_path(state.loss_path.into());
                w.set_drive(state.drive);
                w.set_fold(state.fold);
                w.set_asym(state.asym);
                w.set_animation_tick(state.animation_tick);
                w.set_resonant_color(color);
            }
        },
    );

    window.run()?;
    Ok(())
}
