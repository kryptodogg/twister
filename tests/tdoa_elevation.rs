use std::f32::consts::FRAC_PI_2;
use twister::spatial::elevation_estimator::ElevationEstimator; // Check path

#[test]
fn test_elevation_computation_synthetic() {
    let mut estimator = ElevationEstimator::new();
    let amps = [1.0, 1.0, 1.0, 1.0];
    let (elev, _) = estimator.estimate_elevation(&amps, 0.0);
    assert!(elev.abs() < 0.1); // ≈ 0
}

#[test]
fn test_horizontal_plane_detection() {
    let mut estimator = ElevationEstimator::new();
    let amps = [2.0, 2.0, 2.0, 2.0];
    let (elev, _) = estimator.estimate_elevation(&amps, 0.0);
    assert!(elev.abs() < 0.1); // ≈ 0
}

#[test]
fn test_elevated_source_detection() {
    let mut estimator = ElevationEstimator::new();
    let amps = [4.0, 1.0, 1.0, 2.0]; // E_top / E_bottom = 8.0 > 1
    let (elev, _) = estimator.estimate_elevation(&amps, 0.0);
    assert!(elev > 0.0);
}

#[test]
fn test_below_plane_source_detection() {
    let mut estimator = ElevationEstimator::new();
    let amps = [1.0, 4.0, 2.0, 1.0]; // E_top / E_bottom = 1/8 < 1
    let (elev, _) = estimator.estimate_elevation(&amps, 0.0);
    assert!(elev < 0.0);
}

#[test]
fn test_confidence_bounds() {
    let mut estimator = ElevationEstimator::new();
    let amps = [0.01, 0.01, 0.01, 0.01]; // Low energy = low confidence
    let (_, conf) = estimator.estimate_elevation(&amps, 0.0);
    assert!(conf >= 0.0 && conf <= 1.0);
    assert!(conf < 0.5);

    let mut estimator_high = ElevationEstimator::new();
    let amps_high = [10.0, 10.0, 10.0, 10.0]; // High energy = high confidence
    let (_, conf_high) = estimator_high.estimate_elevation(&amps_high, 0.0);
    assert_eq!(conf_high, 1.0);
}

#[test]
fn test_smoothing_stability() {
    let mut estimator = ElevationEstimator::new();
    let amps1 = [4.0, 1.0, 1.0, 2.0]; // Elevated
    let (elev1, _) = estimator.estimate_elevation(&amps1, 0.0);

    // Smooth should move closer to new value but not jump fully
    let amps2 = [4.0, 1.0, 1.0, 2.0]; // Same elevation
    let (elev2, _) = estimator.estimate_elevation(&amps2, 0.0);
    assert!(elev2 > elev1); // It approaches the true value asymptotically
}
