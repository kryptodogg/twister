// src/dispatch/signal_dispatch.rs — Multi-Modal Signal Dispatch Loop
//
// 100 Hz (10ms) central dispatch loop for Track B.
// Aggregates Audio, RF, and Visual telemetry into V-Buffers and Feature Payloads.
// Full implementation of Forensic Reconstruction, Mamba Anomaly Gate, and PDM wideband analysis.

use crate::audio::TaggedSamples;
use crate::bispectrum::{BISPEC_FFT_SIZE, BispectrumEngine};
use crate::detection::{DetectionEvent, HardwareLayer};
use crate::forensic::ForensicLogger;
use crate::gpu::GpuContext;
use crate::gpu_shared::GpuShared;
use crate::ml::modular_features::SignalFeaturePayload;
use crate::pdm::PdmEngine;
use crate::state::AppState;
use crate::training::{MambaTrainer, TrainingSession};
use crate::vbuffer::{GpuVBuffer, V_DEPTH, V_FREQ_BINS};
use crate::waterfall::WaterfallEngine;

use burn::backend::NdArray;
use burn::tensor::Tensor;
use crossbeam_channel::{Receiver, Sender};
use parking_lot::Mutex;
use rustfft::{FftPlanner, num_complex::Complex};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::time::{Duration, interval};

pub struct SignalDispatchLoop {
    state: Arc<AppState>,
    gpu_shared: Arc<GpuShared>,

    // Ingest channels
    merge_rx: Receiver<Vec<f32>>,
    sdr_rx: Receiver<(Vec<f32>, f32, f32)>,
    record_rx: Receiver<TaggedSamples>,

    // Egress channels
    feature_tx: Sender<(SignalFeaturePayload, Tensor<NdArray, 1>)>,

    // Engines
    waterfall: WaterfallEngine,
    sdr_waterfall: WaterfallEngine,
    pdm: PdmEngine,
    bispec: BispectrumEngine,
    gpu_ctx: GpuContext,
    fusion: crate::fusion::FusionEngine,
    crystal_ball: Arc<crate::reconstruct::CrystalBall>,
    qdrant: Arc<Option<crate::embeddings::EmbeddingStore>>,
    neo4j: Arc<tokio::sync::Mutex<Option<crate::graph::ForensicGraph>>>,
    vbuffer: Arc<Mutex<GpuVBuffer>>,
    sdr_vbuffer: Arc<Mutex<GpuVBuffer>>,

    // Forensic/ML
    mamba_trainer: Arc<MambaTrainer>,
    training_session: Arc<TrainingSession>,
    forensic: ForensicLogger,

    // Mutable state (Internal)
    acc: Vec<f32>,
    frame_idx: u64,
    vbuf_snapshot: Vec<[f32; V_FREQ_BINS]>,
    last_bispec_event: Option<DetectionEvent>,
    last_mamba_reconstruction: Option<Vec<f32>>,
    tx_frame_acc: Vec<f32>,
    rx_frame_acc: Vec<f32>,
    frame_count: usize,
}

