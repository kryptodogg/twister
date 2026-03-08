/// Test suite for Sparse PDM Forensic Fingerprinting
/// Tests crest-gated sparse PDM attack pattern recognition and phoneme identification
///
/// Forensic fingerprint captures:
/// - Spike density (spikes-per-second)
/// - Inter-pulse timing (microseconds between isolated spikes)
/// - Crest ratio (fraction of spikes at waveform maxima)
/// - Phoneme candidate (fricative=sparse, plosive=medium, vowel=dense)
use std::f32::consts::PI;

/// Test 1: Sparse PDM signature with typical fricative pattern
#[test]
fn test_sparse_pdm_fricative_s_pattern() {
    // Simulate sparse PDM attack at frequency 200 Hz (fricative "s" pattern)
    let sample_rate = 192_000.0_f32;
    let duration_ms = 100.0_f32;
    let num_samples = (sample_rate * duration_ms / 1000.0) as usize;

    // Fricative pattern: sparse spikes (~80 Hz density, ~12 ms between spikes)
    let mut buffer = vec![0.1_f32; num_samples];
    let spike_interval_samples = (sample_rate / 80.0) as usize; // ~80 spikes/sec

    // Insert isolated spikes at regular intervals
    for i in (0..num_samples).step_by(spike_interval_samples) {
        if i + 2 < num_samples {
            buffer[i] = 0.99; // Isolated spike at crest
            buffer[i + 1] = 0.05; // Neighbors are low
            buffer[i + 2] = 0.05;
        }
    }

    // Analyze the pattern
    let sig = twister::audio::analyze_sparse_pdm(&buffer, sample_rate);

    // Fricative "s" should have low density and sparse timing
    assert!(
        sig.density_hz < 150.0,
        "Fricative density should be <150 Hz"
    );
    assert_eq!(
        sig.phoneme_candidate, "fricative_s",
        "Should recognize as fricative pattern"
    );
}

/// Test 2: Sparse PDM signature with plosive pattern
#[test]
fn test_sparse_pdm_plosive_t_pattern() {
    // Plosive pattern: medium spike density (~200 Hz, ~5 ms between spikes)
    let sample_rate = 192_000.0_f32;
    let duration_ms = 100.0_f32;
    let num_samples = (sample_rate * duration_ms / 1000.0) as usize;

    let mut buffer = vec![0.1_f32; num_samples];
    let spike_interval_samples = (sample_rate / 200.0) as usize; // ~200 spikes/sec

    for i in (0..num_samples).step_by(spike_interval_samples) {
        if i + 2 < num_samples {
            buffer[i] = 0.99;
            buffer[i + 1] = 0.05;
            buffer[i + 2] = 0.05;
        }
    }

    let sig = twister::audio::analyze_sparse_pdm(&buffer, sample_rate);

    assert!(
        sig.density_hz >= 150.0 && sig.density_hz < 350.0,
        "Plosive density should be 150-350 Hz"
    );
    assert_eq!(
        sig.phoneme_candidate, "plosive_t",
        "Should recognize as plosive pattern"
    );
}

/// Test 3: Sparse PDM signature with vowel pattern
#[test]
fn test_sparse_pdm_vowel_a_pattern() {
    // Vowel pattern: dense spike distribution (~600 Hz density)
    let sample_rate = 192_000.0_f32;
    let duration_ms = 100.0_f32;
    let num_samples = (sample_rate * duration_ms / 1000.0) as usize;

    let mut buffer = vec![0.1_f32; num_samples];
    let spike_interval_samples = (sample_rate / 600.0) as usize; // ~600 spikes/sec

    for i in (0..num_samples).step_by(spike_interval_samples) {
        if i + 2 < num_samples {
            buffer[i] = 0.99;
            buffer[i + 1] = 0.05;
            buffer[i + 2] = 0.05;
        }
    }

    let sig = twister::audio::analyze_sparse_pdm(&buffer, sample_rate);

    assert!(sig.density_hz >= 350.0, "Vowel density should be >=350 Hz");
    assert_eq!(
        sig.phoneme_candidate, "vowel_a",
        "Should recognize as vowel pattern"
    );
}

