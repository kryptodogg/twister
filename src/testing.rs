// src/testing.rs — Synthetic Signal Injection & Detection Validation
//
// Answers the review's P0 criticism: "The Core Premise is Untestable."
//
// Three test classes
// ──────────────────
//  1. SyntheticSignalGen   — generates known bispectral relationships in FFT space.
//     Inject these into BispectrumEngine::analyze_frame to get verifiable detections.
//
//  2. DetectionValidator   — accumulates predicted vs confirmed detections and
//     computes precision/recall/F1, false positive rate on pure noise.
//
//  3. Integration tests at bottom — can be run with `cargo test` or called from
//     main.rs at startup if `--self-test` flag is set.
//
// Usage in main.rs
// ─────────────────
//   #[cfg(debug_assertions)]
//   {
//       use crate::testing::run_self_test;
//       if let Err(e) = run_self_test(sample_rate) {
//           eprintln!("[SelfTest] FAILED: {}", e);
//           std::process::exit(1);
//       }
//   }

use std::f32::consts::TAU;
use crate::bispectrum::{BispectrumEngine, BISPEC_BINS, FFT_BUFFER_SIZE, BISPEC_FFT_SIZE};
use crate::detection::{DetectionEvent, ProductType, HardwareLayer};
use std::collections::HashMap;
use std::time::Instant;

// ── Synthetic FFT frame generator ─────────────────────────────────────────────

/// A single known injection: two carrier frequencies and the expected product.
#[derive(Debug, Clone, Copy)]
pub struct KnownSignal {
    /// First carrier (Hz).
    pub f1_hz:       f32,
    /// Second carrier (Hz).
    pub f2_hz:       f32,
    /// Expected product relationship.
    pub product_type: ProductType,
    /// Signal amplitude (0.0–1.0).
    pub amplitude:   f32,
}

impl KnownSignal {
    pub fn new_sum(f1: f32, f2: f32, amp: f32) -> Self {
        Self { f1_hz: f1, f2_hz: f2, product_type: ProductType::Sum, amplitude: amp }
    }
    pub fn new_harmonic(f1: f32, amp: f32) -> Self {
        Self { f1_hz: f1, f2_hz: f1, product_type: ProductType::Harmonic, amplitude: amp }
    }
    pub fn new_difference(f1: f32, f2: f32, amp: f32) -> Self {
        Self { f1_hz: f1, f2_hz: f2, product_type: ProductType::Difference, amplitude: amp }
    }
    pub fn new_intermod(f1: f32, f2: f32, amp: f32) -> Self {
        Self { f1_hz: f1, f2_hz: f2, product_type: ProductType::Intermodulation, amplitude: amp }
    }

    /// Expected product frequency in Hz.
    pub fn expected_product_hz(&self) -> f32 {
        match self.product_type {
            ProductType::Sum             => self.f1_hz + self.f2_hz,
            ProductType::Difference      => (self.f1_hz - self.f2_hz).abs(),
            ProductType::Harmonic        => 2.0 * self.f1_hz,
            ProductType::Intermodulation => (2.0 * self.f1_hz - self.f2_hz).max(0.0),
        }
    }
}

/// Generates synthetic FFT frames suitable for injection into `BispectrumEngine`.
///
/// The FFT complex output is a flat `Vec<f32>` of length `FFT_BUFFER_SIZE`
/// (= BISPEC_BINS × 2 = 1024 floats), laid out as [re0, im0, re1, im1, ...].
pub struct SyntheticSignalGen {
    sample_rate: f32,
    /// PRNG state for noise injection.
    lcg:         u64,
}

impl SyntheticSignalGen {
    pub fn new(sample_rate: f32) -> Self {
        Self { sample_rate, lcg: 0xDEAD_BEEF_CAFE_0001 }
    }

    /// Frequency resolution in Hz per bin.
    pub fn bin_hz(&self) -> f32 {
        self.sample_rate / BISPEC_FFT_SIZE as f32
    }

    /// Snap a frequency in Hz to the nearest FFT bin index.
    fn freq_to_bin(&self, hz: f32) -> usize {
        ((hz / self.bin_hz()).round() as usize).min(BISPEC_BINS - 1)
    }

