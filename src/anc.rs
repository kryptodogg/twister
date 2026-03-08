// src/anc.rs — Acoustic Noise Cancellation: Phase Calibration + LMS Filter
//
// Why anti-phase mode was physically broken
// ──────────────────────────────────────────
// The previous implementation synthesised −sin(2π·f·t) and assumed that would
// cancel the incoming sin(2π·f·t) at the microphone position.  This ignores the
// round-trip acoustic delay τ between speaker and microphone, which introduces
// a phase offset φ = 2π·f·τ.  At 1 kHz with 34 cm speaker–mic distance:
//   τ = 0.34 / 343 ≈ 1 ms
//   φ = 2π × 1000 × 0.001 ≈ 6.28 rad = one full cycle → no net cancellation.
//
// Solution: measure τ once during calibration, then apply the correct phase
// advance so the emitted wave arrives anti-phase at the target location.
//
// Architecture
// ─────────────
//  1. PhaseCalibrator — one-shot calibration.
//     Plays a swept sine, records the response, estimates the impulse response
//     H(f) = Y(f)/X(f).  Stores per-frequency phase offset Φ(f).
//
//  2. LmsFilter — online adaptive filter.
//     Single-channel FxLMS (Filtered-x LMS) loop.
//     Reference:  delayed copy of the synthesised signal
//     Error:      primary microphone input (what we hear — target is silence)
//     Update:     w += μ · e · filtered_x
//
// Integration into main.rs dispatch loop
// ────────────────────────────────────────
//   let mut anc = AncEngine::new(sample_rate, mic_to_speaker_m);
//
//   // Once at startup (or on user request):
//   anc.calibrate(&speaker_samples, &mic_samples);
//
//   // Every dispatch frame:
//   let corrected_phase = anc.corrected_phase(detected_freq_hz);
//   gpu.params.targets[0].phase = corrected_phase;
//
//   // If you have a reference mic near the target:
//   let cancel = anc.lms_update(&mic_input, &synth_output);
//   // `cancel` is the LMS correction signal; mix it into synth_output.

use rustfft::{FftPlanner, num_complex::Complex};
use std::f32::consts::{PI, TAU};

// ── Constants ─────────────────────────────────────────────────────────────────

/// LMS step size μ.  Small enough for stability at audio rates, large enough
/// for ~10 ms convergence at 192 kHz.  Tune downward if the filter oscillates.
#[allow(dead_code)]
const LMS_MU: f32 = 0.0005;

/// LMS filter order (number of taps).  64 taps ≈ 0.33 ms at 192 kHz — long
/// enough to model the direct path in a typical room without modelling all
/// early reflections.
#[allow(dead_code)]
const LMS_TAPS: usize = 64;

/// Maximum acoustic delay the calibrator will search over (seconds).
/// 10 ms = 3.43 m maximum speaker–mic distance.
#[allow(dead_code)]
const MAX_DELAY_S: f32 = 0.010;

/// Calibration sweep duration (seconds).
#[allow(dead_code)]
pub const CALIB_SWEEP_S: f32 = 0.5;

// ── Per-frequency phase offset table ─────────────────────────────────────────

/// Estimated acoustic transfer function at a single frequency.
#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)]
pub struct AcousticTransfer {
    /// Acoustic delay from speaker to microphone in seconds.
    pub delay_s: f32,
    /// Phase offset at this frequency in radians.  The synthesiser should
    /// advance its phase by `phase_offset` to arrive anti-phase at the mic.
    pub phase_offset: f32,
    /// Magnitude ratio |Y(f)| / |X(f)|.  < 1 means loss in the acoustic path.
    pub magnitude_ratio: f32,
    /// Whether this entry was measured (true) or estimated by interpolation.
    pub measured: bool,
}

// ── Phase calibrator ──────────────────────────────────────────────────────────

/// One-shot acoustic phase calibrator.
///
/// Records the impulse response between the speaker and the primary
/// microphone, then provides per-frequency phase offsets that let the
/// synthesiser output the correct anti-phase signal.
#[allow(dead_code)]
pub struct PhaseCalibrator {
    sample_rate: f32,
    /// Measured transfer functions keyed by integer frequency in Hz.
    phase_table: Vec<(f32, AcousticTransfer)>,
    /// Best-estimate broadband delay (seconds).
    pub broadband_delay_s: f32,
}

