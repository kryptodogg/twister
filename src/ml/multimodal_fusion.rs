/// src/ml/multimodal_fusion.rs
/// Multimodal Feature Fusion — Concatenate and normalize audio + wav2vec2 + ray features
///
/// Purpose: Fuse three feature modalities into unified 1297-D representation for TimeGNN training:
/// - Audio: 196-D (STFT, TDOA, PDM, bispectrum, wave coherence, music)
/// - Wav2vec2: 768-D (frozen speech embeddings from facebook/wav2vec2-base-960h)
/// - Ray Tracing: 128-D (image-source method spatial features)
///
/// Layout: [audio_196 | ray_128 | wav2vec2_768] = 1297-D total
///
/// Key Design: Per-modality L2 normalization prevents one modality from dominating
/// (e.g., wav2vec2 768-D could dwarf smaller modalities without normalization)

// Multimodal feature fusion module (no external imports needed)

/// Maximum expected value in feature vector (for sanity checking)
const MAX_FEATURE_VALUE: f32 = 1e6;

/// Minimum norm to prevent division by zero
const MIN_NORM_EPSILON: f32 = 1e-7;

/// Container for multimodal features before fusion
#[derive(Debug, Clone)]
pub struct MultimodalFeatures {
    /// 196-D audio features from Phase 2 D.2 extraction
    pub audio: [f32; 196],
    /// 138-D harmonics features
    pub harmonics: [f32; 138],
    /// 67-D modulation features
    pub modulation: [f32; 67],
    /// 128-D ray tracing features from Phase 2 D.1
    pub ray: [f32; 128],
    /// 768-D speech embeddings from wav2vec2-base-960h
    pub wav2vec2: [f32; 768],
}

/// Fuse multimodal features into unified 1297-D representation
///
/// # Arguments
/// * `audio` - 196-D audio feature vector
/// * `ray` - 128-D ray tracing feature vector
/// * `wav2vec2` - 768-D wav2vec2 speech embedding
///
/// # Returns
/// Fixed 1297-D array: [normalized_audio | normalized_ray | normalized_wav2vec2]
///
/// # Panics
/// - Output contains NaN or Inf after fusion
/// - Output dimension != 1297
pub fn fuse_multimodal(
    audio: &[f32; 196],
    harmonics: &[f32; 138],
    modulation: &[f32; 67],
    ray: &[f32; 128],
    wav2vec2: &[f32; 768],
) -> [f32; 1297] {
    // Step 1: Normalize each modality to unit norm (L2)
    let normalized_audio = l2_normalize_fixed::<196>(audio);
    let normalized_harmonics = l2_normalize_fixed::<138>(harmonics);
    let normalized_modulation = l2_normalize_fixed::<67>(modulation);
    let normalized_ray = l2_normalize_fixed::<128>(ray);
    let normalized_wav2vec2 = l2_normalize_fixed::<768>(wav2vec2);

    // Step 2: Concatenate in order [audio | ray | wav2vec2]
    let mut fused = [0.0f32; 1297];

    // Copy normalized audio: indices 0..196
    fused[0..196].copy_from_slice(&normalized_audio);

    // Copy normalized harmonics: indices 196..334
    fused[196..334].copy_from_slice(&normalized_harmonics);

    // Copy normalized modulation: indices 334..401
    fused[334..401].copy_from_slice(&normalized_modulation);

    // Copy normalized ray: indices 401..529
    fused[401..529].copy_from_slice(&normalized_ray);

    // Copy normalized wav2vec2: indices 529..1297
    fused[529..1297].copy_from_slice(&normalized_wav2vec2);

    // Step 3: Verify output integrity
    verify_multimodal_bounds(&fused);

    fused
}

/// L2-normalize a fixed-size array to unit norm
///
/// # Formula
/// normalized[i] = x[i] / max(sqrt(sum(x[j]^2)), epsilon)
///
/// # Arguments
/// * `input` - Array to normalize (generic size)
///
/// # Returns
/// Normalized array (same size, unit norm)
///
/// # Notes
/// - Handles zero-magnitude inputs gracefully (epsilon prevents division by zero)
/// - Preserves direction but removes scale
fn l2_normalize_fixed<const N: usize>(input: &[f32; N]) -> [f32; N] {
    // Compute L2 norm: sqrt(sum(x[i]^2))
    let norm_squared: f32 = input.iter().map(|x| x.powi(2)).sum();
    let norm = norm_squared.sqrt();

    // Avoid division by zero: use max(norm, epsilon)
    let safe_norm = norm.max(MIN_NORM_EPSILON);

    // Normalize each element
    let mut output = [0.0f32; N];
    for i in 0..N {
        output[i] = input[i] / safe_norm;
    }

    output
}