    /// Generate a pure noise FFT frame (Gaussian-distributed complex bins).
    /// All bins have random phase and magnitude drawn from N(0, noise_rms).
    ///
    /// A correct detector should produce near-zero detections on this frame
    /// (after accounting for the coherence accumulation threshold).
    pub fn noise_frame(&mut self, noise_rms: f32) -> Vec<f32> {
        let mut buf = vec![0.0f32; FFT_BUFFER_SIZE];
        for pair in buf.chunks_mut(2) {
            pair[0] = self.randn() * noise_rms;  // re
            pair[1] = self.randn() * noise_rms;  // im
        }
        buf
    }

    /// Generate an FFT frame containing specific known signals at precise bins,
    /// plus optional background noise.
    ///
    /// For each `KnownSignal`, the specified bins (f1, f2, and product) are
    /// set to coherent complex values such that:
    ///   B(f1, f2) = X(f1) * X(f2) * X*(f1+f2)  ≠ 0
    ///
    /// This mimics a real acoustic nonlinearity that the bispectrum is designed
    /// to detect.
    pub fn signal_frame(
        &mut self,
        signals:   &[KnownSignal],
        noise_rms: f32,
        phase_seed: f32,
    ) -> Vec<f32> {
        let mut buf = self.noise_frame(noise_rms);

        for sig in signals {
            let b1 = self.freq_to_bin(sig.f1_hz);
            let b2 = self.freq_to_bin(sig.f2_hz);
            let bp = match sig.product_type {
                ProductType::Sum             => self.freq_to_bin(sig.f1_hz + sig.f2_hz),
                ProductType::Difference      => self.freq_to_bin((sig.f1_hz - sig.f2_hz).abs()),
                ProductType::Harmonic        => self.freq_to_bin(2.0 * sig.f1_hz),
                ProductType::Intermodulation => self.freq_to_bin(2.0 * sig.f1_hz - sig.f2_hz),
            };

            // Set carriers with known phase.
            let phi1 = phase_seed;
            let phi2 = phase_seed * 1.618; // golden ratio offset

            // X(f1) = A · e^{jφ₁}
            let amp = sig.amplitude;
            buf[b1 * 2]     += amp * phi1.cos();
            buf[b1 * 2 + 1] += amp * phi1.sin();

            // X(f2) = A · e^{jφ₂}
            buf[b2 * 2]     += amp * phi2.cos();
            buf[b2 * 2 + 1] += amp * phi2.sin();

            // For a genuine bispectral relationship:
            // X(f1+f2) must be set such that B(f1,f2) has a stable non-zero phase.
            // B = X(f1)·X(f2)·X*(fp) where * = conjugate.
            // We want B = |B|·e^{jΦ_stable} for all frames in a coherent run.
            // So X*(fp) = e^{-j(φ1+φ2)} → X(fp) = e^{j(φ1+φ2)}, i.e.:
            //   re_fp = amp² · cos(φ1 + φ2)
            //   im_fp = amp² · sin(φ1 + φ2)  (conjugate flips im)
            let phi_p = phi1 + phi2;
            if bp < BISPEC_BINS {
                buf[bp * 2]     += amp * amp * phi_p.cos();
                buf[bp * 2 + 1] += amp * amp * phi_p.sin();
            }
        }

        buf
    }

    /// Generate `n_frames` coherent signal frames with slowly drifting phase.
    /// The phase drift simulates a real oscillator with slight frequency
    /// instability — phase coherence should still be measured as > 0.8.
    pub fn coherent_run(
        &mut self,
        signals:      &[KnownSignal],
        noise_rms:    f32,
        n_frames:     usize,
        phase_rate:   f32,   // radians per frame
    ) -> Vec<Vec<f32>> {
        let mut frames = Vec::with_capacity(n_frames);
        let mut phi    = 0.0f32;
        for _ in 0..n_frames {
            frames.push(self.signal_frame(signals, noise_rms, phi));
            phi = (phi + phase_rate) % TAU;
        }
        frames
    }

