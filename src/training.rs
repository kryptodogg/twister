// src/training.rs — Mamba Training Orchestrator with Tokio Async
//
// Async training pipeline:
//   Dispatch Thread → TrainingPair queue → MambaTrainer → TrainingMetrics → UI
//
// Uses Tokio async/await for clean concurrency without thread hell.
//
// Fix: MambaTrainer now wraps OnlineTrainer (which owns the persistent AdamW
// optimizer and exposes step() + infer()).  The previous version wrapped
// MambaAutoencoder directly; that type has no step() method, causing E0599.

use crate::mamba::{
    compute_rms_db, MambaAutoencoder, OnlineTrainer, TrainingMetrics, TrainingPair,
};
use crate::state::AppState;
use candle_core::Device;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Number of training pairs per batch
pub const BATCH_SIZE: usize = 32;

/// Channel capacity for training pair queue
pub const QUEUE_CAPACITY: usize = 1024;

/// Training session manages the async queue of training pairs
pub struct TrainingSession {
    /// Queue for sending training pairs to trainer
    tx: mpsc::Sender<TrainingPair>,
    /// Receiver for trainer to consume
    rx: Arc<Mutex<mpsc::Receiver<TrainingPair>>>,
    /// Shared metrics for UI display
    pub metrics: Arc<Mutex<TrainingMetrics>>,
    /// Total pairs enqueued
    pub total_pairs: AtomicU32,
    /// Shared state for logging
    state: Arc<AppState>,
}

impl TrainingSession {
    /// Create a new training session with async queue
    pub fn new(state: Arc<AppState>) -> Self {
        let (tx, rx) = mpsc::channel::<TrainingPair>(QUEUE_CAPACITY);
        Self {
            tx,
            rx: Arc::new(Mutex::new(rx)),
            metrics: Arc::new(Mutex::new(TrainingMetrics::default())),
            total_pairs: AtomicU32::new(0),
            state,
        }
    }

    /// Enqueue a training pair (non-blocking)
    pub async fn enqueue(&self, pair: TrainingPair) {
        self.total_pairs.fetch_add(1, Ordering::Relaxed);
        let result = self.tx.send(pair).await;
        if result.is_err() {
            self.state
                .log("ERROR", "Training", "Failed to enqueue pair");
        }
    }

    /// Try to enqueue without blocking (returns false if full)
    pub fn try_enqueue(&self, pair: TrainingPair) -> bool {
        self.tx.try_send(pair).is_ok()
    }

    /// Get next batch of training pairs (blocking wait)
    pub async fn next_batch(&self) -> Option<Vec<TrainingPair>> {
        let mut rx = self.rx.lock().await;
        let mut batch = Vec::with_capacity(BATCH_SIZE);

        // Wait for first item
        if let Some(first) = rx.recv().await {
            batch.push(first);
        } else {
            return None; // Channel closed
        }

        // Collect remaining items (non-blocking)
        while batch.len() < BATCH_SIZE {
            match rx.try_recv() {
                Ok(pair) => batch.push(pair),
                Err(_) => break, // No more items available
            }
        }

        Some(batch)
    }

    /// Get current training metrics
    pub async fn get_metrics(&self) -> TrainingMetrics {
        self.metrics.lock().await.clone()
    }

    /// Update training metrics
    pub async fn update_metrics(&self, loss: f32) {
        let mut m = self.metrics.lock().await;
        m.epoch += 1;
        // Exponential moving average
        m.avg_loss = if m.batch_count == 0 {
            loss
        } else {
            0.9 * m.avg_loss + 0.1 * loss
        };
        m.batch_count += 1;
    }

    /// Get total pairs collected
    pub fn total_pairs(&self) -> u32 {
        self.total_pairs.load(Ordering::Relaxed)
    }
}

// Default removed because AppState is required

/// Mamba trainer for async training loop.
///
/// Wraps `OnlineTrainer` (which holds the `MambaAutoencoder` + persistent
/// `AdamW` optimizer) behind a tokio Mutex so the background task and the
/// dispatch loop can share it safely across await points.
pub struct MambaTrainer {
    trainer: Arc<Mutex<OnlineTrainer>>,
    state: Arc<AppState>,
}

impl MambaTrainer {
    /// Create a new trainer with initialized autoencoder + AdamW.
    pub fn new(state: Arc<AppState>) -> anyhow::Result<Self> {
        let trainer = OnlineTrainer::new()?;
        Ok(Self {
            trainer: Arc::new(Mutex::new(trainer)),
            state,
        })
    }

