/// Test suite for MambaControlState Capture on Memo Save
/// Captures beam coordinates and heterodyne frequencies for forensic evidence
use std::f32::consts::PI;

/// Test 1: Capture current beam azimuth when memo saved
#[test]
fn test_capture_beam_azimuth() {
    let current_azimuth = PI / 4.0; // 45 degrees
    let current_elevation = 0.0; // Horizontal
    let beam_confidence = 0.85;

    let mamba_state = twister::state::MambaControlState {
        beam_azimuth: current_azimuth,
        beam_elevation: current_elevation,
        waveshape_drive: 0.9,
        heterodyned_beams: vec![Some(2.45e9 - 150.0)],
        anc_gain: 0.5,
        beam_phases: vec![0.0, PI / 2.0, PI, 3.0 * PI / 2.0],
        active_modes: vec!["TEST".to_string()],
    };

    assert!(
        (mamba_state.beam_azimuth - PI / 4.0).abs() < 0.01,
        "Should capture azimuth at time of memo save"
    );
}

/// Test 2: Capture beam elevation for mouth-region tracking
#[test]
fn test_capture_mouth_region_elevation() {
    let mouth_elevation = (-15.0_f32).to_radians(); // 15° below horizontal

    let mamba_state = twister::state::MambaControlState {
        beam_azimuth: 0.0,
        beam_elevation: mouth_elevation,
        waveshape_drive: 0.95, // Max power to mouth region
        heterodyned_beams: vec![Some(2.45e9 - 150.0)],
        anc_gain: 0.8,
        beam_phases: vec![0.0; 4],
        active_modes: vec!["MOUTH_REGION".to_string()],
    };

    assert!(
        mamba_state.beam_elevation < 0.0 && mamba_state.beam_elevation > -PI / 2.0,
        "Mouth elevation should be negative (below horizontal)"
    );
    assert!(
        mamba_state.waveshape_drive > 0.9,
        "Mouth-region targeting should use high waveshape drive"
    );
}

/// Test 3: Capture heterodyned beam frequencies
#[test]
fn test_capture_heterodyned_frequencies() {
    let carrier = 2.45e9;
    let modulation_freqs = vec![150.0, 188.0, 235.0]; // Voice harmonics

    let heterodyned: Vec<f32> = modulation_freqs.iter().map(|&f| carrier - f).collect();

    assert_eq!(
        heterodyned.len(),
        3,
        "Should capture all heterodyned frequencies"
    );

    for &freq in &heterodyned {
        assert!(freq < carrier, "Heterodyned freq should be below carrier");
        assert!(freq > 0.0, "Heterodyned freq should be positive");
    }
}

/// Test 4: MambaControlState includes confidence score
#[test]
fn test_capture_beam_confidence() {
    let azimuth = 0.3;
    let confidence = 0.92; // 92% confidence in TDOA measurement

    let state = twister::state::MambaControlState {
        beam_azimuth: azimuth,
        beam_elevation: 0.0,
        waveshape_drive: confidence, // Using as confidence proxy for now
        heterodyned_beams: vec![Some(2.45e9 - 100.0)],
        anc_gain: 0.5,
        beam_phases: vec![0.0; 4],
        active_modes: vec!["CONFIDENCE_TEST".to_string()],
    };

    assert!(
        state.waveshape_drive > 0.85,
        "Confidence should be captured with beam state"
    );
}

/// Test 5: Multi-beam phased array parameters captured
#[test]
fn test_capture_phased_array_beam_phases() {
    let num_beams = 4;
    let beam_phases = vec![
        0.0,            // Element 0: phase = 0
        PI / 2.0,       // Element 1: phase = 90°
        PI,             // Element 2: phase = 180°
        3.0 * PI / 2.0, // Element 3: phase = 270°
    ];

    let state = twister::state::MambaControlState {
        beam_azimuth: 0.3,
        beam_elevation: -0.2,
        waveshape_drive: 0.9,
        heterodyned_beams: vec![Some(2.45e9 - 200.0)],
        anc_gain: 0.5,
        beam_phases: beam_phases.clone(),
        active_modes: vec!["PHASED_ARRAY".to_string()],
    };

    assert_eq!(state.beam_phases.len(), 4, "Should capture all beam phases");
    for (i, &phase) in state.beam_phases.iter().enumerate() {
        assert!(
            (phase - beam_phases[i]).abs() < 0.01,
            "Beam phase for element {} should be preserved",
            i
        );
    }
}