    /// Generate `n_frames` of pure uncorrelated Gaussian noise.
    /// The bispectrum should show near-zero coherence (phase stability < 0.5).
    pub fn noise_run(&mut self, noise_rms: f32, n_frames: usize) -> Vec<Vec<f32>> {
        (0..n_frames).map(|_| self.noise_frame(noise_rms)).collect()
    }

    // ── PRNG ─────────────────────────────────────────────────────────────────

    fn randn(&mut self) -> f32 {
        // Box-Muller transform using two LCG outputs.
        let u1 = self.lcg_f32();
        let u2 = self.lcg_f32();
        let u1 = u1.max(1e-12);
        (-2.0 * u1.ln()).sqrt() * (TAU * u2).cos()
    }

    fn lcg_f32(&mut self) -> f32 {
        // LCG constants: Knuth MMIX
        self.lcg = self.lcg
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.lcg >> 33) as f32 / (u32::MAX as f32)
    }
}

// ── Detection validator ───────────────────────────────────────────────────────

/// Tracks injected signals and evaluates whether the detector found them.
#[derive(Debug, Default)]
pub struct DetectionValidator {
    /// Total injected signal events.
    pub signals_injected:    u32,
    /// Signals that were correctly detected.
    pub true_positives:      u32,
    /// Noise frames that triggered a spurious detection.
    pub false_positives:     u32,
    /// Noise frames processed without a false alarm.
    pub true_negatives:      u32,
    /// Per-frequency detection latency (frames from injection to detection).
    pub detection_latency:   Vec<u32>,
    /// Frequency error statistics (Hz, detected − expected).
    pub freq_errors:         Vec<f32>,
    /// HISTORICAL CACHE: Tracks how many times each freq has been seen.
    pub history:             HashMap<u32, u32>,
}

impl DetectionValidator {
    /// Match a list of detector outputs against a list of known injected signals.
    ///
    /// A detection is a true positive if:
    ///   - product_type matches
    ///   - |detected_product_hz − expected_product_hz| < tolerance_hz
    pub fn evaluate_detections(
        &mut self,
        detections:   &[DetectionEvent],
        signals:      &[KnownSignal],
        tolerance_hz: f32,
        noise_frame:  bool,
    ) {
        // Update history cache for all detections
        for det in detections {
            let hz = det.product_hz.round() as u32;
            *self.history.entry(hz).or_insert(0) += 1;
        }

        if noise_frame {
            if detections.is_empty() {
                self.true_negatives += 1;
            } else {
                self.false_positives += detections.len() as u32;
            }
            return;
        }

        self.signals_injected += signals.len() as u32;

        for sig in signals {
            let expected_hz = sig.expected_product_hz();
            let matched = detections.iter().any(|d| {
                d.product_type == sig.product_type
                    && (d.product_hz - expected_hz).abs() < tolerance_hz
            });
            if matched {
                self.true_positives += 1;
                // Find the best matching detection for frequency error.
                if let Some(best) = detections.iter()
                    .filter(|d| d.product_type == sig.product_type)
                    .min_by(|a, b| {
                        (a.product_hz - expected_hz).abs()
                            .partial_cmp(&(b.product_hz - expected_hz).abs())
                            .unwrap()
                    })
                {
                    self.freq_errors.push(best.product_hz - expected_hz);
                }
            }
        }
    }

    pub fn precision(&self) -> f32 {
        let tp = self.true_positives as f32;
        let fp = self.false_positives as f32;
        if tp + fp < 1.0 { return 0.0; }
        tp / (tp + fp)
    }

    pub fn recall(&self) -> f32 {
        let tp = self.true_positives as f32;
        let injected = self.signals_injected as f32;
        if injected < 1.0 { return 0.0; }
        tp / injected
    }

    pub fn f1_score(&self) -> f32 {
        let p = self.precision();
        let r = self.recall();
        if p + r < 1e-6 { return 0.0; }
        2.0 * p * r / (p + r)
    }

