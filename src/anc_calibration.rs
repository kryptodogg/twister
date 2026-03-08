// src/anc_calibration.rs — Full-range ANC calibration (1 Hz - 12.288 MHz)
//
// Three-channel simultaneous sweep capture + phase analysis:
//   Channel 0: C925e physical microphone (16-bit 32 kHz, rank-0 primary)
//   Channel 1: Rear panel mic (Pink, 192 kHz)
//   Channel 2: Rear line-in (Blue, 192 kHz) — optional, may be absent
//
// All channels sinc-resampled to 192 kHz reference before FFT.
// Phase analysis across full range (1 Hz - 12.288 MHz Nyquist)
// Mamba-weighted confidence per FFT bin
//

use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::{PI, TAU};

const AUDIO_SR: f32 = 192_000.0;
const PDM_NYQUIST: f32 = 12_288_000.0; // 6.144 MHz (PDM clock = 2 × 12.288 MHz)
const TOTAL_BINS: usize = 8192; // FFT bins across full range
const CALIBRATION_DURATION_S: f32 = 20.0; // Total sweep duration

/// Full-range phase calibration lookup table
#[derive(Clone, Debug)]
pub struct FullRangeCalibration {
    /// Per-frequency phase offsets (0-2π radians)
    /// Index: FFT bin (0-8191 maps 1 Hz - 12.288 MHz)
    phase_lut: Vec<f32>,

    /// Mamba confidence scores per bin (0.0-1.0)
    /// Higher = more confident in phase measurement
    confidence: Vec<f32>,

    /// Phase difference between channel pairs
    /// [0]: C925e ↔ Pink, [1]: C925e ↔ Blue, [2]: Pink ↔ Blue
    phase_delta: Vec<[f32; 3]>,

    /// Timestamp of last calibration
    last_calibration_time: Option<String>,
}

impl FullRangeCalibration {
    pub fn new() -> Self {
        Self {
            phase_lut: vec![0.0; TOTAL_BINS],
            confidence: vec![0.0; TOTAL_BINS],
            phase_delta: vec![[0.0; 3]; TOTAL_BINS],
            last_calibration_time: None,
        }
    }

    /// Generate 1 Hz - 12.288 MHz log-sweep calibration signal
    pub fn generate_full_range_sweep() -> Vec<f32> {
        let mut sweep = Vec::new();

        // Phase 1: Audio band (1 Hz - 192 kHz) - uses CALIBRATION_DURATION_S * 0.3
        let audio_duration = CALIBRATION_DURATION_S * 0.3; // 6 seconds of 20s total
        let audio_samples = (audio_duration * AUDIO_SR) as usize;
        let audio_sweep = generate_log_chirp(1.0, AUDIO_SR / 2.0, AUDIO_SR, audio_samples);
        sweep.extend(&audio_sweep);

        // Phase 2: PDM band (192 kHz - 12.288 MHz) - uses CALIBRATION_DURATION_S * 0.7
        // Progressive: 192k→1.5M (4s), 1.5M→6M (6s), 6M→12.288M (4s)
        let pdm_duration = CALIBRATION_DURATION_S * 0.7; // 14 seconds of 20s total

        // Sub-phase 2a: 192 kHz → 1.5 MHz (4s = pdm_duration * 4/14)
        let sub1_samples = ((pdm_duration * 4.0 / 14.0) * AUDIO_SR) as usize;
        let sub1 = generate_log_chirp(AUDIO_SR / 2.0, 1_500_000.0, AUDIO_SR, sub1_samples);
        sweep.extend(&sub1);

        // Sub-phase 2b: 1.5 MHz → 6 MHz (6s = pdm_duration * 6/14)
        let sub2_samples = ((pdm_duration * 6.0 / 14.0) * AUDIO_SR) as usize;
        let sub2 = generate_log_chirp(1_500_000.0, 6_000_000.0, AUDIO_SR, sub2_samples);
        sweep.extend(&sub2);

        // Sub-phase 2c: 6 MHz → 12.288 MHz (4s = pdm_duration * 4/14)
        let sub3_samples = ((pdm_duration * 4.0 / 14.0) * AUDIO_SR) as usize;
        let sub3 = generate_log_chirp(6_000_000.0, PDM_NYQUIST, AUDIO_SR, sub3_samples);
        sweep.extend(&sub3);

        sweep
    }