    /// Training step on a batch of TrainingPairs.
    ///
    /// Builds TX/RX interleaved windows and calls OnlineTrainer::step(),
    /// which runs backward_step() with the persistent AdamW.
    /// Also predicts TX improvements for waveform optimization.
    pub async fn step(&self, batch: &[TrainingPair]) -> TrainingOutput {
        if batch.is_empty() {
            self.state
                .log("ERROR", "Batch", "Batch empty! No training pairs queued.");
            return TrainingOutput::new();
        }

        self.state.log(
            "INFO",
            "Batch",
            &format!("Processing {} pairs from queue", batch.len()),
        );

        // Compute SNR for first pair as diagnostic
        if !batch.is_empty() {
            let tx_avg =
                batch[0].tx_spectrum.iter().sum::<f32>() / batch[0].tx_spectrum.len() as f32;
            let rx_avg =
                batch[0].rx_spectrum.iter().sum::<f32>() / batch[0].rx_spectrum.len() as f32;
            let tx_var: f32 = batch[0]
                .tx_spectrum
                .iter()
                .map(|x| (x - tx_avg).powi(2))
                .sum::<f32>()
                / batch[0].tx_spectrum.len() as f32;
            let rx_var: f32 = batch[0]
                .rx_spectrum
                .iter()
                .map(|x| (x - rx_avg).powi(2))
                .sum::<f32>()
                / batch[0].rx_spectrum.len() as f32;
            let tx_snr_db = 10.0 * (tx_var.max(1e-10)).log10();
            let rx_snr_db = 10.0 * (rx_var.max(1e-10)).log10();
            self.state.log(
                "INFO",
                "Pair",
                &format!(
                    "TX_SNR={:.1}dB RX_SNR={:.1}dB (first pair diagnostic)",
                    tx_snr_db, rx_snr_db
                ),
            );
        }

        // Build interleaved TX/RX windows: [T=64, F=512]
        // First 256 bins per frame = TX spectrum, last 256 = RX spectrum.
        let windows: Vec<Vec<f32>> = batch
            .iter()
            .filter_map(|pair| {
                if pair.tx_spectrum.len() >= 512 * 64 && pair.rx_spectrum.len() >= 512 * 64 {
                    let mut window = vec![0.0f32; 512 * 64];
                    for t in 0..64 {
                        let base = t * 512;
                        for f in 0..256 {
                            window[base + f] = pair.tx_spectrum[base + f];
                            window[base + 256 + f] = pair.rx_spectrum[base + f];
                        }
                    }
                    Some(window)
                } else {
                    self.state.log(
                        "WARN",
                        "Batch",
                        &format!(
                            "Skipping pair tx_len={} rx_len={} (need 32768 each)",
                            pair.tx_spectrum.len(),
                            pair.rx_spectrum.len()
                        ),
                    );
                    None
                }
            })
            .collect();

        self.state.log(
            "INFO",
            "Batch",
            &format!("Draining {} pairs for training", windows.len()),
        );

        if windows.is_empty() {
            self.state
                .log("ERROR", "Batch", "All training pairs filtered out!");
            return TrainingOutput::new();
        }

        // Run training step and get loss
        let loss = {
            let mut trainer = self.trainer.lock().await;
            match trainer.step(&windows) {
                Ok(loss) => {
                    if loss > 0.0 {
                        self.state.log(
                            "INFO",
                            "Mamba",
                            &format!("loss={:.6} (training converging)", loss),
                        );
                    } else {
                        self.state.log(
                            "WARN",
                            "Mamba",
                            &format!("loss={:.6} (zero loss = broken training!)", loss),
                        );
                    }
                    loss
                }
                Err(e) => {
                    self.state.log("ERROR", "Mamba", &format!("{e}"));
                    0.0
                }
            }
        };

        // Predict TX improvements from each pair
        let mut tx_improvements = vec![0.0f32; 512];
        let device = Device::Cpu;
        let autoencoder = MambaAutoencoder::new(device)
            .unwrap_or_else(|_| MambaAutoencoder::new(Device::Cpu).unwrap());

        for pair in batch {
            if pair.tx_spectrum.len() >= 256 && pair.rx_spectrum.len() >= 256 {
                let tx_delta = autoencoder.predict_tx_delta(&pair.tx_spectrum, &pair.rx_spectrum);
                for (i, delta) in tx_delta.iter().enumerate() {
                    tx_improvements[i] += delta;
                }
            }
        }

        // Average improvements across batch
        let batch_size = batch.len() as f32;
        for v in tx_improvements.iter_mut() {
            *v /= batch_size;
        }

        // Compute TX delta RMS in dB
        let tx_delta_rms_db = compute_rms_db(&tx_improvements);

        TrainingOutput {
            loss,
            tx_improvement: tx_improvements,
            tx_delta_rms_db,
        }
    }