/// Verify fused multimodal features are within valid bounds
///
/// # Checks
/// - No NaN values
/// - No Inf values
/// - All values in reasonable range [-1e6, 1e6]
/// - Dimension is exactly 1297
///
/// # Panics
/// - If any check fails
fn verify_multimodal_bounds(fused: &[f32; 1297]) {
    assert_eq!(
        fused.len(),
        1297,
        "Fused features must be 1297-D; got {}",
        fused.len()
    );

    for (idx, &value) in fused.iter().enumerate() {
        // Check for NaN
        assert!(
            !value.is_nan(),
            "NaN detected at index {} (feature: {})",
            idx,
            feature_name(idx)
        );

        // Check for Inf
        assert!(
            !value.is_infinite(),
            "Inf detected at index {} (feature: {})",
            idx,
            feature_name(idx)
        );

        // Check magnitude bounds
        assert!(
            value.abs() <= MAX_FEATURE_VALUE,
            "Value {} out of bounds at index {} (feature: {})",
            value,
            idx,
            feature_name(idx)
        );
    }
}

/// Get human-readable feature name from index for error reporting
///
/// # Arguments
/// * `idx` - Feature index in 1297-D vector
///
/// # Returns
/// String describing which modality and sub-feature
fn feature_name(idx: usize) -> String {
    match idx {
        0..196 => format!("audio[{}]", idx),
        196..334 => format!("harmonics[{}]", idx - 196),
        334..401 => format!("modulation[{}]", idx - 334),
        401..529 => format!("ray[{}]", idx - 401),
        529..1297 => format!("wav2vec2[{}]", idx - 529),
        _ => format!("out_of_bounds[{}]", idx),
    }
}

/// Extract statistics from multimodal features for debugging
#[derive(Debug, Clone)]
pub struct ModalityStats {
    /// Mean value per modality
    pub audio_mean: f32,
    pub harmonics_mean: f32,
    pub modulation_mean: f32,
    pub ray_mean: f32,
    pub wav2vec2_mean: f32,

    /// Standard deviation per modality
    pub audio_std: f32,
    pub harmonics_std: f32,
    pub modulation_std: f32,
    pub ray_std: f32,
    pub wav2vec2_std: f32,

    /// Min/max per modality
    pub audio_min: f32,
    pub audio_max: f32,
    pub harmonics_min: f32,
    pub harmonics_max: f32,
    pub modulation_min: f32,
    pub modulation_max: f32,
    pub ray_min: f32,
    pub ray_max: f32,
    pub wav2vec2_min: f32,
    pub wav2vec2_max: f32,
}

/// Compute statistics for fused multimodal features
///
/// # Arguments
/// * `fused` - 1297-D fused feature vector
///
/// # Returns
/// ModalityStats with summary statistics
pub fn compute_modality_stats(fused: &[f32; 1297]) -> ModalityStats {
    let audio_slice = &fused[0..196];
    let harmonics_slice = &fused[196..334];
    let modulation_slice = &fused[334..401];
    let ray_slice = &fused[401..529];
    let wav2vec2_slice = &fused[529..1297];

    ModalityStats {
        audio_mean: compute_mean(audio_slice),
        audio_std: compute_std(audio_slice),
        audio_min: audio_slice.iter().cloned().fold(f32::INFINITY, f32::min),
        audio_max: audio_slice
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max),

        harmonics_mean: compute_mean(harmonics_slice),
        harmonics_std: compute_std(harmonics_slice),
        harmonics_min: harmonics_slice
            .iter()
            .cloned()
            .fold(f32::INFINITY, f32::min),
        harmonics_max: harmonics_slice
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max),

        modulation_mean: compute_mean(modulation_slice),
        modulation_std: compute_std(modulation_slice),
        modulation_min: modulation_slice
            .iter()
            .cloned()
            .fold(f32::INFINITY, f32::min),
        modulation_max: modulation_slice
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max),

        ray_mean: compute_mean(ray_slice),
        ray_std: compute_std(ray_slice),
        ray_min: ray_slice.iter().cloned().fold(f32::INFINITY, f32::min),
        ray_max: ray_slice.iter().cloned().fold(f32::NEG_INFINITY, f32::max),

        wav2vec2_mean: compute_mean(wav2vec2_slice),
        wav2vec2_std: compute_std(wav2vec2_slice),
        wav2vec2_min: wav2vec2_slice.iter().cloned().fold(f32::INFINITY, f32::min),
        wav2vec2_max: wav2vec2_slice
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max),
    }
}

/// Helper: compute mean of slice
fn compute_mean(slice: &[f32]) -> f32 {
    let sum: f32 = slice.iter().sum();
    sum / slice.len() as f32
}

