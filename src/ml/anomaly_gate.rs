use crate::ml::field_particle::FieldParticle;
use std::sync::atomic::{AtomicU32, Ordering};

pub struct AnomalyGate {
    pub global_score: AtomicU32,
}

impl AnomalyGate {
    pub fn new() -> Self {
        Self {
            global_score: AtomicU32::new(0f32.to_bits()),
        }
    }

    /// Update global anomaly score from Coral TPU or Pico 2
    pub fn update_score(&self, score: f32) {
        self.global_score.store(score.to_bits(), Ordering::Relaxed);
    }

    pub fn get_score(&self) -> f32 {
        f32::from_bits(self.global_score.load(Ordering::Relaxed))
    }

    /// Process a particle and attach the current global anomaly score
    pub fn process_particle(&self, particle: &mut FieldParticle) {
        particle.anomaly_score = self.get_score();
    }
}

pub struct AnomalyGateConfig;
pub fn evaluate_anomaly_gate(_mags: &[f32], _config: &AnomalyGateConfig) -> f32 { 0.0 }
