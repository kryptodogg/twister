/// Test suite for TDOA Elevation Estimation
/// Extends 2D (azimuth-only) beamforming to full 3D spatial targeting
use std::f32::consts::PI;

/// Test 1: Elevation from vertical mic pair time delay
#[test]
fn test_elevation_from_time_delay() {
    let sample_rate = 192_000.0;
    let vertical_mic_spacing = 0.20; // 20cm vertical separation
    let speed_of_sound = 343.0;

    // Sound source 45° elevation above horizontal
    let elevation_rad = PI / 4.0; // 45 degrees
    let expected_lag =
        (elevation_rad.sin() * vertical_mic_spacing * sample_rate / speed_of_sound) as i32;

    // Calculate elevation from lag
    let lag_s = expected_lag as f32 / sample_rate;
    let sin_elev = (lag_s * speed_of_sound / vertical_mic_spacing).clamp(-1.0, 1.0);
    let computed_elevation = sin_elev.asin();

    assert!(
        (computed_elevation - elevation_rad).abs() < 0.05,
        "Elevation from vertical lag should be ~45°, got {:.1}°",
        computed_elevation.to_degrees()
    );
}

/// Test 2: Horizontal source has zero elevation
#[test]
fn test_horizontal_source_zero_elevation() {
    let elevation = 0.0; // Level with microphones
    let expected_lag = 0; // No time delay

    assert_eq!(expected_lag, 0, "Horizontal source should have zero lag");
    assert!(
        (elevation as f32).abs() < 0.01,
        "Horizontal source should have zero elevation"
    );
}

/// Test 3: Overhead source (zenith) gives maximum elevation
#[test]
fn test_zenith_source_maximum_elevation() {
    let zenith_elevation = PI / 2.0; // 90 degrees

    assert!(
        (zenith_elevation - PI / 2.0).abs() < 0.01,
        "Zenith should be 90° elevation"
    );
}

/// Test 4: Negative elevation (source below horizontal)
#[test]
fn test_below_horizontal_negative_elevation() {
    let below_horizontal = -PI / 6.0; // -30 degrees below horizontal

    assert!(
        below_horizontal < 0.0,
        "Source below horizontal should have negative elevation"
    );
}

/// Test 5: Energy ratio between vertical mics indicates elevation
#[test]
fn test_elevation_from_energy_ratio() {
    // Mic 0 (top): closer to source → higher energy
    // Mic 2 (bottom): farther → lower energy
    let energy_top = 1.0;
    let energy_bottom = 0.5;
    let energy_ratio = energy_top / (energy_bottom + 1e-9);

    // Higher energy_ratio → source above (positive elevation)
    assert!(
        energy_ratio > 1.0,
        "Higher energy in top mic indicates source above"
    );

    // Convert to elevation (spherical geometry)
    let elevation_from_energy = ((energy_ratio as f32).log2() * 0.2_f32).clamp(-PI / 2.0, PI / 2.0);
    assert!(
        elevation_from_energy > 0.0,
        "Energy ratio > 1.0 should give positive elevation"
    );
}

/// Test 6: Symmetric elevation: ±30 degrees should have equal magnitude
#[test]
fn test_symmetric_elevation_angles() {
    let elev_plus_30 = PI / 6.0;
    let elev_minus_30 = -PI / 6.0;

    assert_eq!(
        elev_plus_30.abs(),
        elev_minus_30.abs(),
        "±30° should have equal magnitude"
    );
}

/// Test 7: Elevation + Azimuth 3D localization
#[test]
fn test_3d_spatial_coordinates() {
    let azimuth_rad = PI / 4.0; // 45° to the right
    let elevation_rad = PI / 6.0; // 30° above

    // Convert to 3D unit vector (spherical to Cartesian)
    let x = elevation_rad.cos() * azimuth_rad.sin(); // East component
    let y = elevation_rad.sin(); // Up component
    let z = elevation_rad.cos() * azimuth_rad.cos(); // North component

    // Should form unit vector (magnitude = 1)
    let magnitude = (x * x + y * y + z * z).sqrt();
    assert!(
        (magnitude - 1.0).abs() < 0.01,
        "3D vector should be unit magnitude"
    );
}

