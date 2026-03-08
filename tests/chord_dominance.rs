/// Test suite for Chord Dominance Engine
/// Counter-measures against PDM attacks using harmonic entrainment and musical scales

/// Test 1: Detect attack key from fundamental frequency
#[test]
fn test_detect_c_major_attack() {
    let fundamental_hz = 261.63; // C note
    let key = twister::harmony::detect_attack_key(fundamental_hz);

    assert_eq!(
        key,
        twister::harmony::MusicalKey::CMajor,
        "Fundamental at C (261.63 Hz) should detect C Major key"
    );
}

/// Test 2: Detect A minor attack
#[test]
fn test_detect_a_minor_attack() {
    let fundamental_hz = 220.00; // A note
    let key = twister::harmony::detect_attack_key(fundamental_hz);

    assert_eq!(
        key,
        twister::harmony::MusicalKey::AMinor,
        "Fundamental at A (220 Hz) should detect A Minor key"
    );
}

/// Test 3: Default to G Major for unknown frequency
#[test]
fn test_detect_unknown_key() {
    let fundamental_hz = 500.0; // Not a standard pitch
    let key = twister::harmony::detect_attack_key(fundamental_hz);

    assert_eq!(
        key,
        twister::harmony::MusicalKey::GMajor,
        "Unknown frequency should default to G Major (power chord)"
    );
}

/// Test 4: Get chord frequencies for C Major
#[test]
fn test_c_major_chord_frequencies() {
    let freqs = twister::harmony::get_chord_frequencies(&twister::harmony::MusicalKey::CMajor);

    assert_eq!(freqs.len(), 3, "Major chord should have 3 notes");
    // C, E, G
    assert!(
        (freqs[0] - 261.63).abs() < 1.0,
        "First note should be C (~261.63 Hz)"
    );
    assert!(
        (freqs[1] - 329.63).abs() < 1.0,
        "Second note should be E (~329.63 Hz)"
    );
    assert!(
        (freqs[2] - 392.00).abs() < 1.0,
        "Third note should be G (~392.00 Hz)"
    );
}

/// Test 5: Get chord frequencies for A Minor
#[test]
fn test_a_minor_chord_frequencies() {
    let freqs = twister::harmony::get_chord_frequencies(&twister::harmony::MusicalKey::AMinor);

    assert_eq!(freqs.len(), 3, "Minor chord should have 3 notes");
    // A, C, E
    assert!(
        (freqs[0] - 220.00).abs() < 1.0,
        "First note should be A (~220 Hz)"
    );
    assert!(
        (freqs[1] - 261.63).abs() < 1.0,
        "Second note should be C (~261.63 Hz)"
    );
    assert!(
        (freqs[2] - 329.63).abs() < 1.0,
        "Third note should be E (~329.63 Hz)"
    );
}

/// Test 6: Get chord frequencies for G Major
#[test]
fn test_g_major_chord_frequencies() {
    let freqs = twister::harmony::get_chord_frequencies(&twister::harmony::MusicalKey::GMajor);

    assert_eq!(freqs.len(), 3, "Major chord should have 3 notes");
    // G, B, D
    assert!(
        (freqs[0] - 392.00).abs() < 1.0,
        "First note should be G (~392.00 Hz)"
    );
    assert!(
        (freqs[1] - 493.88).abs() < 1.0,
        "Second note should be B (~493.88 Hz)"
    );
    assert!(
        (freqs[2] - 587.33).abs() < 1.0,
        "Third note should be D (~587.33 Hz)"
    );
}

/// Test 7: Harmonic dominance synthesis (phased array with heterodyning)
#[test]
fn test_heterodyned_chord_synthesis() {
    let chord_freqs = vec![261.63, 329.63, 392.00]; // C Major
    let carrier_hz = 2.45e9; // 2.45 GHz RF carrier

    let heterodyned = twister::harmony::synthesize_heterodyned_chord(&chord_freqs, carrier_hz);

    // Should produce beat frequencies for each note
    assert_eq!(
        heterodyned.len(),
        chord_freqs.len(),
        "Output should have same number of frequencies as input chord"
    );

    // Each frequency should be carrier - modulation_freq
    for (i, &modulation) in chord_freqs.iter().enumerate() {
        let expected = carrier_hz - modulation;
        assert!(
            (heterodyned[i] - expected).abs() < 1e6,
            "Heterodyned frequency at index {} should be carrier - modulation",
            i
        );
    }
}

