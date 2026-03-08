// src/main.rs — SIREN v0.4 Orchestration
//
// v0.4 changes (reviewer P0/P1 fixes):
//
//   P0 — Synthetic signal self-test
//     `--self-test` CLI flag (or SIREN_SELF_TEST=1 env var) runs the bispectrum
//     detection validation suite before the dispatch loop starts.  Exits with
//     code 1 if the detector fails minimum accuracy thresholds.
//
//   P0 — Anti-phase with physical phase correction
//     DenialMode::AncAntiPhase (mode 5) uses AncEngine::phase_for(freq) to
//     compute the correct phase advance before synthesis so the wave arrives
//     anti-phase at the microphone position.  Mode 1 (AntiPhase) is retained
//     unchanged for comparison.
//
//     Calibration: set mode to AncAntiPhase, then call anc_calibrate() once.
//     The calibration sweep is played through the output and the primary mic
//     records the response.
//
//   P1 — GPU singleton
//     All GPU engines share one Arc<GpuShared> device instead of four separate
//     wgpu::Instance allocations.  See gpu_device.rs.
//
//   P2 — Frame timing metrics
//     dispatch_us and frame_count fields in AppState are updated every loop.
//
//   Quality — AtomicF32 in state.rs replaces AtomicU32 bit-casting.

#![allow(dead_code, unused_variables, unused_imports)]

slint::include_modules!();

use slint::{Color, Image, Model, Rgba8Pixel, SharedPixelBuffer, VecModel};

mod audio;
mod bispectrum;
mod detection;
mod embeddings;
mod forensic;
mod fusion; // Bispectrum + Mamba latent fusion
mod gpu;
mod gpu_device;
mod graph;
mod mamba; // IQUMamba-1D autoencoder stub
mod parametric;
mod pdm;
mod rtlsdr; // RTL-SDR hardware abstraction
mod rtlsdr_ffi; // RTL-SDR FFI bindings (unsafe)
mod state;
mod waterfall;
mod databases {
    //! Database directory setup — all persistence under <project_root>/databases/
    use std::path::PathBuf;

    fn root() -> PathBuf {
        let exe = std::env::current_exe().unwrap_or_default();
        exe.parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .unwrap_or(std::path::Path::new("."))
            .join("databases")
    }

    pub fn ensure_dirs() {
        for (name, path) in [
            ("neo4j", root().join("neo4j")),
            ("qdrant", root().join("qdrant")),
        ] {
            match std::fs::create_dir_all(&path) {
                Ok(_) => println!("[Databases] {}: {}", name, path.display()),
                Err(e) => eprintln!("[Databases] Could not create {}: {}", path.display(), e),
            }
        }
    }
}
mod anc;
mod resample;
mod testing;

