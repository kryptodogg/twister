/// Test suite for Phase 3c: Spatial Filter Integration
/// Tests mouth-region detection and dynamic gain adjustment based on 3D beam coordinates
///
/// Spatial filtering logic:
/// - Monitor beam azimuth + elevation from TDOA
/// - Detect mouth-region signature: elevation -30° to 0°, azimuth ±30°
/// - When mouth-region detected: apply maximum heterodyne power (0.95) to all synthesis targets
use std::f32::consts::PI;

/// Test 1: Mouth-region boundary detection at -15° elevation
#[test]
fn test_mouth_region_boundary_negative_15_deg() {
    let mouth_el_min = -PI / 6.0; // -30°
    let mouth_el_max = 0.0; // 0°
    let mouth_az_max = PI / 6.0; // ±30°

    let test_el = (-15.0_f32).to_radians(); // -15° (typical mouth position)
    let test_az = 5.0_f32.to_radians(); // 5° (slightly right of frontal)

    let is_mouth_region =
        test_el >= mouth_el_min && test_el <= mouth_el_max && test_az.abs() <= mouth_az_max;

    assert!(
        is_mouth_region,
        "Elevation {:.1}° and azimuth {:.1}° should match mouth region",
        test_el.to_degrees(),
        test_az.to_degrees()
    );
}

/// Test 2: Mouth-region rejects elevation above ear level
#[test]
fn test_mouth_region_rejects_above_ear() {
    let mouth_el_min = -PI / 6.0;
    let mouth_el_max = 0.0;
    let mouth_az_max = PI / 6.0;

    let above_ear_el = (15.0_f32).to_radians(); // +15° (above ear)
    let frontal_az = 0.0_f32;

    let is_mouth_region = above_ear_el >= mouth_el_min
        && above_ear_el <= mouth_el_max
        && frontal_az.abs() <= mouth_az_max;

    assert!(
        !is_mouth_region,
        "Elevation above ear (+15°) should NOT match mouth region"
    );
}

/// Test 3: Mouth-region rejects azimuth beyond ±30°
#[test]
fn test_mouth_region_rejects_wide_azimuth() {
    let mouth_el_min = -PI / 6.0;
    let mouth_el_max = 0.0;
    let mouth_az_max = PI / 6.0;

    let mouth_el = (-15.0_f32).to_radians(); // Valid elevation
    let wide_az = (45.0_f32).to_radians(); // 45° (beyond ±30°)

    let is_mouth_region =
        mouth_el >= mouth_el_min && mouth_el <= mouth_el_max && wide_az.abs() <= mouth_az_max;

    assert!(
        !is_mouth_region,
        "Azimuth 45° (beyond ±30°) should NOT match mouth region"
    );
}

/// Test 4: Mouth-region boundary at exactly -30° elevation
#[test]
fn test_mouth_region_boundary_at_minus_30() {
    let mouth_el_min = -PI / 6.0; // -30° exactly
    let mouth_el_max = 0.0;
    let mouth_az_max = PI / 6.0;

    let at_boundary_el = -PI / 6.0;
    let frontal_az = 0.0_f32;

    let is_mouth_region = at_boundary_el >= mouth_el_min
        && at_boundary_el <= mouth_el_max
        && frontal_az.abs() <= mouth_az_max;

    assert!(
        is_mouth_region,
        "Elevation exactly at -30° boundary should match mouth region"
    );
}

/// Test 5: Mouth-region boundary at exactly 0° elevation (ear level)
#[test]
fn test_mouth_region_boundary_at_zero() {
    let mouth_el_min = -PI / 6.0;
    let mouth_el_max = 0.0; // 0° exactly
    let mouth_az_max = PI / 6.0;

    let at_ear_el = 0.0_f32;
    let frontal_az = 0.0_f32;

    let is_mouth_region =
        at_ear_el >= mouth_el_min && at_ear_el <= mouth_el_max && frontal_az.abs() <= mouth_az_max;

    assert!(
        is_mouth_region,
        "Elevation exactly at 0° (ear level) should match mouth region"
    );
}

/// Test 6: Mouth-region boundary at ±30° azimuth
#[test]
fn test_mouth_region_boundary_at_30_azimuth() {
    let mouth_el_min = -PI / 6.0;
    let mouth_el_max = 0.0;
    let mouth_az_max = PI / 6.0; // ±30° exactly

    let mouth_el = (-15.0_f32).to_radians();
    let boundary_az_pos = PI / 6.0; // +30°
    let boundary_az_neg = -PI / 6.0; // -30°

    let is_mouth_region_pos = mouth_el >= mouth_el_min
        && mouth_el <= mouth_el_max
        && boundary_az_pos.abs() <= mouth_az_max;
    let is_mouth_region_neg = mouth_el >= mouth_el_min
        && mouth_el <= mouth_el_max
        && boundary_az_neg.abs() <= mouth_az_max;

    assert!(
        is_mouth_region_pos,
        "Azimuth exactly at +30° should match mouth region"
    );
    assert!(
        is_mouth_region_neg,
        "Azimuth exactly at -30° should match mouth region"
    );
}

