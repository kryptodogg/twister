//! Control policy - main coordinator for mode decisions

use crate::forensics::event::ControlMode;
use crate::control::snr::{SNREstimator, SNREstimate};
use crate::control::mode::{ModeEngine, ModeConfig, ModeDecision};
use crate::control::fade::FadeController;

/// Control policy configuration
#[derive(Debug, Clone)]
pub struct PolicyConfig {
    /// SNR estimator configuration
    pub snr_noise_floor_db: f32,
    /// Mode configuration
    pub mode: ModeConfig,
    /// Fade duration (ms)
    pub fade_duration_ms: f32,
    /// Control update rate (ms)
    pub update_rate_ms: u32,
    /// Enable stress reduction profile
    pub stress_reduction_profile: bool,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            snr_noise_floor_db: -60.0,
            mode: ModeConfig::default(),
            fade_duration_ms: 100.0,
            update_rate_ms: 10,
            stress_reduction_profile: false,
        }
    }
}

/// Control output
#[derive(Debug, Clone)]
pub struct ControlOutput {
    /// Current mode
    pub mode: ControlMode,
    /// Target SNR (dB)
    pub target_snr_db: f32,
    /// Fade position (0-1)
    pub fade_position: f32,
    /// ANC weights (if applicable)
    pub anc_weights: Option<Vec<f32>>,
    /// Music mix level (0-1)
    pub music_level: f32,
    /// Decision info
    pub decision: Option<ModeDecision>,
    /// SNR estimate
    pub snr_estimate: Option<SNREstimate>,
}

/// Control policy coordinator
pub struct ControlPolicy {
    /// SNR estimator
    snr_estimator: SNREstimator,
    /// Mode engine
    mode_engine: ModeEngine,
    /// Fade controller
    fade_controller: FadeController,
    /// Configuration
    config: PolicyConfig,
    /// Last control output
    last_output: Option<ControlOutput>,
    /// Update counter
    update_count: u64,
}

impl ControlPolicy {
    /// Create a new control policy
    pub fn new(config: PolicyConfig) -> Self {
        let snr_estimator = SNREstimator::new(config.snr_noise_floor_db, 0.95);
        let mode_engine = ModeEngine::new(config.mode.clone(), ControlMode::Silence);
        let fade_controller = FadeController::new(config.fade_duration_ms);

        Self {
            snr_estimator,
            mode_engine,
            fade_controller,
            config,
            last_output: None,
            update_count: 0,
        }
    }

    /// Update control policy with new measurements
    pub fn update(
        &mut self,
        audio_samples: &[f32],
        rf_stress: f32,
        ml_probs: [f32; 3],
    ) -> ControlOutput {
        self.update_count += 1;

        // Estimate SNR
        let snr_estimate = self.snr_estimator.estimate(audio_samples);

        // Make mode decision
        let decision = self.mode_engine.decide(
            snr_estimate.snr_db,
            rf_stress,
            ml_probs,
        );

        // Check if mode changed
        let current_mode = self.fade_controller.current_mode();
        if decision.mode != current_mode {
            self.fade_controller.start_fade(current_mode, decision.mode);
        }

        // Update fade
        let fade_position = self.fade_controller.update();

        // Generate control output based on mode
        let (target_snr_db, anc_weights, music_level) = match decision.mode {
            ControlMode::Anc => {
                let weights = self.generate_anc_weights(snr_estimate.snr_db);
                (self.config.mode.anc_snr_threshold_db, Some(weights), 0.0)
            }
            ControlMode::Silence => {
                (0.0, None, 0.0)
            }
            ControlMode::Music => {
                let level = if self.config.stress_reduction_profile {
                    self.compute_stress_reduction_level(&snr_estimate, rf_stress)
                } else {
                    0.7
                };
                (self.config.mode.music_snr_threshold_db, None, level)
            }
        };

        let output = ControlOutput {
            mode: decision.mode,
            target_snr_db,
            fade_position,
            anc_weights,
            music_level,
            decision: Some(decision),
            snr_estimate: Some(snr_estimate),
        };

        self.last_output = Some(output.clone());
        output
    }

    /// Generate ANC weights based on SNR
    fn generate_anc_weights(&self, snr_db: f32) -> Vec<f32> {
        // In production, would use adaptive filter coefficients
        // For now, generate simple weights based on SNR
        
        let num_taps = 64;
        let gain = (snr_db / 108.0).clamp(0.0, 1.0);
        
        // Simple low-pass filter coefficients
        let mut weights = Vec::with_capacity(num_taps);
        for i in 0..num_taps {
            let t = i as f32 / num_taps as f32;
            let weight = (1.0 - t) * gain;
            weights.push(weight);
        }
        
        weights
    }