#[allow(dead_code)]
impl PhaseCalibrator {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            phase_table: Vec::new(),
            broadband_delay_s: 0.0,
        }
    }

    /// Run calibration from a pair of buffers: `speaker_out` is what was
    /// played, `mic_in` is what the microphone recorded.  Both must be at
    /// `self.sample_rate`.
    ///
    /// The function estimates H(f) = Y(f)/X(f) and populates the internal
    /// phase table.  Call this once after the sweep completes.
    pub fn calibrate(&mut self, speaker_out: &[f32], mic_in: &[f32]) {
        let n = speaker_out.len().min(mic_in.len()).next_power_of_two();
        if n < 256 {
            eprintln!("[ANC] Calibration buffer too short ({} samples)", n);
            return;
        }

        // GCC to estimate broadband delay.
        self.broadband_delay_s = self.estimate_delay_gcc(speaker_out, mic_in, n);
        println!(
            "[ANC] Broadband acoustic delay: {:.3} ms ({:.1} cm at 343 m/s)",
            self.broadband_delay_s * 1e3,
            self.broadband_delay_s * 343.0 * 100.0
        );

        // Transfer function H(f) = Y(f) / X(f) with regularisation.
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(n);

        let window: Vec<f32> = (0..n)
            .map(|k| 0.5 * (1.0 - (TAU * k as f32 / (n - 1) as f32).cos()))
            .collect();

        // FAIL-SAFE: Clamp buffer indexing to available slice lengths
        let _nx = speaker_out.len().min(n);
        let _ny = mic_in.len().min(n);

        // FAIL-SAFE: Zero-pad to ensure exact FFT size 'n'
        let mut x: Vec<Complex<f32>> = vec![Complex { re: 0.0, im: 0.0 }; n];
        let mut y: Vec<Complex<f32>> = vec![Complex { re: 0.0, im: 0.0 }; n];

        for (i, (&s, &w)) in speaker_out.iter().zip(&window).enumerate() {
            if i >= n {
                break;
            }
            x[i] = Complex { re: s * w, im: 0.0 };
        }
        for (i, (&s, &w)) in mic_in.iter().zip(&window).enumerate() {
            if i >= n {
                break;
            }
            y[i] = Complex { re: s * w, im: 0.0 };
        }

        fft.process(&mut x);
        fft.process(&mut y);

        let bin_hz = self.sample_rate / n as f32;
        self.phase_table.clear();

        for (k, (xk, yk)) in x.iter().zip(y.iter()).enumerate().take(n / 2) {
            let freq_hz = k as f32 * bin_hz;
            if freq_hz < 1.0 {
                continue;
            }

            let xmag = xk.norm();
            if xmag < 1e-8 {
                continue;
            } // speaker had no energy at this bin

            // H(f) = Y(f) / X(f)
            let h = yk / xk;
            let hmag = h.norm();
            let phase_of_h = h.arg();

            // Anti-phase phase advance: we want to emit (phase_of_h + π) so the
            // acoustic path cancels itself.  Equivalently, subtract π from our
            // phase offset to flip the sign of the contribution.
            let phase_offset = phase_of_h + PI;

            let transfer = AcousticTransfer {
                delay_s: self.broadband_delay_s,
                phase_offset: phase_offset % TAU,
                magnitude_ratio: (1.0 / hmag.max(1e-6)).min(10.0), // inverse gain
                measured: true,
            };
            self.phase_table.push((freq_hz, transfer));
        }

        println!(
            "[ANC] Calibration complete: {} frequency points",
            self.phase_table.len()
        );
    }

    /// Look up the required phase advance for a given frequency.
    ///
    /// If the frequency was measured directly, returns the stored value.
    /// Otherwise linearly interpolates between the two nearest measured bins.
    /// If no calibration data exists, falls back to the broadband delay model.
    pub fn phase_advance_for(&self, freq_hz: f32) -> f32 {
        if self.phase_table.is_empty() {
            // No calibration: use the broadband delay estimate.
            // This is better than nothing but still approximate.
            let phi = TAU * freq_hz * self.broadband_delay_s + PI;
            return phi % TAU;
        }

        // Binary search for the nearest bin.
        let pos = self.phase_table.partition_point(|(f, _)| *f < freq_hz);

        let lerp = |a: &AcousticTransfer, b: &AcousticTransfer, t: f32| -> f32 {
            // Circular linear interpolation of phase.
            let da = a.phase_offset;
            let db = b.phase_offset;
            // Unwrap phase difference to shortest arc.
            let diff = {
                let d = db - da;
                if d > PI {
                    d - TAU
                } else if d < -PI {
                    d + TAU
                } else {
                    d
                }
            };
            (da + t * diff) % TAU
        };

        match (
            self.phase_table.get(pos.saturating_sub(1)),
            self.phase_table.get(pos),
        ) {
            (Some((fa, ta)), Some((fb, tb))) => {
                let t = (freq_hz - fa) / (fb - fa + 1e-9);
                lerp(ta, tb, t.clamp(0.0, 1.0))
            }
            (Some((_, t)), None) | (None, Some((_, t))) => t.phase_offset,
            (None, None) => unreachable!(),
        }
    }

    /// Generate the calibration sweep signal (linear chirp, 20 Hz → 96 kHz).
    /// `n_samples` should be at least `CALIB_SWEEP_S × sample_rate`.
    pub fn generate_sweep(sample_rate: f32, n_samples: usize) -> Vec<f32> {
        let f0 = 20.0f32;
        let f1 = (sample_rate / 2.0).min(96_000.0);
        let t_end = n_samples as f32 / sample_rate;
        let k = (f1 - f0) / t_end; // Hz/s
        (0..n_samples)
            .map(|i| {
                let t = i as f32 / sample_rate;
                // Linear chirp: instantaneous phase = 2π(f₀·t + k/2·t²)
                let phi = TAU * (f0 * t + 0.5 * k * t * t);
                phi.sin() * 0.5 // -6 dB amplitude to avoid clipping
            })
            .collect()
    }

    // ── Private ───────────────────────────────────────────────────────────────

    fn estimate_delay_gcc(&self, x: &[f32], y: &[f32], n: usize) -> f32 {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(n);
        let ifft = planner.plan_fft_inverse(n);

        let mut xa: Vec<Complex<f32>> = x
            .iter()
            .take(n)
            .map(|&s| Complex { re: s, im: 0.0 })
            .collect();
        let mut ya: Vec<Complex<f32>> = y
            .iter()
            .take(n)
            .map(|&s| Complex { re: s, im: 0.0 })
            .collect();
        xa.resize(n, Complex::default());
        ya.resize(n, Complex::default());

        fft.process(&mut xa);
        fft.process(&mut ya);

        // GCC-PHAT whitening.
        let mut cross: Vec<Complex<f32>> = xa
            .iter()
            .zip(ya.iter())
            .map(|(a, b)| {
                let p = *a * b.conj();
                let mag = p.norm() + 1e-12;
                p / mag
            })
            .collect();

        ifft.process(&mut cross);

        let max_lag = (MAX_DELAY_S * self.sample_rate) as usize;
        let (lag, _peak) = (0..=max_lag)
            .chain((n - max_lag)..n)
            .map(|k| {
                let l = if k <= max_lag {
                    k as i32
                } else {
                    k as i32 - n as i32
                };
                (l, cross[k].re.abs())
            })
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap_or((0, 0.0));

        // Only accept positive (causal) delays.
        lag.max(0) as f32 / self.sample_rate
    }
}

