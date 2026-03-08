// tests/features_audio.rs — Test suite for 196-D audio feature extractor
//
// TDD pattern: All tests written first, then implementation follows
// Tests verify:
// 1. Feature vector has exactly 196 dimensions
// 2. STFT extraction produces 162-D Mel-scale magnitude + phase
// 3. TDOA features normalize azimuth/elevation correctly
// 4. Sparse PDM features extract 8-D from SparsePdmSignature
// 5. Bispectrum anomaly produces 3-D vector
// 6. Wave topology coherence produces 9-D vector
// 7. Musical features extract 12-D chromatic energy

// Mock function to create test audio buffer (sine wave)
fn create_test_buffer(sample_rate: f32, freq_hz: f32, duration_ms: f32) -> Vec<f32> {
    let num_samples = (sample_rate * duration_ms / 1000.0) as usize;
    let mut buffer = vec![0.0; num_samples];

    for (i, sample) in buffer.iter_mut().enumerate() {
        let t = i as f32 / sample_rate;
        let angle = 2.0 * std::f32::consts::PI * freq_hz * t;
        *sample = angle.sin();
    }
    buffer
}

// Mock SparsePdmSignature for testing
fn create_test_pdm_signature() -> twister::audio::SparsePdmSignature {
    twister::audio::SparsePdmSignature {
        spike_count: 100,
        total_samples: 10000,
        density_hz: 500.0,
        inter_pulse_micros: vec![
            100.0, 150.0, 120.0, 180.0, 110.0,
            130.0, 160.0, 140.0, 125.0, 155.0,
        ],
        crest_ratio: 0.75,
        phoneme_candidate: "s".to_string(),
    }
}

#[test]
fn test_audio_features_dimension_196() {
    use twister::features::audio::extract_audio_features;

    let buffer = create_test_buffer(192000.0, 1000.0, 50.0);
    let pdm_sig = create_test_pdm_signature();
    let wave_coherence = [0.5; 9];

    let features = extract_audio_features(
        &buffer,
        192000.0,
        0.5,  // azimuth_rad
        0.3,  // elevation_rad
        &pdm_sig,
        &wave_coherence,
    );

    // Must be exactly 196 dimensions
    assert_eq!(
        features.feature_vector.len(),
        196,
        "Feature vector must be exactly 196-D"
    );
    assert_eq!(features.total_dimension, 196);
}

#[test]
fn test_stft_mel_extraction() {
    use twister::features::audio::extract_audio_features;

    let buffer = create_test_buffer(192000.0, 5000.0, 50.0);
    let pdm_sig = create_test_pdm_signature();
    let wave_coherence = [0.5; 9];

    let features = extract_audio_features(
        &buffer,
        192000.0,
        0.0,
        0.0,
        &pdm_sig,
        &wave_coherence,
    );

    // STFT magnitude + phase = 162-D (81 mag + 81 phase)
    assert_eq!(
        features.stft_mel_magnitude.len(),
        81,
        "STFT magnitude must be 81-D (Mel bins)"
    );
    assert_eq!(
        features.stft_mel_phase.len(),
        81,
        "STFT phase must be 81-D"
    );

    // All values should be normalized
    for &val in &features.stft_mel_magnitude {
        assert!(val.is_finite(), "STFT magnitude values must be finite");
        assert!(val >= 0.0 && val <= 1.0, "STFT magnitude must be normalized to [0, 1]");
    }

    for &val in &features.stft_mel_phase {
        assert!(val.is_finite(), "STFT phase values must be finite");
        assert!(val >= -1.0 && val <= 1.0, "STFT phase must be normalized to [-1, 1]");
    }
}

#[test]
fn test_tdoa_feature_normalization() {
    use twister::features::audio::extract_audio_features;
    use std::f32::consts::PI;

    let buffer = create_test_buffer(192000.0, 1000.0, 50.0);
    let pdm_sig = create_test_pdm_signature();
    let wave_coherence = [0.5; 9];

    // Test with positive azimuth and elevation
    let azimuth_rad = PI / 4.0;  // 45 degrees
    let elevation_rad = PI / 6.0;  // 30 degrees

    let features = extract_audio_features(
        &buffer,
        192000.0,
        azimuth_rad,
        elevation_rad,
        &pdm_sig,
        &wave_coherence,
    );

    // TDOA features should be 2-D: [azimuth_norm, elevation_norm]
    assert_eq!(features.tdoa_features.len(), 2, "TDOA must be 2-D");

    // Verify normalization to [-1, 1]
    assert!(
        features.tdoa_features[0] >= -1.0 && features.tdoa_features[0] <= 1.0,
        "Azimuth must be normalized to [-1, 1], got {}",
        features.tdoa_features[0]
    );
    assert!(
        features.tdoa_features[1] >= -1.0 && features.tdoa_features[1] <= 1.0,
        "Elevation must be normalized to [-1, 1], got {}",
        features.tdoa_features[1]
    );
}

#[test]
fn test_sparse_pdm_integration() {
    use twister::features::audio::extract_audio_features;

    let buffer = create_test_buffer(192000.0, 1000.0, 50.0);
    let pdm_sig = create_test_pdm_signature();
    let wave_coherence = [0.5; 9];

    let features = extract_audio_features(
        &buffer,
        192000.0,
        0.0,
        0.0,
        &pdm_sig,
        &wave_coherence,
    );

    // Sparse PDM must be 8-D: [density, inter_pulse_var, crest_ratio, phoneme_conf, 4 timing stats]
    assert_eq!(
        features.sparse_pdm_signature.len(),
        8,
        "Sparse PDM must be 8-D"
    );

    // All values should be normalized to [0, 1]
    for &val in &features.sparse_pdm_signature {
        assert!(val.is_finite(), "PDM values must be finite");
        assert!(
            val >= 0.0 && val <= 1.0,
            "PDM values must be normalized to [0, 1], got {}",
            val
        );
    }
}