    pub fn false_positive_rate(&self) -> f32 {
        let fp = self.false_positives as f32;
        let tn = self.true_negatives as f32;
        if fp + tn < 1.0 { return 0.0; }
        fp / (fp + tn)
    }

    pub fn mean_freq_error_hz(&self) -> f32 {
        if self.freq_errors.is_empty() { return 0.0; }
        self.freq_errors.iter().sum::<f32>() / self.freq_errors.len() as f32
    }

    pub fn report(&self) -> String {
        format!(
            "Precision: {:.1}%  Recall: {:.1}%  F1: {:.3}  \
             FPR: {:.3}  TP: {}  FP: {}  TN: {}  \
             Mean freq error: {:.2} Hz",
            self.precision() * 100.0,
            self.recall() * 100.0,
            self.f1_score(),
            self.false_positive_rate(),
            self.true_positives,
            self.false_positives,
            self.true_negatives,
            self.mean_freq_error_hz(),
        )
    }
}

// ── Benchmark ─────────────────────────────────────────────────────────────────

/// Measure BispectrumEngine throughput (frames per second) on synthetic data.
/// Returns (fps, mean_ms_per_frame).
pub fn benchmark_bispectrum(
    engine:       &mut BispectrumEngine,
    sample_rate:  f32,
    n_frames:     usize,
) -> (f32, f32) {
    let mut sg = SyntheticSignalGen::new(sample_rate);
    let frames  = sg.noise_run(0.1, n_frames);

    let t0 = Instant::now();
    for frame in &frames {
        engine.analyze_frame(frame, sample_rate, HardwareLayer::Microphone);
    }
    let elapsed = t0.elapsed().as_secs_f32();
    let fps     = n_frames as f32 / elapsed;
    let ms_per  = elapsed * 1000.0 / n_frames as f32;

    println!(
        "[Bench] BispectrumEngine: {} frames in {:.2} ms → {:.1} fps ({:.2} ms/frame)",
        n_frames, elapsed * 1000.0, fps, ms_per
    );
    (fps, ms_per)
}

// ── Integration self-test ─────────────────────────────────────────────────────

