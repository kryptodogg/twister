// tests/ray_tracer_integration.rs
// Integration tests for RayTracer image-source method (Task D.1a)

use twister::visualization::ray_tracer::{RayImage, compute_ray_features};

#[test]
fn test_ray_features_dimension_128() {
    let ray_image = RayImage {
        source_azimuth_rad: 0.5,              // ~29°
        source_elevation_rad: -0.5,           // ~-29° (below horizontal)
        room_dimension_m: [4.0, 5.0, 3.0],   // 4m × 5m × 3m room
        reflections_per_wall: 2,
    };

    let features = compute_ray_features(&ray_image);

    assert_eq!(features.feature_vector.len(), 128);
    assert_eq!(features.total_dimension, 128);
}

#[test]
fn test_direct_path_at_max_strength() {
    let ray_image = RayImage {
        source_azimuth_rad: 0.0,
        source_elevation_rad: 0.0,
        room_dimension_m: [4.0, 5.0, 3.0],
        reflections_per_wall: 1,
    };

    let features = compute_ray_features(&ray_image);

    // Direct path strength should always be 1.0 (maximum)
    assert_eq!(features.direct_path_strength, 1.0);
}

#[test]
fn test_reflection_delays_histogram() {
    let ray_image = RayImage {
        source_azimuth_rad: 0.0,
        source_elevation_rad: 0.0,
        room_dimension_m: [4.0, 5.0, 3.0],
        reflections_per_wall: 2,
    };

    let features = compute_ray_features(&ray_image);

    // Should have 32 delay bins
    assert_eq!(features.reflection_delays_32.len(), 32);

    // All delays should be in [0, 1] (normalized)
    for &delay in &features.reflection_delays_32 {
        assert!(delay >= 0.0 && delay <= 1.0, "Delay out of range: {}", delay);
    }

    // Delays should be generally decreasing (more energy at shorter delays)
    // at least on average
    let first_half_sum: f32 = features.reflection_delays_32[0..16].iter().sum();
    let second_half_sum: f32 = features.reflection_delays_32[16..32].iter().sum();
    assert!(first_half_sum >= second_half_sum, "Delays should decrease over time");
}

#[test]
fn test_room_modes_from_dimensions() {
    let ray_image = RayImage {
        source_azimuth_rad: 0.0,
        source_elevation_rad: 0.0,
        room_dimension_m: [4.0, 5.0, 3.0],   // 4m × 5m × 3m
        reflections_per_wall: 1,
    };

    let features = compute_ray_features(&ray_image);

    // Should have 4 room mode features
    assert_eq!(features.room_modes_4.len(), 4);

    // Room modes should be normalized [0, 1]
    for &mode in &features.room_modes_4 {
        assert!(mode >= 0.0 && mode <= 1.0, "Room mode out of range: {}", mode);
    }

    // Room modes should have meaningful values (not all zero)
    let sum: f32 = features.room_modes_4.iter().sum();
    assert!(sum > 0.0, "Room modes should not all be zero");
}

#[test]
fn test_azimuth_elevation_distribution() {
    let ray_image = RayImage {
        source_azimuth_rad: 0.5,
        source_elevation_rad: -0.5,
        room_dimension_m: [4.0, 5.0, 3.0],
        reflections_per_wall: 2,
    };

    let features = compute_ray_features(&ray_image);

    // Azimuth density: 16 bins
    assert_eq!(features.azimuth_ray_density_16.len(), 16);

    // Elevation density: 16 bins
    assert_eq!(features.elevation_ray_density_16.len(), 16);

    // All values should be normalized [0, 1]
    for &az in &features.azimuth_ray_density_16 {
        assert!(az >= 0.0 && az <= 1.0, "Azimuth density out of range: {}", az);
    }

    for &el in &features.elevation_ray_density_16 {
        assert!(el >= 0.0 && el <= 1.0, "Elevation density out of range: {}", el);
    }

    // Should have at least some energy in azimuth/elevation
    let az_sum: f32 = features.azimuth_ray_density_16.iter().sum();
    let el_sum: f32 = features.elevation_ray_density_16.iter().sum();
    assert!(az_sum > 0.0, "Azimuth density should not be all zero");
    assert!(el_sum > 0.0, "Elevation density should not be all zero");
}

