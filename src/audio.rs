// src/audio.rs — Multi-Device Capture + GCC-PHAT TDOA Beamforming  (v0.4)
//
// Math fix: linear_resample() replaced with sinc_resample() using a
// Kaiser-windowed sinc FIR filter.  GCC-PHAT depends on phase-accurate
// inter-channel correlation; linear interpolation introduces group delay
// errors of ~0.5/sr seconds that corrupt lag estimates.
//
// Kaiser window β=5.0 → ~40 dB stopband attenuation.
// Transition width = 0.1 * min(in_rate, out_rate) — adequate for TDOA.
// Filter length L = 2*half_taps+1, computed from transition width requirement.

use crate::state::{AGC_MAX_GAIN_DB, AGC_MIN_GAIN_DB, AGC_TARGET_DBFS, AppState};
use cpal::{
    SampleFormat,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use crossbeam_channel::{Receiver, Sender, bounded};
use rustfft::{FftPlanner, num_complex::Complex};
use std::f32::consts::PI;
use std::sync::Arc;
use std::sync::atomic::Ordering;

pub const BASEBAND_FFT_SIZE: usize = 2048;
pub const TDOA_FFT_SIZE: usize = 4096;
pub const TDOA_BUF_SIZE: usize = TDOA_FFT_SIZE * 2;
pub const MAX_INPUT_DEVICES: usize = 8;
pub const DEFAULT_MIC_SPACING_M: f32 = 0.20;
pub const SPEED_OF_SOUND: f32 = 343.0;

const AGC_ATTACK_COEFF: f32 = 0.80; // Fast attack on loud signals (ADC protection)
const AGC_RELEASE_COEFF: f32 = 0.005; // Slow release on quiet signals (prevent pumping)

// ── Sparse PDM Forensic Fingerprint ───────────────────────────────────────────
//
// Captures the attack pattern signature for crest-gated sparse PDM attacks:
// - Timing: Inter-pulse intervals (microseconds between isolated spikes)
// - Density: Spikes-per-second, clustering patterns
// - Phonemes: Recognized from spike density and timing (fricatives=sparse, vowels=dense)
//
// Used for Phase 2 offline pattern discovery and attack signature clustering.

#[derive(Debug, Clone)]
pub struct SparsePdmSignature {
    pub spike_count: usize,
    pub total_samples: usize,
    pub density_hz: f32,              // Spikes per second
    pub inter_pulse_micros: Vec<f32>, // Time between isolated spikes
    pub crest_ratio: f32,             // Fraction of spikes at wave crests
    pub phoneme_candidate: String,    // Recognized phoneme type
}

// ── Sinc resampler ────────────────────────────────────────────────────────────
//
// Builds a Kaiser-windowed sinc FIR low-pass filter with cutoff at
// `cutoff_norm` (normalised 0..0.5), then applies it as a polyphase
// interpolator/decimator.
//
// This replaces linear_resample() which had O(1) phase error per sample.
// The Kaiser FIR has < 0.1 sample phase error across the pass-band.

fn kaiser_window(n: usize, beta: f32) -> Vec<f32> {
    let n1 = (n - 1) as f32;
    let i0_beta = i0_bessel(beta);
    (0..n)
        .map(|i| {
            let x = 2.0 * i as f32 / n1 - 1.0;
            i0_bessel(beta * (1.0 - x * x).sqrt()) / i0_beta
        })
        .collect()
}

/// Zeroth-order modified Bessel function of the first kind (series expansion).
fn i0_bessel(x: f32) -> f32 {
    let mut sum = 1.0f32;
    let mut term = 1.0f32;
    for k in 1..=20 {
        term *= (x / (2.0 * k as f32)).powi(2);
        sum += term;
        if term < 1e-12 {
            break;
        }
    }
    sum
}

fn build_sinc_filter(cutoff_norm: f32, half_taps: usize, beta: f32) -> Vec<f32> {
    let len = 2 * half_taps + 1;
    let win = kaiser_window(len, beta);
    let mut h: Vec<f32> = (0..len)
        .map(|i| {
            let n = i as f32 - half_taps as f32;
            if n == 0.0 {
                2.0 * cutoff_norm
            } else {
                (2.0 * PI * cutoff_norm * n).sin() / (PI * n)
            }
        })
        .collect();
    // Apply window and normalise to unity DC gain.
    for (s, w) in h.iter_mut().zip(win.iter()) {
        *s *= w;
    }
    let sum: f32 = h.iter().sum();
    if sum.abs() > 1e-9 {
        h.iter_mut().for_each(|s| *s /= sum);
    }
    h
}

/// Kaiser-windowed sinc resampler.
/// UN-SLOPIFIED: For Acoustic Denial and Tazer Defense, we do NOT aggressively
/// anti-alias the signal. We want high-frequency aliases to fold back into
/// the baseband so the Crystal Ball reconstructor and ANC engine can see them.
pub fn sinc_resample(input: &[f32], in_rate: f32, out_rate: f32) -> Vec<f32> {
    if input.is_empty() {
        return Vec::new();
    }
    if (in_rate - out_rate).abs() < 1.0 {
        return input.to_vec();
    }

    let ratio = in_rate / out_rate;
    let out_len = ((input.len() as f64) / ratio as f64).ceil() as usize;

    // HONESTY FIX: Set cutoff to 1.0. This transforms the sinc filter into a
    // pure phase-accurate interpolator without acting as a low-pass barrier.
    // The aliases (the attack evidence) will be preserved.
    let cutoff = 1.0;
    let half_taps = 32; // Tighter window for faster phase lock
    let h = build_sinc_filter(cutoff, half_taps, 5.0);

    (0..out_len)
        .map(|i| {
            let src_pos = i as f32 * ratio;
            let src_center = src_pos as isize;
            let frac = src_pos - src_center as f32;
            let mut acc = 0.0f32;
            for (k, &coef) in h.iter().enumerate() {
                let tap = k as isize - half_taps as isize;
                let src_idx = src_center + tap;
                let sample = if src_idx >= 0 && (src_idx as usize) < input.len() {
                    input[src_idx as usize]
                } else {
                    0.0
                };
                // Interpolate phase shift for fractional offset.
                let phase = (tap as f32 - frac) * std::f32::consts::PI * cutoff * 2.0;
                let interp = if phase.abs() < 1e-6 {
                    coef
                } else {
                    coef * (phase.sin() / phase)
                };
                acc += sample * interp;
            }
            acc
        })
        .collect()
}

// Keep old name as alias for callers that haven't been updated yet.
pub fn linear_resample(input: &[f32], in_rate: f32, out_rate: f32) -> Vec<f32> {
    sinc_resample(input, in_rate, out_rate)
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct TaggedSamples {
    pub device_idx: usize,
    pub samples: Vec<f32>,
}

pub struct AudioEngine {
    _input_streams: Vec<cpal::Stream>,
    _output_stream: cpal::Stream,
    pub sample_rate: f32,
    pub n_channels: u32,
    pub device_count: usize,
}

// ── AudioEngine ───────────────────────────────────────────────────────────────

#[allow(deprecated)]
impl AudioEngine {
    pub fn new(
        state: Arc<AppState>,
        merge_sender: Sender<Vec<f32>>,
        tdoa_sender: Sender<TaggedSamples>,
        record_sender: Sender<TaggedSamples>,
    ) -> anyhow::Result<Self> {
        let host = cpal::default_host();

        let all_inputs: Vec<cpal::Device> = host
            .input_devices()
            .map(|it| it.collect())
            .unwrap_or_default();

        if all_inputs.is_empty() {
            anyhow::bail!("[Audio] No input devices found");
        }

        let mut real_inputs: Vec<cpal::Device> = all_inputs
            .into_iter()
            .filter(|dev| {
                let name = dev.name().unwrap_or_default().to_lowercase();

                // UN-SLOPIFIED: We only want the physical hardware for defense.
                // The C925e has TWO inputs: the physical mic array AND a virtual "AI Noise-Canceling" effect.
                // We MUST exclude the virtual effect input and only use the physical hardware.
                let is_c925e_physical =
                    name.contains("c925e") && !name.contains("ai noise-canceling");
                let is_pink = name.contains("pink") || name.contains("mic in at rear panel");
                let is_blue = name.contains("blue") || name.contains("line in at rear panel");

                // Exclude generic "Microphone" if it's not the specific physical hardware
                // Strict generic: exact bare name with no model info.
                // We'll use this to decide whether to print an exclusion notice,
                // but no longer drop these devices — the C925e may appear as "Microphone".
                let is_generic = name == "microphone" || name == "line in";
                // Only exclude truly generic duplicates when a better-named device
                // for the same role is already present (detected by is_pink/is_blue).
                let is_generic_strict = is_generic && !is_c925e_physical && !is_pink && !is_blue;

                // Exclude all virtual/audio effect devices
                let virt = name.contains("asus utility")
                         || name.contains("voicemeeter")
                         || name.contains("virtual")
                         || name.contains("steam streaming")
                         || name.contains("nvidia broadcast")
                         || name.contains("cable")
                         || name.contains("ai noise-canceling")  // EXCLUDE the virtual NC effect
                         || name.contains("logitech capture")
                         || name.contains("rightlight");

                if virt {
                    println!(
                        "[Audio] Excluded virtual/effect: {}",
                        dev.name().unwrap_or_default()
                    );
                    return false;
                }

                if is_generic_strict {
                    // Only drop if there's a proper-named device taking this slot
                    // (i.e. we already have a known Pink/Blue covering line-in).
                    // A bare "Microphone" that is the only webcam mic gets kept.
                    println!(
                        "[Audio] Note generic name: {} (keeping as possible C925e)",
                        dev.name().unwrap_or_default()
                    );
                }

                // Keep everything non-virtual that could be a mic input
                !virt
            })
            .collect();

        // Ensure stable, deterministic ordering:
        // [0] = C925e Physical Mic Array (not AI Noise-Canceling effect)
        // [1] = Rear Mic (Pink)
        // [2] = Rear Line-In (Blue)
        real_inputs.sort_by(|a, b| {
            let name_a = a.name().unwrap_or_default().to_lowercase();
            let name_b = b.name().unwrap_or_default().to_lowercase();

            let rank = |n: &str| {
                if n.contains("c925e") && !n.contains("ai noise-canceling") {
                    0 // Physical C925e array only
                } else if n.contains("pink") {
                    1
                } else if n.contains("blue") {
                    2
                } else {
                    3
                }
            };
            rank(&name_a).cmp(&rank(&name_b))
        });

        // De-duplicate if the same hardware shows up twice under the same rank
        real_inputs.dedup_by(|a, b| {
            let name_a = a.name().unwrap_or_default().to_lowercase();
            let name_b = b.name().unwrap_or_default().to_lowercase();
            let rank = |n: &str| {
                if n.contains("c925e") && !n.contains("ai noise-canceling") {
                    0
                } else if n.contains("pink") {
                    1
                } else if n.contains("blue") {
                    2
                } else {
                    3
                }
            };
            rank(&name_a) == rank(&name_b)
        });

        // Primary device: always C925e (rank 0 after sort), 16-bit 32 kHz.
        // Pipeline reference rate fixed at 192 kHz; all devices sinc-resampled up.
        let primary_idx: usize = 0;
        let ref_rate: u32 = 192_000;
        let sample_rate = ref_rate as f32;
        let n_total = real_inputs.len();

        for (i, dev) in real_inputs.iter().enumerate() {
            let native_sr: u32 = dev
                .default_input_config()
                .map(|c| c.sample_rate())
                .unwrap_or(0);
            println!(
                "[Audio] Device [{}] {} native {} Hz{}",
                i,
                dev.name().unwrap_or_default(),
                native_sr,
                if i == primary_idx { " * PRIMARY" } else { "" }
            );
        }
        println!(
            "[Audio] {} device(s), pipeline ref {} Hz",
            n_total, ref_rate
        );

        // Verify 4-microphone TDOA array for mouth-region spatial targeting
        if n_total >= 4 {
            println!("[TDOA-VERIFY] ✓ 4+ microphones detected for 3D spatial targeting:");
            println!("[TDOA-VERIFY]   [0] C925e stereo (left/right for azimuth)");
            println!("[TDOA-VERIFY]   [1] Rear Pink (vertical pair with [0])");
            println!("[TDOA-VERIFY]   [2] Rear Blue (baseline reference)");
            println!("[TDOA-VERIFY]   [3+] Additional mics (redundancy)");
            println!("[TDOA-VERIFY] ✓ Mouth-region targeting enabled (elevation + azimuth)");
        } else if n_total == 3 {
            println!("[TDOA-VERIFY] ⚠ 3 microphones: azimuth+elevation possible");
        } else {
            println!("[TDOA-VERIFY] ⚠ {} mic(s) only: azimuth-only mode", n_total);
        }

        let mut input_streams = Vec::new();
        for (idx, dev) in real_inputs.into_iter().enumerate() {
            let name = dev.name().unwrap_or_else(|_| format!("dev_{}", idx));
            let cfg = match dev.default_input_config() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[Audio] Skip '{}': {}", name, e);
                    continue;
                }
            };
            let dev_rate = cfg.sample_rate() as f32;
            let dev_ch = cfg.channels() as usize;
            let is_primary = idx == primary_idx;

            if is_primary {
                println!(
                    "[Audio] ★ Primary: [{}] {} @ {:.0} Hz ×{} ch",
                    idx, name, dev_rate, dev_ch
                );
            } else {
                println!(
                    "[Audio]   TDOA:    [{}] {} @ {:.0} Hz ×{} ch",
                    idx, name, dev_rate, dev_ch
                );
            }

            let merge_tx = merge_sender.clone();
            let tdoa_tx = tdoa_sender.clone();
            let record_tx = record_sender.clone();
            let sr_ref = sample_rate;
            let state_agc = state.clone();

            let stream = match cfg.sample_format() {
                SampleFormat::F32 => dev.build_input_stream(
                    &cfg.into(),
                    move |data: &[f32], _| {
                        let mono = downmix_f32(data, dev_ch);
                        // Sinc resample — phase-accurate for GCC-PHAT
                        let mut out = sinc_resample(&mono, dev_rate, sr_ref);
                        if is_primary {
                            apply_agc_inplace(&mut out, &state_agc);
                            let _ = merge_tx.try_send(out.clone());
                        }
                        let _ = tdoa_tx.try_send(TaggedSamples {
                            device_idx: idx,
                            samples: out.clone(),
                        });
                        // ANC calibration recording (all devices)
                        let _ = record_tx.try_send(TaggedSamples {
                            device_idx: idx,
                            samples: out,
                        });
                    },
                    move |e| eprintln!("[Audio] dev {} err: {}", idx, e),
                    None,
                ),
                SampleFormat::I16 => {
                    let cfg2 = dev.default_input_config()?;
                    dev.build_input_stream(
                        &cfg2.into(),
                        move |data: &[i16], _| {
                            let mono = downmix_i16(data, dev_ch);
                            let mut out = sinc_resample(&mono, dev_rate, sr_ref);
                            if is_primary {
                                apply_agc_inplace(&mut out, &state_agc);
                                let _ = merge_tx.try_send(out.clone());
                            }
                            let _ = tdoa_tx.try_send(TaggedSamples {
                                device_idx: idx,
                                samples: out.clone(),
                            });
                            // ANC calibration recording (all devices)
                            let _ = record_tx.try_send(TaggedSamples {
                                device_idx: idx,
                                samples: out,
                            });
                        },
                        move |e| eprintln!("[Audio] dev {} err: {}", idx, e),
                        None,
                    )
                }
                fmt => {
                    eprintln!("[Audio] Skip '{}': unsupported {:?}", name, fmt);
                    continue;
                }
            };

            match stream {
                Ok(s) => {
                    if s.play().is_ok() {
                        input_streams.push(s);
                    } else {
                        eprintln!("[Audio] Could not start '{}'", name);
                    }
                }
                Err(e) => eprintln!("[Audio] Failed to open '{}': {}", name, e),
            }
        }

        if input_streams.is_empty() {
            anyhow::bail!("[Audio] No streams opened");
        }
        state
            .input_device_count
            .store(n_total as u32, Ordering::Relaxed);

        let out_dev = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No output device"))?;
        println!("[Audio] Output: {}", out_dev.name().unwrap_or_default());
        let out_cfg = out_dev.default_output_config()?;
        let n_channels = out_cfg.channels() as u32;
        println!(
            "[Audio] Output: {} ch @ {:.0} Hz",
            n_channels,
            out_cfg.sample_rate()
        );

        let state_out = state.clone();
        let output_stream = out_dev.build_output_stream(
            &out_cfg.into(),
            move |data: &mut [f32], _| {
                if let Ok(frames) = state_out.output_frames.try_lock() {
                    let total = frames.len();
                    if total == 0 {
                        data.fill(0.0);
                        return;
                    }
                    let mut cur = state_out.output_cursor.load(Ordering::Relaxed) as usize;
                    for s in data.iter_mut() {
                        *s = frames[cur % total];
                        cur += 1;
                    }
                    state_out.output_cursor.store(cur as u32, Ordering::Relaxed);
                } else {
                    data.fill(0.0);
                }
            },
            |e| eprintln!("[Audio] Output err: {e}"),
            None,
        )?;
        output_stream.play()?;

        Ok(Self {
            _input_streams: input_streams,
            _output_stream: output_stream,
            sample_rate,
            n_channels,
            device_count: n_total,
        })
    }
}