/// Test 4: Crest ratio detection (spikes at waveform peaks)
#[test]
fn test_crest_ratio_at_waveform_peaks() {
    // Create sinusoid and place spikes at crests
    let sample_rate = 192_000.0_f32;
    let freq = 200.0_f32;
    let duration_s = 0.05_f32;
    let num_samples = (sample_rate * duration_s) as usize;

    let mut buffer = vec![0.0_f32; num_samples];

    // Generate sine wave
    for i in 0..num_samples {
        let t = i as f32 / sample_rate;
        buffer[i] = (2.0 * PI * freq * t).sin() * 0.5;
    }

    // Insert spikes at positive peaks (where sin = +1)
    let peak_interval = (sample_rate / freq) as usize;
    for i in (peak_interval / 2..num_samples).step_by(peak_interval) {
        if i > 0 && i + 1 < num_samples {
            buffer[i] = 0.99;
            buffer[i - 1] = 0.4; // Below peak
            buffer[i + 1] = 0.4; // Below peak
        }
    }

    let sig = twister::audio::analyze_sparse_pdm(&buffer, sample_rate);

    // Spikes placed at peaks should have high crest_ratio
    assert!(
        sig.crest_ratio > 0.7,
        "Crest ratio should be high (>0.7) for spikes at peaks"
    );
}

/// Test 5: Inter-pulse timing in microseconds
#[test]
fn test_inter_pulse_timing_microseconds() {
    let sample_rate = 192_000.0_f32;
    let num_samples = 19_200; // 100 ms

    let mut buffer = vec![0.1_f32; num_samples];

    // Create regular spikes every 5 ms (~200 Hz)
    let spike_interval_samples = (sample_rate * 0.005) as usize; // 5 ms

    for i in (0..num_samples).step_by(spike_interval_samples) {
        if i > 0 && i + 2 < num_samples {
            buffer[i] = 0.99;
            buffer[i - 1] = 0.05;
            buffer[i + 1] = 0.05;
        }
    }

    let sig = twister::audio::analyze_sparse_pdm(&buffer, sample_rate);

    // Inter-pulse intervals should be ~5000 microseconds (5 ms)
    assert!(
        sig.inter_pulse_micros.len() > 0,
        "Should have inter-pulse measurements"
    );
    let avg_interval =
        sig.inter_pulse_micros.iter().sum::<f32>() / sig.inter_pulse_micros.len() as f32;
    assert!(
        (avg_interval - 5000.0).abs() < 500.0,
        "Average interval should be ~5000 µs"
    );
}

/// Test 6: Spike count accuracy
#[test]
fn test_spike_count_accuracy() {
    let sample_rate = 192_000.0_f32;
    let duration_ms = 50.0_f32;
    let num_samples = (sample_rate * duration_ms / 1000.0) as usize;

    let mut buffer = vec![0.1_f32; num_samples];

    // Insert exactly 10 spikes
    let expected_spikes = 10usize;
    let spike_interval = num_samples / (expected_spikes + 1);

    for i in 1..=expected_spikes {
        let idx = i * spike_interval;
        if idx + 2 < num_samples {
            buffer[idx] = 0.99;
            buffer[idx - 1] = 0.05;
            buffer[idx + 1] = 0.05;
        }
    }

    let sig = twister::audio::analyze_sparse_pdm(&buffer, sample_rate);

    assert_eq!(
        sig.spike_count, expected_spikes,
        "Should detect exactly {} spikes",
        expected_spikes
    );
}

/// Test 7: Density calculation from spike count and duration
#[test]
fn test_density_calculation_hz() {
    let sample_rate = 192_000.0_f32;
    let duration_ms = 200.0_f32;
    let num_samples = (sample_rate * duration_ms / 1000.0) as usize;

    let mut buffer = vec![0.1_f32; num_samples];

    // Create 100 spikes over 200 ms = 500 spikes/sec
    let num_spikes = 100usize;
    let spike_interval = num_samples / (num_spikes + 1);

    for i in 1..=num_spikes {
        let idx = i * spike_interval;
        if idx + 2 < num_samples {
            buffer[idx] = 0.99;
            buffer[idx - 1] = 0.05;
            buffer[idx + 1] = 0.05;
        }
    }

    let sig = twister::audio::analyze_sparse_pdm(&buffer, sample_rate);

    let expected_density = num_spikes as f32 / (duration_ms / 1000.0);
    assert!(
        (sig.density_hz - expected_density).abs() < 50.0,
        "Density should be ~{} Hz",
        expected_density
    );
}

