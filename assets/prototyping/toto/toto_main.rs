// examples/toto.rs
// Project Synesthesia — Toto Core Field Probe
//
// This is the Rust entry point for the widget. It does three things:
//
//   1. Spawns a mock data thread that generates realistic synthetic state
//      (anomaly score, wave path, Drive/Fold/Asym, dominant frequency).
//
//   2. Runs a Slint timer at 60 Hz that atomically pushes all changed
//      properties to the UI in a single batch. Never sets individual
//      properties in a loop — one batch update = one dirty mark on
//      Slint's property graph = one repaint pass.
//
//   3. Cycles the dominant frequency through 60 Hz → 85 kHz → 2.4 GHz
//      every 2 seconds, proving that the wave color transition works
//      before real FieldParticle data is wired in.
//
// The wave path is the key connection point. In production, the Rust
// signal processing loop (Cyclone) computes a cubic bezier approximation
// of the FFT envelope each frame and calls window.set_wave_path().
// The Slint Path element re-renders it automatically — no polling,
// no explicit redraw call needed.

slint::include_modules!();

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ── Mock state ───────────────────────────────────────────────────────────────

struct MockStream {
    phase:     f64,
    epoch:     u32,
    start:     Instant,
    freq_step: usize,   // 0=60Hz, 1=85kHz, 2=2.4GHz
    last_freq_change: Instant,
}

impl MockStream {
    fn new() -> Self {
        Self {
            phase:     0.0,
            epoch:     0,
            start:     Instant::now(),
            freq_step: 2,   // Start on 2.4 GHz violet
            last_freq_change: Instant::now(),
        }
    }

    fn tick(&mut self, delta_ms: u64) -> WidgetState {
        self.phase += delta_ms as f64 / 1000.0;
        self.epoch += 1;

        // Cycle dominant frequency every 2 seconds to prove color transitions.
        // In production, this value comes from Cyclone's FFT peak detector.
        if self.last_freq_change.elapsed() > Duration::from_secs(2) {
            self.freq_step = (self.freq_step + 1) % 3;
            self.last_freq_change = Instant::now();
        }

        let dominant_freq_hz = match self.freq_step {
            0 => 60.0_f32,
            1 => 85_000.0_f32,
            _ => 2_400_000_000.0_f32,
        };

        // Anomaly score: gentle sinusoidal drift simulating real noise floor
        // variation, with an occasional spike above 1.0 to show the Obsidian tier.
        let anomaly = (0.25_f64
            + 0.20 * (self.phase * 0.4).sin()
            + 0.12 * (self.phase * 1.8).sin()
            + 0.08 * (self.phase * 4.3).sin().abs()) as f32;

        // Drive/Fold/Asym: the three Mamba projection scalars.
        // In production these come from project_latent_to_waveshape().
        // Here they drift slowly to show the progress bars animating.
        let drive = (0.25 + 0.15 * (self.phase * 0.3).sin()) as f32;
        let fold  = (0.70 + 0.20 * (self.phase * 0.2).cos()) as f32;
        let asym  = (0.15 + 0.10 * (self.phase * 0.5).sin().abs()) as f32;

        // Wave path: a cubic bezier oscilloscope trace.
        // Coordinate space: 0–320 wide, 0–100 tall (matches Slint Path viewport).
        // The amplitude and frequency of the wave responds to the dominant freq:
        //   60 Hz   → slow, large amplitude (mains hum characteristic)
        //   85 kHz  → medium frequency, medium amplitude
        //   2.4 GHz → high frequency, tighter oscillation
        let wave_path = generate_wave_path(self.phase, self.freq_step);

        WidgetState {
            anomaly_score:     anomaly,
            auto_steer:        true,
            dominant_freq_hz,
            wave_path,
            drive: drive.clamp(0.0, 1.0),
            fold:  fold.clamp(0.0, 1.0),
            asym:  asym.clamp(0.0, 1.0),
            animation_tick:    self.start.elapsed().as_secs_f32(),
        }
    }
}

/// Generates a cubic bezier path string that changes character based on
/// which frequency cluster is dominant. This is the prototype version —
/// in production, Cyclone computes this from actual FFT bin magnitudes.
///
/// The path is in the 0–320 × 0–100 coordinate space that the Slint
/// Path element's viewport maps to whatever physical size the canvas is.
fn generate_wave_path(phase: f64, freq_step: usize) -> String {
    // Number of bezier segments and amplitude vary by frequency character.
    // More segments = higher apparent frequency on the oscilloscope face.
    let (segments, amplitude, freq_mult) = match freq_step {
        0 => (4_usize,  35.0_f64, 1.0_f64),   // 60 Hz: slow, large
        1 => (6_usize,  22.0_f64, 2.5_f64),   // 85 kHz: medium
        _ => (8_usize,  14.0_f64, 4.0_f64),   // 2.4 GHz: fast, tight
    };

    let step = 320.0 / segments as f64;
    let mut path = format!("M 0 50");

    for i in 0..segments {
        let x0 = i as f64 * step;
        let x3 = (i + 1) as f64 * step;
        let x1 = x0 + step * 0.33;
        let x2 = x0 + step * 0.67;

        // Phase-shifted sin for the control points so the wave animates smoothly.
        // The phase argument comes from the mock stream's elapsed time.
        let y1 = 50.0 - amplitude * (phase * freq_mult + i as f64 * 0.8).sin();
        let y2 = 50.0 + amplitude * (phase * freq_mult + i as f64 * 0.8 + 1.0).sin();
        let y3 = 50.0 + (amplitude * 0.3) * (phase * freq_mult * 0.5 + i as f64).cos();

        path.push_str(&format!(
            " C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1}",
            x1, y1, x2, y2, x3, y3
        ));
    }

    path
}