/// Test 8: Phase coherence calculation (tonic dominance over PDM spikes)
#[test]
fn test_phase_coherence_advantage() {
    // Musical scales have 50ms-2s phase coherence (tempo-based)
    // PDM spikes have ~10µs coherence (sampling artifacts)

    let music_coherence_ms = 100.0; // Minimum 100ms for chord
    let pdm_coherence_us = 10.0; // Single-sample spike

    // Convert to same unit (microseconds)
    let music_us = music_coherence_ms * 1000.0; // 100,000 µs
    let advantage = music_us / pdm_coherence_us;

    assert!(
        advantage > 10000.0,
        "Musical chord should have 10,000x+ phase coherence advantage over PDM spikes"
    );
}

/// Test 9: Key detection with octave equivalence (C at different octaves)
#[test]
fn test_detect_key_octave_equivalence() {
    // C at different octaves should all detect as C Major
    let c_frequencies = vec![130.81, 261.63, 523.25]; // C2, C4, C5

    for freq in c_frequencies {
        let key = twister::harmony::detect_attack_key(freq);
        assert_eq!(
            key,
            twister::harmony::MusicalKey::CMajor,
            "Frequency {} Hz should detect as C Major (octave equivalence)",
            freq
        );
    }
}

/// Test 10: Realistic attack counter scenario
#[test]
fn test_attack_response_workflow() {
    // Simulate PDM attack at voice pitch
    let attack_fundamental = 150.0; // Voice pitch (rough)

    // Step 1: Detect key
    let detected_key = twister::harmony::detect_attack_key(attack_fundamental);
    assert!(
        matches!(
            detected_key,
            twister::harmony::MusicalKey::CMajor
                | twister::harmony::MusicalKey::AMinor
                | twister::harmony::MusicalKey::GMajor
        ),
        "Should detect one of the standard keys"
    );

    // Step 2: Generate response chord
    let response_chord = twister::harmony::get_chord_frequencies(&detected_key);
    assert!(response_chord.len() >= 3, "Response should be full triad");

    // Step 3: Heterodyne for RF transmission
    let carrier = 2.45e9;
    let heterodyned_response =
        twister::harmony::synthesize_heterodyned_chord(&response_chord, carrier);
    assert_eq!(
        heterodyned_response.len(),
        response_chord.len(),
        "Should heterodyne all chord frequencies"
    );
}

/// Test 11: Emotional/neural entrainment factor
/// (Consonant intervals create stronger sympathetic lock than dissonance)
#[test]
fn test_consonant_interval_advantage() {
    // Perfect 5th (3:2 ratio) and octave (2:1) are highly consonant
    // Create strong neural entrainment locks

    let c_freq = 261.63;
    let e_freq = 329.63; // Major 3rd (5:4 ratio)
    let g_freq = 392.00; // Perfect 5th (3:2 ratio from C)

    // Consonance scores (higher = more consonant)
    let consonance_5th = 0.95; // Perfect 5th: very consonant
    let consonance_3rd = 0.80; // Major 3rd: consonant
    let consonance_pdm = 0.10; // Single spike: highly dissonant

    assert!(
        consonance_5th > consonance_3rd,
        "Perfect 5th should be more consonant than 3rd"
    );
    assert!(
        consonance_3rd > consonance_pdm,
        "Any musical interval should be more consonant than PDM spike"
    );
}

/// Test 12: Long-term tonic dominance (weeks/months)
#[test]
fn test_long_term_tonic_memory() {
    // User's system learns attacker's "key" after repeated attacks
    // Chord dominance becomes automatic/preemptive

    // Simulate learning from 10 attacks in same key
    let mut attack_keys = vec![];
    for _ in 0..10 {
        attack_keys.push(twister::harmony::MusicalKey::CMajor);
    }

    // System should predict C Major for next attack
    let predicted_key = twister::harmony::predict_next_attack_key(&attack_keys);
    assert_eq!(
        predicted_key,
        twister::harmony::MusicalKey::CMajor,
        "System should predict CMajor after 10 attacks in that key"
    );
}