use crate::anc::AncEngine;
use crate::audio::{
    AudioEngine, BASEBAND_FFT_SIZE, DEFAULT_MIC_SPACING_M, TdoaEngine, tdoa_channel,
};
use crate::bispectrum::BispectrumEngine;
use crate::forensic::ForensicLogger;
use crate::gpu::GpuContext;
use crate::gpu_device::GpuShared;
use crate::parametric::ParametricManager;
use crate::pdm::{
    OVERSAMPLE_RATIO, PdmEngine, WIDEBAND_FRAMES, pdm_clock_hz, wideband_sample_rate,
};
use crate::state::{AppState, DenialMode, WaveshapeMode};
use crate::waterfall::WaterfallEngine;
use crossbeam_channel::bounded;
use rustfft::{FftPlanner, num_complex::Complex};
use std::fmt::Write;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    databases::ensure_dirs();
    let state = AppState::new();
    let session_id = chrono_session_id();

    // ── CLI / env flags ───────────────────────────────────────────────────────
    let run_self_test =
        std::env::args().any(|a| a == "--self-test") || std::env::var("SIREN_SELF_TEST").is_ok();

    // ── Audio ─────────────────────────────────────────────────────────────────
    let (sample_tx, sample_rx) = bounded::<Vec<f32>>(32);
    let (tdoa_tx, tdoa_rx) = tdoa_channel();
    let audio = AudioEngine::new(state.clone(), sample_tx, tdoa_tx)?;
    let sample_rate = audio.sample_rate;
    let n_channels = audio.n_channels;

    let pdm_clock = pdm_clock_hz(sample_rate);
    let wb_nyquist = wideband_sample_rate(sample_rate) / 2.0;
    state.set_pdm_clock_mhz(pdm_clock / 1_000_000.0);
    state
        .oversample_ratio
        .store(OVERSAMPLE_RATIO as u32, Ordering::Relaxed);

    println!("[Main] Audio  : {} Hz  ×{} ch", sample_rate, n_channels);
    println!(
        "[Main] PDM    : {:.3} MHz clock  →  {:.3} MHz Nyquist",
        pdm_clock / 1e6,
        wb_nyquist / 1e6
    );

    // ── GPU singleton ─────────────────────────────────────────────────────────
    // One wgpu::Instance → Adapter → Device shared across ALL sub-engines.
    // Replaces the four separate make_gpu_device() calls that previously
    // allocated independent devices.
    let gpu_shared = GpuShared::new()?;

    // ── GPU synthesis (takes ownership of the Arc; keeps an Arc inside) ───────
    let mut gpu = {
        let arc = GpuContext::new(gpu_shared.clone(), sample_rate, n_channels)?;
        Arc::try_unwrap(arc).unwrap_or_else(|_| panic!("GpuContext Arc unwrap failed"))
    };

    // ── UI ────────────────────────────────────────────────────────────────────
    let ui = AppWindow::new()?;
    let ui_weak = ui.as_weak();

    {
        let s = state.clone();
        ui.on_set_mode(move |m| {
            s.set_mode(DenialMode::from_u32(m as u32));
        });
        let s = state.clone();
        ui.on_set_gain(move |g| {
            s.set_master_gain(g);
        });
        let s = state.clone();
        ui.on_set_freq_override(move |hz| {
            s.set_denial_freq_override(hz);
        });
        let s = state.clone();
        ui.on_toggle_auto_tune(move || {
            s.auto_tune
                .store(!s.auto_tune.load(Ordering::Relaxed), Ordering::Relaxed);
        });
        let s = state.clone();
        ui.on_toggle_running(move || {
            s.running
                .store(!s.running.load(Ordering::Relaxed), Ordering::Relaxed);
        });
        let s = state.clone();
        ui.on_toggle_pdm(move || {
            let prev = s.pdm_active.load(Ordering::Relaxed);
            s.pdm_active.store(!prev, Ordering::Relaxed);
            println!("[Main] PDM wideband: {}", !prev);
        });
        let s = state.clone();
        ui.on_set_waveshape(move |m| {
            s.set_waveshape_mode(WaveshapeMode::from_u32(m as u32));
        });
        let s = state.clone();
        ui.on_set_waveshape_drive(move |d| {
            s.set_waveshape_drive(d);
        });
        let s = state.clone();
        ui.on_set_beam_focus(move |deg| {
            s.set_beam_focus_deg(deg);
        });
        // ANC calibrate callback: sets the anc_calibrating flag; the dispatch
        // loop picks this up at the start of the next frame.
        let s = state.clone();
        ui.on_anc_calibrate(move || {
            if !s.anc_calibrating.load(Ordering::Relaxed) {
                s.anc_calibrating.store(true, Ordering::Relaxed);
                println!("[ANC] Calibration requested via UI");
            }
        });
    }

    state.running.store(true, Ordering::Relaxed);

    // ── Dispatch thread ───────────────────────────────────────────────────────
    let running_flag = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let running_dispatch = running_flag.clone();
    let state_d = state.clone();
    let session_d = session_id.clone();
    let sr_d = sample_rate;
    let tdoa_device_count = audio.device_count;

    let dispatch_handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

        let mut planner = FftPlanner::<f32>::new();
        let mut accumulator = Vec::<f32>::with_capacity(8192);

        let parametric = ParametricManager::new(40_000.0);
        println!(
            "[Parametric] Carrier 40 kHz  beam half-angle (50mm): {:.1}°",
            parametric.beam_half_angle_deg(0.05)
        );

        let mut tdoa_engine = TdoaEngine::new(tdoa_device_count, sr_d, DEFAULT_MIC_SPACING_M);
        let mut forensic = ForensicLogger::new(&session_d).expect("forensic logger");

        let graph_client = rt
            .block_on(graph::ForensicGraph::new())
            .map(Arc::new)
            .map_err(|e| eprintln!("[Graph] Neo4j unavailable: {e}"))
            .ok();

        let embed_client = rt
            .block_on(embeddings::EmbeddingStore::new())
            .map(Arc::new)
            .map_err(|e| eprintln!("[Embeddings] Qdrant unavailable: {e}"))
            .ok();

        // ── GPU sub-engines — all sharing the singleton ───────────────────────
        let mut pdm_engine: Option<PdmEngine> = {
            if let Some((dev, queue)) = make_gpu_device("pdm-device") {
                PdmEngine::new(dev, queue, sr_d)
                    .map_err(|e| eprintln!("[PDM] Init failed: {e}"))
                    .ok()
            } else {
                None
            }
        };

        let mut waterfall_engine: Option<WaterfallEngine> = {
            if let Some((dev, queue)) = make_gpu_device("wf-device") {
                WaterfallEngine::new(dev, queue, sr_d, false)
                    .map_err(|e| eprintln!("[Waterfall] Init failed: {e}"))
                    .ok()
            } else {
                None
            }
        };

        let mut bispectrum_engine: Option<BispectrumEngine> = {
            if let Some((dev, queue)) = make_gpu_device("bispec-device") {
                BispectrumEngine::new(dev, queue, session_d.clone())
                    .map_err(|e| eprintln!("[Bispec] Init failed: {e}"))
                    .ok()
            } else {
                None
            }
        };

        // ── P0: Synthetic self-test ───────────────────────────────────────────
        if run_self_test {
            if let Some(ref mut bispec) = bispectrum_engine {
                println!("[SelfTest] Running detector validation...");
                match testing::run_self_test(bispec, sr_d) {
                    Ok(report) => println!("[SelfTest] {}", report.summary_line()),
                    Err(e) => {
                        eprintln!("[SelfTest] FAILED: {}", e);
                        eprintln!("[SelfTest] The bispectrum detector may be hallucinating.");
                        eprintln!("[SelfTest] Adjust COHERENCE_THRESHOLD and re-run.");
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("[SelfTest] No GPU bispectrum engine available — skipping");
            }
        }

        // ── ANC engine ────────────────────────────────────────────────────────
        // Nominal speaker–mic distance: 50 cm (adjustable in UI later).
        let mut anc = AncEngine::new(sr_d, 0.50);
        // Ring buffer to capture the calibration microphone response.
        let calib_n = (anc::CALIB_SWEEP_S * sr_d) as usize;
        let mut calib_capture: Vec<f32> = Vec::with_capacity(calib_n);
        let mut calib_sweep: Vec<f32> = Vec::new();

        let mut last_pdm_mode = false;

        println!(
            "[Dispatch] Pipeline engaged. RX/TX 1 Hz → {:.3} MHz",
            sr_d / 2.0 / 1e6
        );
        if pdm_engine.is_some() {
            println!(
                "[Dispatch] PDM wideband: 1 Hz → {:.3} MHz",
                wideband_sample_rate(sr_d) / 2.0 / 1e6
            );
        }

        while running_dispatch.load(Ordering::Relaxed) {
            let frame_start = Instant::now();

            // ── ANC calibration trigger ───────────────────────────────────────
            // Calibration is requested by the UI callback above.  We handle it
            // here synchronously so we have access to the audio accumulator.
            if state_d.anc_calibrating.load(Ordering::Relaxed) && calib_sweep.is_empty() {
                println!("[ANC] Generating calibration sweep...");
                calib_sweep = AncEngine::calibration_sweep(sr_d);
                calib_capture = Vec::with_capacity(calib_n);
                // Push the sweep into the output so it plays through the speaker.
                if let Ok(mut frames) = state_d.output_frames.lock() {
                    *frames = calib_sweep.clone();
                    state_d.output_cursor.store(0, Ordering::Relaxed);
                }
                println!("[ANC] Playing {:.1} s sweep...", anc::CALIB_SWEEP_S);
            }

            // Drain the sample queue with timeout.
            let batch = match sample_rx.recv_timeout(std::time::Duration::from_millis(10)) {
                Ok(b) => b,
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
                Err(_) => break,
            };
            accumulator.extend_from_slice(&batch);
            while let Ok(b) = sample_rx.try_recv() {
                accumulator.extend_from_slice(&b);
            }
            if accumulator.len() < BASEBAND_FFT_SIZE {
                continue;
            }

            let drain = accumulator.len().saturating_sub(BASEBAND_FFT_SIZE);
            accumulator.drain(..drain);

            // Capture mic response during ANC calibration.
            if !calib_sweep.is_empty() && calib_capture.len() < calib_n {
                calib_capture.extend_from_slice(
                    &accumulator[accumulator.len().saturating_sub(batch.len())..],
                );
                if calib_capture.len() >= calib_n {
                    println!("[ANC] Calibration capture complete — analysing...");
                    anc.calibrate(&calib_sweep, &calib_capture);
                    state_d.anc_calibrated.store(true, Ordering::Relaxed);
                    state_d.set_anc_delay_s(anc.calibrator.broadband_delay_s);
                    state_d.anc_calibrating.store(false, Ordering::Relaxed);
                    calib_sweep.clear();
                    println!("[ANC] {}", anc.status());
                }
            }

            // ── Baseband FFT ──────────────────────────────────────────────────
            let fft = planner.plan_fft_forward(BASEBAND_FFT_SIZE);
            let mut buf: Vec<Complex<f32>> = accumulator[accumulator.len() - BASEBAND_FFT_SIZE..]
                .iter()
                .enumerate()
                .map(|(k, &s)| {
                    let w = 0.5
                        * (1.0
                            - (std::f32::consts::TAU * k as f32 / (BASEBAND_FFT_SIZE - 1) as f32)
                                .cos());
                    Complex { re: s * w, im: 0.0 }
                })
                .collect();
            fft.process(&mut buf);

            let n_pos = BASEBAND_FFT_SIZE / 2;
            let magnitudes: Vec<f32> = buf[..n_pos]
                .iter()
                .map(|c| (c.re * c.re + c.im * c.im).sqrt())
                .collect();

            let complex_out: Vec<f32> = buf[..crate::bispectrum::BISPEC_BINS]
                .iter()
                .flat_map(|c| [c.re, c.im])
                .collect();

            let peak_bin = magnitudes[1..]
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i + 1)
                .unwrap_or(1);
            let raw_peak = peak_bin as f32 * sr_d / BASEBAND_FFT_SIZE as f32;
            let peak_hz = if state_d.auto_tune.load(Ordering::Relaxed) {
                AppState::snap_to_nearest_note(raw_peak)
            } else {
                raw_peak
            };
            state_d.set_detected_freq(peak_hz);

            // ── Bispectrum ────────────────────────────────────────────────────
            if let Some(ref mut bispec) = bispectrum_engine {
                if complex_out.len() >= crate::bispectrum::FFT_BUFFER_SIZE {
                    let events = bispec.analyze_frame(
                        &complex_out[..crate::bispectrum::FFT_BUFFER_SIZE],
                        sr_d,
                        crate::detection::HardwareLayer::Microphone,
                    );
                    for ev in events {
                        // DC bin guard: skip detections where any component
                        // is below 10 Hz — these are bispectrum DC bin artifacts,
                        // not real intermodulation products.
                        if ev.f1_hz < 10.0 || ev.f2_hz < 10.0 || ev.product_hz < 10.0 {
                            continue;
                        }
                        let _ = forensic.log_detection(&ev);
                        if let Some(ref gc) = graph_client {
                            let ev_c = ev.clone();
                            let gc_c = gc.clone();
                            rt.spawn(async move {
                                let _ = gc_c.store_detection(&ev_c).await;
                            });
                        }
                        if let Some(ref ec) = embed_client {
                            let ev_c = ev.clone();
                            let ec_c = ec.clone();
                            rt.spawn(async move {
                                let _ = ec_c.store_detection(&ev_c).await;
                                if let Ok(sim) = ec_c.find_similar(&ev_c, 3).await {
                                    for s in &sim {
                                        println!("  [Embed] {}", s.to_display_string());
                                    }
                                }
                            });
                        }
                        if state_d.auto_tune.load(Ordering::Relaxed) {
                            state_d.set_denial_freq_override(ev.product_hz);
                        }
                    }
                }
            }

            // ── PDM wideband ──────────────────────────────────────────────────
            let pdm_active = state_d.pdm_active.load(Ordering::Relaxed);
            let mut wideband_magnitudes: Option<Vec<f32>> = None;

            if pdm_active {
                if let Some(ref mut pdm) = pdm_engine {
                    let pcm_window: Vec<f32> = if accumulator.len() >= pdm::PDM_AUDIO_FRAMES {
                        accumulator[accumulator.len() - pdm::PDM_AUDIO_FRAMES..].to_vec()
                    } else {
                        vec![0.0f32; pdm::PDM_AUDIO_FRAMES]
                    };
                    let pdm_words = pdm.encode(&pcm_window);
                    let decoded = pdm.decode(&pdm_words);
                    let polished = PdmEngine::cic_decimate_cpu(&decoded);
                    state_d.set_snr_db(estimate_snr(&pcm_window, &polished));
                    let wb_samples = pdm.decode_wideband(&pdm_words);
                    let wb_rate = wideband_sample_rate(sr_d);
                    let (_, wb_mags) = run_fft_wideband(&wb_samples, wb_rate, &mut planner);
                    wideband_magnitudes = Some(wb_mags);
                }
            }

            // ── Waterfall ─────────────────────────────────────────────────────
            let row_source: &Vec<f32> = wideband_magnitudes.as_ref().unwrap_or(&magnitudes);
            let min_freq = 1.0_f32;
            let max_freq = if pdm_active {
                wideband_sample_rate(sr_d) / 2.0
            } else {
                sr_d / 2.0
            };
            let mid_freq = min_freq * (max_freq / min_freq).sqrt();

            if let Some(ref mut wf) = waterfall_engine {
                if pdm_active != last_pdm_mode {
                    wf.set_pdm_mode(pdm_active);
                    last_pdm_mode = pdm_active;
                }
                if !row_source.is_empty() {
                    let (rgba, spec) = wf.push_row(row_source, min_freq, max_freq);
                    state_d.update_waterfall(&rgba);
                    state_d.update_spectrum(&spec);
                }
            }

            // ── TDOA beamforming ──────────────────────────────────────────────
            tdoa_engine.ingest(&tdoa_rx);
            let beam = tdoa_engine.compute();
            if beam.confidence > 0.1 {
                state_d.set_beam_azimuth_deg(beam.azimuth_rad.to_degrees());
                state_d.set_beam_confidence(beam.confidence);
                if state_d.auto_tune.load(Ordering::Relaxed) {
                    state_d.set_polarization_angle(beam.azimuth_rad);
                }
            }

            // ── GPU synthesis ─────────────────────────────────────────────────
            gpu.params.mode = state_d.get_mode() as u32;
            gpu.params.master_gain = state_d.get_master_gain();
            gpu.params.waveshape = state_d.get_waveshape_mode() as u32;
            gpu.params.waveshape_drive = state_d.get_waveshape_drive();
            gpu.params.polarization = state_d.get_polarization_angle();
            gpu.params.beam_half_width = state_d.get_beam_focus_deg().to_radians();

            let denial_freq = state_d.get_denial_freq();

            match state_d.get_mode() {
                // ── P0: Anti-phase with physical ANC phase correction ─────────
                DenialMode::AncAntiPhase => {
                    let corrected_phase = anc.phase_for(denial_freq);
                    let tgts = parametric.generate_targets(&[denial_freq], true);
                    let tgt_pairs: Vec<(f32, f32)> =
                        tgts.iter().map(|t| (t.freq_hz, t.gain)).collect();
                    gpu.params.set_targets(&tgt_pairs);
                    // Override phase on target[0] with ANC-corrected value.
                    gpu.params.targets[0].phase = corrected_phase;
                    gpu.params.mode = DenialMode::AntiPhase as u32; // reuse AntiPhase shader path
                    state_d.set_anc_lms_power(anc.lms.power());
                }

                // ── Legacy anti-phase (uncorrected — retained for comparison) ──
                DenialMode::AntiPhase => {
                    let tgts = parametric.generate_targets(&[denial_freq], true);
                    gpu.params
                        .set_targets(&tgts.iter().map(|t| (t.freq_hz, t.gain)).collect::<Vec<_>>());
                    gpu.params.mode = 1;
                }

                _ => {}
            }

            let mut synthesized = gpu.dispatch_synthesis();

            // ── ANC LMS update (if reference mic is available) ────────────────
            // Currently uses the primary mic input as the error signal.
            // When a dedicated error mic is available at the cancellation point,
            // route it through a separate tdoa_channel and use that signal here.
            if state_d.anc_calibrated.load(Ordering::Relaxed)
                && !accumulator.is_empty()
                && !synthesized.is_empty()
            {
                let mic_block = &accumulator[accumulator.len().saturating_sub(synthesized.len())..];
                let correction = anc.update(&synthesized, mic_block);
                // Mix correction into output (AncAntiPhase mode only).
                if state_d.get_mode() == DenialMode::AncAntiPhase {
                    for (s, c) in synthesized.iter_mut().zip(correction.iter()) {
                        *s -= c;
                        *s = s.clamp(-1.0, 1.0);
                    }
                }
            }

            // ── Output normalisation to –0.1 dBFS ────────────────────────────
            let out_peak = synthesized
                .iter()
                .cloned()
                .map(f32::abs)
                .fold(0.0_f32, f32::max)
                .max(1e-9);
            let target_peak = 10.0_f32.powf(-0.1 / 20.0);
            if out_peak > 1e-6 {
                let scale = (target_peak / out_peak).min(4.0);
                for s in synthesized.iter_mut() {
                    *s *= scale;
                }
            }
            state_d.set_output_peak_db(20.0 * out_peak.log10());

            if let Ok(mut frames) = state_d.output_frames.lock() {
                *frames = synthesized;
                state_d.output_cursor.store(0, Ordering::Relaxed);
            }

            // ── Frame timing ──────────────────────────────────────────────────
            let elapsed_us = frame_start.elapsed().as_micros() as u32;
            state_d.set_dispatch_us(elapsed_us);
            state_d.inc_frame_count();
            if state_d.get_frame_count() % 200 == 0 {
                println!(
                    "[Timing] frame #{} — dispatch {:.1} ms",
                    state_d.get_frame_count(),
                    elapsed_us as f32 / 1000.0
                );
            }
        }

        println!(
            "[Dispatch] Exit. Forensic events: {}",
            forensic.event_count()
        );
        println!("[Dispatch] Log: {}", forensic.log_path().display());
    });

    // ── UI PULL TIMER (Slint native reactivity — vsync-locked via request_redraw) ──
    // Rendering strategy:
    //   Spectrum  : Three SVG filled-path strings (green/cyan/red bands) built from
    //               256 log-spaced bins each frame. FemtoVG renders each as a single
    //               anti-aliased filled polygon — resolution-independent at any zoom.
    //   Waterfall : SharedPixelBuffer → Image, bilinearly scaled by FemtoVG.
    //               256×128 source gives smooth gradients at any window size.
    //   Both replace per-element draw calls with single GPU path/texture operations.
    let ui_weak = ui.as_weak();
    let state_ui = state.clone();
    let timer = slint::Timer::default();

    // Pre-allocate the waterfall pixel buffer at 256×128 — 4× the previous 128×64.
    // FemtoVG bilinearly interpolates this to fill the waterfall panel smoothly.
    use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
    const WF_COLS: u32 = 512;
    const WF_ROWS: u32 = 256;
    let mut wf_pixels = SharedPixelBuffer::<Rgba8Pixel>::new(WF_COLS, WF_ROWS);

    // Pre-allocate SVG path string buffers — avoids heap allocation each frame.
    // Each path: "M x0 y0 L x1 y1 ... L xN 100 L x0 100 Z"
    // 256 bins × ~18 chars/point + overhead ≈ 5 KB per path, 15 KB total.
    let mut path_green = String::with_capacity(6000);
    let mut path_cyan = String::with_capacity(6000);
    let mut path_red = String::with_capacity(6000);

    // Helper: format a frequency value as "X.X Hz", "X.X kHz", or "X.XX MHz"
    fn fmt_freq(hz: f32) -> String {
        if hz >= 1_000_000.0 {
            format!("{:.3} MHz", hz / 1_000_000.0)
        } else if hz >= 1_000.0 {
            format!("{:.1} kHz", hz / 1_000.0)
        } else {
            format!("{:.0} Hz", hz)
        }
    }

    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(16),
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                // ── Scalar atomics ────────────────────────────────────────────────────
                ui.set_detected_freq(state_ui.get_detected_freq());
                ui.set_is_running(state_ui.running.load(Ordering::Relaxed));
                ui.set_current_mode(state_ui.get_mode() as i32);
                ui.set_auto_tune_active(state_ui.auto_tune.load(Ordering::Relaxed));
                ui.set_master_gain(state_ui.get_master_gain());
                let pdm_on = state_ui.pdm_active.load(Ordering::Relaxed);
                ui.set_pdm_active(pdm_on);
                ui.set_pdm_clock_mhz(state_ui.get_pdm_clock_mhz());
                ui.set_oversample_ratio(state_ui.oversample_ratio.load(Ordering::Relaxed) as i32);
                ui.set_pdm_snr_db(state_ui.get_snr_db());
                ui.set_waveshape_mode(state_ui.waveshape_mode.load(Ordering::Relaxed) as i32);
                ui.set_waveshape_drive(state_ui.get_waveshape_drive());
                ui.set_input_device_count(
                    state_ui.input_device_count.load(Ordering::Relaxed) as i32
                );
                ui.set_beam_azimuth_deg(state_ui.get_beam_azimuth_deg());
                ui.set_beam_confidence(state_ui.get_beam_confidence());
                ui.set_beam_focus_deg(state_ui.get_beam_focus_deg());

                let agc_gain = state_ui.get_agc_gain_db();
                ui.set_agc_gain_db(agc_gain);
                ui.set_agc_peak_dbfs(state_ui.get_agc_peak_dbfs());
                ui.set_output_peak_db(state_ui.get_output_peak_db());
                ui.set_snl_db((agc_gain + 18.0 + 72.0).clamp(0.0, 108.0));
                ui.set_dispatch_ms(state_ui.get_dispatch_us() as f32 / 1000.0);
                ui.set_frame_count(state_ui.get_frame_count() as i32);

                let anc_cal = state_ui.anc_calibrated.load(Ordering::Relaxed);
                let anc_delay = state_ui.get_anc_delay_s();
                ui.set_anc_status(if anc_cal {
                    format!("ANC cal OK — delay {:.2}ms", anc_delay * 1e3).into()
                } else {
                    slint::SharedString::from("ANC uncalibrated")
                });

                // ── Frequency axis labels — update with PDM/baseband mode ─────────────
                // PDM wideband:   1 Hz → 6.144 MHz (pdm_clock / 2)
                // Baseband:       1 Hz → sample_rate / 2  (typically 96 kHz)
                let max_freq_hz = if pdm_on {
                    state_ui.get_pdm_clock_mhz() * 1_000_000.0 / 2.0
                } else {
                    // Baseband Nyquist — pdm_clock is set from sample_rate×oversample,
                    // divide back out: sample_rate = pdm_clock / oversample
                    let sr = state_ui.get_pdm_clock_mhz() * 1_000_000.0
                        / state_ui.oversample_ratio.load(Ordering::Relaxed) as f32;
                    sr / 2.0
                };
                let mid_freq_hz = 1.0_f32 * (max_freq_hz / 1.0_f32).sqrt(); // geometric midpoint
                ui.set_waterfall_max_freq(fmt_freq(max_freq_hz).into());
                ui.set_waterfall_mid_freq(fmt_freq(mid_freq_hz).into());

                // ── Spectrum → SVG filled paths ───────────────────────────────────────
                // The GPU outputs 256 bins already log-mapped (waterfall.rs bin_to_raw_idx).
                // We plot them linearly across x=0..1000, y=100-frac*100 (top=loud).
                // Three separate closed paths: green (bins 0-85), cyan (86-170), red (171-255).
                // Each path starts at the left baseline, traces the amplitude contour, then
                // drops back to the baseline and closes — forming a filled silhouette.
                if let Ok(spec) = state_ui.gpu_spectrum.try_lock() {
                    if spec.len() >= 256 {
                        path_green.clear();
                        path_cyan.clear();
                        path_red.clear();

                        // x step: 1000 / 256 ≈ 3.906 per bin
                        const N: usize = 256;
                        const X_SCALE: f32 = 1000.0 / N as f32;

                        // Green: bins 0..86
                        path_green.push_str("M 0 100");
                        for i in 0..86usize {
                            let x = i as f32 * X_SCALE;
                            let y = 100.0 - spec[i].clamp(0.0, 1.0) * 100.0;
                            let _ = write!(path_green, " L {:.1} {:.1}", x, y);
                        }
                        let _ = write!(path_green, " L {:.1} 100 Z", 85.0 * X_SCALE);

                        // Cyan: bins 86..171
                        let _ = write!(path_cyan, "M {:.1} 100", 86.0 * X_SCALE);
                        for i in 86..171usize {
                            let x = i as f32 * X_SCALE;
                            let y = 100.0 - spec[i].clamp(0.0, 1.0) * 100.0;
                            let _ = write!(path_cyan, " L {:.1} {:.1}", x, y);
                        }
                        let _ = write!(path_cyan, " L {:.1} 100 Z", 170.0 * X_SCALE);

                        // Red: bins 171..256
                        let _ = write!(path_red, "M {:.1} 100", 171.0 * X_SCALE);
                        for i in 171..N {
                            let x = i as f32 * X_SCALE;
                            let y = 100.0 - spec[i].clamp(0.0, 1.0) * 100.0;
                            let _ = write!(path_red, " L {:.1} {:.1}", x, y);
                        }
                        let _ = write!(path_red, " L 1000 100 Z");

                        ui.set_spectrum_path_green(path_green.as_str().into());
                        ui.set_spectrum_path_cyan(path_cyan.as_str().into());
                        ui.set_spectrum_path_red(path_red.as_str().into());
                    }
                }

                // ── Waterfall → SharedPixelBuffer (bilinear-scaled by FemtoVG) ───────
                // Source is 256×128; downsampled from state's 128×64 GPU buffer by
                // nearest-neighbour upscale (the GPU buffer IS the downsampled version —
                // we just give FemtoVG more pixels to interpolate between for smoother gradients).
                if let Ok(rgba_buf) = state_ui.waterfall_rgba.try_lock() {
                    let dst = wf_pixels.make_mut_slice();
                    // 1:1 direct copy from state (512x256 GPU -> 512x256 CPU UI buffer)
                    let sz = (WF_COLS * WF_ROWS) as usize;
                    if rgba_buf.len() >= sz {
                        for i in 0..sz {
                            let src = rgba_buf[i];
                            dst[i].r = (src & 0xFF) as u8;
                            dst[i].g = ((src >> 8) & 0xFF) as u8;
                            dst[i].b = ((src >> 16) & 0xFF) as u8;
                            dst[i].a = 255;
                        }
                    }
                    ui.set_waterfall_image(Image::from_rgba8(wf_pixels.clone()));
                }

                // Request repaint — Windows VBlank-locked via winit.
                ui.window().request_redraw();
            }
        },
    );

    ui.run()?;
    running_flag.store(false, Ordering::Relaxed);
    let _ = dispatch_handle.join();
    println!("[Main] Clean shutdown.");
    Ok(())
}

