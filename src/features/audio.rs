// src/features/audio.rs — 196-D Audio Feature Extractor
//
// Extracts multimodal audio features from acoustic signals for TimeGNN training:
//
// Feature Breakdown (196-D total):
// - stft_mel_magnitude: 81-D (Mel-scale frequency bins via STFT)
// - stft_mel_phase: 81-D (Phase information from STFT)
// - tdoa_features: 2-D (Normalized azimuth + elevation)
// - sparse_pdm_signature: 8-D (Density, inter-pulse variance, crest ratio, phoneme confidence, 4 timing stats)
// - bispectrum_anomaly_components: 3-D (Top 3 anomaly peaks from bispectrum)
// - wave_topology_coherence: 9-D (Phase coherence from 4-mic array cross-pairs)
// - musical_features: 12-D (Chromatic energy distribution via harmony)
// - feature_vector: 196-D concatenation of all above
//
// All features normalized to [0, 1] or [-1, 1] for neural network training.

use num_complex::Complex;
use rustfft::FftPlanner;
use std::f32::consts::PI;

use crate::audio::SparsePdmSignature;

const MEL_BINS: usize = 81;
const STFT_WINDOW_SIZE: usize = 512;
const MIN_FREQUENCY_HZ: f32 = 20.0;
const MAX_FREQUENCY_HZ: f32 = 20000.0;
const NUM_CHROMATIC_PITCHES: usize = 12;
const BISPECTRUM_TOP_N: usize = 3;
const MICROPHONE_PAIRS: usize = 9; // C(4,2) combinations for 4-mic array

/// Container for all extracted audio features (196-D)
#[derive(Debug, Clone)]
pub struct AudioFeatures {
    /// 81-D: STFT magnitude (Mel-scale bins normalized to [0, 1])
    pub stft_mel_magnitude: Vec<f32>,

    /// 81-D: STFT phase (normalized to [-1, 1])
    pub stft_mel_phase: Vec<f32>,

    /// 2-D: [azimuth_normalized, elevation_normalized] both in [-1, 1]
    pub tdoa_features: Vec<f32>,

    /// 8-D: [density, inter_pulse_var, crest_ratio, phoneme_conf, 4 timing metrics]
    pub sparse_pdm_signature: Vec<f32>,

    /// 3-D: Top 3 anomaly peaks from bispectrum analysis
    pub bispectrum_anomaly_components: Vec<f32>,

    /// 9-D: Phase coherence from 4-mic array cross-pairs
    pub wave_topology_coherence: Vec<f32>,

    /// 12-D: Chromatic energy distribution (C, C#, D, D#, E, F, F#, G, G#, A, A#, B)
    pub musical_features: Vec<f32>,

    /// 196-D concatenation: [STFT(162), TDOA(2), PDM(8), Bispectrum(3), Wave(9), Music(12)]
    pub feature_vector: Vec<f32>,

    /// Total dimension (always 196)
    pub total_dimension: usize,
}