impl SignalDispatchLoop {
    pub fn new(
        state: Arc<AppState>,
        gpu_shared: Arc<GpuShared>,
        merge_rx: Receiver<Vec<f32>>,
        sdr_rx: Receiver<(Vec<f32>, f32, f32)>,
        record_rx: Receiver<TaggedSamples>,
        feature_tx: Sender<(SignalFeaturePayload, Tensor<NdArray, 1>)>,
        waterfall: WaterfallEngine,
        sdr_waterfall: WaterfallEngine,
        pdm: PdmEngine,
        bispec: BispectrumEngine,
        gpu_ctx: GpuContext,
        fusion: crate::fusion::FusionEngine,
        crystal_ball: Arc<crate::reconstruct::CrystalBall>,
        qdrant: Arc<Option<crate::embeddings::EmbeddingStore>>,
        neo4j: Arc<tokio::sync::Mutex<Option<crate::graph::ForensicGraph>>>,
        vbuffer: Arc<Mutex<GpuVBuffer>>,
        sdr_vbuffer: Arc<Mutex<GpuVBuffer>>,
        mamba_trainer: Arc<MambaTrainer>,
        training_session: Arc<TrainingSession>,
        forensic: ForensicLogger,
    ) -> Self {
        Self {
            state,
            gpu_shared,
            merge_rx,
            sdr_rx,
            record_rx,
            feature_tx,
            waterfall,
            sdr_waterfall,
            pdm,
            bispec,
            gpu_ctx,
            fusion,
            crystal_ball,
            qdrant,
            neo4j,
            vbuffer,
            sdr_vbuffer,
            mamba_trainer,
            training_session,
            forensic,
            acc: Vec::with_capacity(crate::bispectrum::BISPEC_FFT_SIZE * 2),
            frame_idx: 0,
            vbuf_snapshot: vec![[0.0f32; V_FREQ_BINS]; V_DEPTH],
            last_bispec_event: None,
            last_mamba_reconstruction: None,
            tx_frame_acc: Vec::with_capacity(512 * 128),
            rx_frame_acc: Vec::with_capacity(512 * 128),
            frame_count: 0,
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        let mut ticker = interval(Duration::from_millis(10));
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(BISPEC_FFT_SIZE);
        let sample_rate = 192_000.0; // Audio pipeline rate

        eprintln!("[SignalDispatch] Starting high-fidelity 100Hz loop...");

        loop {
            ticker.tick().await;

            let pdm_enabled = self.state.pdm_active.load(Ordering::Relaxed);
            self.waterfall.set_pdm_mode(pdm_enabled);

            // 1. Ingest Audio
            while let Ok(chunk) = self.merge_rx.try_recv() {
                self.acc.extend_from_slice(&chunk);
            }

            // 1b. ANC Recording Logic
            let mut run_anc_analysis = false;
            if let Ok(mut rec) = self.state.anc_recording.lock() {
                if rec.state == crate::anc_recording::CalibrationState::Recording {
                    while let Ok(tagged) = self.record_rx.try_recv() {
                        rec.push_samples(tagged.device_idx, &tagged.samples);
                    }
                    if rec.is_complete() {
                        rec.state = crate::anc_recording::CalibrationState::Analyzing;
                        run_anc_analysis = true;
                    }
                }
            }

            if run_anc_analysis {
                let mut c0 = vec![];
                let mut c1 = vec![];
                let mut c2 = vec![];
                if let Ok(rec) = self.state.anc_recording.lock() {
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
                if let Ok(mut cal) = self.state.anc_calibration.lock() {
                    cal.calibrate_from_sweep(&c0, &c1, &c2, |bin| {
                        let a = self.state.mamba_anomaly_score.load(Ordering::Relaxed);
                        a * (0.5 + 0.5 * (bin as f32 / 8192.0).min(1.0))
                    });
                    self.state.set_anc_ok(true);
                }
                if let Ok(mut rec) = self.state.anc_recording.lock() {
                    rec.state = crate::anc_recording::CalibrationState::Idle;
                    rec.channels.clear();
                }
            }

            // 2. Ingest SDR
            while let Ok((mags, _sdr_last_center_hz, sdr_last_rate)) = self.sdr_rx.try_recv() {
                if !self.state.rtl_connected.load(Ordering::Relaxed) {
                    self.state.rtl_connected.store(true, Ordering::Relaxed);
                }
                let (rgba, sbars) = self.sdr_waterfall.push_row(&mags, 1.0, sdr_last_rate / 2.0);
                if let Ok(mut w) = self.state.sdr_waterfall_rgba.lock() {
                    *w = rgba;
                }
                if let Ok(mut sb) = self.state.sdr_spectrum_bars.lock() {
                    *sb = sbars;
                }
                if let Ok(mut sm) = self.state.sdr_mags.lock() {
                    *sm = mags.clone();
                }
                let dc = mags.get(mags.len() / 2).cloned().unwrap_or(0.0);
                self.state.set_sdr_dc_bias(dc);
                let mut vb = self.sdr_vbuffer.lock();
                vb.push_frame_f32(&self.gpu_shared.queue, &mags);
            }

            // 3. Process Audio (if enough samples)
            if self.acc.len() >= BISPEC_FFT_SIZE {
                let mut chunk: Vec<f32> = self.acc.drain(..BISPEC_FFT_SIZE).collect();
                self.frame_idx += 1;
                let frame_start = std::time::Instant::now();

                // PDM spike rejection
                let (filtered_chunk, pdm_spike_count) = crate::audio::reject_pdm_spikes(&chunk);
                chunk = filtered_chunk;

                // Feature Extraction
                let payload = SignalFeaturePayload {
                    audio_samples: chunk.clone(),
                    freq_hz: self.state.get_detected_freq(),
                    tdoa_confidence: Some(self.state.get_beam_confidence()),
                    device_corr: None,
                    vbuffer_coherence: None,
                    impulse_detection: None,
                    video_frame: None,
                    video_frame_timestamp_us: 0,
                    visual_features: None,
                    anc_phase: None,
                    harmonic_energy: None,
                };
                let device = burn::backend::ndarray::NdArrayDevice::Cpu;
                let extractor = crate::ml::modular_features::ModularFeatureExtractor::<
                    burn::backend::NdArray,
                >::new(&device);
                let (feature_vec, _) = extractor.extract(
                    &payload,
                    &crate::ml::modular_features::FeatureFlags::default(),
                );
                let _ = self.feature_tx.try_send((payload, feature_vec));

                if pdm_spike_count > 0 {
                    self.state
                        .pdm_spike_count
                        .fetch_add(pdm_spike_count as u64, Ordering::Relaxed);
                }

                let dc_audio = chunk.iter().sum::<f32>() / chunk.len() as f32;
                self.state.set_audio_dc_bias(dc_audio);

                // FFT
                let n = BISPEC_FFT_SIZE;
                let mut cbuf: Vec<Complex<f32>> = chunk
                    .iter()
                    .enumerate()
                    .map(|(i, &s)| {
                        let w =
                            0.5 * (1.0 - (std::f32::consts::TAU * i as f32 / (n - 1) as f32).cos());
                        Complex { re: s * w, im: 0.0 }
                    })
                    .collect();
                fft.process(&mut cbuf);
                let mags: Vec<f32> = cbuf.iter().take(V_FREQ_BINS).map(|c| c.norm()).collect();

                // Update V-buffer
                let vbuf_ver = {
                    let mut vb = self.vbuffer.lock();
                    vb.push_frame_f32(&self.gpu_shared.queue, &mags);
                    vb.version()
                };
                let slot = (vbuf_ver as usize).wrapping_sub(1) % V_DEPTH;
                self.vbuf_snapshot[slot][..mags.len()].copy_from_slice(&mags);

                // Twister auto-tune
                if self.state.auto_tune.load(Ordering::Relaxed) {
                    let (mts, freq_scale) = if pdm_enabled && !self.vbuf_snapshot[slot].is_empty() {
                        let pc = crate::pdm::pdm_clock_hz(sample_rate);
                        (
                            &self.vbuf_snapshot[slot][..],
                            pc / self.vbuf_snapshot[slot].len() as f32,
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
                    self.state.set_detected_freq(note.freq_hz);
                    self.state.set_note_name(note.name.clone());
                    self.state.set_note_cents(note.cents_offset);
                }

                // SNR
                {
                    let half = mags.len() / 2;
                    let peak = mags.iter().cloned().fold(0.0f32, f32::max).max(1e-10);
                    let noise =
                        mags[half..].iter().cloned().sum::<f32>() / mags[half..].len() as f32;
                    self.state
                        .set_snr_db((20.0 * (peak / noise.max(1e-10)).log10()).clamp(-20.0, 120.0));
                }

                // Waterfall
                let max_freq = if pdm_enabled {
                    crate::pdm::pdm_clock_hz(sample_rate) / 2.0
                } else {
                    sample_rate / 2.0
                };
                let (rgba, sbars) = self.waterfall.push_row(&mags, 1.0, max_freq);
                self.state.update_waterfall(&rgba);
                if let Ok(mut sb) = self.state.spectrum_bars.lock() {
                    *sb = sbars;
                }

                // PDM Wideband & Forensic Reconstruction
                let true_hz_for_neo4j = if pdm_enabled {
                    let words = self.pdm.encode(&chunk);
                    let _decoded = self.pdm.decode(&words);
                    let wide = self.pdm.decode_wideband(&words);
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

                    let mut vb = self.vbuffer.lock();
                    vb.push_frame_f32(&self.gpu_shared.queue, &wide_mags);

                    if let Ok(mut tm) = self.state.tx_mags.lock() {
                        *tm = wide_mags.clone();
                    }

                    let true_signal = self.crystal_ball.resolve_aliases(
                        &mags,
                        &wide_mags,
                        self.last_mamba_reconstruction.as_deref(),
                    );
                    self.state
                        .reconstructed_peak
                        .store(true_signal.peak_voltage, Ordering::Relaxed);

                    if let Ok(mut rm) = self.state.reconstruction_mags.lock() {
                        rm.resize(wide_mags.len(), 0.0);
                        rm.fill(0.0);
                        let target_bin = (true_signal.rf_carrier_hz / (24_576_000.0 / 2.0)
                            * wide_mags.len() as f32)
                            as usize;
                        if target_bin < rm.len() {
                            rm[target_bin] = true_signal.peak_voltage;
                        }
                    }
                    true_signal.rf_carrier_hz
                } else {
                    if let Ok(mut tm) = self.state.tx_mags.lock() {
                        *tm = mags.clone();
                    }
                    0.0
                };

                // Bispectrum
                let fft_il: Vec<f32> = cbuf
                    .iter()
                    .take(crate::bispectrum::BISPEC_BINS)
                    .flat_map(|c| [c.re, c.im])
                    .collect();
                let events =
                    self.bispec
                        .analyze_frame(&fft_il, sample_rate, HardwareLayer::Microphone);
                let has_events = !events.is_empty();

                for event in events {
                    self.last_bispec_event = Some(event.clone());
                    let mut enriched = event.clone();
                    self.state.enrich_event_forensics(&mut enriched);
                    let _ = self.forensic.log_detection(&enriched);

                    crate::forensic::rt_store_async(
                        self.qdrant.clone(),
                        self.neo4j.clone(),
                        event.clone(),
                        self.state.clone(),
                    );

                    let ng = self.neo4j.clone();
                    let eid = event.id.clone();
                    let ahz = event.f1_hz;
                    let rfhz = self.state.get_sdr_center_hz();
                    let adcb = self.state.get_audio_dc_bias();
                    let rdcb = self.state.get_sdr_dc_bias();
                    tokio::spawn(async move {
                        if let Some(g) = ng.lock().await.as_ref() {
                            let _ = g
                                .link_detection(&eid, ahz, rfhz, true_hz_for_neo4j, adcb, rdcb)
                                .await;
                        }
                    });

                    let qd = self.qdrant.clone();
                    let ev = event.clone();
                    let forensic_logger = self.forensic.clone();
                    tokio::spawn(async move {
                        if let Some(store) = qd.as_ref() {
                            if let Ok(similar) = store.find_similar(&ev, 5).await {
                                if !similar.is_empty() && similar[0].score > 0.85 {
                                    let _ = forensic_logger.log_detection(&similar[0].event);
                                }
                            }
                        }
                    });

                    if !self.state.auto_tune.load(Ordering::Relaxed) {
                        self.state.set_detected_freq(event.f1_hz);
                    }
                }

                // Mamba Inference
                if !self.state.get_mamba_emergency_off() {
                    match self.mamba_trainer.infer(&mags).await {
                        Ok((anomaly, mut latent, recon)) => {
                            self.last_mamba_reconstruction = Some(recon.clone());
                            latent.push(self.state.get_audio_dc_bias());
                            latent.push(self.state.get_sdr_dc_bias());
                            self.state.set_mamba_anomaly(anomaly);

                            let mut fft_mag = [0.0f32; 128];
                            for (i, m) in mags.iter().take(128).enumerate() {
                                fft_mag[i] = *m;
                            }
                            let frame = crate::ml::spectral_frame::SpectralFrame {
                                timestamp_micros: chrono::Utc::now().timestamp_micros() as u64,
                                fft_magnitude: fft_mag.to_vec(),
                                bispectrum: vec![0.0; 64],
                                itd_ild: [0.0; 4],
                                beamformer_outputs: [0.0; 3],
                                mamba_anomaly_score: anomaly,
                                confidence: 1.0,
                            };
                            let gate = crate::ml::anomaly_gate::evaluate_anomaly_gate(
                                &frame,
                                &crate::ml::anomaly_gate::AnomalyGateConfig::default(),
                            );
                            if let Ok(mut gs) = self.state.gate_status.lock() {
                                *gs = if gate.forward_to_trainer {
                                    "FORWARD".to_string()
                                } else {
                                    "REJECTED".to_string()
                                };
                            }

                            if gate.forward_to_trainer {
                                if self.state.get_training_recording_enabled()
                                    && gate.confidence > 0.8
                                {
                                    let tx_cur = self
                                        .state
                                        .tx_mags
                                        .lock()
                                        .map(|t| {
                                            let mut v = t.clone();
                                            v.resize(512, 0.0);
                                            v
                                        })
                                        .unwrap_or(vec![0.0; 512]);
                                    let rx_cur = self
                                        .state
                                        .sdr_mags
                                        .try_lock()
                                        .map(|t| {
                                            let mut v = t.clone();
                                            v.resize(512, 0.0);
                                            v
                                        })
                                        .unwrap_or(vec![0.0; 512]);
                                    let pair = crate::mamba::TrainingPair::new(
                                        self.state.get_sdr_center_hz() as u32,
                                        tx_cur,
                                        rx_cur,
                                    );
                                    let _ = self.training_session.try_enqueue(pair);
                                }
                            }

                            self.state.set_latent_embedding(latent.clone());
                            let fusion_r = self.fusion.fuse(
                                self.last_bispec_event.as_ref(),
                                anomaly,
                                &latent,
                                self.state.get_beam_azimuth_deg().to_radians(),
                                self.state.get_beam_confidence(),
                            );
                            self.state.set_detected_freq(fusion_r.freq_hz);

                            if self.state.get_training_recording_enabled() && !has_events {
                                if let Ok(tx_guard) = self.state.tx_mags.lock() {
                                    self.tx_frame_acc.extend_from_slice(&tx_guard);
                                } else {
                                    self.tx_frame_acc.extend_from_slice(&vec![0.0; 512]);
                                }
                                if let Ok(rx_guard) = self.state.sdr_mags.try_lock() {
                                    self.rx_frame_acc.extend_from_slice(&rx_guard);
                                } else {
                                    self.rx_frame_acc.extend_from_slice(&vec![0.0; 512]);
                                }
                                self.frame_count += 1;
                                if self.frame_count >= 64 {
                                    let pair = crate::mamba::TrainingPair::new(
                                        self.state.get_sdr_center_hz() as u32,
                                        self.tx_frame_acc.clone(),
                                        self.rx_frame_acc.clone(),
                                    );
                                    self.training_session.enqueue(pair).await;
                                    self.tx_frame_acc.clear();
                                    self.rx_frame_acc.clear();
                                    self.frame_count = 0;
                                }
                            }
                        }
                        Err(e) => eprintln!("[Dispatch] Mamba infer failed: {}", e),
                    }
                }

                // Defensive Responses
                let mut chord_freqs = Vec::new();
                if self.state.pdm_spike_count.load(Ordering::Relaxed) > 0 {
                    if let Some((bin, _)) = mags
                        .iter()
                        .enumerate()
                        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                    {
                        let df = (bin as f32 / BISPEC_FFT_SIZE as f32) * sample_rate;
                        if df > 50.0 && df < 500.0 {
                            chord_freqs = crate::harmony::get_chord_frequencies(
                                &crate::harmony::detect_attack_key(df),
                            );
                        }
                    }
                }

                let denial_freq = self.state.get_denial_freq();
                let mut multi_targets = if self.state.get_twister_active()
                    && self.state.auto_tune.load(Ordering::Relaxed)
                {
                    crate::twister::twister_targets(denial_freq, crate::twister::ChordMode::Major)
                } else {
                    vec![(denial_freq, 0.2)]
                };
                for f in chord_freqs {
                    multi_targets.push((f, 0.9));
                }

                // Mouth region spatial filtering
                let az_rad = self.state.get_beam_azimuth_deg().to_radians();
                let el_rad = self.state.beam_elevation_rad.load(Ordering::Relaxed);
                if self.state.get_detected_freq() > 50.0
                    && el_rad >= -std::f32::consts::PI / 6.0
                    && el_rad <= 0.0
                    && az_rad.abs() <= std::f32::consts::PI / 6.0
                {
                    for t in &mut multi_targets {
                        t.1 = 0.98;
                    }
                }

                let mut fg_pairs = Vec::new();
                for (f, g) in multi_targets {
                    let pair = crate::parametric::ParametricPair::new(
                        40000.0,
                        f,
                        g * self.state.get_master_gain(),
                    );
                    for t in pair.to_denial_targets() {
                        fg_pairs.push((t.freq_hz, t.gain));
                    }
                }
                self.gpu_ctx.params.set_targets(&fg_pairs);
                self.gpu_ctx.params.master_gain = self.state.get_master_gain();
                self.gpu_ctx.params.mode = self.state.mode.load(Ordering::Relaxed);
                self.gpu_ctx.params.waveshape = self.state.waveshape_mode.load(Ordering::Relaxed);
                self.gpu_ctx.params.waveshape_drive = self.state.get_waveshape_drive();
                self.gpu_ctx.params.polarization = self.state.get_polarization_angle() + az_rad;

                let mut synth_out = self.gpu_ctx.dispatch_synthesis();

                if self.state.anc_calibrated.load(Ordering::Relaxed) {
                    if let Ok(mut anc) = self.state.anc_engine.lock() {
                        let cancel = anc.update_hybrid(
                            &synth_out,
                            &chunk,
                            self.last_mamba_reconstruction.as_deref(),
                            self.state.get_smart_anc_blend(),
                        );
                        for (s, c) in synth_out.iter_mut().zip(cancel.iter()) {
                            *s += c;
                        }
                    }
                }

                let peak = synth_out
                    .iter()
                    .cloned()
                    .fold(0.0f32, |a, b| a.abs().max(b.abs()));
                self.state.set_output_peak_db(if peak > 1e-10 {
                    20.0 * peak.log10()
                } else {
                    -100.0
                });
                if self.state.running.load(Ordering::Relaxed) {
                    if let Ok(mut f) = self.state.output_frames.lock() {
                        *f = synth_out;
                    }
                }

                self.state
                    .set_dispatch_us(frame_start.elapsed().as_micros() as u32);
                self.state.inc_frame_count();
            }
        }
    }
}

pub fn snr_db(original: &[f32], decoded: &[f32]) -> f32 {
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