// ── LMS adaptive filter (FxLMS) ───────────────────────────────────────────────

/// Single-channel FxLMS adaptive filter.
///
/// Models the acoustic path from speaker to error microphone and adapts the
/// cancellation filter coefficients online to minimise the error signal.
///
/// ```text
///   reference x(n) ─── secondary path model S(z) ──→ filtered_x(n) ──┐
///              │                                                        │
///              └──── W(z) (adaptive filter) ──→ speaker ──→ room ──→ +Σ──→ e(n)
///                                                              ↑
///                                             desired d(n) = 0 (silence)
/// ```
///
/// The "secondary path" (speaker → microphone) is estimated during calibration.
/// Without a measured secondary path model, this falls back to a one-tap delay
/// model using the broadband acoustic delay.
#[allow(dead_code)]
pub struct LmsFilter {
    /// Adaptive filter coefficients w[k].
    weights: Vec<f32>,
    /// Reference signal delay line (ring buffer).
    ref_delay: Vec<f32>,
    ref_pos: usize,
    /// Filtered reference signal delay line (for FxLMS update).
    filt_ref_delay: Vec<f32>,
    filt_ref_pos: usize,
    /// LMS step size.
    mu: f32,
    /// Power estimate for NLMS normalisation.
    power: f32,
    /// Secondary path model (one impulse response).
    secondary_path: Vec<f32>,
    sec_delay: Vec<f32>,
    sec_pos: usize,
}