    /// Analyze 3-channel response to calibration sweep
    ///
    /// # Arguments
    /// - `ch0_response`: C925e microphone response to sweep
    /// - `ch1_response`: Rear panel mic response
    /// - `ch2_response`: Rear line-in response
    /// - `mamba_anomaly_fn`: Function returning anomaly score (0=normal, high=anomalous)
    ///
    /// Fills phase_lut, confidence, and phase_delta via FFT analysis
    pub fn calibrate_from_sweep(
        &mut self,
        ch0_raw: &[f32],
        ch1_raw: &[f32],
        ch2_raw: &[f32],
        mamba_anomaly_fn: impl Fn(usize) -> f32,
    ) {
        // ch0 = C925e physical mic: native 16-bit 32 kHz → upsample to 192 kHz reference
        let ch0 = crate::anc_recording::resample_to_ref(ch0_raw, 32_000.0, 192_000.0);
        let ch1 = crate::anc_recording::resample_to_ref(ch1_raw, 192000.0, 192000.0);
        let ch2 = crate::anc_recording::resample_to_ref(ch2_raw, 192000.0, 192000.0);

        println!(
            "[ANC] Resampled: ch0={}→{} samples, ch1={}→{} samples, ch2={}→{} samples",
            ch0_raw.len(),
            ch0.len(),
            ch1_raw.len(),
            ch1.len(),
            ch2_raw.len(),
            ch2.len()
        );
        // UN-SLOPIFIED: We actually use the generated sweep as the reference signal for correlation.
        let sweep = Self::generate_full_range_sweep();

        // FFT size: largest power-of-2 <= min of available channels.
        // ch2 may be empty if Blue line-in is not connected — exclude it from min.
        let ch2_len = if ch2.is_empty() {
            usize::MAX
        } else {
            ch2.len()
        };
        let min_len = ch0.len().min(ch1.len()).min(ch2_len).min(sweep.len());
        let fft_size = (min_len as f32).log2().floor() as u32;
        let fft_size = 2_usize.pow(fft_size.max(12).min(20));

        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(fft_size);

        // 1. Transform the reference sweep
        let mut ref_fft = to_complex_fft(&sweep[..fft_size], fft_size);
        fft.process(&mut ref_fft);

        // 2. Transform the responses (using resampled data).
        // Guard: ch2 (Blue line-in) may be absent if only 2 devices are connected.
        // Use a zero-filled FFT buffer as a neutral placeholder — xcorr will be
        // near-zero magnitude so confidence for ch2-dependent bins stays at 0.
        let ch2_safe: &[f32] = if ch2.len() >= fft_size {
            &ch2[..fft_size]
        } else {
            &[]
        };

        let mut ch0_fft = to_complex_fft(&ch0[..fft_size], fft_size);
        let mut ch1_fft = to_complex_fft(&ch1[..fft_size], fft_size);
        let mut ch2_fft = to_complex_fft(ch2_safe, fft_size);

        // Each channel must be FFT-processed independently before cross-correlation.
        // Previously ch0/ch1/ch2 were left in the time domain — multiplying them
        // against ref_fft (frequency domain) produced garbage phase offsets.
        fft.process(&mut ch0_fft);
        fft.process(&mut ch1_fft);
        fft.process(&mut ch2_fft);

        // Map FFT bins to full-range LUT bins
        for lut_bin in 0..TOTAL_BINS {
            let freq_hz = bin_to_freq(lut_bin, TOTAL_BINS, PDM_NYQUIST);
            let fft_bin = freq_to_fft_bin(freq_hz, AUDIO_SR, fft_size);

            if fft_bin >= fft_size {
                continue;
            }

            // UN-SLOPIFIED CROSS-CORRELATION:
            // Multiply the response by the complex conjugate of the reference to find the true phase offset
            // caused by the acoustic environment and hardware latency.
            let ref_conj = ref_fft[fft_bin].conj();
            let xcorr0 = ch0_fft[fft_bin] * ref_conj;
            let xcorr1 = ch1_fft[fft_bin] * ref_conj;
            let xcorr2 = ch2_fft[fft_bin] * ref_conj;

            // Extract magnitude and phase from the correlated signals
            let mag0 = xcorr0.norm() + 1e-12;
            let mag1 = xcorr1.norm() + 1e-12;
            let mag2 = xcorr2.norm() + 1e-12;

            let phase0 = xcorr0.arg();
            let phase1 = xcorr1.arg();
            let phase2 = xcorr2.arg();

            // Primary phase = C925e (reference channel absolute offset)
            self.phase_lut[lut_bin] = phase0;

            // Phase differences between channels (relative offsets for beamforming/ANC)
            self.phase_delta[lut_bin][0] = wrap_phase(phase1 - phase0); // Pink - C925e
            self.phase_delta[lut_bin][1] = wrap_phase(phase2 - phase0); // Blue - C925e
            self.phase_delta[lut_bin][2] = wrap_phase(phase2 - phase1); // Blue - Pink

            // Mamba-weighted confidence: high anomaly = heterodyne product
            let anomaly = mamba_anomaly_fn(lut_bin);
            let mag_product = (mag0 * mag1 * mag2).sqrt();
            self.confidence[lut_bin] =
                (1.0 - anomaly).clamp(0.0, 1.0) * (mag_product / 1e6).min(1.0);
        }

        // Record timestamp
        // Record timestamp
        let now = chrono::Local::now();
        self.last_calibration_time = Some(now.to_rfc3339());
    }

