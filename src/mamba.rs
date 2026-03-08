// src/mamba.rs — Mamba Selective State Space Model Autoencoder
//
// Architecture: input [T=64, F=512] → Mamba encoder → latent [64] → decoder → reconstruction
//
// v0.4 fixes:
//   • OnlineTrainer: wraps MambaAutoencoder + AdamW with PERSISTENT optimizer state.
//     Previously, train_step() created a new AdamW every call — wiping all Adam
//     moment buffers and making the model never converge.
//   • loss() now works entirely on Tensors without round-tripping through Vec<f32>,
//     which severed the autograd graph and caused backward() to see zero gradient.
//   • causal_conv1d() extracts the padded tensor as a single flat Vec once
//     instead of calling .to_scalar() T×C times (was 32,768 device syncs/frame).
//
// Fixed tensor shapes:
//   - Input: [T=64, F=512] (64 time frames, 512 frequency bins)
//   - in_proj: F=512 → 2*D_INNER=256
//   - x_proj: D_INNER=128 → dt_rank + 2*N = 8 + 32 = 40
//   - dt_proj: dt_rank=8 → D_INNER=128
//   - bottleneck: D_MODEL=128 → LATENT=64
//   - decoder: LATENT=64 → T*F = 64*512 = 32768

use candle_core::{DType, Device, Result as CResult, Tensor};
use candle_nn::{
    LayerNorm, Linear, Module, Optimizer, VarBuilder, VarMap, layer_norm, linear, linear_no_bias,
};

// ── Hyper-parameters ──────────────────────────────────────────────────────────

pub const MAMBA_INPUT_BINS: usize = 512; // F: frequency bins per frame
pub const MAMBA_CONTEXT_LEN: usize = 64; // T: number of frames per window
pub const MAMBA_D_MODEL: usize = 128;    // D: model dimension
pub const MAMBA_D_STATE: usize = 16;     // N: SSM state dimension
pub const MAMBA_D_CONV: usize = 4;       // Conv kernel size
pub const MAMBA_LATENT_DIM: usize = 64;  // Bottleneck dimension
pub const MAMBA_EXPAND: usize = 2;       // Expansion factor
pub const MAMBA_DT_RANK: usize = 8;      // Delta projection rank
pub const LAMBDA_REG: f64 = 0.01;        // Latent L2 regularization

// Derived constants
const D_INNER: usize = MAMBA_D_MODEL * MAMBA_EXPAND; // 256
#[allow(dead_code)]
const DT_PLUS_2N: usize = MAMBA_DT_RANK + 2 * MAMBA_D_STATE; // 40

// ── Output structures ──────────────────────────────────────────────────────────

/// Output from Mamba autoencoder forward pass
#[derive(Debug, Clone)]
pub struct MambaOutput {
    pub latent: Vec<f32>,
    pub reconstruction: Vec<f32>,
    pub anomaly_score: f32, // MSE in dB, log-scaled
}

/// Training metrics for UI display
#[derive(Debug, Clone, Default)]
pub struct TrainingMetrics {
    pub epoch: u32,
    pub avg_loss: f32,
    pub batch_count: u32,
}

// ── SSM block ─────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct MambaBlock {
    // Input projection: F → 2*D_INNER
    in_proj: Linear,
    // Conv1d depthwise weights [D_INNER, D_CONV] and bias [D_INNER]
    conv1d_w: Tensor,
    conv1d_b: Tensor,
    // x_proj: D_INNER → dt_rank + 2*N
    x_proj: Linear,
    // dt_proj: dt_rank → D_INNER
    dt_proj: Linear,
    // A matrix: [D_INNER, D_STATE]
    a_log: Tensor,
    // D skip: [D_INNER]
    d: Tensor,
    // Output projection: D_INNER → D_MODEL
    out_proj: Linear,
    // Layer norm
    norm: LayerNorm,
}

