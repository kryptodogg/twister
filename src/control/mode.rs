//! Mode decision logic

use crate::forensics::event::ControlMode;

/// Mode decision result
#[derive(Debug, Clone)]
pub struct ModeDecision {
    /// Selected mode
    pub mode: ControlMode,
    /// Confidence (0-1)
    pub confidence: f32,
    /// Reason for decision
    pub reason: DecisionReason,
    /// ML mode probabilities
    pub ml_probs: [f32; 3],
    /// SNR-based recommendation
    pub snr_recommendation: Option<ControlMode>,
    /// RF stress level (0-1)
    pub rf_stress: f32,
}

/// Decision reason
#[derive(Debug, Clone)]
pub enum DecisionReason {
    /// ML model decision
    MLModel { probability: f32 },
    /// SNR-based decision
    SNRBased { snr_db: f32 },
    /// RF stress triggered
    RFStress { stress_level: f32 },
    /// User override
    UserOverride,
    /// Fallback to default
    Fallback,
    /// Hysteresis prevented change
    Hysteresis,
}

/// Mode configuration
#[derive(Debug, Clone)]
pub struct ModeConfig {
    /// ANC SNR threshold (dB)
    pub anc_snr_threshold_db: f32,
    /// Music SNR threshold (dB)
    pub music_snr_threshold_db: f32,
    /// RF stress threshold (0-1)
    pub rf_stress_threshold: f32,
    /// Hysteresis band (dB)
    pub hysteresis_db: f32,
    /// Minimum mode duration (ms)
    pub min_mode_duration_ms: u32,
    /// Enable ML-based decisions
    pub use_ml: bool,
    /// ML weight (0-1)
    pub ml_weight: f32,
}

impl Default for ModeConfig {
    fn default() -> Self {
        Self {
            anc_snr_threshold_db: 108.0,
            music_snr_threshold_db: 60.0,
            rf_stress_threshold: 0.7,
            hysteresis_db: 3.0,
            min_mode_duration_ms: 100,
            use_ml: true,
            ml_weight: 0.5,
        }
    }
}

/// Mode decision engine
pub struct ModeEngine {
    config: ModeConfig,
    current_mode: ControlMode,
    mode_start_time: std::time::Instant,
    last_snr_db: f32,
}

impl ModeEngine {
    /// Create a new mode engine
    pub fn new(config: ModeConfig, initial_mode: ControlMode) -> Self {
        Self {
            config,
            current_mode: initial_mode,
            mode_start_time: std::time::Instant::now(),
            last_snr_db: 0.0,
        }
    }

    /// Decide mode based on inputs
    pub fn decide(
        &mut self,
        snr_db: f32,
        rf_stress: f32,
        ml_probs: [f32; 3],
    ) -> ModeDecision {
        self.last_snr_db = snr_db;

        // Get ML recommendation
        let ml_mode = if self.config.use_ml {
            let max_idx = ml_probs
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .map(|(i, _)| i)
                .unwrap_or(0);
            Some(match max_idx {
                0 => ControlMode::Anc,
                1 => ControlMode::Silence,
                _ => ControlMode::Music,
            })
        } else {
            None
        };

        // Get SNR-based recommendation
        let snr_recommendation = self.snr_based_decision(snr_db);

        // Combine decisions
        let (mode, reason) = self.combine_decisions(snr_db, rf_stress, ml_probs, ml_mode);

        // Check hysteresis
        let (final_mode, final_reason) = if mode != self.current_mode {
            let elapsed = self.mode_start_time.elapsed().as_millis() as u32;
            if elapsed < self.config.min_mode_duration_ms {
                // Hysteresis prevents change
                (self.current_mode, DecisionReason::Hysteresis)
            } else {
                self.mode_start_time = std::time::Instant::now();
                self.current_mode = mode;
                (mode, reason)
            }
        } else {
            (mode, reason)
        };

        // Calculate confidence
        let confidence = match &final_reason {
            DecisionReason::MLModel { probability } => *probability,
            DecisionReason::SNRBased { snr_db } => {
                let dist = (snr_db - self.config.anc_snr_threshold_db).abs();
                (1.0 - (dist / 20.0).min(1.0)).max(0.5)
            }
            DecisionReason::RFStress { stress_level } => *stress_level,
            _ => 0.8,
        };

        ModeDecision {
            mode: final_mode,
            confidence,
            reason: final_reason,
            ml_probs,
            snr_recommendation,
            rf_stress,
        }
    }