    /// Compute stress reduction music level
    fn compute_stress_reduction_level(&self, snr: &SNREstimate, rf_stress: f32) -> f32 {
        // Higher level when RF stress is high or SNR is moderate
        let snr_factor = (snr.snr_db / 80.0).clamp(0.0, 1.0);
        let rf_factor = rf_stress;
        
        // Blend factors
        0.3 + 0.4 * rf_factor + 0.3 * (1.0 - snr_factor)
    }

    /// Get current mode
    pub fn current_mode(&self) -> ControlMode {
        self.fade_controller.current_mode()
    }

    /// Get last output
    pub fn last_output(&self) -> Option<&ControlOutput> {
        self.last_output.as_ref()
    }

    /// Force mode change
    pub fn force_mode(&mut self, mode: ControlMode) {
        self.mode_engine.force_mode(mode);
        self.fade_controller.instant_change(mode);
    }

    /// Update noise floor estimate
    pub fn update_noise_floor(&mut self, samples: &[f32]) {
        self.snr_estimator.update_noise_floor(samples);
    }

    /// Get SNR estimator
    pub fn snr_estimator(&self) -> &SNREstimator {
        &self.snr_estimator
    }

    /// Get mode engine
    pub fn mode_engine(&self) -> &ModeEngine {
        &self.mode_engine
    }

    /// Get fade controller
    pub fn fade_controller(&self) -> &FadeController {
        &self.fade_controller
    }

    /// Get update count
    pub fn update_count(&self) -> u64 {
        self.update_count
    }

    /// Enable stress reduction profile
    pub fn set_stress_reduction(&mut self, enabled: bool) {
        self.config.stress_reduction_profile = enabled;
    }

    /// Update fade duration
    pub fn set_fade_duration(&mut self, duration_ms: f32) {
        self.config.fade_duration_ms = duration_ms;
        self.fade_controller.set_duration(duration_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_creation() {
        let config = PolicyConfig::default();
        let policy = ControlPolicy::new(config);

        assert_eq!(policy.current_mode(), ControlMode::Silence);
        assert_eq!(policy.update_count(), 0);
    }

    #[test]
    fn test_policy_update_anc() {
        let config = PolicyConfig::default();
        let mut policy = ControlPolicy::new(config);

        // Low SNR audio (noise)
        let audio: Vec<f32> = (0..10000).map(|i| (i as f32 * 0.001).sin() * 0.1).collect();
        
        let output = policy.update(&audio, 0.3, [0.8, 0.1, 0.1]);

        assert_eq!(output.mode, ControlMode::Anc);
        assert!(output.anc_weights.is_some());
        assert_eq!(output.music_level, 0.0);
    }

    #[test]
    fn test_policy_update_music() {
        let config = PolicyConfig::default();
        let mut policy = ControlPolicy::new(config);

        // High SNR audio (quiet)
        let audio: Vec<f32> = vec![0.0; 10000];
        
        let output = policy.update(&audio, 0.1, [0.1, 0.1, 0.8]);

        assert_eq!(output.mode, ControlMode::Music);
        assert!(output.anc_weights.is_none());
        assert!(output.music_level > 0.0);
    }

    #[test]
    fn test_policy_rf_stress_override() {
        let config = PolicyConfig::default();
        let mut policy = ControlPolicy::new(config);

        // High RF stress should force ANC
        let audio: Vec<f32> = vec![0.0; 10000];
        
        let output = policy.update(&audio, 0.9, [0.1, 0.1, 0.8]);

        assert_eq!(output.mode, ControlMode::Anc);
    }

    #[test]
    fn test_force_mode() {
        let config = PolicyConfig::default();
        let mut policy = ControlPolicy::new(config);

        policy.force_mode(ControlMode::Anc);
        assert_eq!(policy.current_mode(), ControlMode::Anc);
    }

    #[test]
    fn test_stress_reduction_profile() {
        let mut config = PolicyConfig::default();
        config.stress_reduction_profile = true;
        let mut policy = ControlPolicy::new(config);

        let audio: Vec<f32> = vec![0.0; 10000];
        let output = policy.update(&audio, 0.8, [0.1, 0.1, 0.8]);

        assert_eq!(output.mode, ControlMode::Music);
        assert!(output.music_level > 0.5); // Higher due to stress reduction
    }
}