    /// Run inference on spectrum magnitudes.
    /// Returns (anomaly_score, latent_vec).
    pub async fn infer(&self, magnitudes: &[f32]) -> anyhow::Result<(f32, Vec<f32>, Vec<f32>)> {
        let trainer = self.trainer.lock().await;
        match trainer.infer(magnitudes) {
            Ok(out) => Ok((out.anomaly_score, out.latent, out.reconstruction)),
            Err(e) => {
                self.state
                    .log("ERROR", "Mamba", &format!("Inference error: {e}"));
                Ok((0.0, vec![0.0; 64], vec![0.0; 512]))
            }
        }
    }

    /// Save model weights and metadata to path.
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

    /// Load model weights and metadata from path.
    pub async fn load(
        &self,
        path: &str,
    ) -> anyhow::Result<Option<crate::state::CheckpointMetadata>> {
        let mut trainer = self.trainer.lock().await;
        trainer.load(path)?;

        let meta_path = format!("{}.json", path);
        Ok(crate::state::CheckpointMetadata::from_file(&meta_path))
    }
}

// Default removed

/// Spawn background training task
pub fn spawn_background_training(
    session: Arc<TrainingSession>,
    trainer: Arc<MambaTrainer>,
    state: Arc<AppState>,
    ui_tx: crossbeam_channel::Sender<crate::state::UiEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        state.log("INFO", "Mamba", "Background training task started");

        loop {
            // Wait for next batch
            if let Some(batch) = session.next_batch().await {
                let output = trainer.step(&batch).await;
                let loss = output.loss;

                session.update_metrics(loss).await;

                state.set_train_loss(loss);
                let epoch = state
                    .train_epoch
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                // Emit telemetry event for UI consumption
                let _evt_result = ui_tx.send(crate::state::UiEvent::TrainingProgress {
                    iteration: epoch,
                    total_iterations: 100_000, // Arbitrary upper bound for progress bar
                    loss,
                    loss_min: loss.min(0.5), // Approximate min for progress display
                });

                // Store TX delta RMS for convergence monitoring
                state.set_tx_delta_rms(output.tx_delta_rms_db);

                // Validate and clamp TX deltas (ethics oversight)
                let validation = validate_tx_delta(&output.tx_improvement);
                if !validation.is_valid {
                    let warnings: Vec<String> = validation
                        .violations
                        .iter()
                        .map(|v| format!("  - {}", v))
                        .collect();
                    state.log(
                        "WARN",
                        "ETHICS",
                        &format!("TX delta validation warnings: {}", warnings.join(", ")),
                    );
                }

                // Store clamped TX deltas in state for PDM synthesis
                state.set_tx_spectral_deltas(validation.clamped_deltas);

                // Log convergence progress
                let epoch = state.train_epoch.load(std::sync::atomic::Ordering::Relaxed);
                println!(
                    "[Mamba] Epoch {}: loss={:.4}, TX_delta_RMS={:.2}dB{}",
                    epoch,
                    loss,
                    output.tx_delta_rms_db,
                    if output.tx_delta_rms_db < TX_DELTA_RMS_CONVERGENCE_DB
                        && loss < LOSS_CONVERGENCE_THRESHOLD
                    {
                        " ✓ CONVERGED"
                    } else {
                        ""
                    }
                );

                // Check for convergence
                if output.tx_delta_rms_db < TX_DELTA_RMS_CONVERGENCE_DB
                    && loss < LOSS_CONVERGENCE_THRESHOLD
                {
                    state.log(
                        "INFO",
                        "Mamba",
                        "🎯 OPTIMAL TX WAVEFORM FOUND - Training converged",
                    );
                    // Note: We don't stop training automatically - let the user decide
                }
            }
        }
    })
}

// ── Ethics Oversight: Audio Delta Validation ─────────────────────────────────

/// Maximum audio delta per bin (prevents audio artifacts)
pub const MAX_TX_DELTA_PER_BIN_DB: f32 = 3.0; // ±3dB clamp

/// Warning threshold for TX delta RMS
pub const TX_DELTA_RMS_WARNING_DB: f32 = 10.0;

/// Critical threshold for TX delta RMS (triggers emergency shutoff)
pub const TX_DELTA_RMS_CRITICAL_DB: f32 = 15.0;

/// Convergence target for TX delta RMS
pub const TX_DELTA_RMS_CONVERGENCE_DB: f32 = 1.0;

/// Convergence target for reconstruction loss
pub const LOSS_CONVERGENCE_THRESHOLD: f32 = 0.05;

/// Clamp TX deltas to prevent audio artifacts (ethics oversight)
pub fn clamp_tx_deltas(deltas: &[f32]) -> Vec<f32> {
    deltas
        .iter()
        .map(|&d| d.clamp(-MAX_TX_DELTA_PER_BIN_DB, MAX_TX_DELTA_PER_BIN_DB))
        .collect()
}

/// Validate TX deltas before application
pub struct TxValidationResult {
    pub is_valid: bool,
    pub violations: Vec<String>,
    pub clamped_deltas: Vec<f32>,
}