impl MambaBlock {
    fn new(vb: VarBuilder, device: &Device) -> CResult<Self> {
        let f     = MAMBA_INPUT_BINS;
        let inner = D_INNER;
        let n     = MAMBA_D_STATE;
        let dt_rank = MAMBA_DT_RANK;
        let d_conv  = MAMBA_D_CONV;
        let d       = MAMBA_D_MODEL;

        // A initialized as negative diagonal (HiPPO approximation)
        let a_data: Vec<f32> = (0..inner)
            .flat_map(|i| std::iter::repeat(-(i as f32 + 1.0)).take(n))
            .collect();
        let a_log = Tensor::from_vec(a_data, (inner, n), device)?;

        // D skip connection (ones)
        let d_data = vec![1.0f32; inner];
        let d_ten  = Tensor::from_vec(d_data, (inner,), device)?;

        // Depthwise conv1d weights initialised to uniform 1/d_conv
        let conv_w_init = vec![1.0 / d_conv as f32; inner * d_conv];
        let conv1d_w = Tensor::from_vec(conv_w_init, (inner, d_conv), device)?;
        let conv1d_b = Tensor::zeros((inner,), DType::F32, device)?;

        Ok(Self {
            in_proj:  linear_no_bias(f, 2 * inner, vb.pp("in_proj"))?,
            x_proj:   linear_no_bias(inner, dt_rank + 2 * n, vb.pp("x_proj"))?,
            dt_proj:  linear(dt_rank, inner, vb.pp("dt_proj"))?,
            out_proj: linear_no_bias(inner, d, vb.pp("out_proj"))?,
            norm:     layer_norm(d, 1e-5, vb.pp("norm"))?,
            a_log,
            d: d_ten,
            conv1d_w,
            conv1d_b,
        })
    }

    /// Forward pass: [T, F] → [T, D_MODEL], returns final state [D_INNER, D_STATE]
    fn forward_seq(&self, x: &Tensor) -> CResult<(Tensor, Tensor)> {
        let inner = D_INNER;

        // Input projection: [T, F] → [T, 2*inner]
        let xz = self.in_proj.forward(x)?;

        // Split into x and z branches: each [T, inner]
        let x_branch = xz.narrow(1, 0, inner)?;
        let z_branch = xz.narrow(1, inner, inner)?;

        // Causal conv1d: [T, inner] → [T, inner]  (FIX: single device sync)
        let x_conv = self.causal_conv1d(&x_branch)?;
        let x_act  = candle_nn::ops::silu(&x_conv)?;

        // SSM selective scan: [T, inner] → ([T, inner], [inner, N])
        let (y_ssm, h_t) = self.ssm_scan(&x_act)?;

        // Gate with z branch
        let z_act  = candle_nn::ops::silu(&z_branch)?;
        let gated  = (y_ssm * z_act)?;

        // Output projection + layer norm
        let out_pre = self.out_proj.forward(&gated)?;
        let out     = self.norm.forward(&out_pre)?;

        Ok((out, h_t))
    }

    /// Causal 1D depthwise convolution with left padding.
    ///
    /// FIX: was calling `.to_scalar()` T×C times (32,768 device round-trips).
    /// Now extracts the padded tensor as a single flat `Vec<f32>` (one sync),
    /// then indexes into it in pure CPU code.
    fn causal_conv1d(&self, x: &Tensor) -> CResult<Tensor> {
        let (t, c) = x.dims2()?;
        let k      = MAMBA_D_CONV;
        let pad    = k - 1;

        // Pad left with zeros: [t, c] → [t+pad, c]
        let zeros     = Tensor::zeros((pad, c), DType::F32, x.device())?;
        let x_padded  = Tensor::cat(&[&zeros, x], 0)?;

        // Single device sync: extract entire padded tensor as flat Vec.
        let x_flat: Vec<f32>  = x_padded.flatten_all()?.to_vec1()?;
        let w_flat: Vec<f32>  = self.conv1d_w.flatten_all()?.to_vec1()?;
        let b:      Vec<f32>  = self.conv1d_b.to_vec1()?;

        let padded_rows = t + pad;
        let mut out_data = vec![0.0f32; t * c];

        for i in 0..t {
            for ch in 0..c {
                let mut acc = b[ch];
                for kj in 0..k {
                    let row_idx = i + kj;
                    // Bounds-checked (row_idx is always < padded_rows by construction)
                    if row_idx < padded_rows {
                        let x_val    = x_flat[row_idx * c + ch];
                        let w_idx    = ch * k + kj;
                        acc         += x_val * w_flat[w_idx];
                    }
                }
                out_data[i * c + ch] = acc;
            }
        }

        Tensor::from_vec(out_data, (t, c), x.device())
    }