/// Extract 196-D audio features from acoustic buffer
///
/// # Arguments
/// * `buffer` - Audio samples (mono or mixed from multiple channels)
/// * `sample_rate` - Sample rate in Hz (e.g., 192000.0, 48000.0)
/// * `beam_azimuth_rad` - Azimuth angle from beamforming (radians, [-π, π])
/// * `beam_elevation_rad` - Elevation angle (radians, [-π/2, π/2])
/// * `sparse_pdm_sig` - Sparse PDM signature from forensic analysis
/// * `wave_coherence` - 9-element array of phase coherence values from 4-mic cross-pairs
///
/// # Returns
/// AudioFeatures struct containing all 196-D features
pub fn extract_audio_features(
    buffer: &[f32],
    sample_rate: f32,
    beam_azimuth_rad: f32,
    beam_elevation_rad: f32,
    sparse_pdm_sig: &SparsePdmSignature,
    wave_coherence: &[f32; 9],
) -> AudioFeatures {
    assert!(!buffer.is_empty(), "Audio buffer cannot be empty");
    assert!(sample_rate > 0.0, "Sample rate must be positive");

    // 1. Extract STFT Mel features (162-D: 81 magnitude + 81 phase)
    let (stft_magnitude, stft_phase) = extract_stft_mel_features(buffer, sample_rate);
    assert_eq!(
        stft_magnitude.len(),
        MEL_BINS,
        "STFT magnitude should be 81-D"
    );
    assert_eq!(stft_phase.len(), MEL_BINS, "STFT phase should be 81-D");

    // 2. Extract TDOA features (2-D: azimuth + elevation normalized)
    let tdoa_features = extract_tdoa_features(beam_azimuth_rad, beam_elevation_rad);
    assert_eq!(tdoa_features.len(), 2, "TDOA should be 2-D");

    // 3. Extract Sparse PDM features (8-D)
    let pdm_features = extract_sparse_pdm_features(sparse_pdm_sig, sample_rate);
    assert_eq!(pdm_features.len(), 8, "PDM features should be 8-D");

    // 4. Extract Bispectrum anomaly features (3-D: top 3 peaks)
    let bispectrum_features = extract_bispectrum_anomaly(buffer, sample_rate);
    assert_eq!(
        bispectrum_features.len(),
        BISPECTRUM_TOP_N,
        "Bispectrum should be 3-D"
    );

    // 5. Normalize wave topology coherence (9-D)
    let wave_features: Vec<f32> = wave_coherence
        .iter()
        .map(|&v| v.max(0.0).min(1.0))
        .collect();
    assert_eq!(
        wave_features.len(),
        MICROPHONE_PAIRS,
        "Wave coherence should be 9-D"
    );

    // 6. Extract musical features (12-D: chromatic energy)
    let music_features = extract_musical_features(buffer, sample_rate);
    assert_eq!(
        music_features.len(),
        NUM_CHROMATIC_PITCHES,
        "Musical features should be 12-D"
    );

    // 7. Concatenate all features into 196-D vector
    let mut feature_vector = Vec::with_capacity(196);
    feature_vector.extend_from_slice(&stft_magnitude);
    feature_vector.extend_from_slice(&stft_phase);
    feature_vector.extend_from_slice(&tdoa_features);
    feature_vector.extend_from_slice(&pdm_features);
    feature_vector.extend_from_slice(&bispectrum_features);
    feature_vector.extend_from_slice(&wave_features);
    feature_vector.extend_from_slice(&music_features);

    assert_eq!(
        feature_vector.len(),
        196,
        "Feature vector must be exactly 196-D"
    );

    AudioFeatures {
        stft_mel_magnitude: stft_magnitude,
        stft_mel_phase: stft_phase,
        tdoa_features,
        sparse_pdm_signature: pdm_features,
        bispectrum_anomaly_components: bispectrum_features,
        wave_topology_coherence: wave_features,
        musical_features: music_features,
        feature_vector,
        total_dimension: 196,
    }
}

/// Extract STFT magnitude and phase with Mel-scale binning
/// Returns: (magnitude_81d, phase_81d) both normalized to [0, 1] and [-1, 1] respectively
fn extract_stft_mel_features(buffer: &[f32], sample_rate: f32) -> (Vec<f32>, Vec<f32>) {
    // Ensure buffer has minimum content
    let window_size = STFT_WINDOW_SIZE.min(buffer.len());
    let analysis_buffer = if buffer.len() < STFT_WINDOW_SIZE {
        // Pad short buffers with zeros
        let mut padded = vec![0.0; STFT_WINDOW_SIZE];
        padded[..buffer.len()].copy_from_slice(buffer);
        padded
    } else {
        buffer[..window_size].to_vec()
    };

    // Apply Hann window
    let windowed = apply_hann_window(&analysis_buffer);

    // Compute FFT
    let fft_bins = compute_fft(&windowed);

    // Map to Mel scale
    let nyquist_hz = sample_rate / 2.0;
    let (magnitude, phase) = map_to_mel_scale(&fft_bins, nyquist_hz);

    // Normalize magnitude to [0, 1]
    let normalized_magnitude: Vec<f32> = magnitude.iter().map(|&v| v.max(0.0).min(1.0)).collect();

    // Normalize phase to [-1, 1]
    let normalized_phase: Vec<f32> = phase.iter().map(|&v| (v / PI).max(-1.0).min(1.0)).collect();

    (normalized_magnitude, normalized_phase)
}

