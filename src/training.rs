use anyhow::Context;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::state::AppState;
use crate::mamba::{TrainingPair, TrainingMetrics, TrainingConfig, OnlineTrainer};

pub struct TrainingOutput {
    pub loss: f32,
    pub tx_delta: Vec<f32>,
}

impl TrainingOutput {
    pub fn new() -> Self {
        Self {
            loss: 0.0,
            tx_delta: vec![0.0; 512],
        }
    }
}

pub struct MambaTrainer {
    trainer: Arc<Mutex<OnlineTrainer>>,
    state: Arc<AppState>,
}

impl MambaTrainer {
    pub fn new(state: Arc<AppState>) -> anyhow::Result<Self> {
        let config = crate::mamba::training::TrainingConfig::new();
        let trainer = OnlineTrainer::new(config)?;
        Ok(Self {
            trainer: Arc::new(Mutex::new(trainer)),
            state,
        })
    }

    pub async fn step(&self, batch: &[TrainingPair]) -> TrainingOutput {
        if batch.is_empty() {
            return TrainingOutput::new();
        }

        let mut trainer = self.trainer.lock().await;
        let windows: Vec<Vec<f32>> = batch.iter().map(|p| p.rx_spectrum.clone()).collect();

        match trainer.step(&windows) {
            Ok(loss) => TrainingOutput {
                loss,
                tx_delta: vec![0.0; 512],
            },
            Err(e) => {
                self.state.log("ERROR", "Mamba", &format!("Training step failed: {e}"));
                TrainingOutput::new()
            }
        }
    }

    /// Run inference on spectral magnitudes. Renamed to forward for consistency.
    pub async fn forward(&self, magnitudes: &[f32]) -> anyhow::Result<(f32, Vec<f32>, Vec<f32>)> {
        let trainer = self.trainer.lock().await;
        match trainer.forward(magnitudes) {
            Ok(out) => Ok((out.anomaly_score, out.latent, out.reconstruction)),
            Err(e) => {
                self.state
                    .log("ERROR", "Mamba", &format!("Inference error: {e}"));
                Ok((0.0, vec![0.0; 64], vec![0.0; 512]))
            }
        }
    }

    /// Legacy infer alias
    pub async fn infer(&self, magnitudes: &[f32]) -> anyhow::Result<(f32, Vec<f32>, Vec<f32>)> {
        self.forward(magnitudes).await
    }

    pub async fn save(
        &self,
        path: &str,
        metadata: Option<crate::state::CheckpointMetadata>,
    ) -> anyhow::Result<()> {
        let trainer = self.trainer.lock().await;
        trainer.save(path)?;

        if let Some(meta) = metadata {
            let meta_path = format!("{}.json", path);
            meta.to_file(&meta_path)?;
        }
        Ok(())
    }
}

pub struct TrainingSession;
pub fn spawn_background_training(_state: Arc<AppState>) {}