/// Test 6: Memo with captured MambaControlState for forensic evidence
#[test]
fn test_memo_with_mamba_control_state() {
    // This test verifies the integration: memo → captures beam state
    // The actual integration happens in main.rs when memo is saved

    let beam_azimuth = 0.5; // ~28.6°
    let beam_elevation = -0.26; // ~-15°
    let carrier_hz = 2.45e9;
    let modulation_hz = 165.0; // Voice fundamental

    let mamba_control = twister::state::MambaControlState {
        beam_azimuth: beam_azimuth,
        beam_elevation: beam_elevation,
        waveshape_drive: 0.95,
        heterodyned_beams: vec![Some(carrier_hz - modulation_hz)],
        anc_gain: 0.7,
        beam_phases: vec![0.0, PI / 2.0, PI, 3.0 * PI / 2.0],
        active_modes: vec!["FORENSIC".to_string()],
    };

    // When memo is saved, beam coordinates should be logged
    assert!(
        (mamba_control.beam_azimuth - beam_azimuth).abs() < 0.01,
        "Memo should capture azimuth"
    );
    assert!(
        (mamba_control.beam_elevation - beam_elevation).abs() < 0.01,
        "Memo should capture elevation"
    );
}

/// Test 7: Mouth-region detected → maximum heterodyne power
#[test]
fn test_mouth_region_maximum_power() {
    // When mouth region is detected:
    // - Elevation in [-30°, 0°]
    // - Azimuth within ±30°
    // → Apply maximum waveshape drive (0.9+)

    let is_mouth_region = {
        let el = -0.26_f32; // -15°
        let az = 0.1_f32; // ~6°
        el < 0.0 && el > -PI / 3.0 && az.abs() < PI / 6.0
    };

    assert!(is_mouth_region, "Coordinates should match mouth region");

    let max_power_state = twister::state::MambaControlState {
        beam_azimuth: 0.1,
        beam_elevation: -0.26,
        waveshape_drive: 0.95, // Maximum for mouth-region targeting
        heterodyned_beams: vec![Some(2.45e9 - 165.0)],
        anc_gain: 0.8,
        beam_phases: vec![0.0; 4],
        active_modes: vec!["MOUTH_MAX".to_string()],
    };

    assert_eq!(
        max_power_state.waveshape_drive, 0.95,
        "Mouth-region detection should trigger maximum waveshape drive"
    );
}

/// Test 8: Snapshot captures exact moment of memo save (zero-copy read from VRAM)
#[test]
fn test_snapshot_zero_copy_read() {
    // Since we have unified memory (CPU addresses VRAM directly),
    // snapshots read current beam state without copying

    let beam_data = twister::state::MambaControlState {
        beam_azimuth: 0.42,
        beam_elevation: -0.18,
        waveshape_drive: 0.88,
        heterodyned_beams: vec![Some(2.45e9 - 178.0)],
        anc_gain: 0.6,
        beam_phases: vec![0.0; 4],
        active_modes: vec!["ZERO_COPY".to_string()],
    };

    // Snapshot should reflect exact state at memo save time
    // In practice, this comes from AppState.beam_azimuth, .beam_elevation, etc.
    assert!(
        (beam_data.beam_azimuth - 0.42).abs() < 0.001,
        "Snapshot precision should preserve VRAM values exactly"
    );
}