impl LmsFilter {
    /// Create a new LMS filter.  `secondary_path` is the estimated impulse
    /// response from speaker output to error microphone (in samples).
    /// Pass `&[1.0]` if unknown — equivalent to no secondary path model.
    pub fn new(secondary_path: Vec<f32>, mu: f32) -> Self {
        let sec_len = secondary_path.len().max(1);
        Self {
            weights: vec![0.0f32; LMS_TAPS],
            ref_delay: vec![0.0f32; LMS_TAPS],
            ref_pos: 0,
            filt_ref_delay: vec![0.0f32; LMS_TAPS],
            filt_ref_pos: 0,
            mu,
            power: 1.0,
            secondary_path,
            sec_delay: vec![0.0f32; sec_len],
            sec_pos: 0,
        }
    }

    /// Default LMS filter with no secondary path model.
    pub fn default_for(sample_rate: f32, delay_s: f32) -> Self {
        // Model the secondary path as a pure delay.
        let delay_samples = (delay_s * sample_rate).round() as usize;
        let mut sec = vec![0.0f32; delay_samples.max(1)];
        if let Some(last) = sec.last_mut() {
            *last = 1.0;
        }
        Self::new(sec, LMS_MU)
    }

    /// Process one block.
    ///
    /// - `reference`: what the system intends to emit (synthesiser output).
    /// - `error`: what the error microphone hears (primary mic input).
    ///
    /// Returns the cancellation signal to mix into the output.  Subtract this
    /// from the synthesiser output before sending to the speaker.
    pub fn update(&mut self, reference: &[f32], error: &[f32]) -> Vec<f32> {
        let n = reference.len().min(error.len());
        let mut cancel = Vec::with_capacity(n);

        for k in 0..n {
            let x = reference[k];
            let e = error[k];

            // Push reference into delay line.
            self.ref_delay[self.ref_pos % LMS_TAPS] = x;
            self.ref_pos += 1;

            // Filter reference through secondary path model → filtered_x.
            let sec_len = self.secondary_path.len();
            self.sec_delay[self.sec_pos % sec_len] = x;
            self.sec_pos += 1;
            let mut fx = 0.0f32;
            for (j, &h) in self.secondary_path.iter().enumerate() {
                fx += self.sec_delay[(self.sec_pos + sec_len - j - 1) % sec_len] * h;
            }

            // Push filtered_x into its own delay line.
            self.filt_ref_delay[self.filt_ref_pos % LMS_TAPS] = fx;
            self.filt_ref_pos += 1;

            // Compute adaptive filter output y(n) = w · x_delay.
            let mut y = 0.0f32;
            for (j, &w) in self.weights.iter().enumerate() {
                y += w * self.ref_delay[(self.ref_pos + LMS_TAPS - j - 1) % LMS_TAPS];
            }
            cancel.push(y);

            // NLMS update: w += (μ / (P + ε)) · e · filtered_x_delay.
            self.power = 0.999 * self.power + 0.001 * fx * fx;
            let mu_norm = self.mu / (self.power + 1e-6);
            for (j, w) in self.weights.iter_mut().enumerate() {
                *w += mu_norm
                    * e
                    * self.filt_ref_delay[(self.filt_ref_pos + LMS_TAPS - j - 1) % LMS_TAPS];
            }

            // Clip weights to prevent divergence.
            for w in self.weights.iter_mut() {
                if w.is_nan() || w.abs() > 10.0 {
                    *w = 0.0;
                }
            }
        }

        cancel
    }

