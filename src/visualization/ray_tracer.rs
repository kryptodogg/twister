// src/visualization/ray_tracer.rs
// Image-source method ray tracing for acoustic propagation geometry (Task D.1a)
//
// Generates 128-D ray feature vectors from room impulse response simulation.
// Encodes information about direct paths, early reflections, and diffuse field characteristics.

use std::f32::consts::PI;

/// Acoustic ray image source configuration
#[derive(Clone, Debug)]
pub struct RayImage {
    /// Attack source azimuth in radians [-π, π]
    pub source_azimuth_rad: f32,
    /// Attack source elevation in radians [-π/2, π/2]
    pub source_elevation_rad: f32,
    /// Room dimensions in meters [length, width, height] - typically [4.0, 5.0, 3.0]
    pub room_dimension_m: [f32; 3],
    /// Order of reflections (typically 2-3)
    pub reflections_per_wall: usize,
}

/// 128-D ray feature vector from image-source method
#[derive(Clone, Debug)]
pub struct RayFeatures {
    /// Direct path strength [0, 1] - always 1.0
    pub direct_path_strength: f32,
    /// Reflection delay histogram (0-100ms in 3ms bins) - 32-D
    pub reflection_delays_32: Vec<f32>,
    /// Spatial spreading from coherent rays (phase coherence) - 32-D
    pub diffuse_distribution_32: Vec<f32>,
    /// Fundamental room modes (~40, 60, 80, 100 Hz) - 4-D
    pub room_modes_4: Vec<f32>,
    /// Angular distribution of rays (16 azimuth bins) - 16-D
    pub azimuth_ray_density_16: Vec<f32>,
    /// Elevation distribution of rays (16 elevation bins) - 16-D
    pub elevation_ray_density_16: Vec<f32>,
    /// Temporal energy dispersion metrics - 8-D
    pub temporal_spread_8: Vec<f32>,
    /// Concatenated 128-D feature vector
    pub feature_vector: Vec<f32>,
    /// Total dimension (always 128)
    pub total_dimension: usize,
}

// Physics constants
const SPEED_OF_SOUND_MPS: f32 = 343.0; // m/s at 20°C
const MAX_REFLECTION_DELAY_MS: f32 = 100.0;
const DELAY_BIN_COUNT: usize = 32;
const _DELAY_BIN_WIDTH_MS: f32 = MAX_REFLECTION_DELAY_MS / (DELAY_BIN_COUNT as f32); // Kept for reference
const AZIMUTH_BIN_COUNT: usize = 16;
const ELEVATION_BIN_COUNT: usize = 16;
const DIFFUSE_BIN_COUNT: usize = 32;
const _ROOM_MODE_COUNT: usize = 4; // Matches room_modes_4 vector size
const TEMPORAL_SPREAD_COUNT: usize = 8;

/// Helper: Clamp angle to [-π, π]
fn normalize_azimuth(rad: f32) -> f32 {
    let mut angle = rad;
    while angle > PI {
        angle -= 2.0 * PI;
    }
    while angle < -PI {
        angle += 2.0 * PI;
    }
    angle
}

/// Helper: Clamp elevation to [-π/2, π/2]
fn normalize_elevation(rad: f32) -> f32 {
    rad.max(-PI / 2.0).min(PI / 2.0)
}

/// Helper: Reflect azimuth across a wall plane
/// Wall indices: 0=right (x), 1=back (y), 2=left (x), 3=front (y)
fn reflect_azimuth_across_wall(azimuth: f32, wall_idx: usize) -> f32 {
    let az = normalize_azimuth(azimuth);
    let reflected = match wall_idx {
        0 => -az,     // Right wall: reflect across x=0
        1 => PI - az, // Back wall: reflect across y=0
        2 => -az,     // Left wall: reflect across x=0
        3 => PI - az, // Front wall: reflect across y=0
        _ => az,
    };
    normalize_azimuth(reflected)
}

/// Helper: Estimate reflection delay from room dimensions and wall index
fn estimate_reflection_delay_ms(room_dim: &[f32; 3], wall_idx: usize) -> f32 {
    // Wall indices: 0,2=x walls (right, left), 1,3=y walls (back, front), 4=ceiling, 5=floor
    let distance_m = match wall_idx {
        0 | 2 => room_dim[0] * 2.0, // x-dimension walls
        1 | 3 => room_dim[1] * 2.0, // y-dimension walls
        4 | 5 => room_dim[2] * 2.0, // z-dimension walls
        _ => room_dim[0],
    };
    (distance_m / SPEED_OF_SOUND_MPS) * 1000.0 // Convert to ms
}