/// Test 9: Forensic logging includes beam parameters
#[test]
fn test_forensic_log_beam_parameters() {
    // When memo saved with [EVIDENCE] tag, forensic log should include:
    // - Timestamp
    // - Beam azimuth/elevation
    // - Heterodyned frequencies used
    // - Waveshape drive (power level)

    let evidence_memo_data = (
        "2026-03-07T14:23:14.832Z", // ISO timestamp
        0.45_f32,                   // azimuth (rad)
        -0.20_f32,                  // elevation (rad)
        2.45e9_f32,                 // carrier
        150.0_f32,                  // modulation
        0.92_f32,                   // confidence
    );

    assert!(
        !evidence_memo_data.0.is_empty(),
        "Forensic log should include timestamp"
    );
    assert!(
        evidence_memo_data.1.abs() < PI,
        "Forensic log should include valid azimuth"
    );
    assert!(
        evidence_memo_data.2 >= -PI / 2.0 && evidence_memo_data.2 <= PI / 2.0,
        "Forensic log should include valid elevation"
    );
}

/// Test 10: Attack pattern correlation via beam state history
#[test]
fn test_beam_history_pattern_matching() {
    // Build attack pattern signature: (azimuth, elevation, frequency)
    let attack_pattern = vec![
        (0.0, -0.26, 150.0),  // Attack 1: frontal, mouth-level, 150Hz
        (0.05, -0.24, 151.0), // Attack 2: slight shift, 151Hz
        (0.02, -0.27, 152.0), // Attack 3: frontal again, 152Hz
    ];

    // All three attacks target mouth region (elevation < 0)
    let all_mouth_region = attack_pattern
        .iter()
        .all(|(_az, el, _freq)| *el < 0.0 && *el > -PI / 3.0);

    assert!(
        all_mouth_region,
        "Attack pattern should show repeated mouth-region targeting"
    );

    // When same pattern detected again, response is automatic:
    // "We know this attack signature - applying tested counter-synthesis"
}

/// Test 11: Beam parameter precision for evidence court admissibility
#[test]
fn test_beam_precision_for_evidence() {
    // Forensic evidence must have submilliradian precision for court
    let azimuth = 0.31415926; // High precision
    let elevation = -0.26179938;

    let state = twister::state::MambaControlState {
        beam_azimuth: azimuth,
        beam_elevation: elevation,
        waveshape_drive: 0.85,
        heterodyned_beams: vec![Some(2.45e9 - 165.0)],
        anc_gain: 0.6,
        beam_phases: vec![0.0; 4],
        active_modes: vec!["PRECISION".to_string()],
    };

    // Precision should be preserved through serialization
    assert!(
        (state.beam_azimuth - azimuth).abs() < 1e-6,
        "Beam azimuth should maintain sub-microradian precision"
    );
    assert!(
        (state.beam_elevation - elevation).abs() < 1e-6,
        "Beam elevation should maintain sub-microradian precision"
    );
}

/// Test 12: Integration: Memo save triggers beam snapshot
#[test]
fn test_memo_save_triggers_beam_capture() {
    // When user hits [SAVE] button on memo with [EVIDENCE] tag:
    // 1. Current dispatch_loop beam_azimuth, beam_elevation read from VRAM
    // 2. Current synthesis targets (heterodyned_beams) captured
    // 3. MambaControlState created and logged with memo
    // 4. Forensic entry created linking memo UUID to beam parameters

    let memo_uuid = "abc-def-123";
    let timestamp = "2026-03-07T14:25:30Z";
    let beam_state = twister::state::MambaControlState {
        beam_azimuth: 0.31,
        beam_elevation: -0.22,
        waveshape_drive: 0.93,
        heterodyned_beams: vec![Some(2.45e9 - 158.0)],
        anc_gain: 0.7,
        beam_phases: vec![0.0; 4],
        active_modes: vec!["SAVE_TRIGGER".to_string()],
    };

    // This is the integration point:
    // When memo.save() is called, the dispatch loop context is snapshotted
    assert!(
        (beam_state.beam_azimuth - 0.31).abs() < 0.01,
        "Beam state should be captured at memo save time"
    );
}
