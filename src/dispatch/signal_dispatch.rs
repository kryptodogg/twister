// src/dispatch/signal_dispatch.rs — Multi-Modal Signal Dispatch Loop
//
// 100 Hz (10ms) central dispatch loop for Track B.
// Aggregates Audio, RF, and Visual telemetry into V-Buffers and Feature Payloads.

use crate::audio::TaggedSamples;
use crate::bispectrum::{BISPEC_FFT_SIZE, BispectrumEngine};
use crate::forensic::ForensicLogger;
use crate::gpu::GpuContext;
use crate::gpu_shared::GpuShared;
use crate::ml::modular_features::{ImpulseTrainEvent, SignalFeaturePayload};
use crate::pdm::PdmEngine;
use crate::state::AppState;
use crate::training::MambaTrainer;
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
    gpu_shared: GpuShared,

    // Ingest channels
    merge_rx: Receiver<Vec<f32>>,
    sdr_rx: Receiver<(Vec<f32>, f32, f32)>,
    record_rx: Receiver<TaggedSamples>,

    // Egress channels
    feature_tx: Sender<(SignalFeaturePayload, Tensor<NdArray, 1>)>,
    impulse_tx: Sender<ImpulseTrainEvent>,

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
    session_identity: String,
    vbuffer: Arc<Mutex<GpuVBuffer>>,
    sdr_vbuffer: Arc<Mutex<GpuVBuffer>>,

    // Forensic/ML
    mamba_trainer: Arc<MambaTrainer>,
    training_session: Arc<crate::training::TrainingSession>,
    forensic: ForensicLogger,

    // Internal state
    acc: Vec<f32>,
    vbuf_snapshot: [[f32; V_FREQ_BINS]; V_DEPTH],
    frame_idx: u64,
}