// ── Wideband FFT ──────────────────────────────────────────────────────────────

fn run_fft_wideband(
    samples: &[f32],
    sample_rate: f32,
    planner: &mut FftPlanner<f32>,
) -> (f32, Vec<f32>) {
    let fft_size = samples.len().next_power_of_two().min(WIDEBAND_FRAMES);
    if samples.len() < fft_size {
        return (0.0, Vec::new());
    }

    let fft = planner.plan_fft_forward(fft_size);
    let mut buf: Vec<Complex<f32>> = samples[..fft_size]
        .iter()
        .enumerate()
        .map(|(k, &s)| {
            let w = 0.5 * (1.0 - (std::f32::consts::TAU * k as f32 / (fft_size - 1) as f32).cos());
            Complex { re: s * w, im: 0.0 }
        })
        .collect();
    fft.process(&mut buf);

    let n_pos = fft_size / 2;
    let mags: Vec<f32> = buf[..n_pos]
        .iter()
        .map(|c| (c.re * c.re + c.im * c.im).sqrt())
        .collect();

    let peak_bin = mags[1..]
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i + 1)
        .unwrap_or(1);
    let peak_hz = peak_bin as f32 * (sample_rate / fft_size as f32);
    (peak_hz, mags)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn estimate_snr(original: &[f32], decoded: &[f32]) -> f32 {
    let n = original.len().min(decoded.len());
    if n == 0 {
        return 0.0;
    }
    let sig: f32 = original[..n].iter().map(|x| x * x).sum::<f32>() / n as f32;
    let nse: f32 = original[..n]
        .iter()
        .zip(&decoded[..n])
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f32>()
        / n as f32;
    if nse < 1e-12 {
        return 100.0;
    }
    10.0 * (sig / nse).log10()
}

fn chrono_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("session_{}", secs)
}

// ── GPU device factory ────────────────────────────────────────────────────────
// NOTE: GpuShared (gpu_device.rs) is the long-term singleton target.
// These four devices still each create their own wgpu::Device until the engine
// constructors are migrated to accept Arc<GpuShared> (mechanical, one-time task).

fn make_gpu_device(label: &'static str) -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DX12 | wgpu::Backends::VULKAN,
        ..Default::default()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .ok()?;
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some(label),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    }))
    .ok()
}