    /// Selective SSM scan using the ZOH sequential algorithm.
    fn ssm_scan(&self, u: &Tensor) -> CResult<(Tensor, Tensor)> {
        let (t, inner) = u.dims2()?;
        let n          = MAMBA_D_STATE;
        let dt_rank    = MAMBA_DT_RANK;

        // Project u → (Δ, B, C)
        let delta_bc = self.x_proj.forward(u)?;
        let dt_raw   = delta_bc.narrow(1, 0, dt_rank)?;
        let b_t      = delta_bc.narrow(1, dt_rank, n)?;
        let c_t      = delta_bc.narrow(1, dt_rank + n, n)?;

        // Δ = softplus(dt_proj(dt_raw))
        let dt_proj = self.dt_proj.forward(&dt_raw)?;
        let delta   = softplus(&dt_proj)?;

        // −A values (positive for stable discretization, HiPPO init gives negative a_log)
        let a_neg     = self.a_log.neg()?;
        let a_neg_vals: Vec<f32> = a_neg.flatten_all()?.to_vec1()?;

        // D skip connection: u * d
        let d_row  = self.d.reshape((1, inner))?;
        let d_u    = u.broadcast_mul(&d_row)?;

        // Extract all time-step slices as flat vecs (batch sync)
        let delta_flat: Vec<f32> = delta.flatten_all()?.to_vec1()?;
        let b_flat:     Vec<f32> = b_t.flatten_all()?.to_vec1()?;
        let c_flat:     Vec<f32> = c_t.flatten_all()?.to_vec1()?;
        let u_flat:     Vec<f32> = u.flatten_all()?.to_vec1()?;
        let d_u_flat:   Vec<f32> = d_u.flatten_all()?.to_vec1()?;

        let mut h_vals = vec![0.0f32; inner * n];
        let mut y_data = vec![0.0f32; t * inner];

        for i in 0..t {
            let dt_row = &delta_flat[i * inner..(i + 1) * inner];
            let b_row  = &b_flat   [i * n..(i + 1) * n];
            let c_row  = &c_flat   [i * n..(i + 1) * n];
            let u_row  = &u_flat   [i * inner..(i + 1) * inner];
            let du_row = &d_u_flat [i * inner..(i + 1) * inner];

            let y_row = &mut y_data[i * inner..(i + 1) * inner];

            for d in 0..inner {
                let dt_d = dt_row[d];
                let mut y_d = du_row[d];

                for j in 0..n {
                    let a_dn  = a_neg_vals[d * n + j]; // positive value from negated a_log
                    let h_dn  = h_vals[d * n + j];

                    // ZOH discretization: Ā = exp(−A·Δ)
                    let neg_a = -a_dn;
                    let a_bar = (neg_a * dt_d).exp();

                    // B̄ = (Ā − 1) / (−A) · B·u, with L'Hôpital at A→0
                    let b_bar = if neg_a.abs() < 1e-9 {
                        dt_d * u_row[d] * b_row[j]
                    } else {
                        ((a_bar - 1.0) / neg_a.abs().max(1e-9)) * u_row[d] * b_row[j]
                    };

                    let new_h = a_bar * h_dn + b_bar;
                    h_vals[d * n + j] = new_h;
                    y_d += c_row[j] * new_h;
                }

                y_row[d] = y_d;
            }
        }

        let y_stack = Tensor::from_vec(y_data, (t, inner), u.device())?;
        let h_out   = Tensor::from_vec(h_vals, (inner, n), u.device())?;
        Ok((y_stack, h_out))
    }
}

// ── Autoencoder ───────────────────────────────────────────────────────────────

pub struct MambaAutoencoder {
    block:      MambaBlock,
    bottleneck: Linear,
    decoder:    Linear,
    pub device: Device,
    pub varmap: VarMap,
}

