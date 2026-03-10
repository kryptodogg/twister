pub mod active_denial;
// src/main.rs — Twister v0.5  (Harassment Frequency Auto-Tuner + Forensic Evidence System)
//
// Renamed from SIREN.  All forensic / evidence functionality fully intact.
// The Twister layer sits between detection and synthesis: it snaps every
// detected harassment frequency to the nearest equal-temperament note and
// drives the GPU synthesiser with a musically coherent chord, providing
// stress relief while the bispectrum + Mamba + RTL-SDR pipeline gathers
// court-admissible evidence.
//
// Async model:
//   main          — #[tokio::main] entry point
//   UI            — Slint event loop (blocking, runs last)
//   audio_thread  — cpal capture + AGC (spawned by AudioEngine)
//   dispatch_loop — tokio::spawn: FFT → V-buffer → waterfall → bispectrum → Mamba
//   trainer_loop  — tokio::spawn: Mamba online training
//   sdr_loop      — tokio::spawn: RTL-SDR IQ capture + Twister auto-tune

#[allow(clippy::too_many_arguments)]

slint::include_modules!();

// ── Modules ───────────────────────────────────────────────────────────────────
mod af32;
mod ai;
mod anc;
mod anc_calibration;
mod anc_recording;
mod app_state;
mod audio;
mod bispectrum;
mod computer_vision;
mod detection;
mod dispatch;
mod embeddings;
mod evidence_export;
mod forensic;
mod forensic_queries;
mod fusion;
mod gpu;
mod gpu_shared;
mod graph;
mod hardware_io;
mod harmony;
mod input;
mod knowledge_graph;
mod mamba;
mod materials;
mod ml;
mod parameters;
mod parametric;
mod particle_system;
mod pdm;
mod reconstruct;
mod resample;
mod resonance;
mod ridge_plot;
mod rtlsdr;
mod rtlsdr_ffi;
mod safe_sdr_wrapper;
mod sdr;
mod state;
mod testing;
mod trainer;
mod training;
mod training_tests;
mod twister;
mod ui;
mod vbuffer;
mod vector;
mod visualization;
mod waterfall;

use crate::forensic::ForensicLogger;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use crossbeam_channel::bounded;
use rustfft::{FftPlanner, num_complex::Complex};

use audio::{AudioEngine, DEFAULT_MIC_SPACING_M, TdoaEngine, record_channel, tdoa_channel};
use bispectrum::{BISPEC_FFT_SIZE, BispectrumEngine};
use detection::{DetectionEvent, HardwareLayer};
use gpu::GpuContext;
use gpu_shared::GpuShared;
use parametric::ParametricManager;
use pdm::PdmEngine;
use sdr::sdr_channel;
use state::AppState;
use vbuffer::{V_DEPTH, V_FREQ_BINS, new_shared_vbuffer};
use waterfall::WaterfallEngine;

