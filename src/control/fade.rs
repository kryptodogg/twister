//! Fade controller for smooth mode transitions

use crate::forensics::event::ControlMode;
use std::time::{Duration, Instant};

/// Fade state
#[derive(Debug, Clone)]
pub struct FadeState {
    /// Current fade position (0.0 - 1.0)
    pub position: f32,
    /// Target position
    pub target: f32,
    /// Fade direction
    pub direction: FadeDirection,
    /// Time remaining (ms)
    pub time_remaining_ms: f32,
    /// Total fade duration (ms)
    pub total_duration_ms: f32,
}

/// Fade direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadeDirection {
    /// Fading in (0 → 1)
    In,
    /// Fading out (1 → 0)
    Out,
    /// Cross-fading between modes
    Crossfade,
    /// No fade (instant)
    None,
}

/// Fade controller
pub struct FadeController {
    /// Current state
    state: FadeState,
    /// Fade start time
    start_time: Option<Instant>,
    /// Fade duration (ms)
    duration_ms: f32,
    /// Current mode
    current_mode: ControlMode,
    /// Target mode
    target_mode: Option<ControlMode>,
    /// ANC weights (for interpolation)
    anc_weights: Vec<f32>,
    /// Target ANC weights
    target_anc_weights: Vec<f32>,
}

impl FadeController {
    /// Create a new fade controller
    pub fn new(duration_ms: f32) -> Self {
        Self {
            state: FadeState {
                position: 1.0,
                target: 1.0,
                direction: FadeDirection::None,
                time_remaining_ms: 0.0,
                total_duration_ms: duration_ms,
            },
            start_time: None,
            duration_ms,
            current_mode: ControlMode::Silence,
            target_mode: None,
            anc_weights: Vec::new(),
            target_anc_weights: Vec::new(),
        }
    }

    /// Start a fade to a new mode
    pub fn start_fade(&mut self, from_mode: ControlMode, to_mode: ControlMode) {
        self.current_mode = from_mode;
        self.target_mode = Some(to_mode);
        self.start_time = Some(Instant::now());

        self.state = FadeState {
            position: 0.0,
            target: 1.0,
            direction: FadeDirection::Crossfade,
            time_remaining_ms: self.duration_ms,
            total_duration_ms: self.duration_ms,
        };
    }

    /// Start fade in (enable processing)
    pub fn start_fade_in(&mut self) {
        self.start_time = Some(Instant::now());
        self.state = FadeState {
            position: 0.0,
            target: 1.0,
            direction: FadeDirection::In,
            time_remaining_ms: self.duration_ms,
            total_duration_ms: self.duration_ms,
        };
    }

    /// Start fade out (disable processing)
    pub fn start_fade_out(&mut self) {
        self.start_time = Some(Instant::now());
        self.state = FadeState {
            position: 1.0,
            target: 0.0,
            direction: FadeDirection::Out,
            time_remaining_ms: self.duration_ms,
            total_duration_ms: self.duration_ms,
        };
    }

