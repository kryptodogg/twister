// src/sdr.rs — RTL-SDR Control + IQ Capture + Frequency Bridge
//
// Uses crate::rtlsdr (our own safe FFI wrapper over rtlsdr_ffi.rs).
// The "desperado" crate does not exist; all references removed.
// When built without --features rtlsdr, the FFI stubs return
// "device not found" and this thread parks itself gracefully.

use crossbeam_channel::bounded;
use rustfft::{num_complex::Complex, FftPlanner};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crate::rtlsdr::RtlSdrEngine;
use crate::state::AppState;
use crate::vbuffer::V_FREQ_BINS;

pub const SDR_FFT_SIZE: usize = 2048;
pub const SDR_BLOCK_SAMPLES: usize = 16384;
pub const SDR_DEFAULT_RATE: u32 = 2_048_000;
pub const SDR_DEFAULT_GAIN: f32 = 20.0; // dB; 0.0 = auto AGC

/// Returns candidate tuning frequencies for soundcard harmonic hunting.
pub fn soundcard_harmonic_candidates(audio_rate: f32) -> Vec<f32> {
    let sdr_max = 1_766_000_000.0f32;
    let mut out = Vec::new();
    let mut n = 3u32;
    while (n as f32 * audio_rate) <= sdr_max {
        out.push(n as f32 * audio_rate);
        if out.len() >= 8 {
            break;
        }
        n += 1;
    }
    out
}

// ── IQ block type ─────────────────────────────────────────────────────────────

pub struct IqBlock {
    pub iq: Vec<Complex<f32>>,
    pub center_hz: f32,
    pub sample_rate: f32,
}

// ── FFT processing ─────────────────────────────────────────────────────────────

/// Hann-windowed FFT of IQ samples → fftshift → magnitude → downsampled to V_FREQ_BINS.
pub fn process_iq_to_mags(iq: &[Complex<f32>], planner: &mut FftPlanner<f32>) -> Vec<f32> {
    let n = iq.len().min(SDR_FFT_SIZE).next_power_of_two();
    if n == 0 {
        return vec![0.0; V_FREQ_BINS];
    }

    let mut buf: Vec<Complex<f32>> = iq[..n]
        .iter()
        .enumerate()
        .map(|(k, c)| {
            let w = 0.5 * (1.0 - (std::f32::consts::TAU * k as f32 / (n - 1) as f32).cos());
            Complex {
                re: c.re * w,
                im: c.im * w,
            }
        })
        .collect();

    planner.plan_fft_forward(n).process(&mut buf);

    let mut mags = vec![0.0f32; n];
    for k in 0..n {
        mags[(k + n / 2) % n] = buf[k].norm();
    }

    downsample_spectrum(&mags, V_FREQ_BINS)
}

fn downsample_spectrum(mags: &[f32], target: usize) -> Vec<f32> {
    let src = mags.len();
    (0..target)
        .map(|i| {
            let lo = i * src / target;
            let hi = ((i + 1) * src / target).min(src);
            mags[lo..hi].iter().cloned().fold(0.0f32, f32::max)
        })
        .collect()
}

// ── SDR channel ───────────────────────────────────────────────────────────────

pub type SdrMagSender = crossbeam_channel::Sender<(Vec<f32>, f32, f32)>;
pub type SdrMagReceiver = crossbeam_channel::Receiver<(Vec<f32>, f32, f32)>;

pub fn sdr_channel() -> (SdrMagSender, SdrMagReceiver) {
    bounded(32)
}

struct SdrEngine {
    state: Arc<AppState>,
    device: RtlSdrEngine,
    center_hz: f32,
    last_center_hz: f32,
}

impl SdrEngine {
    fn update_status(&mut self) {
        self.state.set_rtl_connected(self.device.is_open());
        if self.device.is_open() {
            self.state.log(
                "INFO",
                "SDR",
                &format!("CONNECTED {:.1}MHz", self.center_hz / 1e6),
            );
        }
    }
}

// ── Background SDR capture thread ─────────────────────────────────────────────

