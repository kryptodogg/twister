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

use anyhow::Context;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::state::AppState;
use crate::dispatch::audio_ingester::AudioIngester;
use crate::dispatch::rf_ingester::RFIngester;
use crate::dispatch::visual_ingester::VisualIngester;
use crate::dispatch::signal_ingester::{SignalMetadata, SignalType, SampleFormat};
use crate::dispatch::het_synthesizer::HetSynthesizer;
use crate::dispatch::backend::{FileBackend, AudioBackend};
use crate::utils::latency::QpcTimer;
use crate::ml::waveshape_projection::project_latent_to_waveshape;
use crate::ml::field_particle::FieldParticle;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let timer = Arc::new(QpcTimer::new());
    let state = Arc::new(AppState::new());

    let session_identity = format!("twister_{:016x}", timer.now_us());
    state.log("INFO", "System", &format!("[Twister v0.5] Session: {}", session_identity));

    let ui = self::TotoCard::new().context("Slint window creation failed")?;

    let audio_ingester = Arc::new(AudioIngester::new());
    let _rf_ingester = Arc::new(RFIngester::new());
    let _visual_ingester = Arc::new(VisualIngester::new());
    let het_synth = Arc::new(tokio::sync::Mutex::new(HetSynthesizer::new()));

    {
        let mut hs = het_synth.lock().await;
        let session_ts = timer.now_us();
        let _ = std::fs::create_dir_all("forensic");
        if let Ok(file_backend) = FileBackend::new(&format!("forensic/session_{}.pcm", session_ts)) {
            hs.add_backend(Box::new(file_backend));
        }
        hs.add_backend(Box::new(AudioBackend::new("Default")));
    }

    let state_disp = state.clone();
    let audio_ing_disp = audio_ingester.clone();
    let timer_disp = timer.clone();
    let het_synth_disp = het_synth.clone();
    let mamba_trainer = Arc::new(crate::training::MambaTrainer::new(state.clone())?);

    tokio::spawn(async move {
        loop {
            let _ts = timer_disp.now_us();
            let audio_raw: Vec<u8> = Vec::new(); // Real buffers only
            if !audio_raw.is_empty() {
                let meta = SignalMetadata {
                    signal_type: SignalType::Audio,
                    sample_rate_hz: 44100,
                    carrier_freq_hz: None,
                    num_channels: 1,
                    sample_format: SampleFormat::F32,
                };

                if let Some(accumulated) = audio_ing_disp.accumulate(&audio_raw, &meta) {
                    if let Ok((anomaly, latent, _recon)) = mamba_trainer.forward(&accumulated).await {
                        let params = project_latent_to_waveshape(&latent, 44100.0);
                        state_disp.waveshape_drive.store(params.drive, Ordering::Relaxed);
                        state_disp.mamba_anomaly_score.store(anomaly, Ordering::Relaxed);

                        let mut hs = het_synth_disp.lock().await;
                        hs.generate_samples(512, 44100.0);
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    });

    ui.run()?;
    Ok(())
}

pub mod dispatch;
pub mod rtlsdr_ffi;
pub mod safe_sdr_wrapper;
pub mod tuner;
pub mod pdm_utils;

fn snr_db(original: &[f32], decoded: &[f32]) -> f32 {
    let sp: f32 = original.iter().map(|s| s * s).sum::<f32>() / original.len() as f32;
    let ep: f32 = original.iter().zip(decoded.iter()).map(|(o, d)| (o - d).powi(2)).sum::<f32>() / original.len() as f32;
    if ep < 1e-12 { return 120.0; }
    10.0 * (sp / ep).log10()
}