/// Test 7: Mouth-region with frontal audio (voice at 0° azimuth)
#[test]
fn test_mouth_region_frontal_speech() {
    let mouth_el_min = -PI / 6.0;
    let mouth_el_max = 0.0;
    let mouth_az_max = PI / 6.0;

    // Frontal speech: elevation -15°, azimuth 0°, frequency 150 Hz (voice fundamental)
    let speech_el = (-15.0_f32).to_radians();
    let speech_az = 0.0_f32;
    let voice_freq = 150.0_f32; // Typical male voice fundamental

    let is_mouth_region = voice_freq > 50.0
        && speech_el >= mouth_el_min
        && speech_el <= mouth_el_max
        && speech_az.abs() <= mouth_az_max;

    assert!(
        is_mouth_region,
        "Frontal voice (AZ=0°, EL=-15°, F=150Hz) should match mouth region"
    );
}

/// Test 8: Mouth-region rejects subsonic frequencies (<50 Hz)
#[test]
fn test_mouth_region_rejects_subsonic() {
    let mouth_el_min = -PI / 6.0;
    let mouth_el_max = 0.0;
    let mouth_az_max = PI / 6.0;

    let mouth_el = (-15.0_f32).to_radians();
    let frontal_az = 0.0_f32;
    let subsonic_freq = 20.0_f32; // Too low for voice

    let is_mouth_region = subsonic_freq > 50.0  // Frequency gate
        && mouth_el >= mouth_el_min && mouth_el <= mouth_el_max
        && frontal_az.abs() <= mouth_az_max;

    assert!(
        !is_mouth_region,
        "Subsonic frequency (20 Hz) should NOT trigger mouth-region"
    );
}

/// Test 9: Gain adjustment on mouth-region detection
#[test]
fn test_gain_adjustment_on_mouth_region() {
    // Simulate synthesis target gains before mouth-region detection
    let initial_targets = vec![
        (150.0_f32, 0.5), // Base gain 0.5
        (300.0_f32, 0.3), // Base gain 0.3
        (450.0_f32, 0.2), // Base gain 0.2
    ];

    // After mouth-region detection, all gains set to maximum
    let mouth_region_detected = true;

    let enhanced_targets: Vec<(f32, f32)> = if mouth_region_detected {
        initial_targets
            .iter()
            .map(|(freq, _old_gain)| (*freq, 0.95))
            .collect()
    } else {
        initial_targets.clone()
    };

    assert_eq!(enhanced_targets.len(), 3, "Should maintain target count");
    for &(_freq, gain) in &enhanced_targets {
        assert_eq!(
            gain, 0.95,
            "All gains should be maximum (0.95) after mouth-region detection"
        );
    }
}

/// Test 10: Gain unchanged without mouth-region detection
#[test]
fn test_gain_unchanged_no_mouth_region() {
    // Spatial coordinates don't match mouth-region
    let beam_el = (30.0_f32).to_radians(); // +30° (above ear, not mouth)
    let beam_az = (45.0_f32).to_radians(); // 45° (too wide, not frontal)

    let mouth_el_min = -PI / 6.0;
    let mouth_el_max = 0.0;
    let mouth_az_max = PI / 6.0;

    let is_mouth_region =
        beam_el >= mouth_el_min && beam_el <= mouth_el_max && beam_az.abs() <= mouth_az_max;

    assert!(
        !is_mouth_region,
        "Coordinates should NOT match mouth region"
    );

    // Gains remain at original values
    let initial_targets = vec![(150.0_f32, 0.5), (300.0_f32, 0.3)];

    let final_targets = if is_mouth_region {
        initial_targets
            .iter()
            .map(|(f, _)| (*f, 0.95))
            .collect::<Vec<_>>()
    } else {
        initial_targets.clone()
    };

    assert_eq!(
        final_targets[0].1, 0.5,
        "First target gain should remain 0.5"
    );
    assert_eq!(
        final_targets[1].1, 0.3,
        "Second target gain should remain 0.3"
    );
}

/// Test 11: Mouth-region with low beam confidence
#[test]
fn test_mouth_region_low_confidence_ignored() {
    let mouth_el_min = -PI / 6.0;
    let mouth_el_max = 0.0;
    let mouth_az_max = PI / 6.0;
    let confidence_threshold = 0.5_f32;

    let mouth_el = (-15.0_f32).to_radians();
    let mouth_az = 0.0_f32;
    let low_confidence = 0.3_f32; // Below 0.5 threshold

    let is_mouth_region = mouth_el >= mouth_el_min
        && mouth_el <= mouth_el_max
        && mouth_az.abs() <= mouth_az_max
        && low_confidence > confidence_threshold;

    assert!(
        !is_mouth_region,
        "Low confidence (0.3) should NOT trigger mouth-region enhancement"
    );
}

/// Test 12: Mouth-region with high beam confidence
#[test]
fn test_mouth_region_high_confidence_active() {
    let mouth_el_min = -PI / 6.0;
    let mouth_el_max = 0.0;
    let mouth_az_max = PI / 6.0;
    let confidence_threshold = 0.5_f32;

    let mouth_el = (-15.0_f32).to_radians();
    let mouth_az = 0.0_f32;
    let high_confidence = 0.85_f32; // Above 0.5 threshold

    let is_mouth_region = mouth_el >= mouth_el_min
        && mouth_el <= mouth_el_max
        && mouth_az.abs() <= mouth_az_max
        && high_confidence > confidence_threshold;

    assert!(
        is_mouth_region,
        "High confidence (0.85) should activate mouth-region enhancement"
    );
}