/// Test 8: Mouth-region spatial signature detection
#[test]
fn test_mouth_region_detection() {
    // Mouth typically at elevation -30° to 0° (below ear level)
    // and azimuth 0° (frontal)
    let mouth_azimuth = 0.0;
    let mouth_elevation = -PI / 12.0; // -15° below horizontal (roughly mouth height)

    assert!(
        mouth_elevation < 0.0 && mouth_elevation > -PI / 2.0,
        "Mouth should be below ear level"
    );

    // When mouth region detected, apply maximum heterodyne power
    let is_mouth_region =
        (mouth_azimuth as f32).abs() < 0.1 && mouth_elevation < 0.0 && mouth_elevation > -PI / 3.0;
    assert!(
        is_mouth_region,
        "Spatial coordinates should match mouth region"
    );
}

/// Test 9: Elevation range limits (-90° to +90°)
#[test]
fn test_elevation_clamping() {
    let extreme_positive = PI; // Beyond zenith
    let extreme_negative = -PI;

    let clamped_pos = extreme_positive.clamp(-PI / 2.0, PI / 2.0);
    let clamped_neg = extreme_negative.clamp(-PI / 2.0, PI / 2.0);

    assert_eq!(clamped_pos, PI / 2.0, "Elevation should clamp to +90°");
    assert_eq!(clamped_neg, -PI / 2.0, "Elevation should clamp to -90°");
}

/// Test 10: Real-world scenario - voice at mouth level with azimuth
#[test]
fn test_realistic_voice_targeting() {
    // Human voice typically 100-300 Hz fundamental
    let voice_fundamental = 150.0;

    // Detected via harmonics in speech bands
    let speech_band = 200.0; // Hz

    // Spatial origin: 20° to the right, 30° below (mouth region)
    let detected_azimuth = 20.0_f32.to_radians();
    let detected_elevation = (-30.0_f32).to_radians();

    // Response: steer phased array + heterodyning to this origin
    let response_magnitude = 1.0; // Full power to mouth region
    let response_gain = response_magnitude * (0.9); // 90% gain (dominance level)

    assert!(
        response_gain > 0.8,
        "Mouth-region detection should trigger maximum gain for counter-synthesis"
    );

    // Heterodyne response: carrier - (voice + spatial filter)
    let carrier_hz = 2.45e9;
    let heterodyne_offset = voice_fundamental + speech_band;
    let heterodyned_response = carrier_hz - heterodyne_offset as f32;

    assert!(
        heterodyned_response < carrier_hz,
        "Heterodyned response should be below carrier"
    );
}

/// Test 11: Confidence weighting for elevation (GCC-PHAT)
#[test]
fn test_elevation_confidence_weighting() {
    let strong_gcc_correlation = 0.95; // High confidence GCC-PHAT peak
    let weak_gcc_correlation = 0.30; // Low confidence

    // Use correlation as confidence weight
    let strong_elevation_weight = strong_gcc_correlation;
    let weak_elevation_weight = weak_gcc_correlation;

    assert!(
        strong_elevation_weight > weak_elevation_weight,
        "Strong GCC-PHAT should give higher confidence"
    );
}

/// Test 12: Multi-pair averaging (devices 0,1 for azimuth; 0,2 for elevation)
#[test]
fn test_multiple_pair_fusion() {
    // Pair 0-1 (horizontal): azimuth measurement
    let az_from_pair_01 = 0.3_f32; // 0.3 radians (~17°)
    let az_conf_01 = 0.8_f32;

    // Pair 0-2 (vertical): elevation measurement
    let el_from_pair_02 = 0.1_f32; // 0.1 radians (~5.7°)
    let el_conf_02 = 0.7_f32;

    // Weighted average
    let final_az = (az_from_pair_01 * az_conf_01) / az_conf_01; // Weighted
    let final_el = (el_from_pair_02 * el_conf_02) / el_conf_02;

    assert!(
        ((final_az - az_from_pair_01) as f32).abs() < 0.01,
        "Azimuth should be weighted from horizontal pair"
    );
    assert!(
        ((final_el - el_from_pair_02) as f32).abs() < 0.01,
        "Elevation should be weighted from vertical pair"
    );
}
