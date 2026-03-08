//! Gaussian Splatting Visualization Integration
//! 
//! This module integrates the optimized Gaussian Splatting renderer
//! with the Twister AppState for real-time 3D point cloud visualization.
//! 
//! Usage:
//! ```rust
//! let splat_renderer = GaussianSplatViz::new(gpu_shared, state.clone());
//! 
//! // In your render loop:
//! let particles = generate_particles_from_data(&detection_data);
//! splat_renderer.update_and_render(&particles);
//! ```

use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::gpu_shared::GpuShared;
use crate::state::AppState;
use crate::visualization::gaussian_splatting_optimized::{
    GaussianSplatRendererOptimized,
};

/// Gaussian Splatting visualization integrated with AppState
pub struct GaussianSplatViz {
    renderer: Mutex<GaussianSplatRendererOptimized>,
    state: Arc<AppState>,
    last_frame_time: Mutex<Instant>,
    fps_counter: Mutex<FpsCounter>,
}

/// Simple FPS counter for performance monitoring
struct FpsCounter {
    frame_count: u32,
    elapsed_time: std::time::Duration,
    current_fps: f64,
}

impl FpsCounter {
    fn new() -> Self {
        Self {
            frame_count: 0,
            elapsed_time: std::time::Duration::ZERO,
            current_fps: 0.0,
        }
    }
    
    fn tick(&mut self, delta: std::time::Duration) -> f64 {
        self.frame_count += 1;
        self.elapsed_time += delta;
        
        // Update FPS every second
        if self.elapsed_time >= std::time::Duration::from_secs(1) {
            self.current_fps = self.frame_count as f64 / self.elapsed_time.as_secs_f64();
            self.frame_count = 0;
            self.elapsed_time = std::time::Duration::ZERO;
        }
        
        self.current_fps
    }
}

impl GaussianSplatViz {
    /// Create new Gaussian Splatting visualization
    pub fn new(shared: Arc<GpuShared>, state: Arc<AppState>) -> Self {
        let renderer = GaussianSplatRendererOptimized::new(shared);
        
        Self {
            renderer: Mutex::new(renderer),
            state,
            last_frame_time: Mutex::new(Instant::now()),
            fps_counter: Mutex::new(FpsCounter::new()),
        }
    }
    
    /// Update particles and render
    /// Returns RGBA8 image data (1024×1024 × 4 bytes)
    pub fn update_and_render(&self, particles: &[(f32, f32, f32, f32, f32, f32)]) -> Vec<u8> {
        let mut renderer = self.renderer.lock().expect("Renderer lock poisoned");
        
        // Update particles
        renderer.update_particles(particles);
        
        // Render
        let image = renderer.render();
        
        // Update FPS counter
        let now = Instant::now();
        let delta = now.duration_since(*self.last_frame_time.lock().unwrap());
        *self.last_frame_time.lock().unwrap() = now;
        
        let fps = self.fps_counter.lock().unwrap().tick(delta);
        
        // Log performance if below target
        if fps < 169.0 && fps > 0.0 {
            self.state.log(
                "WARN",
                "GaussianSplat",
                &format!("FPS dropped to {:.0} (target: 169)", fps),
            );
        }
        
        image
    }
    
    /// Set Gaussian sigma (spread factor)
    pub fn set_sigma(&self, sigma: f32) {
        if let Ok(mut renderer) = self.renderer.lock() {
            renderer.set_sigma(sigma);
        }
    }
    
    /// Set intensity scale
    pub fn set_intensity_scale(&self, scale: f32) {
        if let Ok(mut renderer) = self.renderer.lock() {
            renderer.set_intensity_scale(scale);
        }
    }
    
    /// Enable/disable debug mode
    pub fn set_debug_mode(&self, enabled: bool) {
        if let Ok(mut renderer) = self.renderer.lock() {
            renderer.set_debug_mode(enabled);
        }
    }
    
    /// Get current FPS
    pub fn get_fps(&self) -> f64 {
        self.fps_counter.lock().unwrap().current_fps
    }
    
    /// Get average frame time in ms
    pub fn get_avg_frame_time_ms(&self) -> f64 {
        if let Ok(renderer) = self.renderer.lock() {
            renderer.get_avg_frame_time_ms()
        } else {
            0.0
        }
    }
    
    /// Get renderer for direct access (use with caution)
    pub fn with_renderer<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut GaussianSplatRendererOptimized) -> R,
    {
        self.renderer.lock().ok().map(|mut r| f(&mut r))
    }
}

/// Helper function to generate test particles for benchmarking
pub fn generate_test_particles(count: usize) -> Vec<(f32, f32, f32, f32, f32, f32)> {
    let mut particles = Vec::with_capacity(count);
    
    for i in 0..count {
        let t = i as f32 / count.max(1) as f32;
        let azimuth = t * std::f32::consts::PI * 2.0;
        let elevation = (t * std::f32::consts::PI) - std::f32::consts::FRAC_PI_2;
        let frequency = t;
        let intensity = 0.5 + 0.5 * (t * 10.0).sin();
        let timestamp = t;
        let confidence = 0.8 + 0.2 * (t * 20.0).sin();
        
        particles.push((azimuth, elevation, frequency, intensity, timestamp, confidence));
    }
    
    particles
}

/// Generate particles from detection data (azimuth, elevation, frequency, etc.)
pub fn particles_from_detections(
    azimuths: &[f32],
    elevations: &[f32],
    frequencies: &[f32],
    intensities: &[f32],
    confidences: &[f32],
) -> Vec<(f32, f32, f32, f32, f32, f32)> {
    let len = azimuths.len().min(elevations.len())
        .min(frequencies.len())
        .min(intensities.len())
        .min(confidences.len());
    
    let mut particles = Vec::with_capacity(len);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f32();
    
    for i in 0..len {
        particles.push((
            azimuths[i],
            elevations[i],
            frequencies[i],
            intensities[i],
            now,  // Current timestamp
            confidences[i],
        ));
    }
    
    particles
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_test_particles() {
        let particles = generate_test_particles(100);
        assert_eq!(particles.len(), 100);
        
        // Check that all values are in valid ranges
        for (az, el, freq, intensity, ts, conf) in &particles {
            assert!((-std::f32::consts::PI..=std::f32::consts::PI * 2.0).contains(az));
            assert!((-std::f32::consts::FRAC_PI_2..=std::f32::consts::FRAC_PI_2).contains(el));
            assert!((0.0..=1.0).contains(freq));
            assert!((0.0..=1.0).contains(intensity));
            assert!((0.0..=1.0).contains(ts));
            assert!((0.0..=1.0).contains(conf));
        }
    }
    
    #[test]
    fn test_fps_counter() {
        let mut counter = FpsCounter::new();
        
        // Simulate 60 frames at 16.67ms each
        for _ in 0..60 {
            let _fps = counter.tick(std::time::Duration::from_millis(16));
        }
        
        // After 60 frames (~1 second), FPS should be approximately 60
        assert!(counter.current_fps > 50.0);
        assert!(counter.current_fps < 70.0);
    }
}
