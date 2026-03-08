diff --git a/src/main.rs b/src/main.rs
index d9ad322..b2780d6 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,938 +1,667 @@
 // src/main.rs — SIREN v0.4 Orchestration
 //
-// v0.4 changes (reviewer P0/P1 fixes):
+// Thread model:
+//   main            — Slint UI event loop (must stay on main thread)
+//   audio_thread    — cpal capture + AGC (spawned by AudioEngine::new)
+//   dispatch_thread — frame pipeline: FFT → V-buffer → waterfall → bispectrum → synthesis
+//   tdoa_thread     — GCC-PHAT beam estimation, writes to AppState
+//   sdr_thread      — RTL-SDR IQ capture → mags, sends via sdr_mag_rx
+//   trainer_thread  — Mamba online training, reads replay windows from trainer_rx
 //
-//   P0 — Synthetic signal self-test
-//     `--self-test` CLI flag (or SIREN_SELF_TEST=1 env var) runs the bispectrum
-//     detection validation suite before the dispatch loop starts.  Exits with
-//     code 1 if the detector fails minimum accuracy thresholds.
+// Shared GPU singleton (Arc<GpuShared>) flows into:
+//   GpuContext (synthesis), PdmEngine, WaterfallEngine, BispectrumEngine
 //
-//   P0 — Anti-phase with physical phase correction
-//     DenialMode::AncAntiPhase (mode 5) uses AncEngine::phase_for(freq) to
-//     compute the correct phase advance before synthesis so the wave arrives
-//     anti-phase at the microphone position.  Mode 1 (AntiPhase) is retained
-//     unchanged for comparison.
-//
-//     Calibration: set mode to AncAntiPhase, then call anc_calibrate() once.
-//     The calibration sweep is played through the output and the primary mic
-//     records the response.
-//
-//   P1 — GPU singleton
-//     All GPU engines share one Arc<GpuShared> device instead of four separate
-//     wgpu::Instance allocations.  See gpu_device.rs.
-//
-//   P2 — Frame timing metrics
-//     dispatch_us and frame_count fields in AppState are updated every loop.
-//
-//   Quality — AtomicF32 in state.rs replaces AtomicU32 bit-casting.
+// V-buffer (SharedVBuffer) flows into:
+//   WaterfallEngine (reads latest row for display)
+//   BispectrumEngine (reads FFT complex from latest frame)
+//   TrainerThread (reads context windows for Mamba)
 
-#![allow(dead_code, unused_variables, unused_imports)]
+#![allow(clippy::too_many_arguments)]
 
 slint::include_modules!();
 
-use slint::{Color, Image, Model, Rgba8Pixel, SharedPixelBuffer, VecModel};
-
+mod af32;
 mod audio;
 mod bispectrum;
+
 mod detection;
 mod embeddings;
 mod forensic;
-mod fusion; // Bispectrum + Mamba latent fusion
 mod gpu;
-mod gpu_device;
+mod gpu_shared;
 mod graph;
-mod mamba; // IQUMamba-1D autoencoder stub
+mod mamba;
 mod parametric;
 mod pdm;
-mod ridge_plot;
-mod rtlsdr; // RTL-SDR hardware abstraction
-mod rtlsdr_ffi; // RTL-SDR FFI bindings (unsafe)
+mod sdr;
 mod state;
-mod training; // RTL-SDR + Mamba Training Tab orchestrator
+mod trainer;
+mod vbuffer;
 mod waterfall;
-mod databases {
-    //! Database directory setup — all persistence under <project_root>/databases/
-    use std::path::PathBuf;
-
-    fn root() -> PathBuf {
-        let exe = std::env::current_exe().unwrap_or_default();
-        exe.parent()
-            .and_then(|p| p.parent())
-            .and_then(|p| p.parent())
-            .unwrap_or(std::path::Path::new("."))
-            .join("databases")
-    }
 
-    pub fn ensure_dirs() {
-        for (name, path) in [
-            ("neo4j", root().join("neo4j")),
-            ("qdrant", root().join("qdrant")),
-        ] {
-            match std::fs::create_dir_all(&path) {
-                Ok(_) => println!("[Databases] {}: {}", name, path.display()),
-                Err(e) => eprintln!("[Databases] Could not create {}: {}", path.display(), e),
-            }
-        }
-    }
-}
-mod anc;
-mod resample;
-mod testing;
-
-use crate::anc::AncEngine;
-use crate::audio::{
-    AudioEngine, BASEBAND_FFT_SIZE, DEFAULT_MIC_SPACING_M, TdoaEngine, tdoa_channel,
-};
-use crate::bispectrum::BispectrumEngine;
-use crate::forensic::ForensicLogger;
-use crate::gpu::GpuContext;
-use crate::gpu_device::GpuShared;
-use crate::parametric::ParametricManager;
-use crate::pdm::{
-    OVERSAMPLE_RATIO, PdmEngine, WIDEBAND_FRAMES, pdm_clock_hz, wideband_sample_rate,
-};
-use crate::state::{AppState, DenialMode, WaveshapeMode};
-use crate::waterfall::WaterfallEngine;
-use crossbeam_channel::bounded;
-use rustfft::{FftPlanner, num_complex::Complex};
-use std::fmt::Write;
 use std::sync::Arc;
 use std::sync::atomic::Ordering;