    /// Initialize filter weights from calibrated phase response.
    /// Maps frequency bins to LMS tap weights for adaptive cancellation.
    pub fn initialize_from_calibration(
        &mut self,
        phase_for_fn: &impl Fn(usize) -> f32,
        confidence_for_fn: &impl Fn(usize) -> f32,
    ) {
        let num_taps = self.weights.len();
        for tap in 0..num_taps {
            let bin = (tap * 8192) / num_taps;
            let phase = phase_for_fn(bin);
            let conf = confidence_for_fn(bin);
            self.weights[tap] = phase * conf;
        }
    }

    /// Reset all filter state (weights and delay lines).
    pub fn reset(&mut self) {
        self.weights.fill(0.0);
        self.ref_delay.fill(0.0);
        self.filt_ref_delay.fill(0.0);
        self.sec_delay.fill(0.0);
        self.ref_pos = 0;
        self.filt_ref_pos = 0;
        self.sec_pos = 0;
        self.power = 1.0;
    }

    /// Current residual error power — useful for convergence monitoring.
    pub fn power(&self) -> f32 {
        self.power
    }

    /// Root sum of squared weights — indicates filter energy.
    pub fn weight_rms(&self) -> f32 {
        (self.weights.iter().map(|w| w * w).sum::<f32>() / LMS_TAPS as f32).sqrt()
    }
}

// ── AncEngine — top-level entry point ─────────────────────────────────────────

/// Combined calibrator + adaptive filter.
///
/// Usage in dispatch loop:
/// ```ignore
/// // In AppState or dispatch locals:
/// let mut anc = AncEngine::new(sample_rate, 0.34);  // 34 cm speaker–mic distance
///
/// // On calibration button press / startup:
/// let sweep = AncEngine::calibration_sweep(sample_rate);
/// // ... play sweep, record mic_response ...
/// anc.calibrate(&sweep, &mic_response);
///
/// // Every frame, before synthesis:
/// let phase = anc.phase_for(detected_hz);    // use instead of 0.0
/// gpu.params.targets[0].phase = phase;
///
/// // If reference mic available, after synthesis:
/// let correction = anc.update(&synthesized, &mic_input);
/// // mix correction into synthesized before output
/// ```
#[allow(dead_code)]
pub struct AncEngine {
    pub calibrator: PhaseCalibrator,
    pub lms: LmsFilter,
    pub calibrated: bool,
    sample_rate: f32,
}

impl AncEngine {
    /// Create an uncalibrated engine.
    /// `nominal_distance_m` is the estimated speaker-to-microphone distance for
    /// the initial delay model — used before calibration.
    pub fn new(sample_rate: f32, nominal_distance_m: f32) -> Self {
        let nominal_delay = nominal_distance_m / 343.0;
        let calibrator = PhaseCalibrator::new(sample_rate);
        let lms = LmsFilter::default_for(sample_rate, nominal_delay);
        let mut engine = Self {
            calibrator,
            lms,
            calibrated: false,
            sample_rate,
        };
        // Initialize LMS filter state
        engine.lms.reset();
        engine
    }

    /// Run calibration.  After this, `phase_for` returns measured phase offsets.
    pub fn calibrate(&mut self, sweep_played: &[f32], mic_recorded: &[f32]) {
        self.calibrator.calibrate(sweep_played, mic_recorded);
        // Rebuild LMS with the measured secondary path.
        let delay = self.calibrator.broadband_delay_s;
        self.lms = LmsFilter::default_for(self.sample_rate, delay);
        self.calibrated = true;
    }

    /// Return the corrected phase advance for a given frequency.
    ///
    /// Synthesising `sin(2π·f·t + phase_for(f))` will cause the acoustic
    /// wave to arrive at the microphone position with phase ≈ π relative to
    /// the original source — i.e. anti-phase cancellation.
    pub fn phase_for(&self, freq_hz: f32) -> f32 {
        self.calibrator.phase_advance_for(freq_hz)
    }

    /// Update the adaptive filter with one block of reference and error signals.
    /// Returns the cancellation correction to subtract from output.
    pub fn update(&mut self, reference: &[f32], error: &[f32]) -> Vec<f32> {
        let cancel = self.lms.update(reference, error);

        // ANC diagnostic: log filter power and weight_rms for convergence monitoring
        let filter_power = self.lms.power();
        let weight_rms = self.lms.weight_rms();
        eprintln!(
            "[ANC] LMS Filter: power={:.6} dB, weight_rms={:.6}",
            10.0 * filter_power.log10().max(-120.0),
            weight_rms
        );

        cancel
    }