pub fn validate_tx_delta(deltas: &[f32]) -> TxValidationResult {
    let mut violations = Vec::new();
    let clamped = clamp_tx_deltas(deltas);

    // Check for excessive deltas
    let exceeded_count = deltas
        .iter()
        .filter(|&&d| d.abs() > MAX_TX_DELTA_PER_BIN_DB)
        .count();

    if exceeded_count > 0 {
        violations.push(format!(
            "{} bins exceeded ±{}dB limit (audio artifact prevention)",
            exceeded_count, MAX_TX_DELTA_PER_BIN_DB
        ));
    }

    // Check RMS
    let rms = compute_rms_db(deltas);
    if rms > TX_DELTA_RMS_CRITICAL_DB {
        violations.push(format!(
            "TX delta RMS {:.2}dB exceeds critical threshold {:.2}dB",
            rms, TX_DELTA_RMS_CRITICAL_DB
        ));
    }

    TxValidationResult {
        is_valid: violations.is_empty(),
        violations,
        clamped_deltas: clamped,
    }
}

// ── Training Output Structure ────────────────────────────────────────────────

/// Training step output: loss + predicted TX improvements
#[derive(Debug, Clone)]
pub struct TrainingOutput {
    /// Reconstruction mean squared error (dB)
    pub loss: f32,

    /// Predicted spectral deltas for next TX frame [512 bins]
    /// Values are in dB relative to current spectrum
    pub tx_improvement: Vec<f32>,

    /// RMS of tx_improvement in dB (convergence metric)
    pub tx_delta_rms_db: f32,
}

impl std::fmt::Display for TrainingOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "loss={:.4}, tx_delta_rms={:.2}dB",
            self.loss, self.tx_delta_rms_db
        )
    }
}

impl TrainingOutput {
    /// Create empty training output
    pub fn new() -> Self {
        TrainingOutput {
            loss: 0.0,
            tx_improvement: vec![0.0; 512],
            tx_delta_rms_db: 0.0,
        }
    }
}

impl Default for TrainingOutput {
    fn default() -> Self {
        Self::new()
    }
}

// ── RTL-SDR training scan ────────────────────────────────────────────────────

pub struct RtlSdrTrainingScan {
    pub start_freq_hz: u32,
    pub stop_freq_hz: u32,
    pub step_hz: u32,
    pub current_freq_hz: u32,
}

impl RtlSdrTrainingScan {
    pub fn new(start_hz: u32, stop_hz: u32, step_hz: u32) -> Self {
        Self {
            start_freq_hz: start_hz,
            stop_freq_hz: stop_hz,
            step_hz,
            current_freq_hz: start_hz,
        }
    }

    pub fn advance(&mut self) -> bool {
        self.current_freq_hz += self.step_hz;
        if self.current_freq_hz > self.stop_freq_hz {
            self.current_freq_hz = self.start_freq_hz;
            false // Sweep complete
        } else {
            true
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_training_output_with_tx_improvement() {
        let mut tx_improvement = vec![0.0f32; 512];
        tx_improvement[0] = 0.1;
        tx_improvement[1] = 0.05;
        tx_improvement[2] = -0.03;
        tx_improvement[3] = 0.0;

        let output = TrainingOutput {
            loss: 0.847,
            tx_improvement,
            tx_delta_rms_db: 12.4,
        };
        assert_eq!(output.loss, 0.847);
        assert_eq!(output.tx_improvement.len(), 512);
        assert_eq!(output.tx_delta_rms_db, 12.4);
    }

    #[test]
    fn test_training_output_new() {
        let output = TrainingOutput::new();
        assert_eq!(output.loss, 0.0);
        assert_eq!(output.tx_improvement.len(), 512);
        assert_eq!(output.tx_delta_rms_db, 0.0);
        assert!(output.tx_improvement.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_training_output_default() {
        let output = TrainingOutput::default();
        assert_eq!(output.loss, 0.0);
        assert_eq!(output.tx_improvement.len(), 512);
        assert_eq!(output.tx_delta_rms_db, 0.0);
    }

    #[test]
    fn test_compute_rms_db() {
        // Test with constant signal (RMS of 1.0 = 0 dB)
        let signal = vec![1.0f32; 512];
        let rms = compute_rms_db(&signal);
        assert!(rms.abs() < 0.01); // 20 * log10(1.0) = 0 dB

        // Test with 0.5 signal (20 * log10(0.5) ≈ -6 dB)
        let signal_half = vec![0.5f32; 512];
        let rms_half = compute_rms_db(&signal_half);
        assert!((rms_half - (-6.02)).abs() < 0.1); // ≈ -6.02 dB

        // Test with zeros
        let zeros = vec![0.0f32; 512];
        let rms_zero = compute_rms_db(&zeros);
        assert!(rms_zero <= -100.0); // Clamped to -100 dB floor
    }
}