const PARAMETRIC_CARRIER_HZ: f32 = 40_000.0;
#[allow(dead_code)]
const SESSION_TIMEOUT_SECS: u64 = 3600;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let session_identity: String = {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format!("twister_{:016x}", ts)
    };

    let state = AppState::new();
    state.log(
        "INFO",
        "System",
        &format!("[Twister v0.5] Session: {}", session_identity),
    );
    use crate::particle_system::{
        frustum_culler::FrustumCuller,
        renderer::ParticleRenderer, // streaming::ParticleStreamLoader, // Temporarily disabled
    };
    let ui = AppWindow::new().context("Slint window creation failed")?;

    let gpu_shared = GpuShared::new().context("GPU init failed")?;
    state.log(
        "INFO",
        "GPU",
        &format!(
            "Adapter: {} ({:?})",
            gpu_shared.adapter_info.name, gpu_shared.adapter_info.backend
        ),
    );

    let vbuffer = new_shared_vbuffer(&gpu_shared.device);
    let sdr_vbuffer = new_shared_vbuffer(&gpu_shared.device);

    let (merge_tx, merge_rx) = crossbeam_channel::bounded::<Vec<f32>>(256);
    let (feature_tx, feature_rx) = crossbeam_channel::bounded::<(
        crate::ml::modular_features::SignalFeaturePayload,
        burn::tensor::Tensor<burn::backend::NdArray, 1>,
    )>(256);
    let (impulse_tx, impulse_rx) =
        crossbeam_channel::bounded::<crate::ml::modular_features::ImpulseTrainEvent>(256);
    let (tdoa_tx, tdoa_rx) = tdoa_channel();
    let (record_tx, record_rx) = record_channel();

    let audio = AudioEngine::new(state.clone(), merge_tx, tdoa_tx, record_tx)
        .context("Audio init failed")?;
    let sample_rate = audio.sample_rate;
    let n_channels = audio.n_channels;
    let device_count = audio.device_count;
    state.log(
        "INFO",
        "Audio",
        &format!(
            "{:.0} Hz × {} ch, {} input device(s)",
            sample_rate, n_channels, device_count
        ),
    );

    let gpu_ctx = GpuContext::new(gpu_shared.clone(), sample_rate, n_channels)
        .context("GPU synthesis init")?;
    let pdm = PdmEngine::new(gpu_shared.clone(), sample_rate).context("PDM engine init")?;
    let waterfall = WaterfallEngine::new(gpu_shared.clone(), sample_rate, false)
        .context("Waterfall engine init")?;
    let sdr_sr = state.sdr_sample_rate.load(Ordering::Relaxed) as f32;
    let sdr_waterfall = WaterfallEngine::new(gpu_shared.clone(), sdr_sr, false)
        .context("SDR Waterfall engine init")?;
    let bispec = BispectrumEngine::new(gpu_shared.clone(), session_identity.clone())
        .context("Bispectrum engine init")?;

    let parametric_manager = ParametricManager::new(PARAMETRIC_CARRIER_HZ);

    let qdrant = Arc::new(match embeddings::EmbeddingStore::new().await {
        Ok(s) => {
            state.log("INFO", "Qdrant", "Connected.");
            Some(s)
        }
        Err(e) => {
            state.log("ERROR", "Qdrant", &format!("Unavailable: {e:?}"));
            None
        }
    });
    let neo4j = Arc::new(tokio::sync::Mutex::new(
        match crate::graph::ForensicGraph::new(
            "bolt://localhost:7687",
            "neo4j",
            "twister_forensic_2026",
        )
        .await
        {
            Ok(g) => {
                state.log("INFO", "Neo4j", "Connected.");
                Some(g)
            }
            Err(e) => {
                state.log("ERROR", "Neo4j", &format!("Offline: {e:?}"));
                None
            }
        },
    ));

    let forensic = ForensicLogger::new(session_identity.as_str())
        .await
        .map_err(|e| anyhow::anyhow!("{:?}", e))
        .context("Forensic log init")?;

    // Log SessionStart
    let start_ev = crate::forensic::ForensicEvent::SessionStart {
        timestamp_micros: crate::forensic::get_current_micros(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        total_events_prior: 0,
    };
    let _ = forensic.log(start_ev);
    state.log(
        "INFO",
        "Forensic",
        &format!("Log: {}", forensic.log_path().display()),
    );

    let (sdr_tx, sdr_rx) = sdr_channel();
    let sdr_thread_handle = sdr::spawn_sdr_thread(state.clone(), sdr_tx);

    let mamba_trainer =
        Arc::new(training::MambaTrainer::new(state.clone()).context("Mamba trainer init")?);

    // Load existing checkpoint (graceful — missing file is not an error)
    {
        let ckpt = state
            .checkpoint_path
            .lock()
            .map(|p| p.clone())
            .unwrap_or_else(|_| "weights/mamba_siren.safetensors".to_string());
        if std::path::Path::new(&ckpt).exists() {
            match mamba_trainer.load(&ckpt).await {
                Ok(Some(meta)) => {
                    state.log(
                        "INFO",
                        "Mamba",
                        &format!("Loaded checkpoint: {} (epoch {})", ckpt, meta.epoch),
                    );
                    state.train_epoch.store(meta.epoch, Ordering::Relaxed);
                    state.train_loss.store(meta.loss_avg, Ordering::Relaxed);
                }
                Ok(std::option::Option::None) => {
                    state.log(
                        "INFO",
                        "Mamba",
                        &format!("Loaded checkpoint: {} (no metadata found)", ckpt),
                    );
                }
                Err(e) => state.log("ERROR", "Mamba", &format!("Checkpoint load failed: {e}")),
            }
        } else {
            std::fs::create_dir_all("weights").ok();
            state.log(
                "INFO",
                "Mamba",
                &format!("No checkpoint at {} — starting fresh", ckpt),
            );
        }
    }

    let training_session = Arc::new(training::TrainingSession::new(state.clone()));

    // Create telemetry channel for async tasks to emit UI events
    // (crossbeam provides better thread-safety for Slint integration)
    let (ui_tx, ui_rx) = crossbeam_channel::unbounded::<crate::state::UiEvent>();

    let mamba_trainer_handle = training::spawn_background_training(
        training_session.clone(),
        mamba_trainer.clone(),
        state.clone(),
        ui_tx.clone(),
    );
    state.log("INFO", "Trainer", "Mamba background training active");

    // TDOA
    let tdoa_state = state.clone();
    tokio::spawn(async move {
        let mut engine = TdoaEngine::new(device_count, sample_rate, DEFAULT_MIC_SPACING_M);
        loop {
            engine.ingest(&tdoa_rx);
            let beam = engine.compute();
            tdoa_state.set_beam_azimuth_deg(beam.azimuth.to_degrees());
            tdoa_state.set_beam_confidence(beam.confidence);
            // Store elevation for spatial filtering (Phase 3c)
            tdoa_state
                .beam_elevation_rad
                .store(beam.elevation.to_radians(), Ordering::Relaxed);
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    });

    // ── Joy-Con Gesture Control (Track B Addendum BB) ─────────────────────────
    // Spawn Joy-Con polling task (60 Hz) with graceful disconnect handling
    let joycon_state = state.clone();
    let joycon_mapping = crate::input::GestureMapping::default();
    let _joycon_handle = crate::input::spawn_joycon_task(joycon_state, joycon_mapping);
    eprintln!("[JoyCon] Task spawned - waiting for controller connection");

    // ── Dispatch loop ─────────────────────────────────────────────────────────
    let state_disp = state.clone();
    let vbuf_disp = vbuffer.clone();
    let sdr_vbuf_disp = sdr_vbuffer.clone();
    let mamba_trainer_disp = mamba_trainer.clone();
    // Task 1: Setup ModularFeatureEncoder with Burn backend
    // For real-time inference in this demo, we can just instantiate it.
    // (A full background training loop with Burn requires optimizer config.)
    let training_session_disp = training_session.clone();
    let qdrant_disp = qdrant.clone();
    let neo4j_disp = neo4j.clone();
    let forensic_disp = forensic.clone();
    let gpu_shared_disp = gpu_shared.clone();
    let session_identity_clone = session_identity.clone();
    let feature_tx = feature_tx.clone();
    let impulse_tx = impulse_tx.clone();

    tokio::spawn(async move {
        let signal_dispatch = crate::dispatch::SignalDispatchLoop::new(
            state_disp,
            gpu_shared_disp,
            merge_rx,
            sdr_rx,
            record_rx,
            feature_tx,
            waterfall,
            sdr_waterfall,
            pdm,
            bispec,
            gpu_ctx,
            crate::fusion::FusionEngine::new(),
            crate::reconstruct::CrystalBall::new(sample_rate, 24_576_000.0).into(),
            qdrant_disp,
            neo4j_disp,
            vbuf_disp,
            sdr_vbuf_disp,
            mamba_trainer_disp,
            training_session_disp,
            forensic_disp,
        );
        if let Err(e) = signal_dispatch.run().await {
            eprintln!("[SignalDispatch] Loop failed: {}", e);
        }
    });

    wire_ui_callbacks(&ui, &state, ui.as_weak());

    // 60 Hz UI refresh timer
    {
        let ui_weak = ui.as_weak();
        let state_timer = state.clone();
        let training_session_timer = training_session.clone();
        let mut loss_ring: std::collections::VecDeque<f32> =
            std::collections::VecDeque::with_capacity(64);
        let ui_rx_timer = ui_rx.clone();

        let timer = slint::Timer::default();
        timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(2),
            move || {
                let Some(ui) = ui_weak.upgrade() else { return };
                let st = state_timer.as_ref();

                ui.set_detected_freq(st.get_detected_freq());
                ui.set_note_name(st.get_note_name().into());
                ui.set_note_cents(st.get_note_cents());
                if st.auto_tune.load(Ordering::Relaxed) && !st.get_sdr_sweeping() {
                    st.set_sdr_center_hz(st.get_detected_freq());
                }
                ui.set_is_running(st.running.load(Ordering::Relaxed));
                ui.set_current_mode(st.mode.load(Ordering::Relaxed) as i32);
                ui.set_auto_tune_active(st.auto_tune.load(Ordering::Relaxed));
                ui.set_master_gain(st.get_master_gain());
                ui.set_pdm_active(st.pdm_active.load(Ordering::Relaxed));
                ui.set_pdm_clock_mhz(st.pdm_clock_mhz.load(Ordering::Relaxed));
                ui.set_oversample_ratio(st.oversample_ratio.load(Ordering::Relaxed) as i32);
                ui.set_pdm_snr_db(st.get_snr_db());
                ui.set_waveshape_mode(st.waveshape_mode.load(Ordering::Relaxed) as i32);
                ui.set_waveshape_drive(st.get_waveshape_drive());
                ui.set_input_device_count(st.input_device_count.load(Ordering::Relaxed) as i32);
                ui.set_beam_azimuth_deg(st.get_beam_azimuth_deg());
                ui.set_beam_confidence(st.get_beam_confidence());
                ui.set_beam_focus_deg(st.get_beam_focus_deg());
                ui.set_agc_gain_db(st.get_agc_gain_db());
                ui.set_agc_peak_dbfs(st.get_agc_peak_dbfs());
                ui.set_output_peak_db(st.get_output_peak_db());

                // Consume telemetry events from async tasks (non-blocking)
                while let Ok(event) = ui_rx_timer.try_recv() {
                    match event {
                        crate::state::UiEvent::TrainingProgress {
                            iteration,
                            total_iterations: _,
                            loss,
                            loss_min: _,
                        } => {
                            // Wire telemetry to UI: training progress display
                            ui.set_training_epoch(iteration as i32);
                            ui.set_training_loss(loss);
                            st.train_epoch.store(iteration, Ordering::Relaxed);
                            st.train_loss.store(loss, Ordering::Relaxed);
                        }
                        crate::state::UiEvent::ClusteringStatus { .. } => {
                            // Clustering events: reserved for Phase 2C visualization
                        }
                        _ => {
                            // Other event types: SDR status, reconstruction, analysis
                        }
                    }
                }

                let active_tab = ui.get_active_tab();
                let (wf_guard, max_hz) = if active_tab == 1 {
                    let sr = st.sdr_sample_rate.load(Ordering::Relaxed) as f32;
                    (st.sdr_waterfall_rgba.lock(), sr / 2.0)
                } else {
                    let pdm_clock = st.pdm_clock_mhz.load(Ordering::Relaxed) * 1e6;
                    let max = if st.pdm_active.load(Ordering::Relaxed) {
                        pdm_clock / 2.0
                    } else {
                        96_000.0
                    };
                    (st.waterfall_rgba.lock(), max)
                };
                if let Ok(wf) = wf_guard {
                    ui.set_waterfall_max_freq(
                        (if max_hz >= 1e6 {
                            format!("{:.3} MHz", max_hz / 1e6)
                        } else {
                            format!("{:.1} kHz", max_hz / 1e3)
                        })
                        .into(),
                    );
                    ui.set_waterfall_mid_freq(
                        (if max_hz / 2.0 >= 1e6 {
                            format!("{:.3} MHz", max_hz / 2e6)
                        } else {
                            format!("{:.1} kHz", max_hz / 2e3)
                        })
                        .into(),
                    );
                    let mut px = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(
                        state::WATERFALL_DISPLAY_COLS as u32,
                        state::WATERFALL_DISPLAY_ROWS as u32,
                    );
                    let dst = px.make_mut_slice();
                    let sz = state::WATERFALL_DISPLAY_COLS * state::WATERFALL_DISPLAY_ROWS;
                    if wf.len() >= sz {
                        for i in 0..sz {
                            let s = wf[i];
                            dst[i].r = (s & 0xFF) as u8;
                            dst[i].g = ((s >> 8) & 0xFF) as u8;
                            dst[i].b = ((s >> 16) & 0xFF) as u8;
                            dst[i].a = 255;
                        }
                    }
                    ui.set_waterfall_image(slint::Image::from_rgba8(px));
                }

                if let Ok(frames) = st.output_frames.lock() {
                    if !frames.is_empty() {
                        // Output is interleaved n_channels samples; ch0 only for display.
                        let stride = (n_channels as usize).max(1);
                        let ch0: Vec<f32> = frames.iter().step_by(stride).cloned().collect();
                        let mut path = String::from("M 0 50");
                        let pts = 100usize;
                        let step = (ch0.len() / pts).max(1);
                        for i in 0..pts {
                            let base = i * step;
                            let slc = &ch0[base..(base + step).min(ch0.len())];
                            if slc.is_empty() {
                                break;
                            }
                            let avg = slc.iter().sum::<f32>() / slc.len() as f32;
                            let x = (i as f32 / pts as f32) * 100.0;
                            let y = 50.0 - avg.clamp(-1.0, 1.0) * 45.0;
                            path.push_str(&format!(" L {:.1} {:.1}", x, y));
                        }
                        ui.set_waveform_path(path.into());
                    }
                }

                let sb_guard = if active_tab == 1 {
                    st.sdr_spectrum_bars.lock()
                } else {
                    st.spectrum_bars.lock()
                };
                if let Ok(sb) = sb_guard {
                    let build = |r: std::ops::Range<usize>| {
                        let mut p = String::from("M 0 100");
                        for i in r {
                            if let Some(&m) = sb.get(i) {
                                let x = (i as f32 / 256.0) * 1000.0;
                                let y = 100.0 - (m * 100.0).clamp(0.0, 99.0);
                                p.push_str(&format!(" L {:.1} {:.1}", x, y));
                            }
                        }
                        p.push_str(" L 1000 100 L 0 100 Z");
                        p
                    };
                    ui.set_spectrum_path_green(build(0..85).into());
                    ui.set_spectrum_path_cyan(build(85..170).into());
                    ui.set_spectrum_path_red(build(170..256).into());
                }

                if let Ok(sm) = st.sdr_mags.try_lock() {
                    let mut p = String::from("M 0 100");
                    let len = sm.len();
                    if len > 0 {
                        let step = (len / 1000).max(1);
                        for (i, &m) in sm.iter().enumerate().step_by(step) {
                            let x = (i as f32 / len as f32) * 1000.0;
                            let y = 100.0 - (m * 200.0).clamp(0.0, 99.0);
                            p.push_str(&format!(" L {:.1} {:.1}", x, y));
                        }
                    } else {
                        p.push_str(" L 0 100 L 1000 100");
                    }
                    p.push_str(" L 1000 100 L 0 100 Z");
                    ui.set_rtl_spectrum_path(p.into());
                }

                if let Ok(tx) = st.tx_mags.lock() {
                    let mut p = String::from("M 0 100");
                    let len = tx.len().min(256);
                    if len > 0 {
                        for (i, &m) in tx.iter().take(len).enumerate() {
                            let x = (i as f32 / len as f32) * 1000.0;
                            let y = 100.0
                                - ((m.max(1e-8).log10() + 8.0).clamp(0.0, 8.0) * 12.0)
                                    .clamp(0.0, 99.0);
                            p.push_str(&format!(" L {:.1} {:.1}", x, y));
                        }
                    } else {
                        p.push_str(" L 0 100 L 1000 100");
                    }
                    p.push_str(" L 1000 100 L 0 100 Z");
                    ui.set_tx_spectrum_path(p.into());
                }

                if let Ok(rm) = st.reconstruction_mags.lock() {
                    let mut p = String::from("M 0 100");
                    let len = rm.len().min(256);
                    if len > 0 {
                        for (i, &m) in rm.iter().take(len).enumerate() {
                            let x = (i as f32 / len as f32) * 1000.0;
                            let y = 100.0 - (m * 200.0).clamp(0.0, 99.0);
                            p.push_str(&format!(" L {:.1} {:.1}", x, y));
                        }
                    } else {
                        p.push_str(" L 0 100 L 1000 100");
                    }
                    p.push_str(" L 1000 100 L 0 100 Z");
                    ui.set_reconstruction_path(p.into());
                }

                // ── Sync Memos ──
                let memos = st.memo_get_all();
                let mut ts = Vec::new();
                let mut tags = Vec::new();
                let mut contents = Vec::new();
                for m in memos {
                    ts.push(m.timestamp_iso8601.into());
                    tags.push(m.tag.into());
                    contents.push(m.content.into());
                }
                ui.set_memo_timestamps(slint::ModelRc::from(std::rc::Rc::new(
                    slint::VecModel::from(ts),
                )));
                ui.set_memo_tags(slint::ModelRc::from(std::rc::Rc::new(
                    slint::VecModel::from(tags),
                )));
                ui.set_memo_contents(slint::ModelRc::from(std::rc::Rc::new(
                    slint::VecModel::from(contents),
                )));

                // ── Sync Recording State ──
                if let Ok(rs) = st.recording_state.lock() {
                    ui.set_manual_rec_active(rs.is_recording());
                    ui.set_manual_rec_countdown_ms(rs.get_remaining_ms() as i32);
                    ui.set_manual_rec_status(
                        match rs.state {
                            crate::state::RecordingStateEnum::Idle => "IDLE",
                            crate::state::RecordingStateEnum::Recording => "RECORDING",
                            crate::state::RecordingStateEnum::Saving => "SAVING",
                        }
                        .into(),
                    );
                }

                ui.set_mamba_anomaly(st.get_mamba_anomaly());
                ui.set_latent_embedding(slint::ModelRc::from(std::rc::Rc::new(
                    slint::VecModel::from(st.get_latent_embedding()),
                )));
                ui.set_training_active(st.training_active.load(Ordering::Relaxed));
                ui.set_training_epoch(st.train_epoch.load(Ordering::Relaxed) as i32);
                ui.set_training_loss(st.get_train_loss());
                ui.set_replay_buf_len(st.replay_buf_len.load(Ordering::Relaxed) as i32);
                ui.set_dispatch_ms(st.get_dispatch_us() as f32 / 1000.0);
                ui.set_frame_count(st.get_frame_count() as i32);

                let loss = st.get_train_loss();
                if st.training_active.load(Ordering::Relaxed) && loss > 0.0 {
                    loss_ring.push_back(loss);
                    if loss_ring.len() > 64 {
                        loss_ring.pop_front();
                    }
                }
                let lmax = loss_ring.iter().cloned().fold(1e-6f32, f32::max);
                let lnorm: Vec<f32> = loss_ring.iter().map(|&v| v / lmax).collect();
                ui.set_loss_history(slint::ModelRc::from(std::rc::Rc::new(
                    slint::VecModel::from(lnorm),
                )));

                ui.set_rtl_connected(st.rtl_connected.load(Ordering::Relaxed));
                ui.set_sdr_active(st.sdr_active.load(Ordering::Relaxed));
                ui.set_sdr_center_mhz(st.get_sdr_center_hz() / 1e6);
                ui.set_sdr_gain_db(st.get_sdr_gain_db());
                ui.set_sdr_peak_dbfs(st.get_sdr_peak_dbfs());
                ui.set_sdr_peak_offset_khz(st.get_sdr_peak_offset_hz() / 1e3);

                // ── Parameter Control Layer Sync (Track B Addendum BB) ───────────
                // Audio Device
                ui.set_audio_device_idx(st.get_audio_device_idx() as i32);
                let audio_devices = st.get_audio_devices();
                let device_names: Vec<slint::SharedString> = audio_devices
                    .iter()
                    .map(|d| d.name.clone().into())
                    .collect();
                ui.set_audio_device_names(slint::ModelRc::from(std::rc::Rc::new(
                    slint::VecModel::from(device_names),
                )));
                ui.set_audio_gain_db(st.get_master_gain() * 120.0);

                // Camera
                ui.set_camera_resolution(st.get_camera_resolution() as i32);
                ui.set_camera_fps(st.get_camera_fps());
                ui.set_camera_active(st.get_camera_active());

                // Frequency
                ui.set_freq_band_index(st.get_freq_band_index() as i32);
                ui.set_freq_manual_mhz(st.get_freq_manual_hz() / 1e6);
                ui.set_freq_actual_mhz(st.get_freq_actual_hz() / 1e6);

                // Joy-Con State
                ui.set_joycon_connected(st.get_joycon_connected());
                ui.set_joycon_active(st.get_joycon_active());
                ui.set_joycon_gyro_roll(st.get_joycon_gyro_roll());
                ui.set_joycon_gyro_pitch(st.get_joycon_gyro_pitch());
                ui.set_joycon_gyro_yaw(st.get_joycon_gyro_yaw());
                ui.set_joycon_accel_x(st.get_joycon_accel_x());
                ui.set_joycon_accel_y(st.get_joycon_accel_y());
                ui.set_joycon_accel_z(st.get_joycon_accel_z());
                ui.set_joycon_stick_left_x(st.get_joycon_stick_left_x());
                ui.set_joycon_stick_left_y(st.get_joycon_stick_left_y());
                ui.set_joycon_stick_right_x(st.get_joycon_stick_right_x());
                ui.set_joycon_stick_right_y(st.get_joycon_stick_right_y());
                ui.set_joycon_trigger_l(st.get_joycon_trigger_l());
                ui.set_joycon_trigger_r(st.get_joycon_trigger_r());
                ui.set_joycon_button_a(st.get_joycon_button_a());
                ui.set_joycon_button_b(st.get_joycon_button_b());
                ui.set_joycon_button_x(st.get_joycon_button_x());
                ui.set_joycon_button_y(st.get_joycon_button_y());

                // Training tab sync
                ui.set_rtl_freq_mhz(st.get_sdr_center_hz() / 1e6);
                ui.set_rtl_scanning(st.get_sdr_sweeping());
                ui.set_training_pairs(training_session_timer.total_pairs() as i32);

                if let Ok(gs) = st.gate_status.try_lock() {
                    ui.set_gate_status(gs.clone().into());
                }
                if let Ok(gr) = st.last_gate_reason.try_lock() {
                    ui.set_last_gate_reason(gr.clone().into());
                }
                ui.set_training_pairs_dropped(
                    st.training_pairs_dropped.load(Ordering::Relaxed) as i32
                );
                ui.set_gate_rejections_low_anomaly(
                    st.gate_rejections_low_anomaly.load(Ordering::Relaxed) as i32,
                );
                ui.set_gate_rejections_low_confidence(
                    st.gate_rejections_low_confidence.load(Ordering::Relaxed) as i32,
                );

                // Mamba safety sync
                ui.set_mamba_emergency_off(st.get_mamba_emergency_off());
                ui.set_smart_anc_blend(st.get_smart_anc_blend());

                let rf_bias = st.get_sdr_dc_bias();
                let audio_bias = st.get_audio_dc_bias();
                ui.set_sdr_dc_bias(rf_bias);
                ui.set_audio_dc_bias(audio_bias);
                ui.set_dc_warning_active(rf_bias > 0.8 || audio_bias > 0.1);

                // Console sync (only when visible for performance)
                if active_tab == 4 {
                    let logs = st.get_logs_all();
                    let mut ts = Vec::with_capacity(logs.len());
                    let mut lvls = Vec::with_capacity(logs.len());
                    let mut mods = Vec::with_capacity(logs.len());
                    let mut msgs = Vec::with_capacity(logs.len());
                    for l in logs {
                        ts.push(slint::SharedString::from(l.timestamp));
                        lvls.push(slint::SharedString::from(l.level));
                        mods.push(slint::SharedString::from(l.module));
                        msgs.push(slint::SharedString::from(l.message));
                    }
                    ui.set_log_timestamps(slint::ModelRc::from(std::rc::Rc::new(
                        slint::VecModel::from(ts),
                    )));
                    ui.set_log_levels(slint::ModelRc::from(std::rc::Rc::new(
                        slint::VecModel::from(lvls),
                    )));
                    ui.set_log_modules(slint::ModelRc::from(std::rc::Rc::new(
                        slint::VecModel::from(mods),
                    )));
                    ui.set_log_messages(slint::ModelRc::from(std::rc::Rc::new(
                        slint::VecModel::from(msgs),
                    )));
                }
            },
        );
        std::mem::forget(timer);
    }

    // Wire "Export Evidence" UI button
    {
        let f = forensic.clone();
        let s = state.clone();
        ui.on_export_evidence(move || {
            let f = f.clone();
            let s = s.clone();
            tokio::spawn(async move {
                std::fs::create_dir_all("evidence").ok();
                let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                let path = format!("evidence/report_{ts}.html");
                let case = format!("TWISTER_{}", s.train_epoch.load(Ordering::Relaxed));
                match f.export_evidence_report(&path, &case, "Operator", "Galveston TX", None, None)
                {
                    Ok(_) => println!("[Forensic] Exported: {}", path),
                    Err(e) => eprintln!("[Forensic] Export failed: {e}"),
                }
            });
        });
    }

    // ── Initialize Particle System (Addendum AA) ────────────────────────────
    let particle_renderer = crate::particle_system::renderer::ParticleRenderer::new(
        gpu_shared.clone(),
        10_000_000,
        wgpu::TextureFormat::Rgba8Unorm,
    );
    let frustum_culler =
        crate::particle_system::frustum_culler::FrustumCuller::new(gpu_shared.clone(), 10_000_000);
    // let particle_streamer =
    //     std::sync::Arc::new(crate::particle_system::streaming::ParticleStreamLoader::new());

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    // let _ = tokio::spawn({
    //     let s = particle_streamer.clone();
    //     async move {
    //         s.load_window(now_ms - 8_380_800_000, now_ms, 1_000_000)
    //             .await;
    //     }
    // });

    state
        .running
        .store(true, std::sync::atomic::Ordering::Relaxed);
    ui.run().context("Slint run failed")?;

    // ── Clean shutdown ────────────────────────────────────────────────────────
    println!("[Twister] UI closed. Saving checkpoint and exporting evidence...");

    // Final Mamba checkpoint (FIX #1: Persist training state with metadata)
    {
        let ckpt = state
            .checkpoint_path
            .lock()
            .map(|p| p.clone())
            .unwrap_or_else(|_| "weights/mamba_siren.safetensors".to_string());

        // Capture current training progress
        let epoch = state.train_epoch.load(Ordering::Relaxed);
        let loss_avg = state.train_loss.load(Ordering::Relaxed);
        let metadata = crate::state::CheckpointMetadata::new(
            epoch, loss_avg,
            loss_avg, // loss_min = current (will be improved on next training session)
            loss_avg, // loss_max = current
        );

        match mamba_trainer.save(&ckpt, Some(metadata)).await {
            Ok(_) => println!(
                "[Mamba] Final checkpoint: {} (epoch {}, loss {:.6})",
                ckpt, epoch, loss_avg
            ),
            Err(e) => eprintln!("[Mamba] Final save failed: {e}"),
        }
    }

    // Final evidence report
    {
        std::fs::create_dir_all("evidence").ok();
        let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let path = format!("evidence/final_{ts}.html");

        match forensic.export_evidence_report(
            &path,
            "TWISTER_FINAL",
            "Operator",
            "Galveston TX",
            None,
            None,
        ) {
            Ok(_) => println!("[Forensic] Final report: {}", path),
            Err(e) => eprintln!("[Forensic] Final export failed: {e}"),
        }

        let _ = forensic.shutdown().await;
    }

    println!("[Twister] Shutdown complete.");
    Ok(())
}