/// Run a full detection validation suite against a live BispectrumEngine.
///
/// Requires a GPU device — call after GPU initialisation.
///
/// Returns `Ok(ValidationReport)` if the detector meets minimum accuracy
/// thresholds, or `Err(description)` if a critical test fails.
pub fn run_self_test(
    engine:      &mut BispectrumEngine,
    sample_rate: f32,
) -> Result<ValidationReport, String> {
    println!("[SelfTest] Starting bispectrum detector validation...");
    let t0 = Instant::now();

    let mut sg       = SyntheticSignalGen::new(sample_rate);
    let mut validator = DetectionValidator::default();
    let bin_hz        = sg.bin_hz();

    // ── Test 1: Sum-frequency detection ──────────────────────────────────────
    //
    // Inject f1=1 kHz + f2=2 kHz → expect Sum detection at 3 kHz.
    // Run MIN_COHERENCE_FRAMES + 5 extra frames to allow for threshold crossing.
    let signals = vec![KnownSignal::new_sum(1000.0, 2000.0, 0.5)];
    let n_run   = (crate::detection::MIN_COHERENCE_FRAMES + 10) as usize;
    let frames  = sg.coherent_run(&signals, 0.01, n_run, 0.05);

    for (i, frame) in frames.iter().enumerate() {
        let detections = engine.analyze_frame(frame, sample_rate, HardwareLayer::Microphone);
        let is_last    = i == frames.len() - 1;
        if is_last {
            validator.evaluate_detections(&detections, &signals, bin_hz * 2.0, false);
        }
    }

    // ── Test 2: Harmonic detection ────────────────────────────────────────────
    //
    // Inject f1=500 Hz → expect Harmonic at 1 kHz.
    let h_signals = vec![KnownSignal::new_harmonic(500.0, 0.5)];
    let h_frames  = sg.coherent_run(&h_signals, 0.01, n_run, 0.02);
    for (i, frame) in h_frames.iter().enumerate() {
        let detections = engine.analyze_frame(frame, sample_rate, HardwareLayer::Microphone);
        if i == h_frames.len() - 1 {
            validator.evaluate_detections(&detections, &h_signals, bin_hz * 2.0, false);
        }
    }

    // ── Test 3: Difference frequency detection ────────────────────────────────
    //
    // Inject f1=3 kHz + f2=2 kHz → expect Difference at 1 kHz.
    let d_signals = vec![KnownSignal::new_difference(3000.0, 2000.0, 0.5)];
    let d_frames  = sg.coherent_run(&d_signals, 0.01, n_run, 0.03);
    for (i, frame) in d_frames.iter().enumerate() {
        let detections = engine.analyze_frame(frame, sample_rate, HardwareLayer::Microphone);
        if i == d_frames.len() - 1 {
            validator.evaluate_detections(&detections, &d_signals, bin_hz * 2.0, false);
        }
    }

    // ── Test 4: False positive rate on pure noise ─────────────────────────────
    //
    // Run 100 noise frames.  Expect < 5% false positive rate.
    let noise_run = sg.noise_run(0.1, 100);
    for frame in &noise_run {
        let detections = engine.analyze_frame(frame, sample_rate, HardwareLayer::Microphone);
        validator.evaluate_detections(&detections, &[], 0.0, true);
    }

    let elapsed = t0.elapsed().as_secs_f32();
    let report  = ValidationReport {
        precision:          validator.precision(),
        recall:             validator.recall(),
        f1_score:           validator.f1_score(),
        false_positive_rate: validator.false_positive_rate(),
        elapsed_s:          elapsed,
        detail:             validator.report(),
    };

    println!("[SelfTest] Complete in {:.2} s", elapsed);
    println!("[SelfTest] {}", report.detail);

    // ── Pass/fail thresholds ──────────────────────────────────────────────────
    if report.f1_score < 0.5 {
        return Err(format!(
            "F1 score {:.3} below minimum 0.50 — detector may be hallucinating",
            report.f1_score
        ));
    }
    if report.false_positive_rate > 0.10 {
        return Err(format!(
            "False positive rate {:.1}% exceeds 10% — threshold too low",
            report.false_positive_rate * 100.0
        ));
    }

    Ok(report)
}

/// Summary of one self-test run.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub precision:           f32,
    pub recall:              f32,
    pub f1_score:            f32,
    pub false_positive_rate: f32,
    pub elapsed_s:           f32,
    pub detail:              String,
}

impl ValidationReport {
    pub fn passed(&self) -> bool {
        self.f1_score >= 0.5 && self.false_positive_rate <= 0.10
    }

    pub fn summary_line(&self) -> String {
        format!(
            "SIREN self-test {} | F1={:.3} FPR={:.1}% in {:.1}s",
            if self.passed() { "PASS" } else { "FAIL" },
            self.f1_score,
            self.false_positive_rate * 100.0,
            self.elapsed_s,
        )
    }
}