#[test]
fn test_second_order_reflections() {
    let ray_image_1st = RayImage {
        source_azimuth_rad: 0.0,
        source_elevation_rad: 0.0,
        room_dimension_m: [4.0, 5.0, 3.0],
        reflections_per_wall: 1,
    };

    let ray_image_2nd = RayImage {
        source_azimuth_rad: 0.0,
        source_elevation_rad: 0.0,
        room_dimension_m: [4.0, 5.0, 3.0],
        reflections_per_wall: 2,
    };

    let features_1st = compute_ray_features(&ray_image_1st);
    let features_2nd = compute_ray_features(&ray_image_2nd);

    // Both should have same dimension
    assert_eq!(features_1st.feature_vector.len(), 128);
    assert_eq!(features_2nd.feature_vector.len(), 128);

    // Second-order reflections should have more rays (longer delays contribute)
    // So the total energy spread should be different
    let total_1st: f32 = features_1st.reflection_delays_32.iter().sum();
    let total_2nd: f32 = features_2nd.reflection_delays_32.iter().sum();

    // Second-order should have at least as much or different distribution
    assert!(total_2nd >= total_1st * 0.9,
        "Second-order should preserve delay distribution: {} vs {}",
        total_2nd, total_1st);
}

#[test]
fn test_diffuse_distribution_histogram() {
    let ray_image = RayImage {
        source_azimuth_rad: 0.0,
        source_elevation_rad: 0.0,
        room_dimension_m: [4.0, 5.0, 3.0],
        reflections_per_wall: 2,
    };

    let features = compute_ray_features(&ray_image);

    // Should have 32 diffuse bins
    assert_eq!(features.diffuse_distribution_32.len(), 32);

    // All values should be normalized [0, 1]
    for &diff in &features.diffuse_distribution_32 {
        assert!(diff >= 0.0 && diff <= 1.0, "Diffuse distribution out of range: {}", diff);
    }
}

#[test]
fn test_temporal_spread_metrics() {
    let ray_image = RayImage {
        source_azimuth_rad: 0.0,
        source_elevation_rad: 0.0,
        room_dimension_m: [4.0, 5.0, 3.0],
        reflections_per_wall: 2,
    };

    let features = compute_ray_features(&ray_image);

    // Should have 8 temporal spread metrics
    assert_eq!(features.temporal_spread_8.len(), 8);

    // All values should be normalized [0, 1]
    for &spread in &features.temporal_spread_8 {
        assert!(spread >= 0.0 && spread <= 1.0, "Temporal spread out of range: {}", spread);
    }
}

#[test]
fn test_edge_case_small_room() {
    // Very small room (1m × 1m × 1m)
    let ray_image = RayImage {
        source_azimuth_rad: 0.0,
        source_elevation_rad: 0.0,
        room_dimension_m: [1.0, 1.0, 1.0],
        reflections_per_wall: 1,
    };

    let features = compute_ray_features(&ray_image);

    assert_eq!(features.feature_vector.len(), 128);
    assert_eq!(features.total_dimension, 128);
}

#[test]
fn test_edge_case_large_room() {
    // Large room (20m × 25m × 10m)
    let ray_image = RayImage {
        source_azimuth_rad: 0.0,
        source_elevation_rad: 0.0,
        room_dimension_m: [20.0, 25.0, 10.0],
        reflections_per_wall: 1,
    };

    let features = compute_ray_features(&ray_image);

    assert_eq!(features.feature_vector.len(), 128);
    assert_eq!(features.total_dimension, 128);
}

#[test]
fn test_edge_case_grazing_angle() {
    // Source nearly parallel to floor (elevation close to -π/2)
    let ray_image = RayImage {
        source_azimuth_rad: 0.0,
        source_elevation_rad: -1.5,
        room_dimension_m: [4.0, 5.0, 3.0],
        reflections_per_wall: 2,
    };

    let features = compute_ray_features(&ray_image);

    assert_eq!(features.feature_vector.len(), 128);
    assert_eq!(features.total_dimension, 128);
}

#[test]
fn test_consistency_same_input() {
    // Same input should produce same output
    let ray_image = RayImage {
        source_azimuth_rad: 0.75,
        source_elevation_rad: 0.25,
        room_dimension_m: [5.0, 6.0, 3.5],
        reflections_per_wall: 2,
    };

    let features1 = compute_ray_features(&ray_image);
    let features2 = compute_ray_features(&ray_image);

    assert_eq!(features1.feature_vector.len(), features2.feature_vector.len());

    for (v1, v2) in features1.feature_vector.iter().zip(features2.feature_vector.iter()) {
        assert_eq!(v1, v2, "Inconsistent feature computation");
    }
}