    /// Hybrid Smart ANC: blend stable LMS with Mamba residual prediction.
    ///
    /// `blend` controls the mix: 0.0 = pure LMS, 1.0 = pure Mamba residual.
    /// Recommended: 0.3 (30% Mamba) for stability during harassment defense.
    ///
    /// The Mamba residual is added to the LMS output, not replacing it:
    ///   output = lms_cancel + blend * mamba_residual
    ///
    /// If `mamba_residual` is None or empty, falls back to pure LMS.
    pub fn update_hybrid(
        &mut self,
        reference: &[f32],
        error: &[f32],
        mamba_residual: Option<&[f32]>,
        blend: f32,
    ) -> Vec<f32> {
        let lms_cancel = self.lms.update(reference, error);
        let blend = blend.clamp(0.0, 1.0);

        match mamba_residual {
            Some(residual) if blend > 0.001 && !residual.is_empty() => {
                let n = lms_cancel.len().min(residual.len());
                let mut output = lms_cancel;
                for i in 0..n {
                    output[i] += blend * residual[i];
                }
                output
            }
            _ => lms_cancel,
        }
    }

    /// Generate a calibration sweep to play through the speaker.
    pub fn calibration_sweep(sample_rate: f32) -> Vec<f32> {
        let n = (CALIB_SWEEP_S * sample_rate) as usize;
        PhaseCalibrator::generate_sweep(sample_rate, n)
    }

    /// Status string for the UI.
    pub fn status(&self) -> String {
        if self.calibrated {
            format!(
                "Calibrated — delay {:.2} ms, LMS power {:.2e}",
                self.calibrator.broadband_delay_s * 1e3,
                self.lms.power()
            )
        } else {
            format!(
                "Uncalibrated — using nominal delay {:.2} ms",
                self.calibrator.broadband_delay_s * 1e3
            )
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    /// Generate a sweep, apply a known delay, and verify the calibrator
    /// recovers the delay to within ±1 sample.
    #[test]
    fn calibrator_recovers_delay() {
        let n_samples = (CALIB_SWEEP_S * SR) as usize;
        let sweep = PhaseCalibrator::generate_sweep(SR, n_samples);

        // Simulate 5 ms acoustic delay.
        let delay_s = 0.005f32;
        let delay_n = (delay_s * SR) as usize;
        let mut mic = vec![0.0f32; n_samples + delay_n];
        for (i, &s) in sweep.iter().enumerate() {
            mic[i + delay_n] = s * 0.8; // 20% loss through air
        }

        let mut cal = PhaseCalibrator::new(SR);
        cal.calibrate(&sweep, &mic[..n_samples]);

        let err_s = (cal.broadband_delay_s - delay_s).abs();
        assert!(
            err_s < 1.0 / SR,
            "delay error {:.3} ms exceeds ±1 sample",
            err_s * 1e3
        );
    }

    /// LMS filter should reduce error RMS over 1000 update steps on a steady tone.
    #[test]
    fn lms_converges_on_tone() {
        let freq = 1000.0f32;
        let n = 512usize;

        let reference: Vec<f32> = (0..n * 10)
            .map(|k| (TAU * freq * k as f32 / SR).sin())
            .collect();

        // Simulate error = reference delayed by 2 samples (simple secondary path).
        let error: Vec<f32> = (0..n * 10)
            .map(|k| if k >= 2 { reference[k - 2] } else { 0.0 })
            .collect();

        let mut lms = LmsFilter::new(vec![0.0, 0.0, 1.0], LMS_MU);

        let mut initial_power = f32::NAN;
        let mut converged_power = 0.0;

        for block in 0..10 {
            let start = block * n;
            let cancel = lms.update(&reference[start..start + n], &error[start..start + n]);
            let residual: f32 = error[start..start + n]
                .iter()
                .zip(cancel.iter())
                .map(|(e, c)| (e - c).powi(2))
                .sum::<f32>()
                / n as f32;
            if block == 0 {
                initial_power = residual;
            }
            if block == 9 {
                converged_power = residual;
            }
        }

        assert!(
            converged_power < initial_power * 0.1,
            "LMS did not converge: initial power {:.4}, final {:.4}",
            initial_power,
            converged_power
        );
    }
}