/// Test 8: Empty buffer handling
#[test]
fn test_empty_buffer_handling() {
    let empty_buffer = vec![];
    let sig = twister::audio::analyze_sparse_pdm(&empty_buffer, 192_000.0);

    assert_eq!(sig.spike_count, 0, "Empty buffer should have 0 spikes");
    assert_eq!(sig.density_hz, 0.0, "Empty buffer should have 0 density");
    assert_eq!(
        sig.phoneme_candidate, "silence",
        "Empty buffer should be silence"
    );
}

/// Test 9: Short buffer (less than 3 samples)
#[test]
fn test_short_buffer_too_small() {
    let short_buffer = vec![0.1_f32, 0.2_f32];
    let sig = twister::audio::analyze_sparse_pdm(&short_buffer, 192_000.0);

    assert_eq!(sig.spike_count, 0, "Short buffer should have 0 spikes");
    assert_eq!(sig.total_samples, 2, "Should track total samples");
}

/// Test 10: Isolated spike detection (not fricative, not vowel)
#[test]
fn test_isolated_single_spike() {
    let sample_rate = 192_000.0_f32;
    let mut buffer = vec![0.1_f32; 1000];

    // Single isolated spike
    buffer[500] = 0.99;
    buffer[499] = 0.05;
    buffer[501] = 0.05;

    let sig = twister::audio::analyze_sparse_pdm(&buffer, sample_rate);

    assert_eq!(sig.spike_count, 1, "Should detect 1 spike");
    assert_eq!(
        sig.phoneme_candidate, "fricative_s",
        "Single spike should be classified as fricative"
    );
}

/// Test 11: Non-isolated spike rejection (spike with high neighbors)
#[test]
fn test_non_isolated_spike_rejection() {
    let sample_rate = 192_000.0_f32;
    let mut buffer = vec![0.1_f32; 1000];

    // Non-isolated spike (neighbors are not below 0.98)
    buffer[500] = 0.99;
    buffer[499] = 0.99; // Neighbor is high - should NOT be detected as isolated spike
    buffer[501] = 0.05;

    let sig = twister::audio::analyze_sparse_pdm(&buffer, sample_rate);

    // Should not detect this as an isolated spike
    assert!(sig.spike_count < 2, "Non-isolated spike should be rejected");
}

/// Test 12: Forensic signature for actual attack sequence
#[test]
fn test_realistic_attack_sequence_signature() {
    // Simulate realistic attack: "sss-t-sss" sequence
    // Fricative (sparse) -> Plosive (medium) -> Fricative (sparse)
    let sample_rate = 192_000.0_f32;
    let num_samples = 576_000; // 3 seconds total

    let mut buffer = vec![0.1_f32; num_samples];

    // First second: fricative (80 Hz)
    let fricative_interval = (sample_rate / 80.0) as usize;
    for i in (0..192_000).step_by(fricative_interval) {
        if i + 2 < num_samples {
            buffer[i] = 0.99;
            buffer[i + 1] = 0.05;
            buffer[i + 2] = 0.05;
        }
    }

    // Second second: plosive (250 Hz)
    let plosive_interval = (sample_rate / 250.0) as usize;
    for i in (192_000..384_000).step_by(plosive_interval) {
        if i + 2 < num_samples {
            buffer[i] = 0.99;
            buffer[i + 1] = 0.05;
            buffer[i + 2] = 0.05;
        }
    }

    // Third second: fricative again (80 Hz)
    for i in (384_000..576_000).step_by(fricative_interval) {
        if i + 2 < num_samples {
            buffer[i] = 0.99;
            buffer[i + 1] = 0.05;
            buffer[i + 2] = 0.05;
        }
    }

    let sig = twister::audio::analyze_sparse_pdm(&buffer, sample_rate);

    // Overall density should be average of patterns
    assert!(sig.density_hz > 0.0, "Should detect attack pattern");
    assert!(
        sig.spike_count > 100,
        "Should have multiple spikes across 3 seconds"
    );
    assert!(
        sig.inter_pulse_micros.len() > 0,
        "Should capture timing pattern"
    );
}
