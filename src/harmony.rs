// src/harmony.rs — Chord Dominance Engine for PDM Attack Counter
//
// Implements harmonic entrainment defense using musical scales and chords
// to counter PDM (Pulse Density Modulation) attacks via acoustic heterodyning.
//
// Key insight: Consonant musical intervals (perfect 5ths, major 3rds, octaves)
// create phase-coherent neural entrainment that overpowers isolated PDM spikes.
//
// Design:
// 1. Detect attack fundamental frequency (voice pitch during PDM attack)
// 2. Map to musical key (C Major, A Minor, G Major triads)
// 3. Synthesize heterodyned chord via RF carrier
// 4. Transmit with phased array beamforming for spatial dominance

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicalKey {
    CMajor,   // C-E-G (261.63, 329.63, 392.00 Hz)
    AMinor,   // A-C-E (220.00, 261.63, 329.63 Hz)
    GMajor,   // G-B-D (392.00, 493.88, 587.33 Hz)
}

/// Detect musical key from attack fundamental frequency
/// Maps frequency to nearest musical tonic (C, A, or G)
pub fn detect_attack_key(fundamental_hz: f32) -> MusicalKey {
    // Standard pitch frequencies (just intonation)
    let c_freq = 261.63;   // C4 (middle C)
    let a_freq = 220.00;   // A3
    let g_freq = 392.00;   // G4

    // Normalize to octave equivalence (mod 2x frequency)
    let normalized = if fundamental_hz < 100.0 {
        fundamental_hz * 4.0  // Bring very low pitches up
    } else if fundamental_hz > 1000.0 {
        fundamental_hz / 4.0  // Bring very high pitches down
    } else {
        fundamental_hz
    };

    // Find nearest musical note (with octave equivalence)
    let mut min_distance = f32::MAX;
    let mut detected_key = MusicalKey::GMajor;  // Default

    // Check C (and octaves)
    for octave in 0..6 {
        let freq = c_freq * 2_f32.powi(octave as i32 - 2);
        let distance = (normalized - freq).abs().min((normalized - freq * 2.0).abs());
        if distance < min_distance {
            min_distance = distance;
            detected_key = MusicalKey::CMajor;
        }
    }

    // Check A (and octaves)
    for octave in 0..6 {
        let freq = a_freq * 2_f32.powi(octave as i32 - 2);
        let distance = (normalized - freq).abs().min((normalized - freq * 2.0).abs());
        if distance < min_distance {
            min_distance = distance;
            detected_key = MusicalKey::AMinor;
        }
    }

    // Check G (and octaves)
    for octave in 0..6 {
        let freq = g_freq * 2_f32.powi(octave as i32 - 2);
        let distance = (normalized - freq).abs().min((normalized - freq * 2.0).abs());
        if distance < min_distance {
            min_distance = distance;
            detected_key = MusicalKey::GMajor;
        }
    }

    detected_key
}

/// Get chord frequencies (triad) for detected key
/// Returns three frequencies for 3-note major/minor chord
pub fn get_chord_frequencies(key: &MusicalKey) -> Vec<f32> {
    match key {
        MusicalKey::CMajor => vec![261.63, 329.63, 392.00],  // C, E, G (just intonation)
        MusicalKey::AMinor => vec![220.00, 261.63, 329.63],  // A, C, E
        MusicalKey::GMajor => vec![392.00, 493.88, 587.33],  // G, B, D
    }
}

/// Synthesize heterodyned chord for RF transmission
/// Creates beat frequencies by mixing audio chord with RF carrier
/// Output: [carrier - freq1, carrier - freq2, carrier - freq3]
pub fn synthesize_heterodyned_chord(chord_freqs: &[f32], carrier_hz: f32) -> Vec<f32> {
    chord_freqs
        .iter()
        .map(|&modulation_freq| carrier_hz - modulation_freq)
        .collect()
}

/// Predict next attack key from history of attacks
/// Uses mode (most common key) from recent attack history
pub fn predict_next_attack_key(attack_history: &[MusicalKey]) -> MusicalKey {
    if attack_history.is_empty() {
        return MusicalKey::GMajor;  // Default
    }

    // Count occurrences of each key
    let mut c_count = 0;
    let mut a_count = 0;
    let mut g_count = 0;

    for &key in attack_history {
        match key {
            MusicalKey::CMajor => c_count += 1,
            MusicalKey::AMinor => a_count += 1,
            MusicalKey::GMajor => g_count += 1,
        }
    }

    // Return most common (mode)
    if c_count >= a_count && c_count >= g_count {
        MusicalKey::CMajor
    } else if a_count >= g_count {
        MusicalKey::AMinor
    } else {
        MusicalKey::GMajor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_octave_equivalence_c() {
        // All C notes in different octaves should map to C Major
        assert_eq!(detect_attack_key(130.81), MusicalKey::CMajor);
        assert_eq!(detect_attack_key(261.63), MusicalKey::CMajor);
        assert_eq!(detect_attack_key(523.25), MusicalKey::CMajor);
    }

    #[test]
    fn test_chord_frequencies_valid() {
        let c_major = get_chord_frequencies(&MusicalKey::CMajor);
        assert!(c_major.iter().all(|&f| f > 0.0 && f < 1000.0));
    }
}
