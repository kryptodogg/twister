pub mod stream_packer;
pub use stream_packer::GpuStreamPacker;

pub mod signal_ingester;
pub use signal_ingester::{SignalIngester, SignalMetadata, SignalType, SampleFormat};

pub mod audio_ingester;
pub use audio_ingester::AudioIngester;

pub mod rf_ingester;
pub use rf_ingester::RFIngester;

pub mod visual_ingester;
pub use visual_ingester::VisualIngester;

pub mod backend;
pub mod het_synthesizer;

use std::sync::Arc;
use tokio::sync::Mutex;
use crate::state::AppState;
use crate::utils::latency::QpcTimer;
use crate::ml::pose_estimator::PoseEstimator;
use crate::training::MambaTrainer;
use crate::ml::waveshape_projection::project_latent_to_waveshape;
use std::sync::atomic::Ordering;

/// Primary Forensic Dispatch Loop
pub async fn start_dispatch_loop(
    state: Arc<AppState>,
    timer: Arc<QpcTimer>,
    mamba_trainer: Arc<MambaTrainer>,
    het_synth: Arc<Mutex<crate::dispatch::het_synthesizer::HetSynthesizer>>,
    audio_ingester: Arc<AudioIngester>,
    pose_estimator: Arc<PoseEstimator<burn_ndarray::NdArray<f32>>>,
) {
    loop {
        let ts = timer.now_us();

        let audio_raw: Vec<u8> = Vec::new();
        if !audio_raw.is_empty() {
            let meta = SignalMetadata {
                signal_type: SignalType::Audio,
                sample_rate_hz: 44100,
                carrier_freq_hz: None,
                num_channels: 1,
                sample_format: SampleFormat::F32,
            };

            if let Some(accumulated) = audio_ingester.accumulate(&audio_raw, &meta) {
                if let Ok((anomaly, latent, _)) = mamba_trainer.forward(&accumulated).await {
                    let params = project_latent_to_waveshape(&latent, 44100.0);
                    state.waveshape_drive.store(params.drive, Ordering::Relaxed);
                    state.mamba_anomaly_score.store(anomaly, Ordering::Relaxed);

                    let mut hs = het_synth.lock().await;
                    hs.generate_samples(512, 44100.0);
                }
            }
        }

        let visual_raw: Vec<u8> = Vec::new();
        if !visual_raw.is_empty() {
             let _keypoints = pose_estimator.estimate(&visual_raw, 128, 128);
        }

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

pub fn generate_density_sparkle(particles: &[crate::ml::field_particle::FieldParticle]) -> String {
    let mut path = String::new();
    for p in particles.iter().take(64) {
        let x = (p.position[0] * 320.0).clamp(0.0, 320.0);
        let y = (p.position[1] * 180.0).clamp(0.0, 180.0);
        path.push_str(&format!("M {:.1} {:.1} L {:.1} {:.1} ", x, y, x + 1.0, y + 1.0));
    }
    if path.is_empty() { "M 0 0".to_string() } else { path }
}