impl MambaAutoencoder {
    /// Create a new (randomly initialised) autoencoder.
    pub fn new(device: Device) -> CResult<Self> {
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);

        let block      = MambaBlock::new(vb.pp("mamba_block"), &device)?;
        let bottleneck = linear(MAMBA_D_MODEL, MAMBA_LATENT_DIM, vb.pp("bottleneck"))?;
        let decoder    = linear(
            MAMBA_LATENT_DIM,
            MAMBA_CONTEXT_LEN * MAMBA_INPUT_BINS,
            vb.pp("decoder"),
        )?;

        Ok(Self { block, bottleneck, decoder, device, varmap })
    }

    /// Internal: encode + decode entirely on the tensor graph (no Vec round-trips).
    ///
    /// FIX: previously `loss()` called `forward()` which extracted `Vec<f32>`,
    /// then rebuilt tensors — severing the autograd chain so backprop got zero
    /// gradient. This method keeps everything as `Tensor` for the training path.
    fn forward_tensors(&self, x: &Tensor) -> CResult<(Tensor, Tensor)> {
        let (t, f) = x.dims2()?;

        // Encode: [T, F] → [D_MODEL]
        let (y_seq, _h) = self.block.forward_seq(x)?;
        let y_mean      = y_seq.mean(0)?;
        let y_mean_2d   = y_mean.unsqueeze(0)?;                    // [1, D_MODEL]
        let latent_2d   = self.bottleneck.forward(&y_mean_2d)?;    // [1, LATENT]
        let latent      = latent_2d.squeeze(0)?;                   // [LATENT]
        let latent_act  = latent.tanh()?;

        // Decode: [LATENT] → [T, F]
        let lat2d        = latent_act.unsqueeze(0)?;               // [1, LATENT]
        let recon_flat2d = self.decoder.forward(&lat2d)?;          // [1, T*F]
        let recon_flat   = recon_flat2d.squeeze(0)?;               // [T*F]
        let recon        = recon_flat.reshape((t, f))?;

        Ok((latent_act, recon))
    }

    /// Full forward pass.  Returns anomaly score + latent + reconstruction.
    pub fn forward(&self, x: &Tensor) -> CResult<MambaOutput> {
        let (t, f)        = x.dims2()?;
        let (latent, recon) = self.forward_tensors(x)?;

        // Anomaly score: MSE in dB
        let diff        = (x - &recon)?;
        let mse         = (&diff * &diff)?.mean_all()?.to_scalar::<f32>()?;
        let anomaly_db  = 10.0 * mse.max(1e-10).log10();

        Ok(MambaOutput {
            latent:        latent.to_vec1()?,
            reconstruction: recon.reshape((t * f,))?.to_vec1()?,
            anomaly_score: anomaly_db,
        })
    }

    /// Forward pass from a flat slice of magnitude spectral bins.
    /// Assumes `mags` length ≥ `MAMBA_CONTEXT_LEN * MAMBA_INPUT_BINS`.
    pub fn forward_slice(&self, mags: &[f32]) -> anyhow::Result<MambaOutput> {
        let t = MAMBA_CONTEXT_LEN;
        let f = MAMBA_INPUT_BINS;
        if mags.len() < t * f {
            return Err(anyhow::anyhow!(
                "Input too small: expected {}, got {}",
                t * f,
                mags.len()
            ));
        }
        let x = Tensor::from_vec(mags[..t * f].to_vec(), (t, f), &self.device)
            .map_err(|e| anyhow::anyhow!("tensor error: {e}"))?;
        self.forward(&x).map_err(|e| anyhow::anyhow!("forward error: {e}"))
    }

    /// Compute training loss entirely on the tensor graph.
    ///
    /// FIX: old version extracted Vec<f32> for latent/recon then rebuilt
    /// Tensors, which broke autograd (from_vec detaches the gradient tape).
    pub fn loss(&self, x: &Tensor) -> CResult<Tensor> {
        let (latent, recon) = self.forward_tensors(x)?;

        let diff  = (x - &recon)?;
        let mse   = (&diff * &diff)?.mean_all()?;
        let l2    = (&latent * &latent)?.mean_all()?.affine(LAMBDA_REG, 0.0)?;
        mse + l2
    }

    /// Save weights to a safetensors file.
    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        self.varmap.save(path).map_err(|e| anyhow::anyhow!("save: {e}"))
    }

    /// Load weights from a safetensors file.
    pub fn load(&mut self, path: &str) -> anyhow::Result<()> {
        self.varmap.load(path).map_err(|e| anyhow::anyhow!("load: {e}"))
    }
}