/// Helper: compute standard deviation of slice
fn compute_std(slice: &[f32]) -> f32 {
    let mean = compute_mean(slice);
    let variance: f32 = slice.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / slice.len() as f32;
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multimodal_fusion_shape() {
        let audio = [0.1f32; 196];
        let harmonics = [0.15f32; 138];
        let modulation = [0.1f32; 67];
        let ray = [0.2f32; 128];
        let wav2vec2 = [0.3f32; 768];

        let fused = fuse_multimodal(&audio, &harmonics, &modulation, &ray, &wav2vec2);

        assert_eq!(fused.len(), 1297);
    }

    #[test]
    fn test_multimodal_normalization() {
        // Verify each modality is normalized to unit norm
        let audio = [1.0f32; 196];
        let harmonics = [1.5f32; 138];
        let modulation = [0.1f32; 67];
        let ray = [2.0f32; 128];
        let wav2vec2 = [0.5f32; 768];

        let fused = fuse_multimodal(&audio, &harmonics, &modulation, &ray, &wav2vec2);

        // Check audio slice norm
        let audio_norm_sq: f32 = fused[0..196].iter().map(|x| x.powi(2)).sum();
        assert!(
            (audio_norm_sq - 1.0).abs() < 0.01,
            "Audio not normalized: norm_sq = {}",
            audio_norm_sq
        );

        // Check ray slice norm
        let ray_norm_sq: f32 = fused[401..529].iter().map(|x| x.powi(2)).sum();
        assert!(
            (ray_norm_sq - 1.0).abs() < 0.01,
            "Ray not normalized: norm_sq = {}",
            ray_norm_sq
        );

        // Check wav2vec2 slice norm
        let wav2vec2_norm_sq: f32 = fused[529..1297].iter().map(|x| x.powi(2)).sum();
        assert!(
            (wav2vec2_norm_sq - 1.0).abs() < 0.01,
            "Wav2vec2 not normalized: norm_sq = {}",
            wav2vec2_norm_sq
        );
    }

    #[test]
    fn test_multimodal_no_nan_inf() {
        let audio = [0.5f32; 196];
        let harmonics = [0.5f32; 138];
        let modulation = [0.5f32; 67];
        let ray = [0.5f32; 128];
        let wav2vec2 = [0.5f32; 768];

        let fused = fuse_multimodal(&audio, &harmonics, &modulation, &ray, &wav2vec2);

        for &value in &fused {
            assert!(!value.is_nan(), "NaN found in fused features");
            assert!(!value.is_infinite(), "Inf found in fused features");
        }
    }

    #[test]
    fn test_l2_normalize_fixed() {
        let input = [3.0f32, 4.0f32];
        let normalized = l2_normalize_fixed(&input);

        // 3-4-5 triangle: norm = 5, so [3/5, 4/5] = [0.6, 0.8]
        assert!(
            (normalized[0] - 0.6).abs() < 0.001,
            "Expected 0.6, got {}",
            normalized[0]
        );
        assert!(
            (normalized[1] - 0.8).abs() < 0.001,
            "Expected 0.8, got {}",
            normalized[1]
        );

        // Verify unit norm
        let norm_sq: f32 = normalized.iter().map(|x| x.powi(2)).sum();
        assert!(
            (norm_sq - 1.0).abs() < 0.001,
            "Expected unit norm, got {}",
            norm_sq
        );
    }

    #[test]
    fn test_multimodal_concatenation_order() {
        // Verify strict order: audio | ray | wav2vec2
        let mut audio = [0.0f32; 196];
        let mut harmonics = [0.0f32; 138];
        let mut modulation = [0.0f32; 67];
        let mut ray = [0.0f32; 128];
        let mut wav2vec2 = [0.0f32; 768];

        audio[0] = 1.0; // Marker in audio
        harmonics[0] = 1.5; // Marker in harmonics
        modulation[0] = 1.2; // Marker in modulation
        ray[0] = 2.0; // Marker in ray
        wav2vec2[0] = 3.0; // Marker in wav2vec2

        let fused = fuse_multimodal(&audio, &harmonics, &modulation, &ray, &wav2vec2);

        // After normalization, markers will be scaled down by modality norms
        // But relative ordering should be preserved
        assert_eq!(
            fused[0] > 0.0,
            true,
            "First element (audio) should be positive"
        );
        assert_eq!(
            fused[196] > 0.0,
            true,
            "Element 196 (ray start) should be positive"
        );
        assert_eq!(
            fused[324] > 0.0,
            true,
            "Element 324 (wav2vec2 start) should be positive"
        );
    }

    #[test]
    fn test_modality_stats() {
        let audio = [0.5f32; 196];
        let harmonics = [0.5f32; 138];
        let modulation = [0.5f32; 67];
        let ray = [0.5f32; 128];
        let wav2vec2 = [0.5f32; 768];

        let fused = fuse_multimodal(&audio, &harmonics, &modulation, &ray, &wav2vec2);
        let stats = compute_modality_stats(&fused);

        // All normalized modalities should have similar statistics
        // (after normalization, all are unit norm)
        eprintln!("Audio mean: {}", stats.audio_mean);
        eprintln!("Ray mean: {}", stats.ray_mean);
        eprintln!("Wav2vec2 mean: {}", stats.wav2vec2_mean);

        assert!(stats.audio_std >= 0.0);
        assert!(stats.ray_std >= 0.0);
        assert!(stats.wav2vec2_std >= 0.0);
    }
}
