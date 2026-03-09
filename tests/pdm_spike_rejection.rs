/// Test suite for PDM spike rejection filter
/// PDM (Pulse Density Modulation) attack detection and mitigation

/// Test 1: Single isolated spike rejection
#[test]
fn test_reject_single_spike() {
    let input = vec![0.5, 0.99, 0.5]; // Single spike at crest
    let (output, spike_count) = twister::audio::reject_pdm_spikes(&input);

    // Spike should be interpolated to ~0.5
    assert!(
        (output[1] - 0.5).abs() < 0.01,
        "Spike should be interpolated to 0.5"
    );
    assert_eq!(spike_count, 1, "Should detect 1 spike");
}

/// Test 2: Multiple isolated spikes
#[test]
fn test_reject_multiple_spikes() {
    let input = vec![0.3, 0.99, 0.3, 0.2, 0.99, 0.2]; // Two spikes (at index 1 and 4)
    let (output, spike_count) = twister::audio::reject_pdm_spikes(&input);

    assert_eq!(spike_count, 2, "Should detect 2 spikes");
    assert!(
        (output[1] - 0.3).abs() < 0.01,
        "First spike should be interpolated"
    );
    assert!(
        (output[4] - 0.2).abs() < 0.01,
        "Second spike should be interpolated"
    );
}

/// Test 3: Sustained high amplitude (not a PDM spike)
#[test]
fn test_preserve_high_amplitude_signals() {
    let input = vec![0.98, 0.99, 0.98]; // Sustained high amplitude - not isolated
    let (output, spike_count) = twister::audio::reject_pdm_spikes(&input);

    assert_eq!(
        spike_count, 0,
        "Should not detect spike in sustained high amplitude"
    );
    assert_eq!(output[1], 0.99, "Sustained signal should be preserved");
}

/// Test 4: Spike at buffer edge (no previous neighbor)
#[test]
fn test_spike_at_start() {
    let input = vec![0.99, 0.5, 0.5]; // Spike at start
    let (output, spike_count) = twister::audio::reject_pdm_spikes(&input);

    // Should handle edge case gracefully
    assert_eq!(
        spike_count, 0,
        "Edge spike without proper neighbors should not be counted"
    );
    assert_eq!(output[0], 0.99, "Start position should be preserved");
}

/// Test 5: Spike at buffer end (no next neighbor)
#[test]
fn test_spike_at_end() {
    let input = vec![0.5, 0.5, 0.99]; // Spike at end
    let (output, spike_count) = twister::audio::reject_pdm_spikes(&input);

    assert_eq!(
        spike_count, 0,
        "End spike without proper neighbors should not be counted"
    );
    assert_eq!(output[2], 0.99, "End position should be preserved");
}

/// Test 6: Consecutive spikes (not isolated, so not detected)
#[test]
fn test_consecutive_spikes() {
    let input = vec![0.3, 0.99, 0.99, 0.3]; // Two consecutive 0.99s
    let (output, spike_count) = twister::audio::reject_pdm_spikes(&input);

    // Consecutive spikes don't match isolation pattern (neighbors both at/above 0.98)
    assert_eq!(
        spike_count, 0,
        "Consecutive 0.99s don't match isolated spike pattern"
    );
}

/// Test 7: Threshold boundary (0.99 is the threshold)
#[test]
fn test_threshold_boundary() {
    let input1 = vec![0.5, 0.99, 0.5]; // Exactly at threshold
    let (_, count1) = twister::audio::reject_pdm_spikes(&input1);

    let input2 = vec![0.5, 0.98, 0.5]; // Just below threshold
    let (_, count2) = twister::audio::reject_pdm_spikes(&input2);

    assert_eq!(count1, 1, "0.99 should be detected as spike");
    assert_eq!(count2, 0, "0.98 should not be detected as spike");
}

/// Test 8: Empty and single-element buffers
#[test]
fn test_edge_case_buffer_sizes() {
    let (output_empty, count_empty) = twister::audio::reject_pdm_spikes(&[]);
    assert_eq!(count_empty, 0, "Empty buffer should have 0 spikes");
    assert!(
        output_empty.is_empty(),
        "Empty buffer should return empty output"
    );

    let (output_single, count_single) = twister::audio::reject_pdm_spikes(&[0.99]);
    assert_eq!(
        count_single, 0,
        "Single element can't be spike (no neighbors)"
    );
    assert_eq!(output_single[0], 0.99, "Single element should be preserved");
}

/// Test 9: Real-world PDM attack pattern (multiple spikes with varying intervals)
#[test]
fn test_realistic_pdm_attack_pattern() {
    // Simulates actual PDM attack: "sss" phoneme with crest-targeting spikes
    let mut input = vec![0.0; 100];
    input[10] = 0.99; // Spike 1
    input[25] = 0.99; // Spike 2
    input[40] = 0.99; // Spike 3
    input[55] = 0.99; // Spike 4

    let (output, spike_count) = twister::audio::reject_pdm_spikes(&input);

    assert_eq!(spike_count, 4, "Should detect all 4 PDM spikes");
    assert_eq!(
        output[10], 0.0,
        "Spike should be interpolated to neighbor average"
    );
    assert_eq!(
        output[25], 0.0,
        "Spike should be interpolated to neighbor average"
    );
}

/// Test 10: Negative spikes (troughs instead of crests)
#[test]
fn test_negative_spikes() {
    let input = vec![-0.5, -0.99, -0.5]; // Negative spike at trough
    let (output, spike_count) = twister::audio::reject_pdm_spikes(&input);

    // Symmetric behavior: spikes below -0.99 should also be detected
    assert_eq!(spike_count, 1, "Should detect negative spikes (troughs)");
    assert!(
        (-output[1] - 0.5).abs() < 0.01,
        "Negative spike should be interpolated"
    );
}