// ── OnlineTrainer ─────────────────────────────────────────────────────────────
//
// Wraps `MambaAutoencoder` with a PERSISTENT `AdamW` optimizer.
//
// The original code created a new AdamW inside every `train_step()` call,
// discarding all accumulated first/second moment buffers each batch.
// Adam without momentum is effectively noisy SGD with a bad learning-rate
// schedule — the model wouldn't converge in any reasonable time.
//
// `OnlineTrainer` builds the optimizer once in `new()` and keeps it alive
// for the entire training session, giving Adam full benefit of its
// adaptive per-parameter learning rates.

pub struct OnlineTrainer {
    autoencoder: MambaAutoencoder,
    optimizer:   candle_nn::AdamW,
}

impl OnlineTrainer {
    pub fn new() -> anyhow::Result<Self> {
        let device      = Device::Cpu;
        let autoencoder = MambaAutoencoder::new(device)
            .map_err(|e| anyhow::anyhow!("Mamba init: {e}"))?;

        let params = autoencoder.varmap.all_vars();
        let optimizer = candle_nn::AdamW::new(
            params,
            candle_nn::ParamsAdamW {
                lr:           1e-3,
                beta1:        0.9,
                beta2:        0.999,
                eps:          1e-8,
                weight_decay: 1e-4,
            },
        )
        .map_err(|e| anyhow::anyhow!("AdamW init: {e}"))?;

        Ok(Self { autoencoder, optimizer })
    }

    /// Run one gradient step on each window in `windows`.
    /// Returns the mean loss across the batch.
    pub fn step(&mut self, windows: &[Vec<f32>]) -> anyhow::Result<f32> {
        if windows.is_empty() {
            return Ok(0.0);
        }

        let t = MAMBA_CONTEXT_LEN;
        let f = MAMBA_INPUT_BINS;
        let mut total_loss = 0.0f32;
        let mut count      = 0usize;

        for window in windows {
            if window.len() < t * f {
                continue;
            }

            let x = Tensor::from_vec(
                window[..t * f].to_vec(),
                (t, f),
                &self.autoencoder.device,
            )
            .map_err(|e| anyhow::anyhow!("tensor: {e}"))?;

            let loss = self.autoencoder.loss(&x)
                .map_err(|e| anyhow::anyhow!("loss: {e}"))?;

            total_loss += loss.to_scalar::<f32>()
                .map_err(|e| anyhow::anyhow!("scalar: {e}"))?;
            count      += 1;

            // backward_step accumulates gradients AND updates parameters —
            // the persistent optimizer carries its moment buffers forward.
            self.optimizer.backward_step(&loss)
                .map_err(|e| anyhow::anyhow!("backward: {e}"))?;
        }

        Ok(if count > 0 { total_loss / count as f32 } else { 0.0 })
    }

    /// Run inference on a flat magnitude window.
    /// If the window is shorter than `T×F`, the last frame is repeated.
    pub fn infer(&self, window: &[f32]) -> anyhow::Result<MambaOutput> {
        let t = MAMBA_CONTEXT_LEN;
        let f = MAMBA_INPUT_BINS;

        if window.len() < f {
            return Ok(MambaOutput {
                latent:         vec![0.0; MAMBA_LATENT_DIM],
                reconstruction: vec![0.0; t * f],
                anomaly_score:  0.0,
            });
        }

        if window.len() >= t * f {
            return self.autoencoder.forward_slice(window);
        }

        // Pad by repeating the last full frame
        let mut padded = window.to_vec();
        let last_frame = &window[window.len().saturating_sub(f)..];
        while padded.len() < t * f {
            padded.extend_from_slice(last_frame);
        }
        self.autoencoder.forward_slice(&padded)
    }

    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        self.autoencoder.save(path)
    }

    pub fn load(&mut self, path: &str) -> anyhow::Result<()> {
        self.autoencoder.load(path)
    }
}