    /// Update fade state
    pub fn update(&mut self) -> f32 {
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed().as_secs_f32() * 1000.0;
            let progress = (elapsed / self.duration_ms).min(1.0);

            // Apply easing (smoothstep)
            let eased = self.smoothstep(progress);

            self.state.position = match self.state.direction {
                FadeDirection::In => eased,
                FadeDirection::Out => 1.0 - eased,
                FadeDirection::Crossfade => eased,
                FadeDirection::None => 1.0,
            };

            self.state.time_remaining_ms = self.duration_ms * (1.0 - progress);

            // Interpolate ANC weights
            if !self.anc_weights.is_empty() && !self.target_anc_weights.is_empty() {
                self.interpolate_weights(eased);
            }

            if progress >= 1.0 {
                // Fade complete
                if let Some(target_mode) = self.target_mode.take() {
                    self.current_mode = target_mode;
                }
                self.start_time = None;
                self.state.direction = FadeDirection::None;
            }

            self.state.position
        } else {
            self.state.position
        }
    }

    /// Smoothstep easing function
    fn smoothstep(&self, t: f32) -> f32 {
        t * t * (3.0 - 2.0 * t)
    }

    /// Smootherstep easing (higher quality)
    fn smootherstep(&self, t: f32) -> f32 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }

    /// Set ANC weights for interpolation
    pub fn set_anc_weights(&mut self, weights: Vec<f32>) {
        self.anc_weights = weights.clone();
        self.target_anc_weights = weights;
    }

    /// Set target ANC weights
    pub fn set_target_anc_weights(&mut self, weights: Vec<f32>) {
        self.target_anc_weights = weights;
    }

    /// Interpolate ANC weights
    fn interpolate_weights(&mut self, t: f32) {
        if self.anc_weights.len() != self.target_anc_weights.len() {
            return;
        }

        for (current, target) in self.anc_weights.iter_mut().zip(self.target_anc_weights.iter()) {
            *current = *current * (1.0 - t) + *target * t;
        }
    }

    /// Get current ANC weights
    pub fn anc_weights(&self) -> &[f32] {
        &self.anc_weights
    }

    /// Get current fade position
    pub fn position(&self) -> f32 {
        self.state.position
    }

    /// Check if fade is in progress
    pub fn is_fading(&self) -> bool {
        self.start_time.is_some()
    }

    /// Get current mode
    pub fn current_mode(&self) -> ControlMode {
        self.current_mode
    }

    /// Get target mode (if fading)
    pub fn target_mode(&self) -> Option<ControlMode> {
        self.target_mode
    }

    /// Get fade state
    pub fn state(&self) -> &FadeState {
        &self.state
    }

    /// Set fade duration
    pub fn set_duration(&mut self, duration_ms: f32) {
        self.duration_ms = duration_ms;
        self.state.total_duration_ms = duration_ms;
    }

    /// Instant mode change (no fade)
    pub fn instant_change(&mut self, mode: ControlMode) {
        self.current_mode = mode;
        self.target_mode = None;
        self.start_time = None;
        self.state.position = 1.0;
        self.state.direction = FadeDirection::None;
    }

    /// Get gain multiplier for current fade position
    pub fn gain(&self) -> f32 {
        // Apply gain curve for smooth audio transitions
        match self.state.direction {
            FadeDirection::In | FadeDirection::Crossfade => self.state.position,
            FadeDirection::Out => 1.0 - self.state.position,
            FadeDirection::None => 1.0,
        }
    }
}

impl Default for FadeController {
    fn default() -> Self {
        Self::new(100.0) // 100ms default fade
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fade_controller_creation() {
        let controller = FadeController::new(100.0);
        assert_eq!(controller.position(), 1.0);
        assert!(!controller.is_fading());
    }

    #[test]
    fn test_fade_in() {
        let mut controller = FadeController::new(100.0);
        controller.start_fade_in();

        assert!(controller.is_fading());
        assert_eq!(controller.position(), 0.0);

        // Let fade progress
        std::thread::sleep(std::time::Duration::from_millis(50));
        controller.update();

        assert!(controller.position() > 0.0);
        assert!(controller.position() < 1.0);
    }

    #[test]
    fn test_fade_complete() {
        let mut controller = FadeController::new(50.0);
        controller.start_fade_in();

        // Wait for fade to complete
        std::thread::sleep(std::time::Duration::from_millis(60));
        controller.update();

        assert!(!controller.is_fading());
        assert!((controller.position() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_mode_transition() {
        let mut controller = FadeController::new(100.0);
        controller.start_fade(ControlMode::Silence, ControlMode::Anc);

        assert_eq!(controller.current_mode(), ControlMode::Silence);
        assert_eq!(controller.target_mode(), Some(ControlMode::Anc));
        assert!(controller.is_fading());

        // Wait for fade to complete
        std::thread::sleep(std::time::Duration::from_millis(110));
        controller.update();

        assert_eq!(controller.current_mode(), ControlMode::Anc);
        assert_eq!(controller.target_mode(), None);
    }

    #[test]
    fn test_gain_curve() {
        let mut controller = FadeController::new(100.0);
        
        controller.start_fade_in();
        assert_eq!(controller.gain(), 0.0);

        controller.state.position = 0.5;
        assert!((controller.gain() - 0.5).abs() < 0.01);

        controller.state.position = 1.0;
        assert_eq!(controller.gain(), 1.0);
    }

    #[test]
    fn test_smoothstep() {
        let controller = FadeController::new(100.0);
        
        assert!((controller.smoothstep(0.0) - 0.0).abs() < 0.001);
        assert!((controller.smoothstep(1.0) - 1.0).abs() < 0.001);
        assert!(controller.smoothstep(0.5) > 0.5); // S-curve
    }
}