pub fn spawn_sdr_thread(state: Arc<AppState>, mag_tx: SdrMagSender) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut planner = FftPlanner::new();

        loop {
            if !state.sdr_active.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }

            let dev_idx = state.sdr_device_index.load(Ordering::Relaxed);
            let rate = state.sdr_sample_rate.load(Ordering::Relaxed);
            let gain_db = state.get_sdr_gain_db();

            let engine = match RtlSdrEngine::with_device(dev_idx) {
                Ok(e) => e,
                Err(e) => {
                    state.rtl_connected.store(false, Ordering::Relaxed);
                    state.log(
                        "ERROR",
                        "SDR",
                        &format!("Cannot open device {dev_idx}: {e:?}. Retrying in 2s."),
                    );
                    std::thread::sleep(Duration::from_secs(2));
                    continue;
                }
            };

            let mut sdr = SdrEngine {
                state: state.clone(),
                device: engine,
                center_hz: state.get_sdr_center_hz(),
                last_center_hz: 0.0,
            };

            sdr.update_status();

            if let Err(e) = sdr.device.set_sample_rate(rate) {
                state.log("ERROR", "SDR", &format!("set_sample_rate failed: {e:?}"));
            }

            if gain_db > 0.0 {
                let _ = sdr.device.set_gain(gain_db);
            } else {
                let _ = sdr.device.set_agc_mode(true);
            }

            if sdr.center_hz < 24_000_000.0 {
                let _ = sdr.device.configure_hf();
                state.log("INFO", "SDR", "→ HF mode (<24MHz)");
            } else {
                let _ = sdr.device.configure_vhf_uhf();
                state.log("INFO", "SDR", "→ VHF/UHF mode (≥24MHz)");
            }
            sdr.last_center_hz = sdr.center_hz;
            let _ = sdr.device.tune(sdr.center_hz as u32);

            'capture: loop {
                if !state.sdr_active.load(Ordering::Relaxed) {
                    break;
                }

                let target = if state.auto_tune.load(Ordering::Relaxed) {
                    (state.get_sdr_center_hz() / 192_000.0).round() * 192_000.0
                } else {
                    state.get_sdr_center_hz()
                };

                if (target - sdr.center_hz).abs() > 10.0 {
                    sdr.center_hz = target;
                    let prev_band = sdr.last_center_hz / 24e6;
                    let curr_band = sdr.center_hz / 24e6;

                    if prev_band.floor() != curr_band.floor() {
                        if sdr.center_hz < 24e6 {
                            let _ = sdr.device.configure_hf();
                            state.log("INFO", "SDR", "→ HF mode (<24MHz)");
                        } else {
                            let _ = sdr.device.configure_vhf_uhf();
                            state.log("INFO", "SDR", "→ VHF/UHF mode (≥24MHz)");
                        }
                    }
                    sdr.last_center_hz = sdr.center_hz;

                    if let Err(e) = sdr.device.tune(sdr.center_hz as u32) {
                        state.log("ERROR", "SDR", &format!("Retune failed: {e:?}"));
                        break 'capture;
                    }
                    sdr.update_status();
                }

                let iq = match sdr.device.read_iq() {
                    Ok(v) => v,
                    Err(e) => {
                        state.log("ERROR", "SDR", &format!("Read error: {e:?}. Reopening."));
                        break 'capture;
                    }
                };

                if iq.is_empty() {
                    continue;
                }

                let mags = process_iq_to_mags(&iq, &mut planner);
                let bin_hz = rate as f32 / SDR_FFT_SIZE as f32;
                if let Some((peak_bin, &peak_mag)) = mags
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                {
                    let offset = (peak_bin as f32 - V_FREQ_BINS as f32 / 2.0) * bin_hz;
                    state.set_sdr_peak_dbfs(if peak_mag > 1e-10 {
                        20.0 * peak_mag.log10()
                    } else {
                        -100.0
                    });
                    state.set_sdr_peak_offset_hz(offset);
                }

                let _ = mag_tx.try_send((mags, sdr.center_hz, rate as f32));
            }

            state.rtl_connected.store(false, Ordering::Relaxed);
            state.log("INFO", "SDR", &format!("Device {} closed.", dev_idx));
        }
    })
}

/// List connected RTL-SDR devices via our own FFI.
pub fn list_devices() -> Vec<String> {
    let count = unsafe { crate::rtlsdr_ffi::rtlsdr_get_device_count() };
    (0..count)
        .map(|i| format!("RTL-SDR device {}", i))
        .collect()
}
