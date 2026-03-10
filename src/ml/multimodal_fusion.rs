/// Minimum norm to prevent division by zero
const MIN_NORM_EPSILON: f32 = 1e-9;

pub struct MultimodalFeature {
    pub audio_features: [f32; 196], // From Track C (spectral extraction)
    pub ray_features: [f32; 128],   // From Track D (TDOA/beamforming)
    pub wav2vec2_embedding: [f32; 768], // From wav2vec2 (J.1)
    pub fused: [f32; 1092],         // Concatenated + normalized
}

impl MultimodalFeature {
    /// Concatenate [196D audio + 128D ray + 768D wav2vec2] → 1092-D
    /// Per-modality L2 normalization prevents one modality from drowning others
    pub fn fuse(audio: &[f32; 196], ray: &[f32; 128], wav2vec2: &[f32; 768]) -> Self {
        let audio_norm = Self::l2_normalize(audio);
        let ray_norm = Self::l2_normalize(ray);
        let wav2vec2_norm = Self::l2_normalize(wav2vec2);

        let mut fused = [0.0; 1092];
        fused[0..196].copy_from_slice(&audio_norm);
        fused[196..324].copy_from_slice(&ray_norm);
        fused[324..1092].copy_from_slice(&wav2vec2_norm);

        Self {
            audio_features: *audio,
            ray_features: *ray,
            wav2vec2_embedding: *wav2vec2,
            fused,
        }
    }

    fn l2_normalize(v: &[f32]) -> Vec<f32> {
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        v.iter().map(|x| x / norm.max(MIN_NORM_EPSILON)).collect()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModalityStats {
    pub audio_mean: f32,
    pub audio_std: f32,
    pub ray_mean: f32,
    pub ray_std: f32,
    pub wav2vec2_mean: f32,
    pub wav2vec2_std: f32,
}

pub fn compute_modality_stats(fused: &[f32]) -> ModalityStats {
    let audio = &fused[0..196];
    let ray = &fused[196..324];
    let wav2vec2 = &fused[324..1092];

    let calc = |data: &[f32]| {
        let mean = data.iter().sum::<f32>() / data.len() as f32;
        let var = data.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / data.len() as f32;
        (mean, var.sqrt())
    };

    let (audio_mean, audio_std) = calc(audio);
    let (ray_mean, ray_std) = calc(ray);
    let (wav2vec2_mean, wav2vec2_std) = calc(wav2vec2);

    ModalityStats {
        audio_mean,
        audio_std,
        ray_mean,
        ray_std,
        wav2vec2_mean,
        wav2vec2_std,
    }
}

pub fn fuse_multimodal(
    audio: &[f32; 196],
    _unused1: &[f32; 138],
    _unused2: &[f32; 67],
    ray: &[f32; 128],
    wav2vec2: &[f32; 768],
) -> Vec<f32> {
    MultimodalFeature::fuse(audio, ray, wav2vec2).fused.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concatenation() {
        let audio = [0.1; 196];
        let ray = [0.2; 128];
        let wav2vec2 = [0.3; 768];

        let feature = MultimodalFeature::fuse(&audio, &ray, &wav2vec2);
        assert_eq!(feature.fused.len(), 1092);
    }

    #[test]
    fn test_normalization() {
        let audio = [1.0; 196];
        let ray = [2.0; 128];
        let wav2vec2 = [3.0; 768];

        let feature = MultimodalFeature::fuse(&audio, &ray, &wav2vec2);

        // Verify L2 norm of each section is ~1.0
        let audio_norm_sq: f32 = feature.fused[0..196].iter().map(|x| x * x).sum();
        let ray_norm_sq: f32 = feature.fused[196..324].iter().map(|x| x * x).sum();
        let wav2vec2_norm_sq: f32 = feature.fused[324..1092].iter().map(|x| x * x).sum();

        assert!((audio_norm_sq - 1.0).abs() < 0.01);
        assert!((ray_norm_sq - 1.0).abs() < 0.01);
        assert!((wav2vec2_norm_sq - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_no_nan_inf() {
        let audio = [0.0; 196]; // Edge case: zero vector
        let ray = [0.0; 128];
        let wav2vec2 = [0.0; 768];

        let feature = MultimodalFeature::fuse(&audio, &ray, &wav2vec2);

        for val in feature.fused.iter() {
            assert!(!val.is_nan());
            assert!(!val.is_infinite());
        }
    }

    #[test]
    fn test_modality_balance() {
        let audio = [1000.0; 196]; // Extremely large audio
        let ray = [0.1; 128]; // Extremely small ray
        let wav2vec2 = [1.0; 768]; // Normal wav2vec2

        let feature = MultimodalFeature::fuse(&audio, &ray, &wav2vec2);

        // Even though audio was 10000x larger, its norm in the fused vector should be 1.0
        let audio_norm_sq: f32 = feature.fused[0..196].iter().map(|x| x * x).sum();
        let ray_norm_sq: f32 = feature.fused[196..324].iter().map(|x| x * x).sum();

        assert!((audio_norm_sq - 1.0).abs() < 0.01);
        assert!((ray_norm_sq - 1.0).abs() < 0.01);
    }
}