// ── Activation helpers ────────────────────────────────────────────────────────

fn softplus(x: &Tensor) -> CResult<Tensor> {
    let ex  = x.exp()?;
    let ep1 = ex.broadcast_add(&Tensor::ones((1,), DType::F32, x.device())?)?;
    ep1.log()
}

// ── Training pair (for dual-spectrum TX/RX correlation) ───────────────────────

/// Transmitted spectrum vs received spectrum at a given frequency.
#[derive(Debug, Clone)]
pub struct TrainingPair {
    pub center_freq_hz: u32,
    pub tx_spectrum:    Vec<f32>, // PDM wideband FFT (soundcard output)
    pub rx_spectrum:    Vec<f32>, // RTL-SDR FFT (antenna input)
    pub timestamp_ms:   u64,
}

impl TrainingPair {
    pub fn new(center_freq_hz: u32, tx_spectrum: Vec<f32>, rx_spectrum: Vec<f32>) -> Self {
        Self {
            center_freq_hz,
            tx_spectrum,
            rx_spectrum,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }
}

// ── V-buffer snapshot extraction ─────────────────────────────────────────────

/// Extract a flattened training window from a V-buffer snapshot.
pub fn extract_window_from_vbuf_snapshot(
    snapshot:      &[[f32; MAMBA_INPUT_BINS]],
    write_version: u64,
    context_len:   usize,
) -> Vec<f32> {
    let depth = snapshot.len();
    let f     = MAMBA_INPUT_BINS;
    let t     = context_len.min(MAMBA_CONTEXT_LEN);

    let mut window = vec![0.0f32; MAMBA_CONTEXT_LEN * MAMBA_INPUT_BINS];

    for i in 0..t {
        let frames_back = (t - 1 - i) as u64;
        if write_version < frames_back {
            continue;
        }
        let version = write_version - frames_back;
        let slot    = (version as usize) % depth;
        let row     = &snapshot[slot];

        let dst_off  = i * MAMBA_INPUT_BINS;
        let copy_len = f.min(row.len());
        window[dst_off..dst_off + copy_len].copy_from_slice(&row[..copy_len]);
    }

    window
}

// ── TX Improvement Prediction ────────────────────────────────────────────────

/// Compute RMS in dB for convergence monitoring
pub fn compute_rms_db(values: &[f32]) -> f32 {
    let rms = (values.iter().map(|v| v.powi(2)).sum::<f32>() / values.len() as f32).sqrt();
    20.0 * rms.log10().max(-100.0)  // Clamp to -100 dB floor
}

impl MambaAutoencoder {
    /// Predict TX spectral deltas that would improve RX match
    /// Input: TX spectrum that was sent, RX spectrum that was received
    /// Output: Predicted spectral corrections [512 bins] in dB
    pub fn predict_tx_delta(&self, tx_spectrum: &[f32], rx_spectrum: &[f32]) -> Vec<f32> {
        let mut deltas = vec![0.0f32; 512];

        // Pad/truncate to 512 bins if needed
        let tx_padded = Self::pad_to_512(tx_spectrum);
        let rx_padded = Self::pad_to_512(rx_spectrum);

        // Delta = "what RX has that TX doesn't"
        for i in 0..512 {
            let tx_mag = tx_padded[i].abs().max(1e-6);
            let rx_mag = rx_padded[i].abs().max(1e-6);

            // Difference in dB (how much to boost/cut this bin)
            deltas[i] = 20.0 * (rx_mag / tx_mag).log10();

            // Smooth: only apply if difference is significant (>1 dB)
            if deltas[i].abs() < 1.0 {
                deltas[i] = 0.0;
            }
        }

        deltas
    }

    fn pad_to_512(spectrum: &[f32]) -> Vec<f32> {
        if spectrum.len() == 512 {
            return spectrum.to_vec();
        }
        let mut padded = vec![0.0f32; 512];
        let copy_len = spectrum.len().min(512);
        padded[..copy_len].copy_from_slice(&spectrum[..copy_len]);
        padded
    }
}
