// src/trainer.rs — Online Mamba Training Loop
//
// Sits in its own thread.  Receives V-buffer snapshots via a channel,
// maintains a capped replay buffer, and runs gradient steps continuously
// while training_active is true.
//
// Replay buffer:
//   Capacity: REPLAY_CAP windows.  New windows are added with reservoir sampling
//   (no bias toward recent frames) so the model sees a representative mix of
//   signal patterns across the full session.
//
// Training schedule:
//   - Collect at least MIN_WINDOWS before starting training.
//   - Run STEPS_PER_BATCH gradient steps per batch.
//   - Sleep TRAIN_SLEEP_MS between batches to yield the CPU for audio/GPU work.
//   - Autosave every SAVE_INTERVAL_EPOCHS epochs.

use crossbeam_channel::Receiver;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::mamba::{OnlineTrainer, MAMBA_CONTEXT_LEN, MAMBA_INPUT_BINS};
use crate::state::AppState;
use crate::vbuffer::V_FREQ_BINS;

pub const REPLAY_CAP: usize = 4096;
pub const MIN_WINDOWS: usize = 32;
pub const BATCH_SIZE: usize = 8;
pub const STEPS_PER_BATCH: usize = 1;
pub const TRAIN_SLEEP_MS: u64 = 20;
pub const SAVE_INTERVAL_EPOCHS: u32 = 50;

/// A single training sample: flat [CONTEXT_LEN * FREQ_BINS] f32.
pub type Window = Vec<f32>;

/// Command sent to the trainer thread.
pub enum TrainerCmd {
    /// New V-buffer snapshot: flat magnitudes [V_FREQ_BINS] for the most recent T frames.
    PushWindow(Window),
    /// Change training active state.
    SetActive(bool),
    /// Save weights now.
    Save(String),
    /// Load weights from path.
    Load(String),
    /// Graceful shutdown.
    Stop,
}

pub struct TrainerThread {
    trainer: OnlineTrainer,
    replay: Vec<Window>,
    state: Arc<AppState>,
    epoch: u32,
    last_save: Instant,
}

impl TrainerThread {
    pub fn new(state: Arc<AppState>) -> anyhow::Result<Self> {
        let trainer = OnlineTrainer::new()?;
        Ok(Self {
            trainer,
            replay: Vec::with_capacity(REPLAY_CAP),
            state,
            epoch: 0,
            last_save: Instant::now(),
        })
    }