    /// Look up phase correction for a given frequency
    pub fn phase_for(&self, freq_hz: f32) -> f32 {
        let lut_bin = freq_to_lut_bin(freq_hz, TOTAL_BINS, PDM_NYQUIST);

        if lut_bin >= TOTAL_BINS {
            return 0.0;
        }

        // Interpolate between nearest bins if needed
        let bin_exact = (freq_hz / PDM_NYQUIST) * (TOTAL_BINS as f32);
        let bin_lo = bin_exact.floor() as usize;
        let bin_hi = (bin_lo + 1).min(TOTAL_BINS - 1);
        let t = bin_exact - bin_lo as f32;

        let phase_lo = self.phase_lut[bin_lo];
        let phase_hi = self.phase_lut[bin_hi];

        circular_lerp(phase_lo, phase_hi, t)
    }

    /// Confidence score for phase measurement at this frequency
    pub fn confidence_for(&self, freq_hz: f32) -> f32 {
        let lut_bin = freq_to_lut_bin(freq_hz, TOTAL_BINS, PDM_NYQUIST);
        if lut_bin >= TOTAL_BINS {
            return 0.0;
        }
        self.confidence[lut_bin]
    }

    /// Get last calibration time
    pub fn last_calibration(&self) -> Option<&str> {
        self.last_calibration_time.as_deref()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Generate logarithmic chirp from f_start to f_end
#[allow(dead_code)]
fn generate_log_chirp(f_start: f32, f_end: f32, sr: f32, n_samples: usize) -> Vec<f32> {
    let k_log = (f_end / f_start).ln() / (n_samples as f32 - 1.0);
    (0..n_samples)
        .map(|i| {
            let t = i as f32;
            let _f_inst = f_start * (k_log * t).exp(); // Will be used in analysis
            let phase = TAU * f_start / k_log * (1.0 - (k_log * t).exp()) / sr;
            (phase * 0.5).sin().clamp(-1.0, 1.0)
        })
        .collect()
}

/// Convert time-domain samples to complex FFT input
fn to_complex_fft(samples: &[f32], fft_size: usize) -> Vec<Complex<f32>> {
    let mut result = vec![Complex::default(); fft_size];
    for (i, &s) in samples.iter().enumerate().take(fft_size) {
        result[i] = Complex { re: s, im: 0.0 };
    }
    result
}

/// Map LUT bin (0-8191) to frequency in Hz
fn bin_to_freq(bin: usize, total_bins: usize, nyquist: f32) -> f32 {
    (bin as f32 / total_bins as f32) * nyquist
}

/// Map frequency to LUT bin index
fn freq_to_lut_bin(freq_hz: f32, total_bins: usize, nyquist: f32) -> usize {
    ((freq_hz / nyquist) * total_bins as f32) as usize
}

/// Map frequency to FFT bin index
fn freq_to_fft_bin(freq_hz: f32, sr: f32, fft_size: usize) -> usize {
    ((freq_hz / sr) * fft_size as f32) as usize
}

/// Wrap phase to [0, 2π)
fn wrap_phase(phase: f32) -> f32 {
    let wrapped = phase % TAU;
    if wrapped < 0.0 {
        wrapped + TAU
    } else {
        wrapped
    }
}

/// Circular linear interpolation between two phases
fn circular_lerp(phase_a: f32, phase_b: f32, t: f32) -> f32 {
    let mut diff = phase_b - phase_a;
    if diff > PI {
        diff -= TAU;
    } else if diff < -PI {
        diff += TAU;
    }
    wrap_phase(phase_a + t * diff)
}