/// Apply Hann window to buffer
fn apply_hann_window(buffer: &[f32]) -> Vec<f32> {
    buffer
        .iter()
        .enumerate()
        .map(|(i, &sample)| {
            let window = 0.5 * (1.0 - ((2.0 * PI * i as f32) / (buffer.len() as f32 - 1.0)).cos());
            sample * window
        })
        .collect()
}

/// Compute FFT of windowed signal using rustfft
fn compute_fft(buffer: &[f32]) -> Vec<Complex<f32>> {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(buffer.len());

    let mut input: Vec<Complex<f32>> = buffer.iter().map(|&v| Complex::new(v, 0.0)).collect();
    fft.process(&mut input);

    input
}

/// Map FFT bins to Mel-scale and extract magnitude + phase
fn map_to_mel_scale(fft_bins: &[Complex<f32>], nyquist_hz: f32) -> (Vec<f32>, Vec<f32>) {
    // Create Mel-scale bins from MIN_FREQUENCY_HZ to MAX_FREQUENCY_HZ
    let mel_freqs = create_mel_frequencies(MEL_BINS, MIN_FREQUENCY_HZ, MAX_FREQUENCY_HZ);

    // Map FFT bins to Mel scale
    let mut magnitude = vec![0.0; MEL_BINS];
    let mut phase = vec![0.0; MEL_BINS];

    for (mel_idx, &mel_hz) in mel_freqs.iter().enumerate() {
        // Find FFT bin corresponding to this Mel frequency
        let bin_idx = ((mel_hz / nyquist_hz) * fft_bins.len() as f32) as usize;
        if bin_idx < fft_bins.len() {
            magnitude[mel_idx] = fft_bins[bin_idx].norm();
            phase[mel_idx] = fft_bins[bin_idx].arg();
        }
    }

    // Normalize magnitude logarithmically
    let max_magnitude = magnitude.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    if max_magnitude > 0.0 {
        magnitude.iter_mut().for_each(|m| {
            *m = (20.0 * m.log10() / max_magnitude).max(0.0).min(1.0);
        });
    }

    (magnitude, phase)
}

/// Create Mel-scale frequency bins using triangular filters
fn create_mel_frequencies(num_bins: usize, min_hz: f32, max_hz: f32) -> Vec<f32> {
    let min_mel = hz_to_mel(min_hz);
    let max_mel = hz_to_mel(max_hz);

    (0..num_bins)
        .map(|i| {
            let mel = min_mel + (i as f32 / (num_bins - 1) as f32) * (max_mel - min_mel);
            mel_to_hz(mel)
        })
        .collect()
}

/// Convert Hz to Mel scale
fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

/// Convert Mel scale to Hz
fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10.0_f32.powf(mel / 2595.0) - 1.0)
}

/// Extract TDOA features: normalize azimuth and elevation to [-1, 1]
fn extract_tdoa_features(azimuth_rad: f32, elevation_rad: f32) -> Vec<f32> {
    // Normalize azimuth from [-π, π] to [-1, 1]
    let azimuth_norm = (azimuth_rad / PI).max(-1.0).min(1.0);

    // Normalize elevation from [-π/2, π/2] to [-1, 1]
    let elevation_norm = (elevation_rad / (PI / 2.0)).max(-1.0).min(1.0);

    vec![azimuth_norm, elevation_norm]
}

