// examples/applet_toto_hud.rs
// Toto HUD (Compact Floating Instrument Widget)
// Real mic + Mamba inference + Emerald City (octave-fold) resonant color.

slint::include_modules!();

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, bounded};
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;

use twister::audio::{AudioEngine, record_channel, tdoa_channel};
use twister::bispectrum::BISPEC_FFT_SIZE;
use twister::state::AppState;
use twister::training::MambaTrainer;
use twister::ui::{enable_acrylic_blur, get_resonant_color};

fn build_wave_path(samples: &[f32]) -> String {
    // Viewbox: 0..100 x 0..100
    let n = 100usize;
    if samples.is_empty() {
        return "M 0 50 L 100 50".to_string();
    }

    let mut out = String::with_capacity(n * 16);
    out.push_str("M 0 50");

    for i in 0..n {
        let t = if n <= 1 { 0.0 } else { i as f32 / (n - 1) as f32 };
        let idx = ((samples.len() - 1) as f32 * t).round() as usize;
        let s = samples[idx].clamp(-1.0, 1.0);
        let x = 100.0 * t;
        let y = 50.0 - s * 35.0;
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

    // Start at the first point.
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

fn drain_latest(rx: &Receiver<Vec<f32>>, ring: &mut Vec<f32>, keep: usize) {
    while let Ok(chunk) = rx.try_recv() {
        ring.extend_from_slice(&chunk);
        if ring.len() > keep {
            let drop_n = ring.len() - keep;
            ring.drain(0..drop_n);
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = Arc::new(AppState::new());

    // Audio: primary (merged) stream plus TDOA/record side channels.
    let (merge_tx, merge_rx) = bounded::<Vec<f32>>(32);
    let (tdoa_tx, _tdoa_rx) = tdoa_channel();
    let (rec_tx, _rec_rx) = record_channel();

    let audio = AudioEngine::new(state.clone(), merge_tx, tdoa_tx, rec_tx)?;
    let sample_rate = audio.sample_rate;

    let trainer = Arc::new(MambaTrainer::new(state.clone())?);

    let ui = TotoHudApplet::new()?;
    ui.set_unit_size(384.0);
    ui.set_dvr_recording(true);
    ui.set_dvr_buffer_days(97);

    // Optional: enable acrylic when compiled with `--features windows-sys`.
    enable_acrylic_blur(ui.window());

    let ui_weak = ui.as_weak();

    // Background loop: consume mic frames, compute FFT peak, run Mamba inference,
    // and push a single batched update into Slint per tick.
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_millis(16));
        let mut last_infer = Instant::now() - Duration::from_millis(999);

        let mut ring: Vec<f32> = Vec::with_capacity(BISPEC_FFT_SIZE * 4);
        let mut loss_hist: VecDeque<f32> = VecDeque::with_capacity(64);

        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(BISPEC_FFT_SIZE);
        let mut fft_buf: Vec<Complex<f32>> = vec![Complex { re: 0.0, im: 0.0 }; BISPEC_FFT_SIZE];
        let mut mags: Vec<f32> = vec![0.0; BISPEC_FFT_SIZE / 2];

        let mut anomaly_score: f32 = 0.0;
        let mut learning_loss: f32 = 0.0;
        let mut dominant_freq_hz: f32 = 0.0;

        loop {
            tick.tick().await;

            drain_latest(&merge_rx, &mut ring, BISPEC_FFT_SIZE * 8);
            if ring.len() < BISPEC_FFT_SIZE {
                continue;
            }

            // Prepare FFT input window.
            let start = ring.len() - BISPEC_FFT_SIZE;
            for (i, c) in fft_buf.iter_mut().enumerate() {
                let s = ring[start + i];
                // Hann window to reduce leakage.
                let w = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (BISPEC_FFT_SIZE as f32)).cos();
                c.re = s * w;
                c.im = 0.0;
            }

            fft.process(&mut fft_buf);

            for (i, m) in mags.iter_mut().enumerate() {
                *m = fft_buf[i].norm();
            }

            if let Some((peak_bin, _)) = mags
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            {
                dominant_freq_hz = (peak_bin as f32 / BISPEC_FFT_SIZE as f32) * sample_rate;
            }

            // Run inference at a lower rate than UI tick to avoid starving the event loop.
            if last_infer.elapsed() >= Duration::from_millis(50) {
                last_infer = Instant::now();

                match trainer.infer(&mags).await {
                    Ok((anomaly, _latent, recon)) => {
                        anomaly_score = anomaly;

                        // Prefer a reconstruction-derived loss when available.
                        let mut mse = 0.0f32;
                        let n = mags.len().min(recon.len());
                        if n > 0 {
                            for i in 0..n {
                                let d = mags[i] - recon[i];
                                mse += d * d;
                            }
                            mse /= n as f32;
                            learning_loss = mse.sqrt();
                        } else {
                            learning_loss = anomaly;
                        }

                        state.set_mamba_anomaly(anomaly_score);
                    }
                    Err(_) => {
                        // Keep previous values; the UI should not flicker.
                    }
                }

                loss_hist.push_back(learning_loss);
                while loss_hist.len() > 64 {
                    loss_hist.pop_front();
                }
            }

            let wave_slice = &ring[start..];
            let wave_path = build_wave_path(wave_slice);
            let loss_path = build_series_path(&loss_hist);
            let resonant_color = get_resonant_color(dominant_freq_hz as f64);

            let ui_weak2 = ui_weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                let Some(ui) = ui_weak2.upgrade() else { return };

                // One batched property update per frame.
                ui.set_anomaly_score(anomaly_score);
                ui.set_auto_steer(true);
                ui.set_dominant_freq_hz(dominant_freq_hz);
                ui.set_wave_path(wave_path.into());
                ui.set_learning_loss(learning_loss);
                ui.set_loss_path(loss_path.into());
                ui.set_resonant_color(resonant_color);
            });
        }
    });

    ui.run()?;
    Ok(())
}