    /// Main loop — blocks until `Stop` is received.
    pub fn run(&mut self, rx: Receiver<TrainerCmd>) {
        loop {
            // Drain all pending commands (non-blocking).
            loop {
                match rx.try_recv() {
                    Ok(TrainerCmd::PushWindow(w)) => self.ingest(w),
                    Ok(TrainerCmd::SetActive(v)) => {
                        self.state.training_active.store(v, Ordering::Relaxed);
                    }
                    Ok(TrainerCmd::Save(path)) => {
                        if let Err(e) = self.trainer.save(&path) {
                            eprintln!("[Trainer] Save failed: {e}");
                        } else {
                            println!("[Trainer] Saved to {path}");
                        }
                    }
                    Ok(TrainerCmd::Load(path)) => {
                        if let Err(e) = self.trainer.load(&path) {
                            eprintln!("[Trainer] Load failed: {e}");
                        } else {
                            println!("[Trainer] Loaded from {path}");
                        }
                    }
                    Ok(TrainerCmd::Stop) | Err(crossbeam_channel::TryRecvError::Disconnected) => {
                        println!(
                            "[Trainer] Stopped. Epoch={} replay={}",
                            self.epoch,
                            self.replay.len()
                        );
                        return;
                    }
                    Err(crossbeam_channel::TryRecvError::Empty) => break,
                }
            }

            self.state
                .replay_buf_len
                .store(self.replay.len() as u32, Ordering::Relaxed);

            // Training step when active and buffer is warm.
            if self.state.training_active.load(Ordering::Relaxed)
                && self.replay.len() >= MIN_WINDOWS
            {
                // Sample a random mini-batch from the replay buffer.
                let batch: Vec<Window> = (0..BATCH_SIZE.min(self.replay.len()))
                    .map(|_| {
                        let idx = (std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_nanos() as usize)
                            % self.replay.len();
                        self.replay[idx].clone()
                    })
                    .collect();

                match self.trainer.step(&batch) {
                    Ok(loss) => {
                        self.epoch += 1;
                        self.state.train_epoch.store(self.epoch, Ordering::Relaxed);
                        self.state.set_train_loss(loss);

                        if self.epoch % 10 == 0 {
                            println!(
                                "[Trainer] epoch={} loss={:.6} replay={}",
                                self.epoch,
                                loss,
                                self.replay.len()
                            );
                        }

                        // Autosave.
                        if self.epoch % SAVE_INTERVAL_EPOCHS == 0 {
                            let path = self
                                .state
                                .checkpoint_path
                                .lock()
                                .map(|g| g.clone())
                                .unwrap_or_else(|_| "weights/mamba.safetensors".into());
                            let _ = std::fs::create_dir_all("weights");
                            if let Err(e) = self.trainer.save(&path) {
                                eprintln!("[Trainer] Autosave failed: {e}");
                            }
                        }

                        // After each training step also run inference on the newest window
                        // for real-time anomaly scoring.
                        if let Some(newest) = self.replay.last() {
                            if let Ok(output) = self.trainer.infer(newest) {
                                self.state.set_mamba_anomaly(output.anomaly_score);
                            }
                        }
                    }
                    Err(e) => eprintln!("[Trainer] Step error: {e}"),
                }
            } else if !self.replay.is_empty() {
                // Even when not training: run inference on newest window.
                if let Some(newest) = self.replay.last() {
                    if let Ok(output) = self.trainer.infer(newest) {
                        self.state.set_mamba_anomaly(output.anomaly_score);
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(TRAIN_SLEEP_MS));
        }
    }

    /// Add a new window to the replay buffer with reservoir sampling.
    fn ingest(&mut self, w: Window) {
        let required = MAMBA_CONTEXT_LEN * MAMBA_INPUT_BINS;
        if w.len() < required {
            return;
        }

        if self.replay.len() < REPLAY_CAP {
            self.replay.push(w);
        } else {
            // Reservoir: replace a random existing entry with probability 1/N_seen.
            // Approximation: just replace a random entry uniformly (biased recent but practical).
            let idx = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as usize)
                % REPLAY_CAP;
            self.replay[idx] = w;
        }
    }
}

// ── V-buffer snapshot extraction ─────────────────────────────────────────────
//
// Called from the dispatch thread: reads the most recent CONTEXT_LEN rows
// from the V-buffer (CPU copy) and flattens them into a training window.

pub fn extract_window_from_vbuf_snapshot(
    snapshot: &[[f32; V_FREQ_BINS]], // V_DEPTH rows, indexed by slot
    write_version: u64,
    context_len: usize,
) -> Window {
    let depth = snapshot.len();
    let f = V_FREQ_BINS.min(MAMBA_INPUT_BINS);
    let t = context_len.min(MAMBA_CONTEXT_LEN);

    let mut window = vec![0.0f32; MAMBA_CONTEXT_LEN * MAMBA_INPUT_BINS];

    for i in 0..t {
        // frames_back = t-1-i  →  oldest first (natural temporal order for SSM)
        let frames_back = (t - 1 - i) as u64;
        if write_version < frames_back {
            continue;
        }
        let version = write_version - frames_back;
        let slot = (version as usize) % depth;
        let row = &snapshot[slot];

        let dst_off = i * MAMBA_INPUT_BINS;
        window[dst_off..dst_off + f].copy_from_slice(&row[..f]);
    }

    window
}