-use std::time::Instant;
+use std::time::{SystemTime, UNIX_EPOCH};
+
+use anyhow::Context;
+use crossbeam_channel::bounded;
+use rustfft::{FftPlanner, num_complex::Complex};
+
+use audio::{AudioEngine, DEFAULT_MIC_SPACING_M, TdoaEngine, tdoa_channel};
+use bispectrum::{BISPEC_FFT_SIZE, BispectrumEngine};
+use detection::{DetectionEvent, HardwareLayer};
+use forensic::ForensicLogger;
+use gpu::GpuContext;
+use gpu_shared::GpuShared;
+use mamba::MAMBA_CONTEXT_LEN;
+use parametric::ParametricManager;
+use pdm::PdmEngine;
+use sdr::sdr_channel;
+use state::AppState;
+use trainer::{TrainerCmd, TrainerThread, extract_window_from_vbuf_snapshot};
+use vbuffer::{V_DEPTH, V_FREQ_BINS, new_shared_vbuffer};
+use waterfall::WaterfallEngine;
+
+const PARAMETRIC_CARRIER_HZ: f32 = 40_000.0;
+const SESSION_TIMEOUT_SECS: u64 = 3600;
 
 fn main() -> anyhow::Result<()> {
-    databases::ensure_dirs();
+    // ── Session ID ────────────────────────────────────────────────────────────
+    let session_id = {
+        let ts = SystemTime::now()
+            .duration_since(UNIX_EPOCH)
+            .unwrap_or_default()
+            .as_secs();
+        format!("siren_{:016x}", ts)
+    };
+    println!("[SIREN v0.4] Session: {}", session_id);
+
+    // ── Shared state ──────────────────────────────────────────────────────────
     let state = AppState::new();
-    let session_id = chrono_session_id();
 
-    // ── CLI / env flags ───────────────────────────────────────────────────────
-    let run_self_test =
-        std::env::args().any(|a| a == "--self-test") || std::env::var("SIREN_SELF_TEST").is_ok();
+    // ── Slint UI ──────────────────────────────────────────────────────────────
+    let ui = AppWindow::new().context("Slint window creation failed")?;
 
-    // ── Audio ─────────────────────────────────────────────────────────────────
-    let (sample_tx, sample_rx) = bounded::<Vec<f32>>(32);
+    // ── GPU singleton ─────────────────────────────────────────────────────────
+    let gpu_shared = GpuShared::new().context("GPU init failed")?;
+    println!(
+        "[GPU] Adapter: {} ({:?})",
+        gpu_shared.adapter_info.name, gpu_shared.adapter_info.backend
+    );
+
+    // ── V-buffer ──────────────────────────────────────────────────────────────
+    let vbuffer = new_shared_vbuffer(&gpu_shared.device);
+
+    // ── Audio engine ──────────────────────────────────────────────────────────
+    let (merge_tx, merge_rx) = bounded::<Vec<f32>>(256);
     let (tdoa_tx, tdoa_rx) = tdoa_channel();
-    let audio = AudioEngine::new(state.clone(), sample_tx, tdoa_tx)?;
+
+    let audio = AudioEngine::new(state.clone(), merge_tx, tdoa_tx).context("Audio init failed")?;
     let sample_rate = audio.sample_rate;
     let n_channels = audio.n_channels;
-
-    let pdm_clock = pdm_clock_hz(sample_rate);
-    let wb_nyquist = wideband_sample_rate(sample_rate) / 2.0;
-    state.set_pdm_clock_mhz(pdm_clock / 1_000_000.0);
-    state
-        .oversample_ratio
-        .store(OVERSAMPLE_RATIO as u32, Ordering::Relaxed);
-
-    println!("[Main] Audio  : {} Hz  ×{} ch", sample_rate, n_channels);
+    let device_count = audio.device_count;
     println!(
-        "[Main] PDM    : {:.3} MHz clock  →  {:.3} MHz Nyquist",
-        pdm_clock / 1e6,
-        wb_nyquist / 1e6
+        "[Audio] {:.0} Hz × {} ch, {} input device(s)",
+        sample_rate, n_channels, device_count
     );
 
-    // ── GPU singleton ─────────────────────────────────────────────────────────
-    // One wgpu::Instance → Adapter → Device shared across ALL sub-engines.
-    // Replaces the four separate make_gpu_device() calls that previously
-    // allocated independent devices.
-    let gpu_shared = GpuShared::new()?;
-
-    // ── GPU synthesis (takes ownership of the Arc; keeps an Arc inside) ───────
-    let mut gpu = {
-        let arc = GpuContext::new(gpu_shared.clone(), sample_rate, n_channels)?;
-        Arc::try_unwrap(arc).unwrap_or_else(|_| panic!("GpuContext Arc unwrap failed"))
-    };
+    // ── GPU engines (all sharing gpu_shared) ──────────────────────────────────
+    let mut gpu_ctx = GpuContext::new(gpu_shared.clone(), sample_rate, n_channels)
+        .context("GPU synthesis init")?;
+    let mut pdm = PdmEngine::new(gpu_shared.clone(), sample_rate).context("PDM engine init")?;
+    let mut waterfall = WaterfallEngine::new(gpu_shared.clone(), sample_rate, false)
+        .context("Waterfall engine init")?;
+    let mut bispec = BispectrumEngine::new(gpu_shared.clone(), session_id.clone())
+        .context("Bispectrum engine init")?;
+
+    // ── Forensic logger ───────────────────────────────────────────────────────
+    let forensic = Arc::new(std::sync::Mutex::new(
+        ForensicLogger::new(&session_id).context("Forensic log init")?,
+    ));
+    println!(
+        "[Forensic] Log: {}",
+        forensic.lock().unwrap().log_path().display()
+    );
 
-    // ── UI ────────────────────────────────────────────────────────────────────
-    let ui = AppWindow::new()?;
-    let ui_weak = ui.as_weak();
+    // ── Parametric speaker manager ────────────────────────────────────────────
+    let parametric = ParametricManager::new(PARAMETRIC_CARRIER_HZ);
 
-    {
-        let s = state.clone();
-        ui.on_set_mode(move |m| {
-            s.set_mode(DenialMode::from_u32(m as u32));
-        });
-        let s = state.clone();
-        ui.on_set_gain(move |g| {
-            s.set_master_gain(g);
-        });
-        let s = state.clone();
-        ui.on_set_freq_override(move |hz| {
-            s.set_denial_freq_override(hz);
-        });
-        let s = state.clone();
-        ui.on_toggle_auto_tune(move || {
-            s.auto_tune
-                .store(!s.auto_tune.load(Ordering::Relaxed), Ordering::Relaxed);
-        });
-        let s = state.clone();
-        ui.on_toggle_running(move || {
-            s.running
-                .store(!s.running.load(Ordering::Relaxed), Ordering::Relaxed);
-        });
-        let s = state.clone();
-        ui.on_toggle_pdm(move || {
-            let prev = s.pdm_active.load(Ordering::Relaxed);
-            s.pdm_active.store(!prev, Ordering::Relaxed);
-            println!("[Main] PDM wideband: {}", !prev);
-        });
-        let s = state.clone();
-        ui.on_set_waveshape(move |m| {
-            s.set_waveshape_mode(WaveshapeMode::from_u32(m as u32));
-        });
-        let s = state.clone();
-        ui.on_set_waveshape_drive(move |d| {
-            s.set_waveshape_drive(d);
-        });
-        let s = state.clone();
-        ui.on_set_beam_focus(move |deg| {
-            s.set_beam_focus_deg(deg);
-        });
-        // ANC calibrate callback: sets the anc_calibrating flag; the dispatch
-        // loop picks this up at the start of the next frame.
-        let s = state.clone();
-        ui.on_anc_calibrate(move || {
-            if !s.anc_calibrating.load(Ordering::Relaxed) {
-                s.anc_calibrating.store(true, Ordering::Relaxed);
-                println!("[ANC] Calibration requested via UI");
+    // ── Optional DB connections ───────────────────────────────────────────────
+    let rt = Arc::new(
+        tokio::runtime::Builder::new_current_thread()
+            .enable_all()
+            .build()?,
+    );
+    let qdrant = rt.block_on(async {
+        match embeddings::EmbeddingStore::new().await {
+            Ok(s) => {
+                println!("[Qdrant] Connected.");
+                Some(s)
             }
-        });
-
-        // ── Training Callbacks ────────────────────────────────────────────────
-        let s = state.clone();
-        ui.on_rtl_connect(move || {
-            s.rtl_connected.store(true, Ordering::Relaxed); // Flag for dispatch loop to init
-        });
-        let s = state.clone();
-        ui.on_rtl_start_scan(move || {
-            s.rtl_scanning.store(true, Ordering::Relaxed);
-        });
-        let s = state.clone();
-        ui.on_rtl_stop_scan(move || {
-            s.rtl_scanning.store(false, Ordering::Relaxed);
-        });
-        let s = state.clone();
-        ui.on_training_start(move || {
-            s.training_active.store(true, Ordering::Relaxed);
-        });
-        let s = state.clone();
-        ui.on_training_stop(move || {
-            s.training_active.store(false, Ordering::Relaxed);
-        });
-    }
-
-    state.running.store(true, Ordering::Relaxed);
-
-    // ── Dispatch thread ───────────────────────────────────────────────────────
-    let running_flag = Arc::new(std::sync::atomic::AtomicBool::new(true));
-    let running_dispatch = running_flag.clone();
-    let state_d = state.clone();
-    let session_d = session_id.clone();
-    let sr_d = sample_rate;
-    let tdoa_device_count = audio.device_count;
-
-    let dispatch_handle = std::thread::spawn(move || {
-        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
-
-        let mut planner = FftPlanner::<f32>::new();
-        let mut accumulator = Vec::<f32>::with_capacity(8192);
-
-        let parametric = ParametricManager::new(40_000.0);
-        println!(
-            "[Parametric] Carrier 40 kHz  beam half-angle (50mm): {:.1}°",
-            parametric.beam_half_angle_deg(0.05)
-        );
-
-        let mut tdoa_engine = TdoaEngine::new(tdoa_device_count, sr_d, DEFAULT_MIC_SPACING_M);
-        let mut forensic = ForensicLogger::new(&session_d).expect("forensic logger");
-
-        let graph_client = rt
-            .block_on(graph::ForensicGraph::new())
-            .map(Arc::new)
-            .map_err(|e| eprintln!("[Graph] Neo4j unavailable: {e}"))
-            .ok();
-
-        let embed_client = rt
-            .block_on(embeddings::EmbeddingStore::new())
-            .map(Arc::new)
-            .map_err(|e| eprintln!("[Embeddings] Qdrant unavailable: {e}"))
-            .ok();
-
-        // ── GPU sub-engines — all sharing the singleton ───────────────────────
-        let mut pdm_engine: Option<PdmEngine> = PdmEngine::new(gpu_shared.clone(), sr_d as f32)
-            .map_err(|e| eprintln!("[PDM] Init failed: {e}"))
-            .ok();
-
-        let mut waterfall_engine: Option<WaterfallEngine> =
-            WaterfallEngine::new(gpu_shared.clone(), sr_d as f32, false)
-                .map_err(|e| eprintln!("[Waterfall] Init failed: {e}"))
-                .ok();
-
-        let mut ridge_engine: Option<ridge_plot::RidgePlotGpu> =
-            ridge_plot::RidgePlotGpu::new(gpu_shared.clone())
-                .map_err(|e| eprintln!("[Ridge] Init failed: {e}"))
-                .ok();
-
-        // ── Training Session ──────────────────────────────────────────────────
-        let mut training = training::TrainingSession::new();
-
-        let mut bispectrum_engine: Option<BispectrumEngine> =
-            BispectrumEngine::new(gpu_shared.clone(), session_d.clone())
-                .map_err(|e| eprintln!("[Bispec] Init failed: {e}"))
-                .ok();
-
-        // ── P0: Synthetic self-test ───────────────────────────────────────────
-        if run_self_test {
-            if let Some(ref mut bispec) = bispectrum_engine {
-                println!("[SelfTest] Running detector validation...");
-                match testing::run_self_test(bispec, sr_d) {
-                    Ok(report) => println!("[SelfTest] {}", report.summary_line()),
-                    Err(e) => {
-                        eprintln!("[SelfTest] FAILED: {}", e);
-                        eprintln!("[SelfTest] The bispectrum detector may be hallucinating.");
-                        eprintln!("[SelfTest] Adjust COHERENCE_THRESHOLD and re-run.");
-                        std::process::exit(1);
-                    }
-                }
-            } else {
-                eprintln!("[SelfTest] No GPU bispectrum engine available — skipping");
+            Err(e) => {
+                eprintln!("[Qdrant] Unavailable: {e}");
+                None
             }
         }
+    });
+    let neo4j = rt.block_on(async {
+        match graph::ForensicGraph::new().await {
+            Ok(g) => {
+                println!("[Neo4j] Connected.");
+                Some(g)
+            }
+            Err(e) => {
+                eprintln!("[Neo4j] Unavailable: {e}");
+                None
+            }
+        }
+    });
+    let qdrant = Arc::new(qdrant);
+    let neo4j = Arc::new(neo4j);
+
+    // ── SDR channel ───────────────────────────────────────────────────────────
+    let (sdr_tx, sdr_rx) = sdr_channel();
+    let sdr_state = state.clone();
+    let _sdr_thread = sdr::spawn_sdr_thread(sdr_state, sdr_tx);
+
+    // ── Trainer channel ───────────────────────────────────────────────────────
+    let (trainer_tx, trainer_rx) = bounded::<TrainerCmd>(512);
+    let trainer_state = state.clone();
+    let _trainer_thread = std::thread::spawn(move || match TrainerThread::new(trainer_state) {
+        Ok(mut t) => t.run(trainer_rx),
+        Err(e) => eprintln!("[Trainer] Init failed: {e}"),
+    });
 
-        // ── ANC engine ────────────────────────────────────────────────────────
-        // Nominal speaker–mic distance: 50 cm (adjustable in UI later).
-        let mut anc = AncEngine::new(sr_d, 0.50);
-        // Ring buffer to capture the calibration microphone response.
-        let calib_n = (anc::CALIB_SWEEP_S * sr_d) as usize;
-        let mut calib_capture: Vec<f32> = Vec::with_capacity(calib_n);
-        let mut calib_sweep: Vec<f32> = Vec::new();
-
-        let mut last_pdm_mode = false;
-
-        println!(
-            "[Dispatch] Pipeline engaged. RX/TX 1 Hz → {:.3} MHz",
-            sr_d / 2.0 / 1e6
-        );
-        if pdm_engine.is_some() {
-            println!(
-                "[Dispatch] PDM wideband: 1 Hz → {:.3} MHz",
-                wideband_sample_rate(sr_d) / 2.0 / 1e6
-            );
+    // ── TDOA thread ───────────────────────────────────────────────────────────
+    let tdoa_state = state.clone();
+    let _tdoa_thread = std::thread::spawn(move || {
+        let mut engine = TdoaEngine::new(device_count, sample_rate, DEFAULT_MIC_SPACING_M);
+        loop {
+            engine.ingest(&tdoa_rx);
+            let beam = engine.compute();
+            tdoa_state.set_beam_azimuth_deg(beam.azimuth_rad.to_degrees());
+            tdoa_state.set_beam_confidence(beam.confidence);
+            std::thread::sleep(std::time::Duration::from_millis(20));
         }
+    });
 
-        while running_dispatch.load(Ordering::Relaxed) {
-            let frame_start = Instant::now();
-
-            // ── ANC calibration trigger ───────────────────────────────────────
-            // Calibration is requested by the UI callback above.  We handle it
-            // here synchronously so we have access to the audio accumulator.
-            if state_d.anc_calibrating.load(Ordering::Relaxed) && calib_sweep.is_empty() {
-                println!("[ANC] Generating calibration sweep...");
-                calib_sweep = AncEngine::calibration_sweep(sr_d);
-                calib_capture = Vec::with_capacity(calib_n);
-                // Push the sweep into the output so it plays through the speaker.
-                if let Ok(mut frames) = state_d.output_frames.lock() {
-                    *frames = calib_sweep.clone();
-                    state_d.output_cursor.store(0, Ordering::Relaxed);
+    // ── Dispatch thread ───────────────────────────────────────────────────────
+    // The dispatch thread owns the mutable GPU engines and the V-buffer write head.
+    // It runs the full frame pipeline on every audio chunk.
+    {
+        let state = state.clone();
+        let vbuf_clone = vbuffer.clone();
+        let trainer_send = trainer_tx.clone();
+        let gpu_shared = gpu_shared.clone(); // Arc clone for V-buffer queue writes inside thread
+        let rt_dispatch = rt.clone();
+        let qdrant_disp = qdrant.clone();
+        let neo4j_disp = neo4j.clone();
+        let forensic_disp = forensic.clone();
+
+        std::thread::spawn(move || {
+            let mut planner = FftPlanner::<f32>::new();
+            let fft = planner.plan_fft_forward(BISPEC_FFT_SIZE);
+            let mut acc = Vec::<f32>::new();
+            let mut frame_idx = 0u64;
+
+            // CPU-side snapshot of V-buffer for trainer windows (one row per frame).
+            // Layout: [V_DEPTH][V_FREQ_BINS]
+            let mut vbuf_snapshot = vec![[0.0f32; V_FREQ_BINS]; V_DEPTH];
+
+            loop {
+                // Drain all pending audio chunks.
+                while let Ok(chunk) = merge_rx.try_recv() {
+                    acc.extend_from_slice(&chunk);
                 }
-                println!("[ANC] Playing {:.1} s sweep...", anc::CALIB_SWEEP_S);
-            }
-
-            // Drain the sample queue with timeout.
-            let batch = match sample_rx.recv_timeout(std::time::Duration::from_millis(10)) {
-                Ok(b) => b,
-                Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
-                Err(_) => break,
-            };
-            accumulator.extend_from_slice(&batch);
-            while let Ok(b) = sample_rx.try_recv() {
-                accumulator.extend_from_slice(&b);
-            }
-            if accumulator.len() < BASEBAND_FFT_SIZE {
-                continue;
-            }
-
-            let drain = accumulator.len().saturating_sub(BASEBAND_FFT_SIZE);
-            accumulator.drain(..drain);
 
-            // Capture mic response during ANC calibration.
-            if !calib_sweep.is_empty() && calib_capture.len() < calib_n {
-                calib_capture.extend_from_slice(
-                    &accumulator[accumulator.len().saturating_sub(batch.len())..],
-                );
-                if calib_capture.len() >= calib_n {
-                    println!("[ANC] Calibration capture complete — analysing...");
-                    anc.calibrate(&calib_sweep, &calib_capture);
-                    state_d.anc_calibrated.store(true, Ordering::Relaxed);
-                    state_d.set_anc_delay_s(anc.calibrator.broadband_delay_s);
-                    state_d.anc_calibrating.store(false, Ordering::Relaxed);
-                    calib_sweep.clear();
-                    println!("[ANC] {}", anc.status());
+                // Also drain SDR magnitude rows and push to V-buffer.
+                while let Ok((mags, _center_hz, _rate)) = sdr_rx.try_recv() {
+                    let mut vb = vbuf_clone.lock();
+                    vb.push_frame(&gpu_shared.queue, &mags);
+                    // Update snapshot for trainer.
+                    let slot = (vb.version() as usize).wrapping_sub(1) % V_DEPTH;
+                    vbuf_snapshot[slot][..mags.len().min(V_FREQ_BINS)]
+                        .copy_from_slice(&mags[..mags.len().min(V_FREQ_BINS)]);
                 }
-            }
 
-            // ── Baseband FFT ──────────────────────────────────────────────────
-            let fft = planner.plan_fft_forward(BASEBAND_FFT_SIZE);
-            let mut buf: Vec<Complex<f32>> = accumulator[accumulator.len() - BASEBAND_FFT_SIZE..]
-                .iter()
-                .enumerate()
-                .map(|(k, &s)| {
-                    let w = 0.5
-                        * (1.0
-                            - (std::f32::consts::TAU * k as f32 / (BASEBAND_FFT_SIZE - 1) as f32)
-                                .cos());
-                    Complex { re: s * w, im: 0.0 }
-                })
-                .collect();
-            fft.process(&mut buf);
-
-            let n_pos = BASEBAND_FFT_SIZE / 2;
-            let magnitudes: Vec<f32> = buf[..n_pos]
-                .iter()
-                .map(|c| (c.re * c.re + c.im * c.im).sqrt())
-                .collect();
-
-            let complex_out: Vec<f32> = buf[..crate::bispectrum::BISPEC_BINS]
-                .iter()
-                .flat_map(|c| [c.re, c.im])
-                .collect();
-
-            let peak_bin = magnitudes[1..]
-                .iter()
-                .enumerate()
-                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
-                .map(|(i, _)| i + 1)
-                .unwrap_or(1);
-            let raw_peak = peak_bin as f32 * sr_d / BASEBAND_FFT_SIZE as f32;
-            let peak_hz = if state_d.auto_tune.load(Ordering::Relaxed) {
-                AppState::snap_to_nearest_note(raw_peak)
-            } else {
-                raw_peak
-            };
-            state_d.set_detected_freq(peak_hz);
-
-            // ── Bispectrum ────────────────────────────────────────────────────
-            if let Some(ref mut bispec) = bispectrum_engine {
-                if complex_out.len() >= crate::bispectrum::FFT_BUFFER_SIZE {
-                    let events = bispec.analyze_frame(
-                        &complex_out[..crate::bispectrum::FFT_BUFFER_SIZE],
-                        sr_d,
-                        crate::detection::HardwareLayer::Microphone,
-                    );
-                    for ev in events {
-                        // DC bin guard: skip detections where any component
-                        // is below 10 Hz — these are bispectrum DC bin artifacts,
-                        // not real intermodulation products.
-                        if ev.f1_hz < 10.0 || ev.f2_hz < 10.0 || ev.product_hz < 10.0 {
-                            continue;
-                        }
-                        let _ = forensic.log_detection(&ev);
-                        if let Some(ref gc) = graph_client {
-                            let ev_c = ev.clone();
-                            let gc_c = gc.clone();
-                            rt.spawn(async move {
-                                let _ = gc_c.store_detection(&ev_c).await;
-                            });
-                        }
-                        if let Some(ref ec) = embed_client {
-                            let ev_c = ev.clone();
-                            let ec_c = ec.clone();
-                            rt.spawn(async move {
-                                let _ = ec_c.store_detection(&ev_c).await;
-                                if let Ok(sim) = ec_c.find_similar(&ev_c, 3).await {
-                                    for s in &sim {
-                                        println!("  [Embed] {}", s.to_display_string());
-                                    }
-                                }
-                            });
-                        }
-                        if state_d.auto_tune.load(Ordering::Relaxed) {
-                            state_d.set_denial_freq_override(ev.product_hz);
-                        }
-                    }
+                if acc.len() < BISPEC_FFT_SIZE {
+                    std::thread::sleep(std::time::Duration::from_millis(1));
+                    continue;
                 }
-            }
 
-            // ── PDM wideband ──────────────────────────────────────────────────
-            let pdm_active = state_d.pdm_active.load(Ordering::Relaxed);
-            let mut wideband_magnitudes: Option<Vec<f32>> = None;
+                let chunk: Vec<f32> = acc.drain(..BISPEC_FFT_SIZE).collect();
+                frame_idx += 1;
+
+                // ── Frame timing ──────────────────────────────────────────────
+                let frame_start = std::time::Instant::now();
+
+                // ── Hann-windowed FFT ─────────────────────────────────────────
+                let n = BISPEC_FFT_SIZE;
+                let mut complex_buf: Vec<Complex<f32>> = chunk
+                    .iter()
+                    .enumerate()
+                    .map(|(i, &s)| {
+                        let w =
+                            0.5 * (1.0 - (std::f32::consts::TAU * i as f32 / (n - 1) as f32).cos());
+                        Complex { re: s * w, im: 0.0 }
+                    })
+                    .collect();
+                fft.process(&mut complex_buf);
+
+                // ── Magnitude spectrum for V-buffer / waterfall ───────────────
+                let mags: Vec<f32> = complex_buf
+                    .iter()
+                    .take(V_FREQ_BINS)
+                    .map(|c| c.norm())
+                    .collect();
+
+                // ── Push to V-buffer ──────────────────────────────────────────
+                let vbuf_version = {
+                    let mut vb = vbuf_clone.lock();
+                    vb.push_frame(&gpu_shared.queue, &mags);
+                    let v = vb.version();
+                    let slot = (v as usize).wrapping_sub(1) % V_DEPTH;
+                    vbuf_snapshot[slot][..mags.len()].copy_from_slice(&mags);
+                    v
+                };
 
-            if pdm_active {
-                if let Some(ref mut pdm) = pdm_engine {
-                    let pcm_window: Vec<f32> = if accumulator.len() >= pdm::PDM_AUDIO_FRAMES {
-                        accumulator[accumulator.len() - pdm::PDM_AUDIO_FRAMES..].to_vec()
-                    } else {
-                        vec![0.0f32; pdm::PDM_AUDIO_FRAMES]
-                    };
-                    let pdm_words = pdm.encode(&pcm_window);
-                    let decoded = pdm.decode(&pdm_words);
-                    let polished = crate::pdm::PdmEngine::cic_decimate_cpu(&decoded);
-                    state_d.set_snr_db(estimate_snr(&pcm_window, &polished));
-                    let wb_samples = pdm.decode_wideband(&pdm_words);
-                    let wb_rate = wideband_sample_rate(sr_d);
-                    let (_, wb_mags) = run_fft_wideband(&wb_samples, wb_rate, &mut planner);
-                    wideband_magnitudes = Some(wb_mags);
+                // ── Auto-tune: snap to nearest note ──────────────────────────
+                if state.auto_tune.load(Ordering::Relaxed) {
+                    let peak_bin = mags
+                        .iter()
+                        .enumerate()
+                        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
+                        .map(|(i, _)| i)
+                        .unwrap_or(0);
+                    let bin_hz = sample_rate / n as f32;
+                    let raw_freq = peak_bin as f32 * bin_hz;
+                    let snapped = AppState::snap_to_nearest_note(raw_freq);
+                    state.set_detected_freq(snapped);
                 }
-            }
 
-            // ── Waterfall ─────────────────────────────────────────────────────
-            let row_source: &Vec<f32> = wideband_magnitudes.as_ref().unwrap_or(&magnitudes);
-            let min_freq = 1.0_f32;
-            let max_freq = if pdm_active {
-                wideband_sample_rate(sr_d) / 2.0
-            } else {
-                sr_d / 2.0
-            };
-            let mid_freq = min_freq * (max_freq / min_freq).sqrt();
-
-            if let Some(ref mut wf) = waterfall_engine {
-                if pdm_active != last_pdm_mode {
-                    wf.set_pdm_mode(pdm_active);
-                    last_pdm_mode = pdm_active;
-                }
-                if !row_source.is_empty() {
-                    let (rgba, spec) = wf.push_row(row_source, min_freq, max_freq);
-                    state_d.update_waterfall(&rgba);
-                    state_d.update_spectrum(&spec);
-                    state_d.push_spectrum_history(&spec);
-
-                    // ── GPU-render the 3D ridge plot ─────────────────
-                    if let Some(ref mut ridge) = ridge_engine {
-                        let hist = state_d.get_spectrum_history();
-                        let n_rows = hist.len();
-                        let flat: Vec<f32> = hist.into_iter().flatten().collect();
-                        let ridge_rgba = ridge.render(&flat, n_rows);
-                        if let Ok(mut buf) = state_d.ridge_rgba.lock() {
-                            *buf = ridge_rgba;
-                        }
-                    }
+                // ── Update SNR ────────────────────────────────────────────────
+                {
+                    let half = mags.len() / 2;
+                    let peak = mags.iter().cloned().fold(0.0f32, f32::max).max(1e-10);
+                    let noise =
+                        mags[half..].iter().cloned().sum::<f32>() / mags[half..].len() as f32;
+                    let snr = 20.0 * (peak / noise.max(1e-10)).log10();
+                    state.set_snr_db(snr.clamp(-20.0, 120.0));
                 }
-            }
 
-            // ── TDOA beamforming ──────────────────────────────────────────────
-            tdoa_engine.ingest(&tdoa_rx);
-            let beam = tdoa_engine.compute();
-            if beam.confidence > 0.1 {
-                state_d.set_beam_azimuth_deg(beam.azimuth_rad.to_degrees());
-                state_d.set_beam_confidence(beam.confidence);
-                if state_d.auto_tune.load(Ordering::Relaxed) {
-                    state_d.set_polarization_angle(beam.azimuth_rad);
+                // ── Waterfall ─────────────────────────────────────────────────
+                let max_freq = if state.pdm_active.load(Ordering::Relaxed) {
+                    pdm::pdm_clock_hz(sample_rate) / 2.0
+                } else {
+                    sample_rate / 2.0
+                };
+                let (rgba, spectrum_bars) = waterfall.push_row(&mags, 1.0, max_freq);
+                state.update_waterfall(&rgba);
+                if let Ok(mut sb) = state.spectrum_bars.lock() {
+                    *sb = spectrum_bars.clone();
                 }
-            }
 
-            // ── GPU synthesis ─────────────────────────────────────────────────
-            gpu.params.mode = state_d.get_mode() as u32;
-            gpu.params.master_gain = state_d.get_master_gain();
-            gpu.params.waveshape = state_d.get_waveshape_mode() as u32;
-            gpu.params.waveshape_drive = state_d.get_waveshape_drive();
-            gpu.params.polarization = state_d.get_polarization_angle();
-            gpu.params.beam_half_width = state_d.get_beam_focus_deg().to_radians();
-
-            let denial_freq = state_d.get_denial_freq();
-
-            match state_d.get_mode() {
-                // ── P0: Anti-phase with physical ANC phase correction ─────────
-                DenialMode::AncAntiPhase => {
-                    let corrected_phase = anc.phase_for(denial_freq);
-                    let tgts = parametric.generate_targets(&[denial_freq], true);
-                    let tgt_pairs: Vec<(f32, f32)> =
-                        tgts.iter().map(|t| (t.freq_hz, t.gain)).collect();
-                    gpu.params.set_targets(&tgt_pairs);
-                    // Override phase on target[0] with ANC-corrected value.
-                    gpu.params.targets[0].phase_offset = corrected_phase;
-                    gpu.params.mode = DenialMode::AntiPhase as u32; // reuse AntiPhase shader path
-                    state_d.set_anc_lms_power(anc.lms.power());
+                // ── PDM encode/decode/wideband ────────────────────────────────
+                if state.pdm_active.load(Ordering::Relaxed) {
+                    let pdm_words = pdm.encode(&chunk);
+                    let decoded = pdm.decode(&pdm_words);
+                    let _snr = snr_db(&chunk, &decoded);
+
+                    // Wideband mode: full PDM clock FFT.
+                    let wide = pdm.decode_wideband(&pdm_words);
+                    let wide_mags: Vec<f32> =
+                        wide.iter().take(V_FREQ_BINS).map(|&s| s.abs()).collect();
+                    let mut vb = vbuf_clone.lock();
+                    vb.push_frame(&gpu_shared.queue, &wide_mags);
+                    let slot = (vb.version() as usize).wrapping_sub(1) % V_DEPTH;
+                    vbuf_snapshot[slot][..wide_mags.len()].copy_from_slice(&wide_mags);
                 }
 
-                // ── Legacy anti-phase (uncorrected — retained for comparison) ──
-                DenialMode::AntiPhase => {
-                    let tgts = parametric.generate_targets(&[denial_freq], true);
-                    gpu.params
-                        .set_targets(&tgts.iter().map(|t| (t.freq_hz, t.gain)).collect::<Vec<_>>());
-                    gpu.params.mode = 1;
+                // ── Bispectrum analysis ───────────────────────────────────────
+                // Pack complex FFT as interleaved [re, im, re, im, ...].
+                let fft_interleaved: Vec<f32> = complex_buf
+                    .iter()
+                    .take(bispectrum::BISPEC_BINS)
+                    .flat_map(|c| [c.re, c.im])
+                    .collect();
+                let events =
+                    bispec.analyze_frame(&fft_interleaved, sample_rate, HardwareLayer::Microphone);
+
+                for event in events {
+                    if let Ok(mut f) = forensic_disp.lock() {
+                        if let Err(e) = f.log_detection(&event) {
+                            eprintln!("[Forensic] Log error: {e}");
+                        }
+                    }
+                    rt_store_async(&rt_dispatch, &qdrant_disp, &neo4j_disp, &event);
+
+                    // Update auto-tune from most-significant detection.
+                    if !state.auto_tune.load(Ordering::Relaxed) {
+                        state.set_detected_freq(event.f1_hz);
+                    }
                 }
 
-                _ => {}
-            }
+                // ── Synthesis ─────────────────────────────────────────────────
+                let denial_freq = state.get_denial_freq();
+                let targets = parametric
+                    .generate_targets(&[denial_freq], state.pdm_active.load(Ordering::Relaxed));
+                let freq_gain_pairs: Vec<(f32, f32)> = targets
+                    .iter()
+                    .map(|t| (t.freq_hz, t.gain * state.get_master_gain()))
+                    .collect();
+                gpu_ctx.params.set_targets(&freq_gain_pairs);
+                gpu_ctx.params.master_gain = state.get_master_gain();
+                gpu_ctx.params.mode = state.mode.load(Ordering::Relaxed);
+                gpu_ctx.params.waveshape = state.waveshape_mode.load(Ordering::Relaxed);
+                gpu_ctx.params.waveshape_drive = state.get_waveshape_drive();
+                gpu_ctx.params.polarization = state.get_polarization_angle();
+                gpu_ctx.params.beam_half_width = state.get_beam_focus_deg().to_radians();
+
+                let synth_out = gpu_ctx.dispatch_synthesis();
+
+                // Peak measure.
+                let peak = synth_out
+                    .iter()
+                    .cloned()
+                    .fold(0.0f32, |a, b| a.abs().max(b.abs()));
+                state.set_output_peak_db(if peak > 1e-10 {
+                    20.0 * peak.log10()
+                } else {
+                    -100.0
+                });
 
-            let mut synthesized = rt.block_on(gpu.dispatch_synthesis_async());
-            // Ensure synthesized is a &[f32] for estimate_snr
-            let synthesized_ref = synthesized.as_slice();
-
-            // ── ANC LMS update (if reference mic is available) ────────────────
-            // Currently uses the primary mic input as the error signal.
-            // When a dedicated error mic is available at the cancellation point,
-            // route it through a separate tdoa_channel and use that signal here.
-            if state_d.anc_calibrated.load(Ordering::Relaxed)
-                && !accumulator.is_empty()
-                && !synthesized.is_empty()
-            {
-                let mic_block = &accumulator[accumulator.len().saturating_sub(synthesized.len())..];
-                let correction = anc.update(&synthesized, mic_block);
-                // Mix correction into output (AncAntiPhase mode only).
-                if state_d.get_mode() == DenialMode::AncAntiPhase {
-                    for (s, c) in synthesized.iter_mut().zip(correction.iter()) {
-                        *s -= c;
-                        *s = s.clamp(-1.0, 1.0);
+                if state.running.load(Ordering::Relaxed) {
+                    if let Ok(mut frames) = state.output_frames.lock() {
+                        *frames = synth_out;
+                    }
+                } else {
+                    if let Ok(mut frames) = state.output_frames.lock() {
+                        frames.fill(0.0);
                     }
                 }
-            }
 
-            // ── Output normalisation to –0.1 dBFS ────────────────────────────
-            let out_peak = synthesized
-                .iter()
-                .cloned()
-                .map(f32::abs)
-                .fold(0.0_f32, f32::max)
-                .max(1e-9);
-            let target_peak = 10.0_f32.powf(-0.1 / 20.0);
-            if out_peak > 1e-6 {
-                let scale = (target_peak / out_peak).min(4.0);
-                for s in synthesized.iter_mut() {
-                    *s *= scale;
+                // ── Trainer: send context window every 4 frames ───────────────
+                if frame_idx % 4 == 0 {
+                    let ctx_len = MAMBA_CONTEXT_LEN;
+                    let write_ver = vbuf_version;
+                    let window =
+                        extract_window_from_vbuf_snapshot(&vbuf_snapshot, write_ver, ctx_len);
+                    let _ = trainer_send.try_send(TrainerCmd::PushWindow(window));
                 }
-            }
-            state_d.set_output_peak_db(20.0 * out_peak.log10());
 
-            if let Ok(mut frames) = state_d.output_frames.lock() {
-                if frames.len() != synthesized.len() {
-                    *frames = vec![0.0; synthesized.len()];
-                }
-                frames.copy_from_slice(&synthesized);
-                state_d.output_cursor.store(0, Ordering::Relaxed);
-            }
+                // ── Frame timing update ───────────────────────────────────────
+                let elapsed_us = frame_start.elapsed().as_micros() as u32;
+                state.set_dispatch_us(elapsed_us);
+                state.inc_frame_count();
 
-            // ── Frame timing ──────────────────────────────────────────────────
-            let elapsed_us = frame_start.elapsed().as_micros() as u32;
-            state_d.set_dispatch_us(elapsed_us);
-            state_d.inc_frame_count();
-            if state_d.get_frame_count() % 200 == 0 {
-                println!(
-                    "[Timing] frame #{} — dispatch {:.1} ms",
-                    state_d.get_frame_count(),
-                    elapsed_us as f32 / 1000.0
-                );
+                // UI is updated by the 60Hz timer in main() reading shared AtomicF32/Mutex state.
             }
+        });
+    }
 
-            // ── Training Module Tick ──────────────────────────────────────────
-            if state_d.rtl_connected.load(Ordering::Relaxed) && !training.is_connected() {
-                rt.block_on(training.connect());
-            } else if !state_d.rtl_connected.load(Ordering::Relaxed) && training.is_connected() {
-                training.disconnect();
-            }
+    // ── UI callbacks ──────────────────────────────────────────────────────────
+    wire_ui_callbacks(&ui, &state, &trainer_tx);
 
-            if training.is_connected() && state_d.rtl_scanning.load(Ordering::Relaxed) {
-                training.scan_range(90_000_000, 110_000_000); // 90 to 110 MHz
-            }
+    // ── UI refresh timer ──────────────────────────────────────────────────────
+    {
+        let ui_weak = ui.as_weak();
+        let state = state.clone();
+        let vbuf = vbuffer.clone();
+
+        // Loss history ring: last 64 normalised loss values.
+        let mut loss_ring: std::collections::VecDeque<f32> =
+            std::collections::VecDeque::with_capacity(64);
+
+        let timer = slint::Timer::default();
+        timer.start(
+            slint::TimerMode::Repeated,
+            std::time::Duration::from_millis(16),
+            move || {
+                let Some(ui) = ui_weak.upgrade() else { return };
+
+                // ── Monitor tab ──────────────────────────────────────────────
+                ui.set_detected_freq(state.get_detected_freq());
+                ui.set_is_running(state.running.load(Ordering::Relaxed));
+                ui.set_current_mode(state.mode.load(Ordering::Relaxed) as i32);
+                ui.set_auto_tune_active(state.auto_tune.load(Ordering::Relaxed));
+                ui.set_master_gain(state.get_master_gain());
+                ui.set_pdm_active(state.pdm_active.load(Ordering::Relaxed));
+                ui.set_pdm_clock_mhz(
+                    pdm::pdm_clock_hz(state.pdm_clock_mhz.load(Ordering::Relaxed)) / 1e6,
+                );
+                ui.set_oversample_ratio(state.oversample_ratio.load(Ordering::Relaxed) as i32);
+                ui.set_pdm_snr_db(state.get_snr_db());
+                ui.set_waveshape_mode(state.waveshape_mode.load(Ordering::Relaxed) as i32);
+                ui.set_waveshape_drive(state.get_waveshape_drive());
+                ui.set_input_device_count(state.input_device_count.load(Ordering::Relaxed) as i32);
+                ui.set_beam_azimuth_deg(state.get_beam_azimuth_deg());
+                ui.set_beam_confidence(state.get_beam_confidence());
+                ui.set_beam_focus_deg(state.get_beam_focus_deg());
+                ui.set_agc_gain_db(state.get_agc_gain_db());
+                ui.set_agc_peak_dbfs(state.get_agc_peak_dbfs());
+                ui.set_output_peak_db(state.get_output_peak_db());
+
+                // Waterfall
+                if let Ok(wf) = state.waterfall_rgba.lock() {
+                    let sr = state.pdm_clock_mhz.load(Ordering::Relaxed) * 1e6;
+                    let max_hz = if state.pdm_active.load(Ordering::Relaxed) {
+                        sr / 2.0
+                    } else {
+                        sr / 2.0 / 64.0
+                    };
+                    ui.set_waterfall_max_freq(
+                        if max_hz >= 1e6 {
+                            format!("{:.2} MHz", max_hz / 1e6)
+                        } else {
+                            format!("{:.1} kHz", max_hz / 1e3)
+                        }
+                        .into(),
+                    );
+                    ui.set_waterfall_mid_freq(
+                        if max_hz / 2.0 >= 1e6 {
+                            format!("{:.2} MHz", max_hz / 2e6)
+                        } else {
+                            format!("{:.1} kHz", max_hz / 2e3)
+                        }
+                        .into(),
+                    );
 
-            if training.is_connected() {
-                rt.block_on(training.tick());
-                if let Some(spec) = training.get_spectrum() {
-                    if let Ok(mut lock) = state_d.rtl_spectrum.lock() {
-                        lock.copy_from_slice(&spec);
-                    }
+                    let pixels: slint::ModelRc<slint::Color> =
+                        slint::ModelRc::new(slint::VecModel::from(
+                            wf.iter()
+                                .map(|&rgba| {
+                                    let r = (rgba & 0xFF) as u8;
+                                    let g = ((rgba >> 8) & 0xFF) as u8;
+                                    let b = ((rgba >> 16) & 0xFF) as u8;
+                                    slint::Color::from_rgb_u8(r, g, b)
+                                })
+                                .collect::<Vec<_>>(),
+                        ));
+                    ui.set_waterfall_pixels(pixels);
                 }
-                state_d
-                    .rtl_center_freq_hz
-                    .store(training.center_freq_hz, Ordering::Relaxed);
-                state_d
-                    .rtl_sample_rate
-                    .store(training.sample_rate, Ordering::Relaxed);
-
-                // Collect a pair if training is active and PDM generated a valid window
-                if state_d.training_active.load(Ordering::Relaxed) {
-                    if let Some(ref wb) = wideband_magnitudes {
-                        // Assuming RTL-SDR magnitude resolution matches baseband for simplicity
-                        // Need the latest RTL-SDR spectrum and the current PDM wideband spectrum
-                        if let Some(rx_spec) = training.get_spectrum() {
-                            training.collect_pair(wb, &rx_spec);
-                            state_d
-                                .training_pairs_collected
-                                .store(training.pairs_collected(), Ordering::Relaxed);
-                        }
-                    }
+
+                // Spectrum bars
+                if let Ok(sb) = state.spectrum_bars.lock() {
+                    ui.set_spectrum_bars(slint::ModelRc::new(slint::VecModel::from(sb.clone())));
                 }
-            }
-        }
 
-        println!(
-            "[Dispatch] Exit. Forensic events: {}",
-            forensic.event_count()
+                // ── Training tab ─────────────────────────────────────────────
+                ui.set_mamba_anomaly(state.get_mamba_anomaly());
+                ui.set_training_active(state.training_active.load(Ordering::Relaxed));
+                ui.set_train_epoch(state.train_epoch.load(Ordering::Relaxed) as i32);
+                ui.set_train_loss(state.get_train_loss());
+                ui.set_replay_buf_len(state.replay_buf_len.load(Ordering::Relaxed) as i32);
+
+                // Dispatch timing
+                ui.set_dispatch_ms(state.get_dispatch_us() as f32 / 1000.0);
+                ui.set_frame_count(state.get_frame_count() as i32);
+
+                // Loss history (normalised to 0..1 for sparkline).
+                let loss = state.get_train_loss();
+                if state.training_active.load(Ordering::Relaxed) && loss > 0.0 {
+                    loss_ring.push_back(loss);
+                    if loss_ring.len() > 64 {
+                        loss_ring.pop_front();
+                    }
+                }
+                let loss_max = loss_ring.iter().cloned().fold(1e-6f32, f32::max);
+                let loss_norm: Vec<f32> = loss_ring.iter().map(|&v| v / loss_max).collect();
+                ui.set_loss_history(slint::ModelRc::new(slint::VecModel::from(loss_norm)));
+
+                // ── SDR tab ───────────────────────────────────────────────────
+                ui.set_sdr_active(state.sdr_active.load(Ordering::Relaxed));
+                ui.set_sdr_center_mhz(state.get_sdr_center_hz() / 1e6);
+                ui.set_sdr_gain_db(state.get_sdr_gain_db());
+                ui.set_sdr_peak_dbfs(state.get_sdr_peak_dbfs());
+                ui.set_sdr_peak_offset_khz(state.get_sdr_peak_offset_hz() / 1e3);
+            },
         );
-        println!("[Dispatch] Log: {}", forensic.log_path().display());
-    });
 
-    // ── UI PULL TIMER (Slint native reactivity — vsync-locked via request_redraw) ──
-    // Rendering strategy:
-    //   Spectrum  : Three SVG filled-path strings (green/cyan/red bands) built from
-    //               256 log-spaced bins each frame. FemtoVG renders each as a single
-    //               anti-aliased filled polygon — resolution-independent at any zoom.
-    //   Waterfall : SharedPixelBuffer → Image, bilinearly scaled by FemtoVG.
-    //               256×128 source gives smooth gradients at any window size.
-    //   Both replace per-element draw calls with single GPU path/texture operations.
-    let ui_weak = ui.as_weak();
-    let state_ui = state.clone();
-    let timer = slint::Timer::default();
-
-    // Ridge plot dimensions for the Slint pixel buffer.
-    let ridge_w = ridge_plot::RIDGE_W;
-    let ridge_h = ridge_plot::RIDGE_H;
-
-    // Pre-allocate SVG path string buffers — avoids heap allocation each frame.
-    // Each path: "M x0 y0 L x1 y1 ... L xN 100 L x0 100 Z"
-    // 256 bins × ~18 chars/point + overhead ≈ 5 KB per path, 15 KB total.
-    let mut path_green = String::with_capacity(6000);
-    let mut path_cyan = String::with_capacity(6000);
-    let mut path_red = String::with_capacity(6000);
-
-    // Helper: format a frequency value as "X.X Hz", "X.X kHz", or "X.XX MHz"
-    fn fmt_freq(hz: f32) -> String {
-        if hz >= 1_000_000.0 {
-            format!("{:.3} MHz", hz / 1_000_000.0)
-        } else if hz >= 1_000.0 {
-            format!("{:.1} kHz", hz / 1_000.0)
-        } else {
-            format!("{:.0} Hz", hz)
-        }
+        std::mem::forget(timer); // keep timer alive
     }
 
-    timer.start(
-        slint::TimerMode::Repeated,
-        std::time::Duration::from_millis(16),
-        move || {
-            if let Some(ui) = ui_weak.upgrade() {
-                // ── Scalar atomics ────────────────────────────────────────────────────
-                ui.set_detected_freq(state_ui.get_detected_freq());
-                ui.set_is_running(state_ui.running.load(Ordering::Relaxed));
-                ui.set_current_mode(state_ui.get_mode() as i32);
-                ui.set_auto_tune_active(state_ui.auto_tune.load(Ordering::Relaxed));
-                ui.set_master_gain(state_ui.get_master_gain());
-                let pdm_on = state_ui.pdm_active.load(Ordering::Relaxed);
-                ui.set_pdm_active(pdm_on);
-                ui.set_pdm_clock_mhz(state_ui.get_pdm_clock_mhz());
-                ui.set_oversample_ratio(state_ui.oversample_ratio.load(Ordering::Relaxed) as i32);
-                ui.set_pdm_snr_db(state_ui.get_snr_db());
-                ui.set_waveshape_mode(state_ui.waveshape_mode.load(Ordering::Relaxed) as i32);
-                ui.set_waveshape_drive(state_ui.get_waveshape_drive());
-                ui.set_input_device_count(
-                    state_ui.input_device_count.load(Ordering::Relaxed) as i32
-                );
-                ui.set_beam_azimuth_deg(state_ui.get_beam_azimuth_deg());
-                ui.set_beam_confidence(state_ui.get_beam_confidence());
-                ui.set_beam_focus_deg(state_ui.get_beam_focus_deg());
-
-                let agc_gain = state_ui.get_agc_gain_db();
-                ui.set_agc_gain_db(agc_gain);
-                ui.set_agc_peak_dbfs(state_ui.get_agc_peak_dbfs());
-                ui.set_output_peak_db(state_ui.get_output_peak_db());
-                ui.set_snl_db((agc_gain + 18.0 + 72.0).clamp(0.0, 108.0));
-                ui.set_dispatch_ms(state_ui.get_dispatch_us() as f32 / 1000.0);
-                ui.set_frame_count(state_ui.get_frame_count() as i32);
-
-                // ── Training State ──────────────────────────────────────────────────
-                ui.set_training_active(state_ui.training_active.load(Ordering::Relaxed));
-                ui.set_rtl_freq_mhz(
-                    state_ui.rtl_center_freq_hz.load(Ordering::Relaxed) as f32 / 1_000_000.0,
-                );
-                ui.set_rtl_scanning(state_ui.rtl_scanning.load(Ordering::Relaxed));
-                ui.set_rtl_connected(state_ui.rtl_connected.load(Ordering::Relaxed));
-                ui.set_training_pairs(
-                    state_ui.training_pairs_collected.load(Ordering::Relaxed) as i32
-                );
-                ui.set_training_loss(state_ui.get_training_loss());
-                ui.set_training_epoch(state_ui.training_epoch.load(Ordering::Relaxed) as i32);
+    // ── Run UI ────────────────────────────────────────────────────────────────
+    state.running.store(true, Ordering::Relaxed);
+    ui.run().context("Slint run failed")?;
+    println!("[SIREN] UI closed. Shutting down.");
+    let _ = trainer_tx.send(TrainerCmd::Stop);
+    Ok(())
+}
 
-                let anc_cal = state_ui.anc_calibrated.load(Ordering::Relaxed);
-                let anc_delay = state_ui.get_anc_delay_s();
-                ui.set_anc_status(if anc_cal {
-                    format!("ANC cal OK — delay {:.2}ms", anc_delay * 1e3).into()
-                } else {
-                    slint::SharedString::from("ANC uncalibrated")
-                });
+// ── UI callback wiring ────────────────────────────────────────────────────────
+
+fn wire_ui_callbacks(
+    ui: &AppWindow,
+    state: &Arc<AppState>,
+    trainer_tx: &crossbeam_channel::Sender<TrainerCmd>,
+) {
+    // Monitor
+    let s = state.clone();
+    ui.on_set_mode(move |m| {
+        s.mode.store(m as u32, Ordering::Relaxed);
+    });
 
-                // ── Frequency axis labels — update with PDM/baseband mode ─────────────
-                // PDM wideband:   1 Hz → 6.144 MHz (pdm_clock / 2)
-                // Baseband:       1 Hz → sample_rate / 2  (typically 96 kHz)
-                let max_freq_hz = if pdm_on {
-                    state_ui.get_pdm_clock_mhz() * 1_000_000.0 / 2.0
-                } else {
-                    // Baseband Nyquist — pdm_clock is set from sample_rate×oversample,
-                    // divide back out: sample_rate = pdm_clock / oversample
-                    let sr = state_ui.get_pdm_clock_mhz() * 1_000_000.0
-                        / state_ui.oversample_ratio.load(Ordering::Relaxed) as f32;
-                    sr / 2.0
-                };
-                let mid_freq_hz = 1.0_f32 * (max_freq_hz / 1.0_f32).sqrt(); // geometric midpoint
-                ui.set_waterfall_max_freq(fmt_freq(max_freq_hz).into());
-                ui.set_waterfall_mid_freq(fmt_freq(mid_freq_hz).into());
-
-                // ── Spectrum → SVG filled paths ───────────────────────────────────────
-                // The GPU outputs 256 bins already log-mapped (waterfall.rs bin_to_raw_idx).
-                // We plot them linearly across x=0..1000, y=100-frac*100 (top=loud).
-                // Three separate closed paths: green (bins 0-85), cyan (86-170), red (171-255).
-                // Each path starts at the left baseline, traces the amplitude contour, then
-                // drops back to the baseline and closes — forming a filled silhouette.
-                if let Ok(spec) = state_ui.gpu_spectrum.try_lock() {
-                    if spec.len() >= 256 {
-                        path_green.clear();
-                        path_cyan.clear();
-
-                        // x step: 1000 / 256 ≈ 3.906 per bin
-                        const N: usize = 256;
-                        const X_SCALE: f32 = 1000.0 / N as f32;
-
-                        // Green: bins 0..86
-                        path_green.push_str("M 0 100");
-                        for i in 0..86usize {
-                            let x = i as f32 * X_SCALE;
-                            let y = 100.0 - spec[i].clamp(0.0, 1.0) * 100.0;
-                            let _ = write!(path_green, " L {:.1} {:.1}", x, y);
-                        }
-                        let _ = write!(path_green, " L {:.1} 100 Z", 85.0 * X_SCALE);
-
-                        // Cyan: bins 86..171
-                        let _ = write!(path_cyan, "M {:.1} 100", 86.0 * X_SCALE);
-                        for i in 86..171usize {
-                            let x = i as f32 * X_SCALE;
-                            let y = 100.0 - spec[i].clamp(0.0, 1.0) * 100.0;
-                            let _ = write!(path_cyan, " L {:.1} {:.1}", x, y);
-                        }
-                        let _ = write!(path_cyan, " L {:.1} 100 Z", 170.0 * X_SCALE);
-
-                        // Red: bins 171..256
-                        let _ = write!(path_red, "M {:.1} 100", 171.0 * X_SCALE);
-                        for i in 171..N {
-                            let x = i as f32 * X_SCALE;
-                            let y = 100.0 - spec[i].clamp(0.0, 1.0) * 100.0;
-                            let _ = write!(path_red, " L {:.1} {:.1}", x, y);
-                        }
-                        let _ = write!(path_red, " L 1000 100 Z");
+    let s = state.clone();
+    ui.on_set_gain(move |g| {
+        s.set_master_gain(g);
+    });
 
-                        ui.set_spectrum_path_green(path_green.as_str().into());
-                        ui.set_spectrum_path_cyan(path_cyan.as_str().into());
-                        ui.set_spectrum_path_red(path_red.as_str().into());
-                    }
-                }
+    let s = state.clone();
+    ui.on_set_freq_override(move |f| {
+        s.set_denial_freq_override(f);
+    });
 
-                // ── Waterfall → 3D Ridge Plot (GPU-rendered, dispatch thread) ────────
-                if let Ok(rgba_buf) = state_ui.ridge_rgba.try_lock() {
-                    let n_pixels = (ridge_w * ridge_h) as usize;
-                    if rgba_buf.len() >= n_pixels {
-                        let mut wf_pixels = SharedPixelBuffer::<Rgba8Pixel>::new(ridge_w, ridge_h);
-                        let dst = wf_pixels.make_mut_slice();
-                        for (i, px) in dst.iter_mut().enumerate() {
-                            let src = rgba_buf[i];
-                            // Unpack u32 RGBA (R in LSB).
-                            px.r = (src & 0xFF) as u8;
-                            px.g = ((src >> 8) & 0xFF) as u8;
-                            px.b = ((src >> 16) & 0xFF) as u8;
-                            px.a = 255;
-                        }
-                        ui.set_waterfall_image(Image::from_rgba8(wf_pixels));
-                    }
-                }
+    let s = state.clone();
+    ui.on_toggle_auto_tune(move || {
+        let prev = s.auto_tune.load(Ordering::Relaxed);
+        s.auto_tune.store(!prev, Ordering::Relaxed);
+    });
 
-                // Request repaint — Windows VBlank-locked via winit.
-                ui.window().request_redraw();
-            }
-        },
-    );
+    let s = state.clone();
+    ui.on_toggle_running(move || {
+        let prev = s.running.load(Ordering::Relaxed);
+        s.running.store(!prev, Ordering::Relaxed);
+    });
 
-    ui.run()?;
-    running_flag.store(false, Ordering::Relaxed);
-    let _ = dispatch_handle.join();
-    println!("[Main] Clean shutdown.");
-    Ok(())
-}
+    let s = state.clone();
+    ui.on_toggle_pdm(move || {
+        let prev = s.pdm_active.load(Ordering::Relaxed);
+        s.pdm_active.store(!prev, Ordering::Relaxed);
+    });
 
-// ── Wideband FFT ──────────────────────────────────────────────────────────────
+    let s = state.clone();
+    ui.on_set_waveshape(move |m| {
+        s.waveshape_mode.store(m as u32, Ordering::Relaxed);
+    });
 
-fn run_fft_wideband(
-    samples: &[f32],
-    sample_rate: f32,
-    planner: &mut FftPlanner<f32>,
-) -> (f32, Vec<f32>) {
-    let fft_size = samples.len().next_power_of_two().min(WIDEBAND_FRAMES);
-    if samples.len() < fft_size {
-        return (0.0, Vec::new());
-    }
+    let s = state.clone();
+    ui.on_set_waveshape_drive(move |d| {
+        s.set_waveshape_drive(d);
+    });
 
-    let fft = planner.plan_fft_forward(fft_size);
-    let mut buf: Vec<Complex<f32>> = samples[..fft_size]
-        .iter()
-        .enumerate()
-        .map(|(k, &s)| {
-            let w = 0.5 * (1.0 - (std::f32::consts::TAU * k as f32 / (fft_size - 1) as f32).cos());
-            Complex { re: s * w, im: 0.0 }
-        })
-        .collect();
-    fft.process(&mut buf);
-
-    let n_pos = fft_size / 2;
-    let mags: Vec<f32> = buf[..n_pos]
-        .iter()
-        .map(|c| (c.re * c.re + c.im * c.im).sqrt())
-        .collect();
+    let s = state.clone();
+    ui.on_set_beam_focus(move |f| {
+        s.set_beam_focus_deg(f);
+    });
 
-    let peak_bin = mags[1..]
-        .iter()
-        .enumerate()
-        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
-        .map(|(i, _)| i + 1)
-        .unwrap_or(1);
-    let peak_hz = peak_bin as f32 * (sample_rate / fft_size as f32);
-    (peak_hz, mags)
-}
+    // ANC calibration
+    let s = state.clone();
+    ui.on_anc_calibrate(move || {
+        s.anc_calibrated.store(true, Ordering::Relaxed);
+        println!("[ANC] Calibration triggered");
+    });
 
-// ── Helpers ───────────────────────────────────────────────────────────────────
+    // Training
+    let s = state.clone();
+    let tx = trainer_tx.clone();
+    ui.on_toggle_training(move || {
+        let prev = s.training_active.load(Ordering::Relaxed);
+        let next = !prev;
+        s.training_active.store(next, Ordering::Relaxed);
+        let _ = tx.try_send(TrainerCmd::SetActive(next));
+        println!(
+            "[Trainer] Training {}",
+            if next { "started" } else { "paused" }
+        );
+    });
 
-fn estimate_snr(original: &[f32], decoded: &[f32]) -> f32 {
-    let n = original.len().min(decoded.len());
-    if n == 0 {
-        return 0.0;
-    }
-    let sig: f32 = original[..n].iter().map(|x| x * x).sum::<f32>() / n as f32;
-    let nse: f32 = original[..n]
-        .iter()
-        .zip(&decoded[..n])
-        .map(|(a, b)| (a - b).powi(2))
-        .sum::<f32>()
-        / n as f32;
-    if nse < 1e-12 {
-        return 100.0;
-    }
-    10.0 * (sig / nse).log10()
+    // RTL-SDR (old UI callbacks)
+    let s = state.clone();
+    ui.on_rtl_connect(move || {
+        println!("[RTL-SDR] Connect requested");
+    });
+
+    let s = state.clone();
+    ui.on_rtl_start_scan(move || {
+        s.sdr_active.store(true, Ordering::Relaxed);
+        println!("[RTL-SDR] Scan started");
+    });
+
+    let s = state.clone();
+    ui.on_rtl_stop_scan(move || {
+        s.sdr_active.store(false, Ordering::Relaxed);
+        println!("[RTL-SDR] Scan stopped");
+    });
 }
 
-fn chrono_session_id() -> String {
-    use std::time::{SystemTime, UNIX_EPOCH};
-    let secs = SystemTime::now()
-        .duration_since(UNIX_EPOCH)
-        .unwrap_or_default()
-        .as_secs();
-    format!("session_{}", secs)
+// ── Async DB store ────────────────────────────────────────────────────────────
+
+fn rt_store_async(
+    rt: &Arc<tokio::runtime::Runtime>,
+    qdrant: &Arc<Option<embeddings::EmbeddingStore>>,
+    neo4j: &Arc<Option<graph::ForensicGraph>>,
+    event: &DetectionEvent,
+) {
+    if let Some(store) = qdrant.as_ref() {
+        let ev = event.clone();
+        rt.block_on(async {
+            if let Err(e) = store.store_detection(&ev).await {
+                eprintln!("[Qdrant] Store error: {e}");
+            }
+        });
+    }
+    if let Some(g) = neo4j.as_ref() {
+        let ev = event.clone();
+        rt.block_on(async {
+            if let Err(e) = g.store_detection(&ev).await {
+                eprintln!("[Neo4j] Store error: {e}");
+            }
+        });
+    }
 }
 
-// ── GPU device factory ────────────────────────────────────────────────────────
-// NOTE: GpuShared (gpu_device.rs) is the long-term singleton target.
-// These four devices still each create their own wgpu::Device until the engine
-// constructors are migrated to accept Arc<GpuShared> (mechanical, one-time task).
+// ── SNR helper ────────────────────────────────────────────────────────────────
 
-fn make_gpu_device(label: &'static str) -> Option<(wgpu::Device, wgpu::Queue)> {
-    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
-        backends: wgpu::Backends::DX12 | wgpu::Backends::VULKAN,
-        ..Default::default()
-    });
-    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
-        power_preference: wgpu::PowerPreference::HighPerformance,
-        compatible_surface: None,
-        force_fallback_adapter: false,
-    }))
-    .ok()?;
-    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
-        label: Some(label),
-        required_features: wgpu::Features::empty(),
-        required_limits: wgpu::Limits::default(),
-        ..Default::default()
-    }))
-    .ok()
+fn snr_db(original: &[f32], decoded: &[f32]) -> f32 {
+    let sig_power: f32 = original.iter().map(|s| s * s).sum::<f32>() / original.len() as f32;
+    let err_power: f32 = original
+        .iter()
+        .zip(decoded.iter())
+        .map(|(o, d)| (o - d).powi(2))
+        .sum::<f32>()
+        / original.len() as f32;
+    if err_power < 1e-12 {
+        return 120.0;
+    }
+    10.0 * (sig_power / err_power).log10()
 }