fn wire_ui_callbacks(ui: &AppWindow, state: &Arc<AppState>, ui_weak: slint::Weak<AppWindow>) {
    let s = state.clone();
    ui.on_set_mode(move |m| {
        s.mode.store(m as u32, Ordering::Relaxed);
    });
    let s = state.clone();
    ui.on_set_gain(move |g| {
        s.set_master_gain(g);
    });
    let s = state.clone();
    ui.on_set_freq_override(move |f| {
        s.set_denial_freq_override(f);
    });
    let s = state.clone();
    ui.on_toggle_auto_tune(move || {
        let p = s.auto_tune.load(Ordering::Relaxed);
        s.auto_tune.store(!p, Ordering::Relaxed);
    });
    let s = state.clone();
    ui.on_toggle_running(move || {
        let p = s.running.load(Ordering::Relaxed);
        s.running.store(!p, Ordering::Relaxed);
    });
    let s = state.clone();
    ui.on_toggle_pdm(move || {
        let p = s.pdm_active.load(Ordering::Relaxed);
        s.pdm_active.store(!p, Ordering::Relaxed);
    });
    let s = state.clone();
    ui.on_set_waveshape(move |m| {
        s.waveshape_mode.store(m as u32, Ordering::Relaxed);
    });
    let s = state.clone();
    ui.on_set_waveshape_drive(move |d| {
        s.set_waveshape_drive(d);
    });
    let s = state.clone();
    ui.on_set_beam_focus(move |f| {
        s.set_beam_focus_deg(f);
    });

    // ANC calibration
    let s = state.clone();
    let ui_h = ui.as_weak();
    ui.on_anc_calibrate(move || {
        let s = s.clone();
        let ui = ui_h.clone();
        tokio::spawn(async move {
            println!("[ANC] Starting full-range calibration (1 Hz – 12.288 MHz)");
            if let Ok(mut r) = s.anc_recording.lock() {
                r.start_recording();
            }
            println!("[ANC] Waiting 20 seconds for multi-channel recording...");
            for i in 0..20 {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                if i % 5 == 0 {
                    let p = s
                        .anc_recording
                        .lock()
                        .map(|r| r.progress() * 100.0)
                        .unwrap_or(0.0);
                    println!("[ANC] {:.1}% ({}s)", p, i);
                }
            }
            let (c0, c1, c2) = if let Ok(mut r) = s.anc_recording.lock() {
                let m = r.finalize();
                (
                    m.get(&0).cloned().unwrap_or_default(),
                    m.get(&1).cloned().unwrap_or_default(),
                    m.get(&2).cloned().unwrap_or_default(),
                )
            } else {
                return;
            };

            let mut cal = crate::anc_calibration::FullRangeCalibration::new();
            cal.calibrate_from_sweep(&c0, &c1, &c2, |bin| {
                let a = s.mamba_anomaly_score.load(Ordering::Relaxed);
                a * (0.5 + 0.5 * (bin as f32 / 8192.0).min(1.0))
            });
            if let Ok(mut ae) = s.anc_engine.lock() {
                let sw = crate::anc::AncEngine::calibration_sweep(192000.0);
                ae.calibrate(&sw, &c0);
                ae.lms.initialize_from_calibration(
                    &|b| cal.phase_for((b as f32 / 8192.0) * 6_144_000.0),
                    &|b| cal.confidence_for((b as f32 / 8192.0) * 6_144_000.0),
                );
            }
            s.anc_calibrated.store(true, Ordering::Relaxed);
            if let Some(u) = ui.upgrade() {
                u.set_anc_status("ANC calibrated: 1 Hz – 12.288 MHz".into());
            }
            println!("[ANC] COMPLETE — ANC Defense ARMED");
        });
    });

    let s = state.clone();
    ui.on_mamba_toggle_emergency(move || {
        let p = s.mamba_emergency_off.load(Ordering::Relaxed);
        s.mamba_emergency_off.store(!p, Ordering::Relaxed);
        println!("[Mamba] EMERGENCY OFF: {}", !p);
    });

    let s = state.clone();
    ui.on_smart_anc_set_blend(move |v| {
        s.set_smart_anc_blend(v);
        println!("[Mamba] Smart ANC Blend: {:.1}%", v * 100.0);
    });

    let s = state.clone();
    ui.on_training_start(move || {
        s.training_active.store(true, Ordering::Relaxed);
    });
    let s = state.clone();
    ui.on_training_stop(move || {
        s.training_active.store(false, Ordering::Relaxed);
    });

    let s = state.clone();
    ui.on_rtl_connect(move || {
        let p = s.sdr_active.load(Ordering::Relaxed);
        s.set_sdr_active(!p);
        println!("[RTL] {}", if !p { "ACTIVE" } else { "OFF" });
    });
    let s = state.clone();
    ui.on_rtl_start_scan(move || {
        s.set_sdr_sweeping(true);
        s.set_sdr_active(true);
        println!("[SDR SWEEP] START 10kHz→300MHz");
    });
    let s = state.clone();
    ui.on_rtl_stop_scan(move || {
        s.set_sdr_sweeping(false);
        println!("[SDR SWEEP] STOP");
    });
    let s = state.clone();
    ui.on_set_sdr_center(move |mhz| {
        s.set_sdr_center_hz(mhz * 1e6);
    });
    let s = state.clone();
    ui.on_set_sdr_gain(move |db| {
        s.set_sdr_gain_db(db);
    });
    let s = state.clone();
    ui.on_tune_to_harmonic(move |n| {
        let hz = n as f32 * 192_000.0;
        s.set_sdr_center_hz(hz);
        println!(
            "[SDR] Tuned to harmonic {} × 192 kHz = {:.3} MHz",
            n,
            hz / 1e6
        );
    });

    // Manual Recording
    let s = state.clone();
    ui.on_manual_rec_start(move || {
        s.manual_rec_start();
    });
    let s = state.clone();
    ui.on_manual_rec_stop(move || {
        s.manual_rec_stop();
    });
    let s = state.clone();
    let ui_rec = ui_weak.clone();
    ui.on_manual_rec_save(move || {
        if let Some(ui) = ui_rec.upgrade() {
            let notes = ui.get_manual_rec_notes().to_string();
            s.manual_rec_save(notes);
        }
    });

    // Memos
    let s = state.clone();
    ui.on_memo_add(move |tag, content| {
        s.memo_add(tag.to_string(), content.to_string());
    });
    let s = state.clone();
    ui.on_memo_delete(move |idx| {
        s.memo_delete(idx as usize);
    });
    let s = state.clone();
    ui.on_memo_export(move || {
        let forensic_dir = "forensic";
        let _ = std::fs::create_dir_all(forensic_dir);
        let path = format!(
            "{}/memo_export_{}.csv",
            forensic_dir,
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        );
        if let Ok(mut wtr) = csv::Writer::from_path(&path) {
            let memos = s.memo_get_all();
            let _ = wtr.write_record(&["Timestamp", "Tag", "Content", "Azimuth", "Elevation"]);
            for m in memos {
                let az = m
                    .mamba_control
                    .as_ref()
                    .map(|c| c.beam_azimuth)
                    .unwrap_or(0.0);
                let el = m
                    .mamba_control
                    .as_ref()
                    .map(|c| c.beam_elevation)
                    .unwrap_or(0.0);
                let _ = wtr.write_record(&[
                    &m.timestamp_iso8601,
                    &m.tag,
                    &m.content,
                    &az.to_string(),
                    &el.to_string(),
                ]);
            }
        }
        println!("[Forensic] Memos exported to {}", path);
    });

    // ── Parameter Control Layer (Track B Addendum BB) ──────────────────────────

    // Parameter Persistence
    let s = state.clone();
    ui.on_save_parameters(move || {
        let params = crate::parameters::TwisterParameters {
            audio_device_idx: s.get_audio_device_idx(),
            audio_devices: s
                .get_audio_devices()
                .iter()
                .map(|d| crate::parameters::audio_device_to_config(d))
                .collect(),
            master_gain_db: s.get_master_gain() * 120.0, // Convert 0-1 to dB
            camera_resolution: s.get_camera_resolution(),
            camera_fps: s.get_camera_fps(),
            camera_active: s.get_camera_active(),
            freq_band_index: s.get_freq_band_index(),
            freq_manual_hz: s.get_freq_manual_hz(),
            pdm_active: s.pdm_active.load(Ordering::Relaxed),
            pdm_clock_mhz: s.get_pdm_clock_mhz(),
            oversample_ratio: s.oversample_ratio.load(Ordering::Relaxed),
            waveshape_mode: s.waveshape_mode.load(Ordering::Relaxed),
            waveshape_drive: s.get_waveshape_drive(),
            beam_azimuth_deg: s.get_beam_azimuth_deg(),
            beam_elevation_deg: s.beam_elevation_rad.load(Ordering::Relaxed).to_degrees(),
            beam_focus_deg: s.get_beam_focus_deg(),
            smart_anc_blend: s.get_smart_anc_blend(),
            anc_calibrated: s.anc_calibrated.load(Ordering::Relaxed),
            sdr_center_hz: s.get_sdr_center_hz(),
            sdr_gain_db: s.get_sdr_gain_db(),
            sdr_active: s.get_sdr_active(),
            joycon_enabled: s.get_joycon_connected(),
            joycon_mapping: crate::parameters::JoyConGestureMapping::default(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            last_modified: chrono::Utc::now().to_rfc3339(),
        };

        match params.save() {
            Ok(_) => println!("[Parameters] Saved to ~/.twister/parameters.json"),
            Err(e) => eprintln!("[Parameters] Save failed: {}", e),
        }
    });

    let s = state.clone();
    ui.on_load_parameters(move || match crate::parameters::TwisterParameters::load() {
        Ok(params) => {
            s.set_audio_device_idx(params.audio_device_idx);
            s.set_audio_devices(
                params
                    .audio_devices
                    .iter()
                    .map(|d| crate::parameters::config_to_audio_device(d))
                    .collect(),
            );
            s.set_master_gain((params.master_gain_db / 120.0).clamp(0.0, 1.0));
            s.set_camera_resolution(params.camera_resolution);
            s.set_camera_fps(params.camera_fps);
            s.set_camera_active(params.camera_active);
            s.set_freq_band_index(params.freq_band_index);
            s.set_freq_manual_hz(params.freq_manual_hz);
            s.pdm_active.store(params.pdm_active, Ordering::Relaxed);
            s.set_pdm_clock_mhz(params.pdm_clock_mhz);
            s.oversample_ratio
                .store(params.oversample_ratio, Ordering::Relaxed);
            s.waveshape_mode
                .store(params.waveshape_mode, Ordering::Relaxed);
            s.set_waveshape_drive(params.waveshape_drive);
            s.set_beam_azimuth_deg(params.beam_azimuth_deg);
            s.beam_elevation_rad
                .store(params.beam_elevation_deg.to_radians(), Ordering::Relaxed);
            s.set_beam_focus_deg(params.beam_focus_deg);
            s.set_smart_anc_blend(params.smart_anc_blend);
            s.anc_calibrated
                .store(params.anc_calibrated, Ordering::Relaxed);
            s.set_sdr_center_hz(params.sdr_center_hz);
            s.set_sdr_gain_db(params.sdr_gain_db);
            s.set_sdr_active(params.sdr_active);
            println!("[Parameters] Loaded from ~/.twister/parameters.json");
        }
        Err(e) => eprintln!("[Parameters] Load failed: {}", e),
    });

    let s = state.clone();
    ui.on_reset_parameters(move || {
        let params = crate::parameters::TwisterParameters::default();
        s.set_audio_device_idx(params.audio_device_idx);
        s.set_camera_resolution(params.camera_resolution);
        s.set_camera_fps(params.camera_fps);
        s.set_camera_active(params.camera_active);
        s.set_freq_band_index(params.freq_band_index);
        s.set_freq_manual_hz(params.freq_manual_hz);
        s.pdm_active.store(params.pdm_active, Ordering::Relaxed);
        s.set_pdm_clock_mhz(params.pdm_clock_mhz);
        s.oversample_ratio
            .store(params.oversample_ratio, Ordering::Relaxed);
        s.waveshape_mode
            .store(params.waveshape_mode, Ordering::Relaxed);
        s.set_waveshape_drive(params.waveshape_drive);
        s.set_beam_azimuth_deg(params.beam_azimuth_deg);
        s.beam_elevation_rad
            .store(params.beam_elevation_deg.to_radians(), Ordering::Relaxed);
        s.set_beam_focus_deg(params.beam_focus_deg);
        s.set_smart_anc_blend(params.smart_anc_blend);
        s.anc_calibrated
            .store(params.anc_calibrated, Ordering::Relaxed);
        s.set_sdr_center_hz(params.sdr_center_hz);
        s.set_sdr_gain_db(params.sdr_gain_db);
        s.set_sdr_active(params.sdr_active);
        println!("[Parameters] Reset to defaults");
    });

    // Audio Device Control
    let s = state.clone();
    let ui_weak_cb = ui_weak.clone();
    ui.on_audio_device_prev(move || {
        let current = s.get_audio_device_idx();
        let devices = s.get_audio_devices();
        if !devices.is_empty() {
            let new_idx = if current > 0 {
                current - 1
            } else {
                (devices.len() - 1) as u32
            };
            s.set_audio_device_idx(new_idx);
            if let Some(ui) = ui_weak_cb.upgrade() {
                ui.set_audio_device_idx(new_idx as i32);
            }
            println!("[Audio] Device: {}", new_idx);
        }
    });

    let s = state.clone();
    let ui_weak_cb = ui_weak.clone();
    ui.on_audio_device_next(move || {
        let current = s.get_audio_device_idx();
        let devices = s.get_audio_devices();
        if !devices.is_empty() {
            let new_idx = (current + 1) % devices.len() as u32;
            s.set_audio_device_idx(new_idx);
            if let Some(ui) = ui_weak_cb.upgrade() {
                ui.set_audio_device_idx(new_idx as i32);
            }
            println!("[Audio] Device: {}", new_idx);
        }
    });

    let s = state.clone();
    ui.on_set_audio_gain(move |db| {
        s.set_master_gain((db / 120.0).clamp(0.0, 1.0));
    });

    // Camera Control
    let s = state.clone();
    ui.on_set_camera_resolution(move |res| {
        s.set_camera_resolution(res as u32);
        let (w, h) = match res {
            0 => (640, 480),
            1 => (1280, 720),
            2 => (1920, 1080),
            _ => (1280, 720),
        };
        println!("[Camera] Resolution: {}x{}", w, h);
    });

    let s = state.clone();
    ui.on_toggle_camera(move || {
        let current = s.get_camera_active();
        s.set_camera_active(!current);
        println!(
            "[Camera] {}",
            if !current { "Activated" } else { "Deactivated" }
        );
    });

    // Frequency Control
    let s = state.clone();
    let ui_weak_cb = ui_weak.clone();
    ui.on_set_freq_band(move |band| {
        s.set_freq_band_index(band as u32);

        // Update actual frequency based on band
        let actual_hz = match band {
            0 => 15_000.0,               // VLF
            1 => 150_000.0,              // LF
            2 => 1_500_000.0,            // MF
            3 => 15_000_000.0,           // HF
            4 => 150_000_000.0,          // VHF
            5 => 1_500_000_000.0,        // UHF
            6 => s.get_freq_manual_hz(), // Manual
            _ => 150_000_000.0,
        };
        s.set_freq_actual_hz(actual_hz);

        if let Some(ui) = ui_weak_cb.upgrade() {
            ui.set_freq_actual_mhz(actual_hz / 1_000_000.0);
        }

        let band_name = match band {
            0 => "VLF (3-30 kHz)",
            1 => "LF (30-300 kHz)",
            2 => "MF (300k-3 MHz)",
            3 => "HF (3-30 MHz)",
            4 => "VHF (30-300 MHz)",
            5 => "UHF (300M-3 GHz)",
            6 => "Manual",
            _ => "Unknown",
        };
        println!("[Frequency] Band: {}", band_name);
    });

    let s = state.clone();
    let ui_weak_cb = ui_weak.clone();
    ui.on_set_freq_manual(move |mhz| {
        s.set_freq_manual_hz(mhz * 1_000_000.0);
        if s.get_freq_band_index() == 6 {
            s.set_freq_actual_hz(mhz * 1_000_000.0);
            if let Some(ui) = ui_weak_cb.upgrade() {
                ui.set_freq_actual_mhz(mhz);
            }
        }
        println!("[Frequency] Manual: {:.3} MHz", mhz);
    });

    // Joy-Con Control
    let s = state.clone();
    ui.on_toggle_joycon(move || {
        let current = s.get_joycon_active();
        s.set_joycon_active(!current);
        println!(
            "[JoyCon] Gesture control {}",
            if !current { "enabled" } else { "disabled" }
        );
    });
}

// rt_store_async moved to forensic.rs

fn snr_db(original: &[f32], decoded: &[f32]) -> f32 {
    let sp: f32 = original.iter().map(|s| s * s).sum::<f32>() / original.len() as f32;
    let ep: f32 = original
        .iter()
        .zip(decoded.iter())
        .map(|(o, d)| (o - d).powi(2))
        .sum::<f32>()
        / original.len() as f32;
    if ep < 1e-12 {
        return 120.0;
    }
    10.0 * (sp / ep).log10()
}

// Add trainer loop
fn _start_impulse_trainer_loop(
    state: std::sync::Arc<std::sync::Mutex<crate::state::AppState>>,
    impulse_rx: crossbeam_channel::Receiver<crate::ml::modular_features::ImpulseTrainEvent>,
) {
    tokio::spawn(async move {
        let impulse_model = crate::ml::modular_features::ImpulsePatternModel::new();
        loop {
            if let Ok(impulse_event) = impulse_rx.recv() {
                let pattern = impulse_model.extract_pattern(&impulse_event);
                let anomaly_score = impulse_model.score_anomaly(&pattern);

                let st = state.lock().unwrap();
                st.impulse_anomaly_score
                    .store(anomaly_score, std::sync::atomic::Ordering::Relaxed);

                if anomaly_score > 0.7 {
                    st.harassment_detected
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
            } else {
                break;
            }
        }
    });
}

fn _start_trainer_loop(
    state: std::sync::Arc<std::sync::Mutex<crate::state::AppState>>,
    feature_rx: crossbeam_channel::Receiver<(
        crate::ml::modular_features::SignalFeaturePayload,
        burn::tensor::Tensor<burn::backend::ndarray::NdArray<f32>, 1>,
    )>,
    mut mamba_trainer: crate::ml::point_mamba_trainer::PointMambaTrainer,
) {
    tokio::spawn(async move {
        let mut batch = Vec::new();
        loop {
            if let Ok((payload, feature_vec)) = feature_rx.recv() {
                batch.push((payload, feature_vec));
                if batch.len() >= 16 {
                    let modular_flags = crate::ml::modular_features::FeatureFlags::default();
                    if let Ok(loss) = mamba_trainer.train_step_modular(&batch, &modular_flags) {
                        state.lock().unwrap().set_train_loss(loss);
                    }
                    batch.clear();
                }
            } else {
                break;
            }
        }
    });
}