impl SignalDispatchLoop {
    pub fn new(
        state: Arc<AppState>,
        gpu_shared: GpuShared,
        merge_rx: Receiver<Vec<f32>>,
        sdr_rx: Receiver<(Vec<f32>, f32, f32)>,
        record_rx: Receiver<TaggedSamples>,
        feature_tx: Sender<(SignalFeaturePayload, Tensor<NdArray, 1>)>,
        impulse_tx: Sender<ImpulseTrainEvent>,
        waterfall: WaterfallEngine,
        sdr_waterfall: WaterfallEngine,
        pdm: PdmEngine,
        bispec: BispectrumEngine,
        gpu_ctx: GpuContext,
        fusion: crate::fusion::FusionEngine,
        crystal_ball: Arc<crate::reconstruct::CrystalBall>,
        qdrant: Arc<Option<crate::embeddings::EmbeddingStore>>,
        neo4j: Arc<tokio::sync::Mutex<Option<crate::graph::ForensicGraph>>>,
        session_identity: String,
        vbuffer: Arc<Mutex<GpuVBuffer>>,
        sdr_vbuffer: Arc<Mutex<GpuVBuffer>>,
        mamba_trainer: Arc<MambaTrainer>,
        training_session: Arc<crate::training::TrainingSession>,
        forensic: ForensicLogger,
    ) -> Self {
        Self {
            state,
            gpu_shared,
            merge_rx,
            sdr_rx,
            record_rx,
            feature_tx,
            impulse_tx,
            waterfall,
            sdr_waterfall,
            pdm,
            bispec,
            gpu_ctx,
            fusion,
            crystal_ball,
            qdrant,
            neo4j,
            session_identity,
            vbuffer,
            sdr_vbuffer,
            mamba_trainer,
            training_session,
            forensic,
            acc: Vec::with_capacity(BISPEC_FFT_SIZE * 2),
            vbuf_snapshot: [[0.0f32; V_FREQ_BINS]; V_DEPTH],
            frame_idx: 0,
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        let mut ticker = interval(Duration::from_millis(10)); // 100 Hz polling
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(BISPEC_FFT_SIZE);
        let sample_rate = 192_000.0; // Audio pipeline rate

        eprintln!("[SignalDispatch] Starting 100Hz loop...");

        loop {
            ticker.tick().await;

            // 1. Ingest Audio
            while let Ok(chunk) = self.merge_rx.try_recv() {
                self.acc.extend_from_slice(&chunk);
            }

            // 1b. ANC Recording Logic
            if let Ok(mut rec) = self.state.anc_recording.lock() {
                if rec.state == crate::anc_recording::CalibrationState::Recording {
                    while let Ok(tagged) = self.record_rx.try_recv() {
                        rec.push_samples(tagged.device_idx, &tagged.samples);
                    }
                    if rec.is_complete() {
                        rec.state = crate::anc_recording::CalibrationState::Analyzing;
                        // Signal or run analysis (simplified for now)
                    }
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
                let mut vb = self.sdr_vbuffer.lock();
                vb.push_frame_f32(&self.gpu_shared.queue, &mags);

                self.state.set_rf_dirty(true);
            }

            // 3. Process Audio (if enough samples for FFT)
            if self.acc.len() >= BISPEC_FFT_SIZE {
                let chunk: Vec<f32> = self.acc.drain(..BISPEC_FFT_SIZE).collect();
                self.frame_idx += 1;

                // PDM spike rejection (Optional forensic filter)
                let (filtered_chunk, pdm_spike_count) = crate::audio::reject_pdm_spikes(&chunk);
                let chunk = filtered_chunk;
                if pdm_spike_count > 0 {
                    self.state
                        .pdm_spike_count
                        .fetch_add(pdm_spike_count as u64, Ordering::Relaxed);
                }

                // Process Bispectrum / FFT
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

                // Waterfall
                let max_freq = sample_rate / 2.0;
                let (rgba, sbars) = self.waterfall.push_row(&mags, 1.0, max_freq);
                self.state.update_waterfall(&rgba);
                if let Ok(mut sb) = self.state.spectrum_bars.lock() {
                    *sb = sbars;
                }

                // Crystal Ball Reconstruction
                // In main.rs, this was done for wideband. Here we use baseband mags.
                // Simplified: we'll just track the peak for now.
                let peak = mags.iter().cloned().fold(0.0f32, f32::max);
                self.state.reconstructed_peak.store(peak, Ordering::Relaxed);

                self.state.set_audio_dirty(true);

                // 4. Feature Extraction & ML Dispatch
                let payload = SignalFeaturePayload {
                    audio_samples: chunk.clone(),
                    freq_hz: self.state.get_detected_freq(),
                    tdoa_confidence: Some(self.state.get_beam_confidence()),
                    vbuffer_coherence: None,
                    impulse_detection: None,
                    video_frame: None,
                    video_frame_timestamp_us: 0,
                    visual_features: None,
                    device_corr: None,
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

                // 5. Mamba Inference & Anomaly Gate
                if !self.state.get_mamba_emergency_off() {
                    match self.mamba_trainer.infer(&mags).await {
                        Ok((anomaly, mut latent, recon)) => {
                            self.state.set_mamba_anomaly(anomaly);
                            latent.push(self.state.get_audio_dc_bias());
                            self.state.set_latent_embedding(latent.clone());

                            // Anomaly Gate
                            let mut fft_mag = [0.0f32; 128];
                            for i in 0..128 {
                                if i < mags.len() {
                                    fft_mag[i] = mags[i];
                                }
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

                            // Fusion
                            let beam_az = self.state.get_beam_azimuth_deg().to_radians();
                            let fusion_r = self.fusion.fuse(
                                None,
                                anomaly,
                                &latent,
                                beam_az,
                                self.state.get_beam_confidence(),
                            );
                            self.state.set_detected_freq(fusion_r.freq_hz);

                            // SDR Sweeping Logic
                            if self.state.get_sdr_sweeping() {
                                let mut center_hz = self.state.get_sdr_center_hz();
                                center_hz += 2_048_000.0;
                                if center_hz > 300_000_000.0 {
                                    center_hz = 10_000.0;
                                }
                                self.state.set_sdr_center_hz(center_hz);
                            }
                        }
                        Err(e) => eprintln!("[Dispatch] Mamba inference failed: {}", e),
                    }
                }

                // 6. Chord Dominance & Defensive Response
                let mut chord_dominance_freqs = Vec::new();
                let pdm_spike_count = self.state.pdm_spike_count.load(Ordering::Relaxed);
                if pdm_spike_count > 0 {
                    if let Some((dominant_bin, _)) = mags
                        .iter()
                        .enumerate()
                        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    {
                        let dominant_freq =
                            (dominant_bin as f32 / BISPEC_FFT_SIZE as f32) * 192000.0; // Assuming 192kHz
                        if dominant_freq > 50.0 && dominant_freq < 500.0 {
                            let attack_key = crate::harmony::detect_attack_key(dominant_freq);
                            chord_dominance_freqs =
                                crate::harmony::get_chord_frequencies(&attack_key);
                        }
                    }
                }

                // 7. Synthesis Target Calculation
                let denial_freq = self.state.get_denial_freq();
                let mut multi_targets = if self.state.get_twister_active()
                    && self.state.auto_tune.load(Ordering::Relaxed)
                {
                    crate::twister::twister_targets(denial_freq, crate::twister::ChordMode::Major)
                } else {
                    vec![(denial_freq, 0.2)]
                };

                for &freq in &chord_dominance_freqs {
                    multi_targets.push((freq, 0.9));
                }

                // Mouth-region spatial enhancement
                let beam_az_rad = self.state.get_beam_azimuth_deg().to_radians();
                let beam_el_rad = self.state.beam_elevation_rad.load(Ordering::Relaxed);
                if self.state.get_detected_freq() > 50.0
                    && beam_el_rad >= -0.5
                    && beam_el_rad <= 0.0
                    && beam_az_rad.abs() <= 0.5
                {
                    for t in &mut multi_targets {
                        t.1 = 0.98;
                    }
                }

                // 8. GPU Context Synthesis
                let mut fg_pairs = Vec::new();
                for &(freq, gain) in &multi_targets {
                    if freq > 0.0 {
                        let pair = crate::parametric::ParametricPair::new(
                            24_000.0, // Default parametric carrier
                            freq,
                            gain * self.state.get_master_gain(),
                        );
                        for t in pair.to_denial_targets() {
                            fg_pairs.push((t.freq_hz, t.gain * self.state.get_master_gain()));
                        }
                    }
                }
                self.gpu_ctx.params.set_targets(&fg_pairs);
                let mut synth_out = self.gpu_ctx.dispatch_synthesis();

                // 9. ANC Integration
                if self.state.anc_calibrated.load(Ordering::Relaxed) {
                    if let Ok(mut anc) = self.state.anc_engine.lock() {
                        let cancel = anc.update_hybrid(
                            &synth_out,
                            &chunk,
                            None,
                            self.state.get_smart_anc_blend(),
                        );
                        for (s, &c) in synth_out.iter_mut().zip(cancel.iter()) {
                            *s += c;
                        }
                    }
                }

                // 10. Forensic Persistence (Neo4j/Qdrant)
                let detection_event = crate::detection::DetectionEvent {
                    id: format!(
                        "{}_{}",
                        self.session_identity,
                        chrono::Utc::now().timestamp_micros()
                    ),
                    timestamp: std::time::SystemTime::now(),
                    f1_hz: self.state.get_detected_freq(),
                    f2_hz: 0.0,
                    product_hz: self.state.get_detected_freq(),
                    product_type: crate::detection::ProductType::Harmonic,
                    magnitude: 1.0,
                    phase_angle: 0.0,
                    coherence_frames: 0,
                    spl_db: 0.0,
                    session_id: self.session_identity.clone(),
                    hardware: crate::detection::HardwareLayer::Microphone,
                    embedding: vec![],
                    frequency_band: crate::bispectrum::FrequencyBand::classify(
                        self.state.get_detected_freq(),
                    ),
                    audio_dc_bias_v: Some(self.state.get_audio_dc_bias()),
                    sdr_dc_bias_v: Some(self.state.get_sdr_dc_bias()),
                    mamba_anomaly_db: 0.0,
                    timestamp_sync_ms: None,
                    is_coordinated: false,
                    detection_method: "anomaly".to_string(),
                };

                // Real-time store (async)
                if self.qdrant.is_some() {
                    let qd = self.qdrant.clone();
                    let nj = self.neo4j.clone();
                    let ev = detection_event.clone();
                    let st = self.state.clone();
                    // Note: rt_store_async will access state through internal mechanisms
                    // For now, skip the async store if we can't pass AppState directly
                    // TODO: Refactor rt_store_async to accept Arc<AppState>
                }

                // 11. Training Accumulation
                if self.state.get_training_recording_enabled() {
                    let tx_cur = if let Ok(tx) = self.state.tx_mags.lock() {
                        tx.clone()
                    } else {
                        vec![0.0; 512]
                    };
                    let rx_cur = if let Ok(sdr_mags) = self.state.sdr_mags.try_lock() {
                        sdr_mags.clone()
                    } else {
                        mags.clone()
                    };

                    let pair = crate::mamba::TrainingPair::new(
                        self.state.get_sdr_center_hz() as u32,
                        tx_cur,
                        rx_cur,
                    );
                    let _ = self.training_session.try_enqueue(pair);
                }

                self.state.set_feature_dirty(true);
            }

            // 5. Visual Ingest (Future)
            // if let Some(frame) = camera.poll() { ... self.state.set_visual_dirty(true); }
        }
    }
}