// ── State bundle ─────────────────────────────────────────────────────────────
// Everything the UI needs in one struct. Updated atomically each frame.

struct WidgetState {
    anomaly_score:    f32,
    auto_steer:       bool,
    dominant_freq_hz: f32,
    wave_path:        String,
    drive:            f32,
    fold:             f32,
    asym:             f32,
    animation_tick:   f32,
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Channel: mock thread → UI thread.
    // sync_channel(1): only the most recent frame matters; older frames are
    // overwritten if the UI thread is briefly slow. This prevents the mock
    // thread from accumulating a queue of stale states.
    let (tx, rx) = std::sync::mpsc::sync_channel::<WidgetState>(1);
    let rx = Arc::new(Mutex::new(rx));

    // Spawn mock data thread at 30 Hz (UI renders at 60 Hz; data updates
    // at half that rate since wave path computation is the heaviest step).
    std::thread::spawn(move || {
        let mut stream = MockStream::new();
        loop {
            std::thread::sleep(Duration::from_millis(33)); // ~30 Hz
            let state = stream.tick(33);
            let _ = tx.try_send(state); // Non-blocking; drop if receiver is behind
        }
    });

    // Create the Slint window. TotoCard is the exported component name
    // from toto.slint's `export component TotoCard`.
    let window = TotoCard::new()?;

    // Platform-specific compositor blur.
    // This is the call that was missing in the original screenshot —
    // without it, Windows treats the transparent background as solid.
    enable_compositor_blur(&window);

    // 60 Hz timer: polls the channel and pushes state to Slint atomically.
    // The key discipline: set_wave_path, set_anomaly_score, etc. are all
    // called in one Rust scope before yielding back to the Slint event loop.
    // Slint batches these into a single repaint rather than repainting
    // after each individual property change.
    let window_weak = window.as_weak();
    let rx_clone = rx.clone();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16), // 60 Hz
        move || {
            let Some(w) = window_weak.upgrade() else { return };

            // Non-blocking poll — if no new state is available, skip this frame.
            // The UI holds its previous values; nothing flickers.
            if let Ok(state) = rx_clone.lock().unwrap().try_recv() {
                // ── Atomic batch update ──────────────────────────────────
                // All property sets happen before returning to the event loop.
                // Slint coalesces them into a single dirty-mark + repaint.
                w.set_anomaly_score(state.anomaly_score);
                w.set_auto_steer(state.auto_steer);
                w.set_dominant_freq_hz(state.dominant_freq_hz);
                w.set_wave_path(state.wave_path.into());
                w.set_drive(state.drive);
                w.set_fold(state.fold);
                w.set_asym(state.asym);
                w.set_animation_tick(state.animation_tick);
                // ── End batch ────────────────────────────────────────────
            }
        },
    );

    window.run()?;
    Ok(())
}

// ── Platform blur ─────────────────────────────────────────────────────────────
// These calls tell the OS compositor to apply its blur pass behind the window.
// Without them, `background: transparent` in Slint renders as solid black.

fn enable_compositor_blur(window: &TotoCard) {
    #[cfg(target_os = "windows")]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(handle) = window.window().window_handle() {
            if let RawWindowHandle::Win32(h) = handle.as_raw() {
                enable_dwm_acrylic(h.hwnd.get() as _);
            }
        }
    }

    #[cfg(all(target_os = "linux", feature = "x11-blur"))]
    {
        // KWin X11: set _KDE_NET_WM_BLUR_BEHIND_REGION atom.
        // On Wayland/KWin, transparent background is sufficient.
        // See SKILL-SLINT-MD3.md §7.3 for the full x11rb implementation.
        eprintln!("[Toto] KWin blur: build with --features x11-blur for full support");
    }
}

#[cfg(target_os = "windows")]
fn enable_dwm_acrylic(hwnd: windows_sys::Win32::Foundation::HWND) {
    use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE};
    // DWMSBT_TRANSIENTWINDOW = 3 → Acrylic blur (correct for tool/utility windows)
    let backdrop: u32 = 3;
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        );
    }
}