#[test]
fn test_bispectrum_anomaly_extraction() {
    use twister::features::audio::extract_audio_features;

    let buffer = create_test_buffer(192000.0, 1000.0, 50.0);
    let pdm_sig = create_test_pdm_signature();
    let wave_coherence = [0.5; 9];

    let features = extract_audio_features(
        &buffer,
        192000.0,
        0.0,
        0.0,
        &pdm_sig,
        &wave_coherence,
    );

    // Bispectrum anomaly must be 3-D: top 3 anomaly peaks
    assert_eq!(
        features.bispectrum_anomaly_components.len(),
        3,
        "Bispectrum anomaly must be 3-D"
    );

    // All values should be normalized to [0, 1]
    for &val in &features.bispectrum_anomaly_components {
        assert!(val.is_finite(), "Bispectrum values must be finite");
        assert!(
            val >= 0.0 && val <= 1.0,
            "Bispectrum values must be normalized to [0, 1]"
        );
    }
}

#[test]
fn test_wave_topology_coherence() {
    use twister::features::audio::extract_audio_features;

    let buffer = create_test_buffer(192000.0, 1000.0, 50.0);
    let pdm_sig = create_test_pdm_signature();
    let wave_coherence = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];

    let features = extract_audio_features(
        &buffer,
        192000.0,
        0.0,
        0.0,
        &pdm_sig,
        &wave_coherence,
    );

    // Wave coherence must be 9-D (from 4-mic array cross-pairs)
    assert_eq!(
        features.wave_topology_coherence.len(),
        9,
        "Wave topology must be 9-D"
    );

    // All values should be normalized to [0, 1]
    for (i, &val) in features.wave_topology_coherence.iter().enumerate() {
        assert!(val.is_finite(), "Wave coherence[{}] must be finite", i);
        assert!(
            val >= 0.0 && val <= 1.0,
            "Wave coherence[{}] must be normalized to [0, 1], got {}",
            i, val
        );
    }
}

#[test]
fn test_musical_features() {
    use twister::features::audio::extract_audio_features;

    let buffer = create_test_buffer(192000.0, 1000.0, 50.0);
    let pdm_sig = create_test_pdm_signature();
    let wave_coherence = [0.5; 9];

    let features = extract_audio_features(
        &buffer,
        192000.0,
        0.0,
        0.0,
        &pdm_sig,
        &wave_coherence,
    );

    // Musical features must be 12-D (chromatic pitches: C, C#, D, D#, E, F, F#, G, G#, A, A#, B)
    assert_eq!(
        features.musical_features.len(),
        12,
        "Musical features must be 12-D"
    );

    // All values should be normalized to [0, 1]
    for &val in &features.musical_features {
        assert!(val.is_finite(), "Musical values must be finite");
        assert!(
            val >= 0.0 && val <= 1.0,
            "Musical values must be normalized to [0, 1]"
        );
    }
}

#[test]
fn test_feature_vector_concatenation() {
    use twister::features::audio::extract_audio_features;

    let buffer = create_test_buffer(192000.0, 1000.0, 50.0);
    let pdm_sig = create_test_pdm_signature();
    let wave_coherence = [0.5; 9];

    let features = extract_audio_features(
        &buffer,
        192000.0,
        0.5,
        0.3,
        &pdm_sig,
        &wave_coherence,
    );

    // Verify concatenation order: 162 + 2 + 8 + 3 + 9 + 12 = 196
    let expected_size = 162 + 2 + 8 + 3 + 9 + 12;
    assert_eq!(
        features.feature_vector.len(),
        expected_size,
        "Feature vector concatenation must be 196-D"
    );

    // Verify all features are finite
    for (i, &val) in features.feature_vector.iter().enumerate() {
        assert!(
            val.is_finite(),
            "Feature vector[{}] must be finite, got {}",
            i,
            val
        );
    }
}

#[test]
fn test_short_buffer_handling() {
    use twister::features::audio::extract_audio_features;

    // Very short buffer (100 samples at 192kHz = ~0.5ms)
    let buffer = create_test_buffer(192000.0, 1000.0, 1.0);
    let pdm_sig = create_test_pdm_signature();
    let wave_coherence = [0.5; 9];

    // Should not panic and should still return 196-D
    let features = extract_audio_features(
        &buffer,
        192000.0,
        0.0,
        0.0,
        &pdm_sig,
        &wave_coherence,
    );

    assert_eq!(features.feature_vector.len(), 196);
}

#[test]
fn test_different_sample_rates() {
    use twister::features::audio::extract_audio_features;

    let buffer_48k = create_test_buffer(48000.0, 1000.0, 50.0);
    let pdm_sig = create_test_pdm_signature();
    let wave_coherence = [0.5; 9];

    // Should work with different sample rates
    let features = extract_audio_features(
        &buffer_48k,
        48000.0,
        0.0,
        0.0,
        &pdm_sig,
        &wave_coherence,
    );

    assert_eq!(features.feature_vector.len(), 196);
    assert_eq!(features.total_dimension, 196);
}
