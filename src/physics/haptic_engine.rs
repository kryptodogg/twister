use crate::ml::field_particle::FieldParticle;
use std::sync::atomic::{AtomicF32, Ordering};

/// Haptic Engine: Translates the Synesthesia Hologram into physical force fields.
/// Target hardware: DualSense (PS5) Actuators.
pub struct HapticForceField {
    pub feedback_intensity: AtomicF32,
    pub spatial_center: [AtomicF32; 3],
}

impl HapticForceField {
    pub fn new() -> Self {
        Self {
            feedback_intensity: AtomicF32::new(0.0),
            spatial_center: [AtomicF32::new(0.0), AtomicF32::new(0.0), AtomicF32::new(0.0)],
        }
    }

    /// Calculates the localized actuator force for a set of particles.
    /// Implementation based on Chronos Slate spatial mapping.
    pub fn compute_localized_force(&self, particles: &[FieldParticle]) -> [f32; 2] {
        if particles.is_empty() {
            return [0.0, 0.0];
        }

        let mut left_vibration = 0.0;
        let mut right_vibration = 0.0;

        for p in particles {
            // Mapping X position to L/R balance
            let balance = (p.position[0] + 1.0) / 2.0;
            let force = p.intensity * p.confidence[3]; // Confidence weight by RF Density

            left_vibration += force * (1.0 - balance);
            right_vibration += force * balance;
        }

        [left_vibration.clamp(0.0, 1.0), right_vibration.clamp(0.0, 1.0)]
    }

    /// Updates the spatial haptic center based on the current field focus.
    pub fn update_focus(&self, x: f32, y: f32, z: f32) {
        self.spatial_center[0].store(x, Ordering::Relaxed);
        self.spatial_center[1].store(y, Ordering::Relaxed);
        self.spatial_center[2].store(z, Ordering::Relaxed);
    }
}