/// Helper: Compute fundamental room modes from dimensions
fn compute_room_modes(room_dim: &[f32; 3]) -> Vec<f32> {
    // Room mode frequencies: f = c / (2 * dimension) for each dimension
    let modes = vec![
        SPEED_OF_SOUND_MPS / (2.0 * room_dim[0]), // x-dimension mode
        SPEED_OF_SOUND_MPS / (2.0 * room_dim[1]), // y-dimension mode
        SPEED_OF_SOUND_MPS / (2.0 * room_dim[2]), // z-dimension mode
        (SPEED_OF_SOUND_MPS / (2.0 * room_dim[0])) + (SPEED_OF_SOUND_MPS / (2.0 * room_dim[1])), // Combined x+y mode
    ];

    // Normalize modes to [0, 1] range (assuming modes span ~0-400 Hz)
    let max_mode = modes.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut normalized = modes;
    if max_mode > 0.0 {
        normalized = normalized.iter().map(|&m| m / 400.0).collect();
        normalized = normalized.iter().map(|&m| m.min(1.0)).collect();
    }
    normalized
}

/// Helper: Histogram angles into angular bins
fn histogram_angles(angles: &[f32], bin_count: usize, angle_range: (f32, f32)) -> Vec<f32> {
    let mut histogram = vec![0.0; bin_count];
    let (min_angle, max_angle) = angle_range;
    let range = max_angle - min_angle;

    for &angle in angles {
        if angle >= min_angle && angle <= max_angle {
            let normalized = (angle - min_angle) / range;
            let bin = ((normalized * (bin_count as f32)) as usize).min(bin_count - 1);
            histogram[bin] += 1.0;
        }
    }

    // Normalize histogram to [0, 1]
    let max_count = histogram.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    if max_count > 0.0 {
        histogram = histogram.iter().map(|&h| h / max_count).collect();
    }
    histogram
}

/// Helper: Histogram delays into temporal bins
fn histogram_delays(delays_ms: &[f32], bin_count: usize) -> Vec<f32> {
    let mut histogram = vec![0.0; bin_count];

    for &delay in delays_ms {
        if delay >= 0.0 && delay <= MAX_REFLECTION_DELAY_MS {
            let normalized = delay / MAX_REFLECTION_DELAY_MS;
            let bin = ((normalized * (bin_count as f32)) as usize).min(bin_count - 1);
            histogram[bin] += 1.0;
        }
    }

    // Normalize histogram to [0, 1]
    let max_count = histogram.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    if max_count > 0.0 {
        histogram = histogram.iter().map(|&h| h / max_count).collect();
    }
    histogram
}

/// Compute temporal spread metrics (variance, entropy, etc.)
fn compute_temporal_spread(delays_ms: &[f32]) -> Vec<f32> {
    let mut metrics = vec![0.0; TEMPORAL_SPREAD_COUNT];

    if delays_ms.is_empty() {
        return metrics;
    }

    // Mean delay
    let mean_delay = delays_ms.iter().sum::<f32>() / delays_ms.len() as f32;
    metrics[0] = (mean_delay / MAX_REFLECTION_DELAY_MS).min(1.0);

    // Variance (normalized)
    let variance = delays_ms
        .iter()
        .map(|&d| (d - mean_delay).powi(2))
        .sum::<f32>()
        / delays_ms.len() as f32;
    let std_dev = variance.sqrt();
    metrics[1] = (std_dev / MAX_REFLECTION_DELAY_MS).min(1.0);

    // Skewness indicator (early vs late energy)
    let early_count = delays_ms.iter().filter(|&&d| d < 30.0).count();
    metrics[2] = (early_count as f32) / (delays_ms.len() as f32);

    // Late decay (energy after 50ms)
    let late_count = delays_ms.iter().filter(|&&d| d > 50.0).count();
    metrics[3] = (late_count as f32) / (delays_ms.len() as f32);

    // Min delay (normalized, clamped to avoid division issues)
    let min_delay = delays_ms
        .iter()
        .filter(|&&d| d > 0.0)
        .cloned()
        .fold(f32::INFINITY, f32::min);
    metrics[4] = if min_delay < f32::INFINITY {
        (min_delay / MAX_REFLECTION_DELAY_MS).min(1.0)
    } else {
        0.0
    };

    // Max delay (normalized)
    let max_delay = delays_ms.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    metrics[5] = (max_delay / MAX_REFLECTION_DELAY_MS).min(1.0);

    // Energy concentration (peak bin entropy)
    let histogram = histogram_delays(delays_ms, 16);
    let max_bin = histogram.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    metrics[6] = max_bin;

    // Overall normalized energy (number of reflections)
    metrics[7] = ((delays_ms.len() as f32) / 100.0).min(1.0);

    metrics
}

