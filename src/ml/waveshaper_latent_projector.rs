// src/ml/waveshaper_latent_projector.rs
// Unified Mamba Latent → Waveshaper Parameter Projection
//
// Maps 128D latent embeddings from UnifiedFieldMamba into Drive, Foldback, and Asymmetry
// parameters for real-time harmonic synthesis defense against RF anomalies.
//
// Projection Strategy:
// - Drive (Latent[0..31]): Controls amplitude into distortion (sigmoid → [0, 1])
// - Foldback (Latent[32..63]): Controls Super-Nyquist alias spread (tanh → [0, 1])
// - Asymmetry (Latent[64..95]): Controls even-order harmonic generation (tanh → [-1, 1])

#[derive(Clone, Debug)]
pub struct WaveshaperLatentProjector {
    /// Drive projection: Mean of latent dims [0..31]
    pub drive_projection: [f32; 32],

    /// Foldback projection: Mean of latent dims [32..63]
    pub foldback_projection: [f32; 32],

    /// Asymmetry projection: Mean of latent dims [64..95]
    pub asymmetry_projection: [f32; 32],
}

impl Default for WaveshaperLatentProjector {
    fn default() -> Self {
        // Initialize with identity-like projection
        Self {
            drive_projection: [1.0 / 32.0; 32],
            foldback_projection: [1.0 / 32.0; 32],
            asymmetry_projection: [1.0 / 32.0; 32],
        }
    }
}

/// Waveshaper parameter output from latent projection
#[derive(Clone, Debug)]
pub struct WaveshaperParams {
    /// Drive: Amplitude scaling before distortion (0.0 = bypass, 1.0 = max)
    pub drive: f32,

    /// Foldback: Controls Super-Nyquist harmonic spread (0.0 = minimal, 1.0 = max)
    pub foldback: f32,

    /// Asymmetry: Harmonic shaping DC offset (-1.0 = even only, 1.0 = odd emphasis)
    pub asymmetry: f32,

    /// Confidence that this prediction is valid (0.0-1.0)
    pub confidence: f32,
}

impl Default for WaveshaperParams {
    fn default() -> Self {
        Self {
            drive: 0.0,
            foldback: 0.0,
            asymmetry: 0.0,
            confidence: 0.0,
        }
    }
}

impl WaveshaperLatentProjector {
    /// Create new projector with default identity mappings
    pub fn new() -> Self {
        Self::default()
    }

    /// Project 128D latent embedding into waveshaper parameters
    ///
    /// # Arguments
    /// * `latent_embedding` - 128-dimensional latent vector from UnifiedFieldMamba
    /// * `anomaly_score` - Reconstruction MSE score (0.0 = normal, 1.0 = max anomaly)
    ///
    /// # Returns
    /// WaveshaperParams with Drive, Foldback, Asymmetry, and confidence
    pub fn project(&self, latent_embedding: &[f32], anomaly_score: f32) -> WaveshaperParams {
        // Validate input dimension
        if latent_embedding.len() < 96 {
            eprintln!(
                "⚠️  Latent embedding has only {} dims (expected ≥96)",
                latent_embedding.len()
            );
            return WaveshaperParams::default();
        }

        // Extract sub-embeddings
        let drive_latent = &latent_embedding[0..32];
        let foldback_latent = &latent_embedding[32..64];
        let asymmetry_latent = &latent_embedding[64..96];

        // Project Drive: [0..31] → weighted sum → sigmoid → [0, 1]
        let drive_raw = self.project_subspace(drive_latent, &self.drive_projection);
        let drive = sigmoid(drive_raw);

        // Project Foldback: [32..63] → weighted sum → sigmoid → [0, 1]
        let foldback_raw = self.project_subspace(foldback_latent, &self.foldback_projection);
        let foldback = sigmoid(foldback_raw);

        // Project Asymmetry: [64..95] → weighted sum → tanh → [-1, 1]
        let asymmetry_raw = self.project_subspace(asymmetry_latent, &self.asymmetry_projection);
        let asymmetry = asymmetry_raw.tanh();

        // Confidence: Product of anomaly magnitude (triggers defense) + projection magnitude
        let drive_mag = drive_raw.abs();
        let foldback_mag = foldback_raw.abs();
        let asymmetry_mag = asymmetry_raw.abs();
        let latent_energy = (drive_mag + foldback_mag + asymmetry_mag) / 3.0;

        // High anomaly + high latent energy = high confidence
        let confidence = (anomaly_score.min(1.0)) * latent_energy.min(1.0);

        WaveshaperParams {
            drive,
            foldback,
            asymmetry,
            confidence,
        }
    }

    /// Project a 32-dim subspace of the latent embedding using weighted sum
    ///
    /// # Arguments
    /// * `subspace` - 32-dimensional slice of latent embedding
    /// * `weights` - 32 weights for linear projection
    ///
    /// # Returns
    /// Scalar projection value (unbounded, typically [-3, 3])
    fn project_subspace(&self, subspace: &[f32], weights: &[f32]) -> f32 {
        subspace
            .iter()
            .zip(weights.iter())
            .map(|(latent_val, weight)| latent_val * weight)
            .sum()
    }

    /// Update the projection weights using supervision (for future training)
    ///
    /// This is a stub for future learning-based adjustment of the projection matrix.
    #[allow(dead_code)]
    pub fn update_weights(&mut self, _gradient_drive: &[f32], _gradient_foldback: &[f32], _gradient_asymmetry: &[f32], _learning_rate: f32) {
        // TODO: Implement gradient descent on projection weights
        // For now, we use fixed learned projections from training
    }
}

/// Sigmoid activation function: maps unbounded values to [0, 1]
///
/// Formula: σ(x) = 1 / (1 + e^(-x))
#[inline]
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_projection_normal_anomaly() {
        let projector = WaveshaperLatentProjector::new();

        // Create a latent embedding with normal distribution
        let mut latent = vec![0.1f32; 128];
        latent[5] = 0.5; // Some activity in drive subspace

        let params = projector.project(&latent, 0.1); // Low anomaly

        // Drive should be moderate (sigmoid of small value)
        assert!(params.drive > 0.4 && params.drive < 0.6);
        // Confidence should be low (low anomaly + moderate energy)
        assert!(params.confidence < 0.5);
    }

    #[test]
    fn test_projection_high_anomaly() {
        let projector = WaveshaperLatentProjector::new();

        // Create a latent embedding with high energy in all subspaces
        let mut latent = vec![0.8f32; 128];
        latent[0] = 1.5; // High drive signal
        latent[40] = 1.2; // High foldback signal

        let params = projector.project(&latent, 0.95); // Very high anomaly

        // Drive and Foldback should be high
        assert!(params.drive > 0.7);
        assert!(params.foldback > 0.6);
        // Confidence should be very high
        assert!(params.confidence > 0.8);
    }

    #[test]
    fn test_asymmetry_range() {
        let projector = WaveshaperLatentProjector::new();

        let latent_pos = vec![2.0f32; 128];
        let params_pos = projector.project(&latent_pos, 0.5);

        let latent_neg = vec![-2.0f32; 128];
        let params_neg = projector.project(&latent_neg, 0.5);

        // Asymmetry should span [-1, 1] range
        assert!(params_pos.asymmetry > 0.0); // Positive latent → positive asymmetry
        assert!(params_neg.asymmetry < 0.0); // Negative latent → negative asymmetry
    }
}