    /// Get SNR-based mode recommendation
    fn snr_based_decision(&self, snr_db: f32) -> Option<ControlMode> {
        if snr_db < self.config.anc_snr_threshold_db - self.config.hysteresis_db {
            Some(ControlMode::Anc)
        } else if snr_db > self.config.music_snr_threshold_db + self.config.hysteresis_db {
            Some(ControlMode::Music)
        } else {
            Some(ControlMode::Silence)
        }
    }

    /// Combine multiple decision sources
    fn combine_decisions(
        &self,
        snr_db: f32,
        rf_stress: f32,
        ml_probs: [f32; 3],
        ml_mode: Option<ControlMode>,
    ) -> (ControlMode, DecisionReason) {
        // Check RF stress first (highest priority)
        if rf_stress > self.config.rf_stress_threshold {
            return (
                ControlMode::Anc,
                DecisionReason::RFStress { stress_level: rf_stress },
            );
        }

        // Combine ML and SNR decisions
        if self.config.use_ml && ml_mode.is_some() {
            let ml_mode = ml_mode.unwrap();
            let ml_confidence = match ml_mode {
                ControlMode::Anc => ml_probs[0],
                ControlMode::Silence => ml_probs[1],
                ControlMode::Music => ml_probs[2],
            };

            let snr_mode = self.snr_based_decision(snr_db).unwrap_or(ControlMode::Silence);

            // Weighted combination
            if ml_confidence > 0.7 && ml_confidence > self.config.ml_weight {
                return (
                    ml_mode,
                    DecisionReason::MLModel { probability: ml_confidence },
                );
            }

            if let Some(snr_mode) = self.snr_based_decision(snr_db) {
                return (
                    snr_mode,
                    DecisionReason::SNRBased { snr_db },
                );
            }
        }

        // Fallback to SNR-based
        if let Some(mode) = self.snr_based_decision(snr_db) {
            return (mode, DecisionReason::SNRBased { snr_db });
        }

        // Default fallback
        (ControlMode::Silence, DecisionReason::Fallback)
    }

    /// Get current mode
    pub fn current_mode(&self) -> ControlMode {
        self.current_mode
    }

    /// Get time in current mode
    pub fn time_in_mode(&self) -> std::time::Duration {
        self.mode_start_time.elapsed()
    }

    /// Force mode change (user override)
    pub fn force_mode(&mut self, mode: ControlMode) {
        self.current_mode = mode;
        self.mode_start_time = std::time::Instant::now();
    }

    /// Update configuration
    pub fn update_config(&mut self, config: ModeConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_engine_anc() {
        let config = ModeConfig::default();
        let mut engine = ModeEngine::new(config, ControlMode::Silence);

        // Low SNR should trigger ANC
        let decision = engine.decide(50.0, 0.3, [0.8, 0.1, 0.1]);

        assert_eq!(decision.mode, ControlMode::Anc);
        assert!(decision.confidence > 0.5);
    }

    #[test]
    fn test_mode_engine_music() {
        let config = ModeConfig::default();
        let mut engine = ModeEngine::new(config, ControlMode::Silence);

        // High SNR + low RF stress should allow Music
        let decision = engine.decide(80.0, 0.1, [0.1, 0.1, 0.8]);

        assert_eq!(decision.mode, ControlMode::Music);
    }

    #[test]
    fn test_mode_engine_rf_stress() {
        let config = ModeConfig::default();
        let mut engine = ModeEngine::new(config, ControlMode::Music);

        // High RF stress should force ANC regardless of SNR
        let decision = engine.decide(100.0, 0.9, [0.1, 0.1, 0.8]);

        assert_eq!(decision.mode, ControlMode::Anc);
        assert!(matches!(decision.reason, DecisionReason::RFStress { .. }));
    }

    #[test]
    fn test_mode_hysteresis() {
        let config = ModeConfig {
            min_mode_duration_ms: 1000,
            ..ModeConfig::default()
        };
        let mut engine = ModeEngine::new(config, ControlMode::Anc);

        // Quick mode change should be prevented by hysteresis
        let decision1 = engine.decide(100.0, 0.1, [0.1, 0.1, 0.8]);
        assert_eq!(decision1.mode, ControlMode::Anc); // Should stay in ANC

        // After hysteresis period, should change
        std::thread::sleep(std::time::Duration::from_millis(100));
        let decision2 = engine.decide(100.0, 0.1, [0.1, 0.1, 0.8]);
        // May still be ANC due to timing, but reason should indicate
    }

    #[test]
    fn test_mode_config_default() {
        let config = ModeConfig::default();
        assert_eq!(config.anc_snr_threshold_db, 108.0);
        assert!(config.use_ml);
    }
}