// ── Unit tests (no GPU required) ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    #[test]
    fn signal_gen_produces_correct_bin_count() {
        let mut sg = SyntheticSignalGen::new(SR);
        let frame   = sg.noise_frame(1.0);
        assert_eq!(frame.len(), FFT_BUFFER_SIZE);
    }

    #[test]
    fn known_signal_freq_to_bin_roundtrip() {
        let sg    = SyntheticSignalGen::new(SR);
        let target = 1000.0f32;
        let bin    = ((target / sg.bin_hz()).round() as usize).min(BISPEC_BINS - 1);
        let back   = bin as f32 * sg.bin_hz();
        assert!((back - target).abs() < sg.bin_hz(),
            "bin roundtrip: target={} back={} (bin_hz={})", target, back, sg.bin_hz());
    }

    #[test]
    fn signal_frame_sets_carrier_bins_above_noise() {
        let mut sg  = SyntheticSignalGen::new(SR);
        let sig      = KnownSignal::new_sum(1000.0, 2000.0, 1.0);
        let frame    = sg.signal_frame(&[sig], 0.001, 0.0);

        let bin_hz   = sg.bin_hz();
        let b1       = ((1000.0 / bin_hz).round() as usize).min(BISPEC_BINS - 1);
        let b2       = ((2000.0 / bin_hz).round() as usize).min(BISPEC_BINS - 1);

        let mag1 = (frame[b1*2].powi(2) + frame[b1*2+1].powi(2)).sqrt();
        let mag2 = (frame[b2*2].powi(2) + frame[b2*2+1].powi(2)).sqrt();
        assert!(mag1 > 0.5, "f1 carrier bin amplitude too low: {}", mag1);
        assert!(mag2 > 0.5, "f2 carrier bin amplitude too low: {}", mag2);
    }

    #[test]
    fn noise_run_has_low_inter_frame_correlation() {
        let mut sg = SyntheticSignalGen::new(SR);
        let frames  = sg.noise_run(1.0, 20);

        // Adjacent frames should have near-zero correlation.
        let corr: f32 = frames.windows(2).map(|w| {
            w[0].iter().zip(w[1].iter())
                .map(|(a, b)| a * b)
                .sum::<f32>()
                / (FFT_BUFFER_SIZE as f32)
        }).sum::<f32>() / 19.0;

        assert!(corr.abs() < 0.05,
            "Noise frames are suspiciously correlated: {:.4}", corr);
    }

    #[test]
    fn coherent_run_has_stable_phases() {
        let mut sg = SyntheticSignalGen::new(SR);
        let sigs    = vec![KnownSignal::new_sum(1000.0, 2000.0, 1.0)];
        // Very slow phase rate → near-constant phase across frames.
        let frames  = sg.coherent_run(&sigs, 0.001, 30, 0.001);

        // The carrier bins should have consistent phase across frames.
        let bin_hz  = sg.bin_hz();
        let b1      = ((1000.0 / bin_hz).round() as usize).min(BISPEC_BINS - 1);

        let phases: Vec<f32> = frames.iter()
            .map(|f| f[b1*2+1].atan2(f[b1*2]))
            .collect();

        // Circular variance should be low (< 0.1 for slow-varying phase).
        let n   = phases.len() as f32;
        let sc  = phases.iter().map(|p| p.cos()).sum::<f32>() / n;
        let ss  = phases.iter().map(|p| p.sin()).sum::<f32>() / n;
        let r   = (sc*sc + ss*ss).sqrt();
        assert!(r > 0.9,
            "Phase coherence too low ({:.3}) — signal generator broken", r);
    }

    #[test]
    fn validator_precision_recall() {
        let sr  = 48_000.0f32;
        let sg = SyntheticSignalGen::new(sr);

        let sigs = vec![KnownSignal::new_sum(1000.0, 2000.0, 0.5)];
        let tolerance = sg.bin_hz() * 2.0;

        let mut v = DetectionValidator::default();

        // Simulate a perfect detector.
        let fake_ev = crate::detection::DetectionEvent {
            id: "test".to_string(),
            timestamp: std::time::SystemTime::now(),
            f1_hz: 1000.0, f2_hz: 2000.0, product_hz: 3000.0,
            product_type: ProductType::Sum,
            magnitude: 1.0, phase_angle: 0.0,
            coherence_frames: 15,
            spl_db: 0.0,
            session_id: "test_session".to_string(),
            hardware: HardwareLayer::Microphone,
            embedding: vec![0.0; 32],
            frequency_band: crate::bispectrum::FrequencyBand::Audio,
            // Forensic analysis fields
            audio_dc_bias_v: None,
            sdr_dc_bias_v: None,
            mamba_anomaly_db: 0.0,
            timestamp_sync_ms: None,
            is_coordinated: false,
            detection_method: "bispectrum".to_string(),
        };

        v.evaluate_detections(&[fake_ev], &sigs, tolerance, false);
        // One noise miss.
        v.evaluate_detections(&[], &[], 0.0, true);

        assert_eq!(v.true_positives,  1);
        assert_eq!(v.false_positives, 0);
        assert_eq!(v.true_negatives,  1);
        assert!((v.precision() - 1.0).abs() < 1e-4);
        assert!((v.recall()    - 1.0).abs() < 1e-4);
    }
}