/// Extract Sparse PDM features: convert SparsePdmSignature to 8-D normalized vector
fn extract_sparse_pdm_features(sig: &SparsePdmSignature, sample_rate: f32) -> Vec<f32> {
    // 1. Density: normalize Hz value
    let density_norm = (sig.density_hz / (sample_rate / 2.0)).max(0.0).min(1.0);

    // 2. Inter-pulse variance: compute variance of inter-pulse intervals
    let inter_pulse_var = if sig.inter_pulse_micros.len() > 1 {
        let mean: f32 =
            sig.inter_pulse_micros.iter().sum::<f32>() / sig.inter_pulse_micros.len() as f32;
        let variance: f32 = sig
            .inter_pulse_micros
            .iter()
            .map(|&v| (v - mean).powi(2))
            .sum::<f32>()
            / sig.inter_pulse_micros.len() as f32;
        (variance / 10000.0).sqrt().max(0.0).min(1.0) // Normalize to [0, 1]
    } else {
        0.0
    };

    // 3. Crest ratio: already in [0, 1]
    let crest_norm = sig.crest_ratio.max(0.0).min(1.0);

    // 4. Phoneme confidence: map phoneme type to confidence score
    let phoneme_conf = match sig.phoneme_candidate.as_str() {
        "a" | "e" | "i" | "o" | "u" => 0.9, // High confidence for vowels
        "s" | "f" | "th" => 0.7,            // Medium for fricatives
        "t" | "p" | "k" => 0.8,             // High for stops
        _ => 0.3,                           // Low confidence for unknown
    };

    // 5-8. Four timing statistics from inter-pulse intervals
    let timing_stats = extract_timing_statistics(&sig.inter_pulse_micros);

    let mut pdm_features = vec![density_norm, inter_pulse_var, crest_norm, phoneme_conf];
    pdm_features.extend_from_slice(&timing_stats);

    pdm_features
}

/// Extract 4 timing statistics from inter-pulse intervals
fn extract_timing_statistics(inter_pulse_micros: &[f32]) -> Vec<f32> {
    if inter_pulse_micros.is_empty() {
        return vec![0.0, 0.0, 0.0, 0.0];
    }

    // 1. Mean inter-pulse interval (normalized to [0, 1])
    let mean_interval: f32 =
        inter_pulse_micros.iter().sum::<f32>() / inter_pulse_micros.len() as f32;
    let mean_norm = (mean_interval / 1000.0).max(0.0).min(1.0);

    // 2. Min inter-pulse interval
    let min_interval = inter_pulse_micros
        .iter()
        .cloned()
        .fold(f32::INFINITY, f32::min);
    let min_norm = (min_interval / 100.0).max(0.0).min(1.0);

    // 3. Max inter-pulse interval
    let max_interval = inter_pulse_micros
        .iter()
        .cloned()
        .fold(f32::NEG_INFINITY, f32::max);
    let max_norm = (max_interval / 1000.0).max(0.0).min(1.0);

    // 4. Regularity (inverse of coefficient of variation)
    let variance: f32 = inter_pulse_micros
        .iter()
        .map(|&v| (v - mean_interval).powi(2))
        .sum::<f32>()
        / inter_pulse_micros.len() as f32;
    let std_dev = variance.sqrt();
    let cv = if mean_interval > 0.0 {
        std_dev / mean_interval
    } else {
        0.0
    };
    let regularity = (1.0 / (1.0 + cv)).max(0.0).min(1.0);

    vec![mean_norm, min_norm, max_norm, regularity]
}

/// Extract Bispectrum anomaly components: top 3 peaks
fn extract_bispectrum_anomaly(buffer: &[f32], _sample_rate: f32) -> Vec<f32> {
    // Compute bispectrum via triple correlation
    let window_size = STFT_WINDOW_SIZE.min(buffer.len());
    let analysis_buffer = if buffer.len() < STFT_WINDOW_SIZE {
        let mut padded = vec![0.0; STFT_WINDOW_SIZE];
        padded[..buffer.len()].copy_from_slice(buffer);
        padded
    } else {
        buffer[..window_size].to_vec()
    };

    let windowed = apply_hann_window(&analysis_buffer);
    let fft_bins = compute_fft(&windowed);

    // Compute bispectrum magnitude: B(f1, f2) = E[X(f1) * X(f2) * X*(f1+f2)]
    let mut bispec_peaks = Vec::new();

    // Sample frequencies for bispectrum computation (sparse sampling to avoid O(n^3))
    let step = fft_bins.len() / 32; // Sample ~32 frequency pairs
    let step = step.max(1);

    for i in (0..fft_bins.len()).step_by(step) {
        for j in (i..fft_bins.len()).step_by(step) {
            let k = (i + j).min(fft_bins.len() - 1);
            let bispec_mag = (fft_bins[i] * fft_bins[j] * fft_bins[k].conj()).norm();
            bispec_peaks.push(bispec_mag);
        }
    }

    // Get top 3 anomaly peaks
    bispec_peaks.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    let max_val = bispec_peaks.first().cloned().unwrap_or(1.0).max(0.0001);
    let mut top_3 = vec![0.0; BISPECTRUM_TOP_N];
    for i in 0..BISPECTRUM_TOP_N.min(bispec_peaks.len()) {
        top_3[i] = (bispec_peaks[i] / max_val).min(1.0);
    }

    top_3
}