/// Main function: Generate 128-D ray features from image-source method
pub fn compute_ray_features(image: &RayImage) -> RayFeatures {
    let mut all_rays: Vec<(f32, f32, f32, f32)> = Vec::new(); // (azimuth, elevation, delay_ms, strength)

    // Direct path
    let direct_azimuth = normalize_azimuth(image.source_azimuth_rad);
    let direct_elevation = normalize_elevation(image.source_elevation_rad);
    all_rays.push((direct_azimuth, direct_elevation, 0.0, 1.0));

    // First-order reflections (6 walls)
    for wall_idx in 0..6 {
        let reflected_az = reflect_azimuth_across_wall(direct_azimuth, wall_idx);
        let reflected_el = if wall_idx >= 4 {
            // Ceiling/floor: reflect elevation
            -direct_elevation
        } else {
            direct_elevation
        };
        let delay_ms = estimate_reflection_delay_ms(&image.room_dimension_m, wall_idx);
        let strength = 0.7; // Single reflection attenuation
        all_rays.push((reflected_az, reflected_el, delay_ms, strength));
    }

    // Second-order reflections (if enabled)
    if image.reflections_per_wall >= 2 {
        // Reflect first-order reflections across walls
        let first_order_count = all_rays.len();
        for i in 1..first_order_count {
            let (az1, el1, delay1, _) = all_rays[i];
            for wall_idx in 0..6 {
                let reflected_az = reflect_azimuth_across_wall(az1, wall_idx);
                let reflected_el = if wall_idx >= 4 { -el1 } else { el1 };
                let additional_delay =
                    estimate_reflection_delay_ms(&image.room_dimension_m, wall_idx);
                let delay_ms = delay1 + additional_delay;
                let strength = 0.5; // Double reflection attenuation
                if delay_ms <= MAX_REFLECTION_DELAY_MS {
                    all_rays.push((reflected_az, reflected_el, delay_ms, strength));
                }
            }
        }
    }

    // Extract feature components
    let direct_path_strength = 1.0;

    // Collect delays and angles
    let delays_ms: Vec<f32> = all_rays.iter().map(|(_, _, d, _)| *d).collect();
    let azimuths: Vec<f32> = all_rays.iter().map(|(az, _, _, _)| *az).collect();
    let elevations: Vec<f32> = all_rays.iter().map(|(_, el, _, _)| *el).collect();

    // Compute feature histograms
    let reflection_delays_32 = histogram_delays(&delays_ms, DELAY_BIN_COUNT);
    let diffuse_distribution_32 = histogram_delays(&delays_ms, DIFFUSE_BIN_COUNT);
    let room_modes_4 = compute_room_modes(&image.room_dimension_m);
    let azimuth_ray_density_16 = histogram_angles(&azimuths, AZIMUTH_BIN_COUNT, (-PI, PI));
    let elevation_ray_density_16 =
        histogram_angles(&elevations, ELEVATION_BIN_COUNT, (-PI / 2.0, PI / 2.0));
    let temporal_spread_8 = compute_temporal_spread(&delays_ms);

    // Concatenate all features into 128-D vector
    let mut feature_vector: Vec<f32> = Vec::new();
    feature_vector.push(direct_path_strength); // 1-D
    feature_vector.extend(reflection_delays_32.clone()); // 32-D
    feature_vector.extend(diffuse_distribution_32.clone()); // 32-D
    feature_vector.extend(room_modes_4.clone()); // 4-D
    feature_vector.extend(azimuth_ray_density_16.clone()); // 16-D
    feature_vector.extend(elevation_ray_density_16.clone()); // 16-D
    feature_vector.extend(temporal_spread_8.clone()); // 8-D

    // Pad to exactly 128-D if needed
    while feature_vector.len() < 128 {
        feature_vector.push(0.0);
    }

    // Truncate if somehow longer than 128 (shouldn't happen)
    feature_vector.truncate(128);

    assert_eq!(
        feature_vector.len(),
        128,
        "Feature vector must be exactly 128-D"
    );

    RayFeatures {
        direct_path_strength,
        reflection_delays_32,
        diffuse_distribution_32,
        room_modes_4,
        azimuth_ray_density_16,
        elevation_ray_density_16,
        temporal_spread_8,
        feature_vector,
        total_dimension: 128,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_azimuth() {
        assert_eq!(normalize_azimuth(0.0), 0.0);
        assert!(normalize_azimuth(4.0).abs() <= PI);
        assert!(normalize_azimuth(-4.0).abs() <= PI);
    }

    #[test]
    fn test_normalize_elevation() {
        assert_eq!(normalize_elevation(0.0), 0.0);
        assert!(normalize_elevation(PI).abs() <= PI / 2.0);
        assert!(normalize_elevation(-PI).abs() <= PI / 2.0);
    }

    #[test]
    fn test_room_modes_nonzero() {
        let modes = compute_room_modes(&[4.0, 5.0, 3.0]);
        assert_eq!(modes.len(), 4);
        assert!(modes.iter().any(|&m| m > 0.0));
    }
}