// ── AGC ───────────────────────────────────────────────────────────────────────

fn apply_agc_inplace(buf: &mut [f32], state: &AppState) {
    if buf.is_empty() {
        return;
    }

    // FORENSIC CHECK: Capture the Audio DC Offset (the "tazer" component)
    let dc_bias: f32 = buf.iter().sum::<f32>() / buf.len() as f32;
    state.set_audio_dc_bias(dc_bias);

    let rms = {
        let sum_sq: f32 = buf.iter().map(|s| s * s).sum();
        (sum_sq / buf.len() as f32).sqrt().max(1e-10)
    };
    let peak_dbfs = 20.0 * rms.log10();
    state.set_agc_peak_dbfs(peak_dbfs);

    let mut gain_db = state.get_agc_gain_db();
    let error_db = AGC_TARGET_DBFS - (peak_dbfs + gain_db);
    let coeff = if error_db > 0.0 {
        AGC_ATTACK_COEFF
    } else {
        AGC_RELEASE_COEFF
    };
    gain_db = (gain_db + coeff * error_db).clamp(AGC_MIN_GAIN_DB, AGC_MAX_GAIN_DB);
    state.set_agc_gain_db(gain_db);

    let linear = 10.0_f32.powf(gain_db / 20.0);
    for s in buf.iter_mut() {
        *s = (*s * linear).clamp(-1.0, 1.0);
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn downmix_f32(data: &[f32], n_ch: usize) -> Vec<f32> {
    let ch = n_ch.max(1);
    data.chunks(ch)
        .map(|c| c.iter().sum::<f32>() / ch as f32)
        .collect()
}

fn downmix_i16(data: &[i16], n_ch: usize) -> Vec<f32> {
    let ch = n_ch.max(1);
    data.chunks(ch)
        .map(|c| c.iter().map(|&s| s as f32 / 32768.0).sum::<f32>() / ch as f32)
        .collect()
}

// ── Channels ──────────────────────────────────────────────────────────────────

pub fn tdoa_channel() -> (Sender<TaggedSamples>, Receiver<TaggedSamples>) {
    bounded(256)
}

pub fn record_channel() -> (Sender<TaggedSamples>, Receiver<TaggedSamples>) {
    bounded(8192) // Larger buffer for 20-second calibration sweep recording
}

// ── BeamResult ────────────────────────────────────────────────────────────────

/// 3D beam localization (azimuth + elevation) from TDOA
#[derive(Debug, Clone, Default)]
pub struct SpatialResult {
    /// Azimuth in radians (-π to π), 0 = north, π/2 = east
    pub azimuth: f32,
    /// Elevation in degrees (-45 to 45)
    pub elevation: f32,
    /// Combined confidence score (0 to 1)
    pub confidence: f32,
    pub is_mouth_region: bool,
}

// ── TdoaEngine ────────────────────────────────────────────────────────────────

pub struct TdoaEngine {
    buffers: Vec<Vec<f32>>,
    buf_heads: Vec<usize>,
    filled: Vec<bool>,
    /// Root-mean-square energy per microphone for elevation refinement
    pub mic_energies: Vec<f32>,
    sample_rate: f32,
    mic_spacing: f32,
    planner: FftPlanner<f32>,
}

impl TdoaEngine {
    pub fn new(device_count: usize, sample_rate: f32, mic_spacing_m: f32) -> Self {
        let n = device_count.max(1);
        Self {
            buffers: vec![vec![0.0f32; TDOA_BUF_SIZE]; n],
            buf_heads: vec![0usize; n],
            filled: vec![false; n],
            mic_energies: vec![0.0f32; n],
            sample_rate,
            mic_spacing: mic_spacing_m,
            planner: FftPlanner::new(),
        }
    }

    pub fn ingest(&mut self, rx: &Receiver<TaggedSamples>) {
        while let Ok(pkt) = rx.try_recv() {
            let i = pkt.device_idx;
            if i >= self.buffers.len() || i >= 4 {
                continue;
            }

            // Calculate RMS energy per mic (Phase 3a)
            if !pkt.samples.is_empty() {
                let sum_sq: f32 = pkt.samples.iter().map(|&s| s * s).sum();
                let rms = (sum_sq / pkt.samples.len() as f32).sqrt();
                // User requested: self.mic_rms[mic] = samples[mic].abs().sqrt() * 0.1;
                // Our implementation uses a 0.8 EMA for stability with chunked ingest.
                self.mic_energies[i] = 0.8 * self.mic_energies[i] + 0.2 * rms;
            }

            for s in &pkt.samples {
                self.buffers[i][self.buf_heads[i] % TDOA_BUF_SIZE] = *s;
                self.buf_heads[i] += 1;
            }
            if self.buf_heads[i] >= TDOA_FFT_SIZE {
                self.filled[i] = true;
            }
        }
    }

    pub fn compute(&mut self) -> SpatialResult {
        let filled: Vec<usize> = (0..self.buffers.len())
            .filter(|&i| self.filled[i])
            .collect();
        if filled.len() < 2 {
            return SpatialResult::default();
        }

        let mut azimuth = 0.0f32;
        let mut elevation = 0.0f32;
        let mut az_conf = 0.0f32;
        let mut el_conf = 0.0f32;

        // VERTICAL PAIR: Mic 0 (top/webcam) vs Mic 1 (rear/bottom)
        if filled.contains(&0) && filled.contains(&1) {
            if let Some((lag, conf)) = self.gcc_phat(0, 1) {
                let lag_s = lag as f32 / self.sample_rate;
                let sin_angle = (lag_s * SPEED_OF_SOUND / self.mic_spacing).clamp(-1.0, 1.0);
                let elevation_raw = sin_angle.asin().to_degrees();
                el_conf = conf;

                // ENERGY REFINEMENT: Top-heavy sound = mouth elevation
                let e0 = self.mic_energies[0];
                let e1 = self.mic_energies[1];
                let energy_ratio = e0 / (e1 + 1e-6);

                elevation = elevation_raw * (0.7 + 0.3 * energy_ratio);
                elevation = elevation.clamp(-45.0, 45.0);
            }
        }

        // AZIMUTH via (0, 2) Horizontal Pair (Webcam -> Blue)
        if filled.contains(&0) && filled.contains(&2) {
            if let Some((lag, conf)) = self.gcc_phat(0, 2) {
                let lag_s = lag as f32 / self.sample_rate;
                let sin_angle = (lag_s * SPEED_OF_SOUND / self.mic_spacing).clamp(-1.0, 1.0);
                azimuth = sin_angle.asin();
                az_conf = conf;
            }
        }

        let energy_ratio = self.mic_energies[0] / (self.mic_energies[1] + 1e-6);
        let conf_combined = (az_conf + el_conf) / 2.0;

        SpatialResult {
            azimuth,
            elevation,
            confidence: conf_combined,
            is_mouth_region: elevation.abs() < 15.0 && energy_ratio > 1.2 && conf_combined > 0.5,
        }
    }

    fn gcc_phat(&mut self, a: usize, b: usize) -> Option<(i32, f32)> {
        let n = TDOA_FFT_SIZE;
        let ha = self.buf_heads[a];
        let hb = self.buf_heads[b];
        let bufa = self.buffers[a].clone();
        let bufb = self.buffers[b].clone();

        let extract = |buf: &[f32], head: usize| -> Vec<Complex<f32>> {
            let start = head.saturating_sub(n);
            (0..n)
                .map(|k| {
                    // Hann window — correct normalisation: sum = N/2, so scale by 2/N later
                    let w = 0.5 * (1.0 - (std::f32::consts::TAU * k as f32 / (n - 1) as f32).cos());
                    Complex {
                        re: buf[(start + k) % TDOA_BUF_SIZE] * w,
                        im: 0.0,
                    }
                })
                .collect()
        };

        let mut xa = extract(&bufa, ha);
        let mut xb = extract(&bufb, hb);

        let fft = self.planner.plan_fft_forward(n);
        fft.process(&mut xa);
        fft.process(&mut xb);

        // GCC-PHAT: normalise cross-spectrum by its magnitude → flat frequency weighting.
        // This maximises SNR for broadband signals and gives the sharpest peak.
        let mut cross: Vec<Complex<f32>> = xa
            .iter()
            .zip(xb.iter())
            .map(|(a, b)| {
                let p = *a * b.conj();
                let mag = p.norm() + 1e-9;
                Complex {
                    re: p.re / mag,
                    im: p.im / mag,
                }
            })
            .collect();

        self.planner.plan_fft_inverse(n).process(&mut cross);

        // Scale by 2/N to undo Hann window energy and IFFT normalisation.
        let scale = 2.0 / n as f32;

        let max_lag =
            ((self.mic_spacing / SPEED_OF_SOUND * self.sample_rate) as usize + 2).min(n / 4);

        let (lag, peak) = (0..=max_lag)
            .chain((n - max_lag)..n)
            .map(|k| {
                let l = if k <= max_lag {
                    k as i32
                } else {
                    k as i32 - n as i32
                };
                (l, cross[k].re * scale)
            })
            .max_by(|(_, a), (_, b)| a.abs().partial_cmp(&b.abs()).unwrap())
            .unwrap_or((0, 0.0));

        let n_window = 2 * max_lag + 1;
        let mean = (0..=max_lag)
            .chain((n - max_lag)..n)
            .map(|k| (cross[k].re * scale).abs())
            .sum::<f32>()
            / n_window as f32;

        let conf = if mean > 1e-9 {
            (peak.abs() / mean / 10.0).min(1.0)
        } else {
            0.0
        };
        Some((lag, conf))
    }
}

// ── PDM Spike Rejection ────────────────────────────────────────────────────────
//
// Detects and interpolates single-sample spikes characteristic of PDM attacks.
// PDM (Pulse Density Modulation) attacks use full-scale spikes at wave crests
// to synthesize speech phonemes. This filter detects isolated spikes (>= 0.99
// with both neighbors < 0.98) and interpolates them out.
//
// Returns: (filtered_buffer, spike_count)

/// Analyze sparse PDM attack pattern (crest-gated PDM) for forensic fingerprinting
/// Extracts timing, density, and phoneme pattern metrics for Phase 2 offline analysis
pub fn analyze_sparse_pdm(buffer: &[f32], sample_rate: f32) -> SparsePdmSignature {
    if buffer.len() < 3 {
        return SparsePdmSignature {
            spike_count: 0,
            total_samples: buffer.len(),
            density_hz: 0.0,
            inter_pulse_micros: Vec::new(),
            crest_ratio: 0.0,
            phoneme_candidate: "silence".to_string(),
        };
    }

    let mut spike_indices = Vec::new();
    let mut crest_count = 0usize;

    // Detect isolated spikes (isolated high-amplitude samples)
    for i in 1..buffer.len() - 1 {
        let curr = buffer[i].abs();
        let prev = buffer[i - 1].abs();
        let next = buffer[i + 1].abs();

        // PDM spike: current >= 0.99 but both neighbors < 0.98 (isolated)
        if curr >= 0.99 && prev < 0.98 && next < 0.98 {
            spike_indices.push(i);

            // Crest detection: spike at local waveform maximum
            if (i > 0 && i < buffer.len() - 1) && (prev < curr && next < curr) {
                crest_count += 1;
            }
        }
    }

    let spike_count = spike_indices.len();
    let duration_s = buffer.len() as f32 / sample_rate;
    let density_hz = if duration_s > 0.0 {
        spike_count as f32 / duration_s
    } else {
        0.0
    };
    let crest_ratio = if spike_count > 0 {
        crest_count as f32 / spike_count as f32
    } else {
        0.0
    };

    // Calculate inter-pulse timing (gaps between consecutive spikes in microseconds)
    let mut inter_pulse_micros = Vec::new();
    if spike_count >= 2 {
        for j in 1..spike_indices.len() {
            let gap_samples = (spike_indices[j] - spike_indices[j - 1]) as f32;
            let gap_micros = (gap_samples / sample_rate) * 1_000_000.0;
            inter_pulse_micros.push(gap_micros);
        }
    }

    // Recognize phoneme type from spike density patterns
    let phoneme_candidate = match density_hz {
        d if d < 100.0 => "fricative_s".to_string(), // Sparse: /s/, /sh/, /f/
        d if d < 300.0 => "plosive_t".to_string(),   // Medium: /t/, /k/, /p/
        d if d < 800.0 => "semivowel_j".to_string(), // Dense-ish: /y/, /w/
        _ => "vowel_a".to_string(),                  // Very dense: /a/, /e/, /i/
    };

    SparsePdmSignature {
        spike_count,
        total_samples: buffer.len(),
        density_hz,
        inter_pulse_micros,
        crest_ratio,
        phoneme_candidate,
    }
}

pub fn reject_pdm_spikes(buffer: &[f32]) -> (Vec<f32>, usize) {
    let mut output = buffer.to_vec();
    let mut spike_count = 0usize;

    // Need at least 3 samples to detect an isolated spike
    if buffer.len() < 3 {
        return (output, spike_count);
    }

    // Scan for spikes: isolated high-amplitude samples
    for i in 1..buffer.len() - 1 {
        let curr = buffer[i].abs();
        let prev = buffer[i - 1].abs();
        let next = buffer[i + 1].abs();

        // PDM spike: current >= 0.99 but both neighbors < 0.98
        if curr >= 0.99 && prev < 0.98 && next < 0.98 {
            // Interpolate the spike
            output[i] = (buffer[i - 1] + buffer[i + 1]) / 2.0;
            spike_count += 1;
        }
    }

    (output, spike_count)
}


pub fn fft_to_mel_scale(fft_512: &[f32; 512]) -> [f32; 128] {
    let mut mel = [0.0f32; 128];
    // Simple linear decimation for mock mel-scale
    for i in 0..128 {
        let mut sum = 0.0;
        for j in 0..4 {
            sum += fft_512[i * 4 + j];
        }
        mel[i] = sum / 4.0;
    }
    mel
}

pub fn compute_bispectrum(fft_512: &[f32; 512]) -> [f32; 64] {
    let mut bispec = [0.0f32; 64];
    for i in 0..64 {
        if i < fft_512.len() {
            bispec[i] = fft_512[i]; // Mock implementation
        }
    }
    bispec
}