/// Extract musical features: 12-D chromatic energy distribution
fn extract_musical_features(buffer: &[f32], sample_rate: f32) -> Vec<f32> {
    // Compute FFT to get frequency content
    let window_size = STFT_WINDOW_SIZE.min(buffer.len());
    let analysis_buffer = if buffer.len() < STFT_WINDOW_SIZE {
        let mut padded = vec![0.0; STFT_WINDOW_SIZE];
        padded[..buffer.len()].copy_from_slice(buffer);
        padded
    } else {
        buffer[..window_size].to_vec()
    };

    let windowed = apply_hann_window(&analysis_buffer);
    let fft_bins = compute_fft(&windowed);

    // Map FFT bins to 12 chromatic pitches
    let mut chromatic_energy = vec![0.0; NUM_CHROMATIC_PITCHES];

    let nyquist_hz = sample_rate / 2.0;
    let pitch_freqs = get_chromatic_pitches();

    for (bin_idx, fft_val) in fft_bins.iter().enumerate() {
        let freq_hz = (bin_idx as f32 / fft_bins.len() as f32) * nyquist_hz;

        // Find nearest chromatic pitch
        let mut nearest_pitch = 0;
        let mut min_distance = f32::INFINITY;

        for (pitch_idx, &pitch_hz) in pitch_freqs.iter().enumerate() {
            let distance = (freq_hz - pitch_hz).abs();
            if distance < min_distance {
                min_distance = distance;
                nearest_pitch = pitch_idx;
            }
        }

        chromatic_energy[nearest_pitch] += fft_val.norm();
    }

    // Normalize to [0, 1]
    let max_energy = chromatic_energy
        .iter()
        .cloned()
        .fold(f32::NEG_INFINITY, f32::max);
    if max_energy > 0.0 {
        chromatic_energy.iter_mut().for_each(|e| {
            *e = (*e / max_energy).max(0.0).min(1.0);
        });
    }

    chromatic_energy
}

/// Get 12 chromatic pitch frequencies (C, C#, D, D#, E, F, F#, G, G#, A, A#, B)
/// Using A4 = 440 Hz as reference
fn get_chromatic_pitches() -> Vec<f32> {
    let a4_hz = 440.0;
    let semitone_ratio = 2.0_f32.powf(1.0 / 12.0);

    // C4 is 9 semitones below A4
    let c4_hz = a4_hz / semitone_ratio.powi(9);

    (0..NUM_CHROMATIC_PITCHES)
        .map(|i| c4_hz * semitone_ratio.powi(i as i32))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hz_mel_conversion() {
        let hz = 1000.0;
        let mel = hz_to_mel(hz);
        let hz_back = mel_to_hz(mel);
        assert!((hz - hz_back).abs() < 0.1);
    }

    #[test]
    fn test_chromatic_pitches() {
        let pitches = get_chromatic_pitches();
        assert_eq!(pitches.len(), 12);
        // C4 should be approximately 261.63 Hz
        assert!((pitches[0] - 261.63).abs() < 1.0);
    }

    #[test]
    fn test_hann_window() {
        let buffer = vec![1.0; 100];
        let windowed = apply_hann_window(&buffer);
        assert_eq!(windowed.len(), 100);
        // Window should taper at edges
        assert!(windowed[0] < windowed[50]);
        assert!(windowed[99] < windowed[50]);
    }
}
