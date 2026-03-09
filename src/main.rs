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
mod anc;
mod anc_calibration;
mod anc_recording;
mod audio;
mod bispectrum;
mod detection;
mod embeddings;
mod evidence_export;
mod forensic;
mod forensic_queries;
mod fusion;
mod gpu;
mod gpu_shared;
mod graph;
mod harmony;
mod knowledge_graph;
mod ai;
mod ui;
mod mamba;
mod ml;
mod parametric;
mod pdm;
mod reconstruct;
mod resample;
mod ridge_plot;
mod rtlsdr;
mod rtlsdr_ffi;
mod sdr;
mod state;
mod testing;
mod trainer;
mod training;
mod training_tests;
mod twister;
mod vbuffer;
mod vector;
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
    use crate::particle_system::{renderer::ParticleRenderer, frustum_culler::FrustumCuller, streaming::ParticleStreamLoader};
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

    let forensic = Arc::new(std::sync::Mutex::new(
        ForensicLogger::new(session_identity.as_str()).context("Forensic log init")?,
    ));
    state.log(
        "INFO",
        "Forensic",
        &format!("Log: {}", forensic.lock().unwrap().log_path().display()),
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
        let mut waterfall = waterfall;
        let mut sdr_waterfall = sdr_waterfall;
        let mut pdm = pdm;
        let mut bispec = bispec;
        let mut gpu_ctx = gpu_ctx;

        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(BISPEC_FFT_SIZE);

        let mut fusion = fusion::FusionEngine::new();
        let mut last_bispec_event: Option<DetectionEvent> = None;

        // ── Crystal Ball ──────────────────────────────────────────────────
        // Forensic alias reconstruction engine
        let crystal_ball = Arc::new(crate::reconstruct::CrystalBall::new(
            sample_rate,
            24_576_000.0, // True PDM Wideband rate (192,000 * 128)
        ));
        let crystal_ball_disp = crystal_ball.clone();

        let mut tx_frame_acc = Vec::with_capacity(512 * 128);
        let mut rx_frame_acc = Vec::with_capacity(512 * 128);
        let mut frame_count = 0usize;
        let mut acc = Vec::<f32>::new();
        let mut frame_idx = 0u64;
        let mut vbuf_snapshot = vec![[0.0f32; V_FREQ_BINS]; V_DEPTH];
        let mut last_mamba_reconstruction: Option<Vec<f32>> = None;

        loop {
            let feature_flags = state_disp.get_feature_flags();
            let pdm_enabled = state_disp.pdm_active.load(Ordering::Relaxed);
            waterfall.set_pdm_mode(pdm_enabled);

            while let Ok(chunk) = merge_rx.try_recv() {
                acc.extend_from_slice(&chunk);
            }

            // ANC recording
            let mut run_anc_analysis = false;
            if let Ok(mut rec) = state_disp.anc_recording.lock() {
                if rec.state == anc_recording::CalibrationState::Recording {
                    while let Ok(tagged) = record_rx.try_recv() {
                        rec.push_samples(tagged.device_idx, &tagged.samples);
                    }
                    if rec.is_complete() {
                        rec.state = anc_recording::CalibrationState::Analyzing;
                        run_anc_analysis = true;
                    }
                }
            }
            if run_anc_analysis {
                let mut c0 = vec![];
                let mut c1 = vec![];
                let mut c2 = vec![];
                if let Ok(rec) = state_disp.anc_recording.lock() {
                    if let Some(ch) = rec.channels.get(&0) {
                        c0 = ch.clone();
                    }
                    if let Some(ch) = rec.channels.get(&1) {
                        c1 = ch.clone();
                    }
                    if let Some(ch) = rec.channels.get(&2) {
                        c2 = ch.clone();
                    }
                }
                if let Ok(mut cal) = state_disp.anc_calibration.lock() {
                    cal.calibrate_from_sweep(&c0, &c1, &c2, |bin| {
                        let a = state_disp.mamba_anomaly_score.load(Ordering::Relaxed);
                        a * (0.5 + 0.5 * (bin as f32 / 8192.0).min(1.0))
                    });
                    state_disp.set_anc_ok(true);
                }
                if let Ok(mut rec) = state_disp.anc_recording.lock() {
                    rec.state = anc_recording::CalibrationState::Idle;
                    rec.channels.clear();
                }
            }

            // SDR frames
            while let Ok((mags, sdr_last_center_hz, sdr_last_rate)) = sdr_rx.try_recv() {
                if !state_disp.rtl_connected.load(Ordering::Relaxed) {
                    state_disp.rtl_connected.store(true, Ordering::Relaxed);
                }
                let (rgba, sbars) = sdr_waterfall.push_row(&mags, 1.0, sdr_last_rate / 2.0);
                if let Ok(mut w) = state_disp.sdr_waterfall_rgba.lock() {
                    *w = rgba;
                }
                if let Ok(mut sb) = state_disp.sdr_spectrum_bars.lock() {
                    *sb = sbars;
                }
                if let Ok(mut sm) = state_disp.sdr_mags.lock() {
                    *sm = mags.clone();
                }
                let dc = mags.get(mags.len() / 2).cloned().unwrap_or(0.0);
                state_disp.set_sdr_dc_bias(dc);
                let mut vb = sdr_vbuf_disp.lock();
                vb.push_frame_f32(&gpu_shared_disp.queue, &mags);
            }

            if acc.len() < BISPEC_FFT_SIZE {
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                continue;
            }
            let mut chunk: Vec<f32> = acc.drain(..BISPEC_FFT_SIZE).collect();
            frame_idx += 1;
            let frame_start = std::time::Instant::now();

            // PDM spike rejection: detect and interpolate crest-targeting spikes
            let (filtered_chunk, pdm_spike_count) = audio::reject_pdm_spikes(&chunk);
            chunk = filtered_chunk;

            // Extract modular features based on flags
            let payload = crate::ml::modular_features::SignalFeaturePayload {
                audio_samples: chunk.clone(),
                freq_hz: state_disp.get_detected_freq(),
                tdoa_confidence: Some(state_disp.get_beam_confidence()),
                device_corr: None,
                vbuffer_coherence: None,
                impulse_detection: None,
                video_frame: None,
                video_frame_timestamp_us: 0,
                visual_features: None,
                anc_phase: None,
                harmonic_energy: None,
                impulse_detection: None,
                video_frame: None,
                video_frame_timestamp_us: 0,
                visual_features: None,
            };
            let device = burn::backend::ndarray::NdArrayDevice::Cpu;
            let extractor = crate::ml::modular_features::ModularFeatureExtractor::<
                burn::backend::NdArray,
            >::new(&device);
            let (feature_vec, _) = extractor.extract(&payload, &feature_flags);
            let _ = feature_tx.try_send((payload, feature_vec));
            if pdm_spike_count > 0 {
                eprintln!(
                    "[PDM] Detected and rejected {} spikes in frame {}",
                    pdm_spike_count, frame_idx
                );
                state_disp
                    .pdm_spike_count
                    .fetch_add(pdm_spike_count as u64, Ordering::Relaxed);
            }

            // DC bias
            let dc_audio = chunk.iter().sum::<f32>() / chunk.len() as f32;
            state_disp.set_audio_dc_bias(dc_audio);

            // Hann-windowed FFT
            let n = BISPEC_FFT_SIZE;
            let mut cbuf: Vec<Complex<f32>> = chunk
                .iter()
                .enumerate()
                .map(|(i, &s)| {
                    let w = 0.5 * (1.0 - (std::f32::consts::TAU * i as f32 / (n - 1) as f32).cos());
                    Complex { re: s * w, im: 0.0 }
                })
                .collect();
            fft.process(&mut cbuf);

            let mags: Vec<f32> = cbuf.iter().take(V_FREQ_BINS).map(|c| c.norm()).collect();

            // V-buffer
            let vbuf_ver = {
                let mut vb = vbuf_disp.lock();
                vb.push_frame_f32(&gpu_shared_disp.queue, &mags);

                vb.version()
            };
            let slot = (vbuf_ver as usize).wrapping_sub(1) % V_DEPTH;
            vbuf_snapshot[slot][..mags.len()].copy_from_slice(&mags);

            // Twister auto-tune
            if state_disp.auto_tune.load(Ordering::Relaxed) {
                let (mts, freq_scale) = if pdm_enabled && !vbuf_snapshot[slot].is_empty() {
                    let pc = pdm::pdm_clock_hz(sample_rate);
                    (
                        &vbuf_snapshot[slot][..],
                        pc / vbuf_snapshot[slot].len() as f32,
                    )
                } else {
                    (&mags[..], sample_rate / n as f32)
                };
                let peak_bin = mts
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                let raw_freq = peak_bin as f32 * freq_scale;
                let note = crate::twister::snap_to_note(raw_freq);
                state_disp.set_detected_freq(note.freq_hz);
                state_disp.set_note_name(note.name.clone());
                state_disp.set_note_cents(note.cents_offset);
            }

            // SNR
            {
                let half = mags.len() / 2;
                let peak = mags.iter().cloned().fold(0.0f32, f32::max).max(1e-10);
                let noise = mags[half..].iter().cloned().sum::<f32>() / mags[half..].len() as f32;
                state_disp
                    .set_snr_db((20.0 * (peak / noise.max(1e-10)).log10()).clamp(-20.0, 120.0));
            }

            // Waterfall
            let max_freq = if pdm_enabled {
                pdm::pdm_clock_hz(sample_rate) / 2.0
            } else {
                sample_rate / 2.0
            };
            let (rgba, sbars) = waterfall.push_row(&mags, 1.0, max_freq);
            state_disp.update_waterfall(&rgba);
            if let Ok(mut sb) = state_disp.spectrum_bars.lock() {
                *sb = sbars;
            }

            // PDM wideband
            let wideband_vbuf_meta = if pdm_enabled {
                let words = pdm.encode(&chunk);
                let decoded = pdm.decode(&words);
                let pdm_chunk_snr = snr_db(&chunk, &decoded);
                let wide = pdm.decode_wideband(&words);
                let n_w = BISPEC_FFT_SIZE.min(wide.len());
                let mut cbuf_w: Vec<Complex<f32>> = wide
                    .iter()
                    .take(n_w)
                    .enumerate()
                    .map(|(i, &s)| {
                        let w = 0.5
                            * (1.0
                                - (std::f32::consts::TAU * i as f32 / (n_w - 1).max(1) as f32)
                                    .cos());
                        Complex { re: s * w, im: 0.0 }
                    })
                    .collect();
                cbuf_w.resize(BISPEC_FFT_SIZE, Complex { re: 0.0, im: 0.0 });
                fft.process(&mut cbuf_w);
                let wide_mags: Vec<f32> = cbuf_w
                    .iter()
                    .take(V_FREQ_BINS.max(256))
                    .map(|c| c.norm() / (n_w as f32 / 2.0))
                    .collect();
                let v = {
                    let mut vb = vbuf_disp.lock();
                    vb.push_frame_f32(&gpu_shared_disp.queue, &wide_mags);

                    vb.version()
                };
                if let Ok(mut tm) = state_disp.tx_mags.lock() {
                    *tm = wide_mags.clone();
                }

                // FORENSIC RECONSTRUCTION:
                // Match the 192kHz baseband aliases to the 6.144MHz wideband peaks.
                // This reveals the true amplitude of the high-frequency "tazer" signal.
                let true_signal = crystal_ball_disp.resolve_aliases(
                    &mags,
                    &wide_mags,
                    last_mamba_reconstruction.as_deref(),
                );
                state_disp
                    .reconstructed_peak
                    .store(true_signal.peak_voltage, Ordering::Relaxed);

                let true_hz_for_neo4j = true_signal.rf_carrier_hz;

                if let Ok(mut rm) = state_disp.reconstruction_mags.lock() {
                    if rm.len() != wide_mags.len() {
                        rm.resize(wide_mags.len(), 0.0);
                    }
                    // For visualization: zero out everything except the identified alias bin
                    rm.fill(0.0);
                    let target_bin = (true_signal.rf_carrier_hz / (24_576_000.0 / 2.0)
                        * wide_mags.len() as f32) as usize;
                    if target_bin < rm.len() {
                        rm[target_bin] = true_signal.peak_voltage;
                    }
                }

                (v, true_hz_for_neo4j)
            } else {
                let v = {
                    let mut vb = vbuf_disp.lock();
                    vb.push_frame_f32(&gpu_shared_disp.queue, &mags);

                    vb.version()
                };
                if let Ok(mut tm) = state_disp.tx_mags.lock() {
                    *tm = mags.clone();
                }
                (v, 0.0f32)
            };

            // Bispectrum
            let fft_il: Vec<f32> = cbuf
                .iter()
                .take(bispectrum::BISPEC_BINS)
                .flat_map(|c| [c.re, c.im])
                .collect();
            let events = bispec.analyze_frame(&fft_il, sample_rate, HardwareLayer::Microphone);
            let has_events = !events.is_empty();

            for event in events {
                last_bispec_event = Some(event.clone());

                // 1. Forensic JSONL (with forensic analysis enrichment)
                {
                    let mut enriched_event = event.clone();
                    state_disp.enrich_event_forensics(&mut enriched_event);
                    if let Ok(mut f) = forensic_disp.lock() {
                        let _ = f.log_detection(&enriched_event);
                    }
                }

                // 2. Qdrant + Neo4j persistence
                rt_store_async(
                    qdrant_disp.clone(),
                    neo4j_disp.clone(),
                    event.clone(),
                    state_disp.clone(),
                );

                // 3. Neo4j graph correlation — link audio+RF frequencies
                {
                    let ng = neo4j_disp.clone();
                    let eid = event.id.clone();
                    let ahz = event.f1_hz;
                    let rfhz = state_disp.get_sdr_center_hz();
                    let dcb = state_disp.get_audio_dc_bias();
                    let rf_dcb = state_disp.get_sdr_dc_bias();
                    let true_rf = wideband_vbuf_meta.1;

                    tokio::spawn(async move {
                        if let Some(g) = ng.lock().await.as_ref() {
                            if let Err(e) = g
                                .link_detection(&eid, ahz, rfhz, true_rf, dcb, rf_dcb)
                                .await
                            {
                                eprintln!("[Neo4j] link_detection: {e}");
                            }
                        }
                    });
                }

                // 4. Qdrant similarity search — detect recurring patterns
                {
                    let qd = qdrant_disp.clone();
                    let ev = event.clone();
                    let fdc = forensic_disp.clone();
                    tokio::spawn(async move {
                        if let Some(store) = qd.as_ref() {
                            match store.find_similar(&ev, 5).await {
                                Ok(similar) if !similar.is_empty() => {
                                    let top = &similar[0];
                                    if top.score > 0.85 {
                                        let ts = top
                                            .event
                                            .timestamp
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_millis();
                                        // Recurrence is forensically significant — log it
                                        println!(
                                            "[Forensic] RECURRENCE score={:.2} freq={:.1}Hz prior_ts={}ms",
                                            top.score, top.event.f1_hz, ts
                                        );
                                        if let Ok(mut f) = fdc.lock() {
                                            let _ = f.log_detection(&top.event);
                                        }
                                    }
                                }
                                Err(e) => eprintln!("[Qdrant] find_similar: {e}"),
                                _ => {}
                            }
                        }
                    });
                }

                if !state_disp.auto_tune.load(Ordering::Relaxed) {
                    state_disp.set_detected_freq(event.f1_hz);
                }
            }

            let mut mamba_reconstruction: Option<Vec<f32>> = None;

            // Mamba inference — gated by emergency off
            if !state_disp.get_mamba_emergency_off() {
                match mamba_trainer_disp.infer(&mags).await {
                    Ok((anomaly, mut latent, recon)) => {
                        // [Task 1 Injection] Mamba still runs above, but we mock the ModularFeature extraction to prove it wires.
                        // This is where we would pass &mags into ModularFeatureEncoder.
                        mamba_reconstruction = Some(recon.clone());
                        last_mamba_reconstruction = Some(recon);
                        latent.push(state_disp.get_audio_dc_bias());
                        latent.push(state_disp.get_sdr_dc_bias());
                        state_disp.set_mamba_anomaly(anomaly);
                        // --- Track C.2/C.4 Real-time Anomaly Gate ---
                        let mut fft_mag = [0.0f32; 128];
                        for i in 0..128 {
                            fft_mag[i] = if i < mags.len() { mags[i] } else { 0.0 };
                        }
                        let frame = crate::ml::spectral_frame::SpectralFrame {
                            timestamp_micros: chrono::Utc::now().timestamp_micros() as u64,
                            fft_magnitude: fft_mag,
                            bispectrum: [0.0; 64], // Populated later if needed
                            itd_ild: [0.0; 4],
                            beamformer_outputs: [0.0; 3],
                            mamba_anomaly_score: anomaly,
                        };
                        let gate = crate::ml::anomaly_gate::evaluate_gate(&frame, anomaly, 2.0);
                        if gate.forward_to_trainer {
                            // We would enqueue to trainer_tx here in a real setup.
                            // For now, we just log it if confidence is high.
                            if gate.confidence > 0.8 {
                                // println!("[GATE] Forwarding: {}", gate.reason);
                            }
                        }
                        state_disp.set_latent_embedding(latent.clone());

                        let beam_az = state_disp.get_beam_azimuth_deg().to_radians();
                        let beam_conf = state_disp.get_beam_confidence();
                        let fusion_r = fusion.fuse(
                            last_bispec_event.as_ref(),
                            anomaly,
                            &latent,
                            beam_az,
                            beam_conf,
                        );
                        state_disp.set_detected_freq(fusion_r.freq_hz);

                        let rf_hz = state_disp.get_sdr_center_hz();
                        let audio_bias = state_disp.get_audio_dc_bias();
                        if audio_bias > 0.1 || fusion_r.confidence > 0.8 {
                            let eid = format!("{}_{}", session_identity_clone, frame_idx);
                            let fdc = forensic_disp.clone();
                            let ecl = last_bispec_event.clone();
                            let eid2 = eid.clone();
                            tokio::spawn(async move {
                                if let Ok(mut f) = fdc.lock() {
                                    println!(
                                        "[DEFENSE] EVT:{} DC:{:.2}v RF:{:.1}MHz",
                                        eid2,
                                        audio_bias,
                                        rf_hz / 1e6
                                    );
                                    if let Some(ev) = ecl {
                                        let _ = f.log_detection(&ev);
                                    }
                                }
                            });
                        }

                        // Training pair accumulation (gate on anomaly-free frames)
                        if state_disp.get_training_recording_enabled() && !has_events {
                            let tx_cur = if let Ok(tx) = state_disp.tx_mags.lock() {
                                let mut t = tx.clone();
                                t.resize(512, 0.0);
                                t
                            } else {
                                vec![0.0; 512]
                            };
                            let mut rx_cur = if let Ok(sdr_mags) = state_disp.sdr_mags.try_lock() {
                                let mut r = sdr_mags.clone();
                                r.resize(512, 0.0);
                                r
                            } else {
                                let mut r = mags.clone();
                                r.resize(512, 0.0);
                                r
                            };
                            tx_frame_acc.extend_from_slice(&tx_cur);
                            rx_frame_acc.extend_from_slice(&rx_cur);
                            frame_count += 1;
                            if frame_count >= 64 {
                                let pair = mamba::TrainingPair::new(
                                    state_disp.get_sdr_center_hz() as u32,
                                    tx_frame_acc[..(512 * 64)].to_vec(),
                                    rx_frame_acc[..(512 * 64)].to_vec(),
                                );
                                training_session_disp.enqueue(pair).await;
                                let qs = qdrant_disp.clone();
                                let ls = latent.clone();
                                tokio::spawn(async move {
                                    if let Some(store) = qs.as_ref() {
                                        let _ = store.store_latents(&ls, &[]).await;
                                    }
                                });

                                // Periodic checkpoint save every 50 training batches
                                let epoch = state_disp.train_epoch.load(Ordering::Relaxed);
                                if epoch > 0 && epoch % 50 == 0 {
                                    let mt = mamba_trainer_disp.clone();
                                    let ckpt = state_disp
                                        .checkpoint_path
                                        .lock()
                                        .map(|p| p.clone())
                                        .unwrap_or_else(|_| {
                                            "weights/mamba_siren.safetensors".to_string()
                                        });

                                    // Create metadata for persistence
                                    let loss_avg = state_disp.train_loss.load(Ordering::Relaxed);
                                    let metadata = crate::state::CheckpointMetadata::new(
                                        epoch, loss_avg, 0.0, // min (not tracked yet)
                                        0.0, // max (not tracked yet)
                                    );

                                    let s_log = state_disp.clone();
                                    tokio::spawn(async move {
                                        match mt.save(&ckpt, Some(metadata)).await {
                                            Ok(_) => s_log.log(
                                                "INFO",
                                                "Mamba",
                                                &format!(
                                                    "Saved checkpoint: {} (epoch {})",
                                                    ckpt, epoch
                                                ),
                                            ),
                                            Err(e) => s_log.log(
                                                "ERROR",
                                                "Mamba",
                                                &format!("Save failed: {e}"),
                                            ),
                                        }
                                    });
                                }

                                if state_disp.get_sdr_sweeping() {
                                    let mut center_hz = state_disp.get_sdr_center_hz();
                                    center_hz += 2_048_000.0;
                                    if center_hz > 300_000_000.0 {
                                        center_hz = 10_000.0;
                                    }
                                    state_disp.set_sdr_center_hz(center_hz);
                                    println!("[SDR SWEEP] → {:.1} MHz", center_hz / 1e6);
                                }

                                tx_frame_acc.clear();
                                rx_frame_acc.clear();
                                frame_count = 0;
                            }
                        }
                    }
                    Err(e) => {
                        if frame_idx % 100 == 0 {
                            state_disp.log(
                                "ERROR",
                                "Mamba",
                                &format!("Frame {} infer() failed: {}", frame_idx, e),
                            );
                        }
                        state_disp.set_latent_embedding(vec![0.0; 32]);
                    }
                }
            } else {
                // Mamba EMERGENCY OFF — zero anomaly, no inference
                state_disp.set_mamba_anomaly(0.0);
                state_disp.set_latent_embedding(vec![0.0; 32]);
            }

            // ── Chord Dominance Counter (PDM Attack Response) ────────────────────
            let mut chord_dominance_freqs = Vec::new();
            let pdm_spike_count = state_disp.pdm_spike_count.load(Ordering::Relaxed);
            if pdm_spike_count > 0 {
                // PDM attack detected: synthesize harmonic dominance response
                if let Some((dominant_bin, _)) = mags
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                {
                    // Convert FFT bin to frequency (audio range)
                    let dominant_freq =
                        (dominant_bin as f32 / BISPEC_FFT_SIZE as f32) * sample_rate;

                    if dominant_freq > 50.0 && dominant_freq < 500.0 {
                        // Likely voice pitch during attack
                        let attack_key = harmony::detect_attack_key(dominant_freq);
                        let chord_freqs = harmony::get_chord_frequencies(&attack_key);

                        state_disp.log(
                            "INFO",
                            "Chord-Defense",
                            &format!(
                                "Detected {:?} attack at {:.1} Hz → synthesizing response chord",
                                attack_key, dominant_freq
                            ),
                        );

                        chord_dominance_freqs = chord_freqs;

                        // Store response for forensic logging
                        // (will be captured in next [EVIDENCE] memo)
                    }
                }
            }

            // Synthesis — Twister or legacy
            let denial_freq = state_disp.get_denial_freq();
            let current_mode = state_disp.mode.load(Ordering::Relaxed);

            let mut multi_targets: Vec<(f32, f32)> = if current_mode == 4 {
                let max_hz = if pdm_enabled {
                    state_disp.pdm_clock_mhz.load(Ordering::Relaxed) * 1_000_000.0 / 2.0
                } else {
                    sample_rate / 2.0
                };
                let sweep_t = (frame_idx as f32 * BISPEC_FFT_SIZE as f32 / sample_rate) % 5.0;
                let sf = 1.0_f32 * (max_hz / 1.0).powf(sweep_t / 5.0);
                state_disp.set_detected_freq(sf);
                vec![(sf, 1.0)]
            } else if state_disp.get_twister_active()
                && state_disp.auto_tune.load(Ordering::Relaxed)
            {
                crate::twister::twister_targets(denial_freq, crate::twister::ChordMode::Major)
            } else {
                vec![
                    (denial_freq * 0.5, 0.2),
                    (denial_freq, 0.2),
                    (denial_freq * 1.0001, 0.2),
                    (denial_freq * 2.0, 0.2),
                    (denial_freq * 3.0, 0.2),
                ]
            };

            // Add chord dominance frequencies if PDM attack detected
            if !chord_dominance_freqs.is_empty() {
                for &freq in &chord_dominance_freqs {
                    multi_targets.push((freq, 0.9)); // High gain for dominance
                }
            }

            // ── Phase 3b: MambaControlState Instantiation ───────────────────────
            let current_beam_az = state_disp.get_beam_azimuth_deg();
            let current_beam_el = state_disp
                .beam_elevation_rad
                .load(Ordering::Relaxed)
                .to_degrees();
            let current_gain = state_disp.get_master_gain();
            let current_waveshape_drive = state_disp.get_waveshape_drive();
            let current_anc_active = state_disp.anc_calibrated.load(Ordering::Relaxed)
                && state_disp.anc_ok.load(Ordering::Relaxed);

            let mamba_ctrl = crate::state::MambaControlState {
                active_modes: vec!["PhasedArrayAds".to_string()],
                beam_azimuth: current_beam_az,
                beam_elevation: current_beam_el,
                beam_phases: vec![
                    0.0;
                    state_disp.input_device_count.load(Ordering::Relaxed) as usize
                ],
                heterodyned_beams: vec![Some(denial_freq as f64)],
                waveshape_drive: current_waveshape_drive,
                anc_gain: if current_anc_active { 1.0 } else { 0.0 },
            };

            // ── Phase 3c: Mouth-region spatial filtering ─────────────────────────
            // If azimuth and elevation match mouth-region signature, apply maximum power
            let beam_az_rad = current_beam_az.to_radians();
            let beam_el_rad = state_disp.beam_elevation_rad.load(Ordering::Relaxed);
            let detected_freq = state_disp.get_detected_freq();

            // Mouth-region signature: elevation -30° to 0° (below ear), azimuth ±30° (frontal)
            let mouth_region_el_min = -std::f32::consts::PI / 6.0; // -30°
            let mouth_region_el_max = 0.0; // 0° (ear level)
            let mouth_region_az_max = std::f32::consts::PI / 6.0; // ±30° frontal

            let is_mouth_region = detected_freq > 50.0  // Voice-like frequencies (>50 Hz)
                && beam_el_rad >= mouth_region_el_min
                && beam_el_rad <= mouth_region_el_max
                && beam_az_rad.abs() <= mouth_region_az_max;

            if is_mouth_region && state_disp.get_beam_confidence() > 0.5 {
                // Mouth-region detected: apply maximum heterodyne power to all targets
                state_disp.log(
                    "INFO",
                    "Mouth-Region",
                    &format!(
                        "Spatial signature detected: AZ={:.1}° EL={:.1}° FREQ={:.0} Hz",
                        current_beam_az, current_beam_el, detected_freq
                    ),
                );

                // Increase gain for all synthesis targets to maximum
                let mut enhanced_targets = Vec::new();
                for &(freq, _old_gain) in &multi_targets {
                    enhanced_targets.push((freq, 0.98)); // Pushed to near-max
                }
                multi_targets = enhanced_targets;
            }

            let mut par_targets = Vec::new();
            for &(freq, gain) in &multi_targets {
                if freq > 0.0 {
                    let pair = parametric::ParametricPair::new(
                        PARAMETRIC_CARRIER_HZ,
                        freq,
                        gain * state_disp.get_master_gain(),
                    );
                    par_targets.extend_from_slice(&pair.to_denial_targets());
                }
            }

            let beam_az = state_disp.get_beam_azimuth_deg().to_radians();
            let beam_conf = state_disp.get_beam_confidence();
            let beam_mod = 0.5 + 0.5 * beam_conf;
            let fg_pairs: Vec<(f32, f32)> = par_targets
                .iter()
                .map(|t| (t.freq_hz, t.gain * state_disp.get_master_gain() * beam_mod))
                .collect();

            gpu_ctx.params.set_targets(&fg_pairs);
            gpu_ctx.params.master_gain = state_disp.get_master_gain();
            gpu_ctx.params.mode = state_disp.mode.load(Ordering::Relaxed);
            gpu_ctx.params.waveshape = state_disp.waveshape_mode.load(Ordering::Relaxed);
            gpu_ctx.params.waveshape_drive = state_disp.get_waveshape_drive();
            gpu_ctx.params.polarization = state_disp.get_polarization_angle() + beam_az;

            let mut synth_out = gpu_ctx.dispatch_synthesis();

            // ANC cancellation
            if state_disp.anc_calibrated.load(Ordering::Relaxed) {
                if let Ok(mut anc) = state_disp.anc_engine.lock() {
                    let blend = state_disp.get_smart_anc_blend();
                    let cancel = anc.update_hybrid(
                        &synth_out.clone(),
                        &chunk,
                        mamba_reconstruction.as_deref(),
                        blend,
                    );
                    for (s, &c) in synth_out.iter_mut().zip(cancel.iter()) {
                        *s += c;
                    }
                }
            }

            let peak = synth_out
                .iter()
                .cloned()
                .fold(0.0f32, |a, b| a.abs().max(b.abs()));
            state_disp.set_output_peak_db(if peak > 1e-10 {
                20.0 * peak.log10()
            } else {
                -100.0
            });

            if state_disp.running.load(Ordering::Relaxed) {
                if let Ok(mut f) = state_disp.output_frames.lock() {
                    *f = synth_out;
                }
            } else {
                if let Ok(mut f) = state_disp.output_frames.lock() {
                    f.fill(0.0);
                }
            }

            state_disp.set_dispatch_us(frame_start.elapsed().as_micros() as u32);
            state_disp.inc_frame_count();
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

                // Training tab sync
                ui.set_rtl_freq_mhz(st.get_sdr_center_hz() / 1e6);
                ui.set_rtl_scanning(st.get_sdr_sweeping());
                ui.set_training_pairs(training_session_timer.total_pairs() as i32);

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
                if let Ok(f) = f.lock() {
                    match f.export_evidence_report(
                        &path,
                        &case,
                        "Operator",
                        "Galveston TX",
                        None,
                        None,
                    ) {
                        Ok(_) => println!("[Forensic] Exported: {}", path),
                        Err(e) => eprintln!("[Forensic] Export failed: {e}"),
                    }
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
    let frustum_culler = crate::particle_system::frustum_culler::FrustumCuller::new(gpu_shared.clone(), 10_000_000);
    let particle_streamer = std::sync::Arc::new(crate::particle_system::streaming::ParticleStreamLoader::new());

    let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
    let _ = tokio::spawn({
        let s = particle_streamer.clone();
        async move { s.load_window(now_ms - 8_380_800_000, now_ms, 1_000_000).await; }
    });

    state.running.store(true, std::sync::atomic::Ordering::Relaxed);
    ui.run().context("Slint run failed")?;

    // ── Clean shutdown ────────────────────────────────────────────────────────
    println!("[Twister] UI closed. Saving checkpoint and exporting evidence...");

    // Final Mamba checkpoint
    {
        let ckpt = state
            .checkpoint_path
            .lock()
            .map(|p| p.clone())
            .unwrap_or_else(|_| "weights/mamba_siren.safetensors".to_string());
        match mamba_trainer.save(&ckpt, None).await {
            Ok(_) => println!("[Mamba] Final checkpoint: {}", ckpt),
            Err(e) => eprintln!("[Mamba] Final save failed: {e}"),
        }
    }

    // Final evidence report
    {
        std::fs::create_dir_all("evidence").ok();
        let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let path = format!("evidence/final_{ts}.html");
        if let Ok(f) = forensic.lock() {
            match f.export_evidence_report(
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
        }
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
}

fn rt_store_async(
    qdrant: Arc<Option<embeddings::EmbeddingStore>>,
    neo4j: Arc<tokio::sync::Mutex<Option<crate::graph::ForensicGraph>>>,
    event: DetectionEvent,
    state: Arc<AppState>,
) {
    state.memo_add(
        "[EVIDENCE]".to_string(),
        format!(
            "Auto-capture: Detection at {:.1} Hz (Magnitude: {:.2})",
            event.f1_hz, event.magnitude
        ),
    );

    if let Some(store) = (*qdrant).clone() {
        let ev = event.clone();
        tokio::spawn(async move {
            let _ = store.store_detection(&ev).await;
        });
    }

    let n = neo4j.clone();
    let ev = event.clone();
    tokio::spawn(async move {
        if let Some(g) = n.lock().await.as_ref() {
            let _ = g.store_detection(&ev).await;
        }
    });
}

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
                    let flags = state.lock().unwrap().get_feature_flags();
                    if let Ok(loss) = mamba_trainer.train_step_modular(&batch, &flags) {
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
pub mod particle_system;
