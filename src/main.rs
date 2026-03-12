pub mod active_denial;
slint::include_modules!();
mod af32;
mod ai;
mod dsp;
mod anc;
mod anc_calibration;
mod anc_recording;
mod computer_vision;
mod app_state;
mod bispectrum;
mod forensic;
mod gpu;
mod gpu_shared;
mod graph;
mod harmony;
mod hardware;
mod hardware_io;
mod knowledge_graph;
mod mamba;
mod ml;
mod pdm;
mod pipeline;
mod reconstruct;
mod sdr;
mod spatial;
mod state;
mod training;
mod twister;
mod vbuffer;
mod waterfall;
mod utils;
pub mod dispatch;
pub mod ui;
pub mod rtlsdr_ffi;
pub mod safe_sdr_wrapper;
pub mod tuner;
pub mod pdm_utils;

use anyhow::Context;
use std::sync::Arc;
use crate::state::AppState;
use crate::utils::latency::QpcTimer;
use crate::dispatch::{AudioIngester, start_dispatch_loop, het_synthesizer::HetSynthesizer};
use crate::dispatch::backend::{FileBackend, AudioBackend};
use crate::ml::PoseEstimator;
use crate::training::MambaTrainer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let timer = Arc::new(QpcTimer::new());
    let state = Arc::new(AppState::new());

    let session_identity = format!("twister_{:016x}", timer.now_us());
    state.log("INFO", "System", &format!("[Twister v0.5] Session: {}", session_identity));

    let ui = self::TotoCard::new().context("Slint window creation failed")?;

    // ── INITIALIZATION ────────────────────────────────────────────────────────
    let audio_ingester = Arc::new(AudioIngester::new());
    let het_synth = Arc::new(tokio::sync::Mutex::new(HetSynthesizer::new()));
    let mamba_trainer = Arc::new(MambaTrainer::new(state.clone())?);
    let pose_estimator = Arc::new(PoseEstimator::<burn_ndarray::NdArray<f32>>::new(
        burn::backend::ndarray::NdArrayDevice::Cpu
    ));

    // Wire Backends
    {
        let mut hs = het_synth.lock().await;
        let _ = std::fs::create_dir_all("forensic");
        if let Ok(file_backend) = FileBackend::new(&format!("forensic/session_{}.pcm", timer.now_us())) {
            hs.add_backend(Box::new(file_backend));
        }
        hs.add_backend(Box::new(AudioBackend::new("Default")));
    }

    // ── SPAWN DISPATCH ────────────────────────────────────────────────────────
    tokio::spawn(start_dispatch_loop(
        state.clone(),
        timer.clone(),
        mamba_trainer,
        het_synth.clone(),
        audio_ingester,
        pose_estimator,
    ));

    ui.run()?;
    Ok(())
}
