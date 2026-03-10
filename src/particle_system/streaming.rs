use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

use crate::particle_system::ParticleGPU;

pub struct ParticleStreamLoader {
    // We could store full historical buffer or mock it dynamically
    pub particles: Arc<Mutex<Vec<ParticleGPU>>>,
    pub is_loading: Arc<Mutex<bool>>,
}

impl ParticleStreamLoader {
    pub fn new() -> Self {
        Self {
            particles: Arc::new(Mutex::new(Vec::new())),
            is_loading: Arc::new(Mutex::new(false)),
        }
    }

    /// Mock a particle stream given a specific time window in milliseconds (unix epoch)
    pub async fn load_window(&self, start_ms: u64, end_ms: u64, max_particles: usize) {
        let is_loading = self.is_loading.clone();
        let particles = self.particles.clone();

        let mut loading_lock = is_loading.lock().await;
        if *loading_lock {
            return; // Already loading
        }
        *loading_lock = true;
        drop(loading_lock); // release so we don't block

        // Spawn a background task to simulate reading/loading chunks from disk/network
        tokio::spawn(async move {
            let mut new_particles = Vec::with_capacity(max_particles);

            // Generate some fake parameters based on time to show variation
            // Say time affects the "attack pattern"
            let duration = end_ms.saturating_sub(start_ms) as f32;
            let time_factor = (start_ms as f64 % 1000000.0) as f32 / 100000.0;

            for _ in 0..max_particles {
                let radius = 5.0 + (rand::random::<f32>() * 95.0);
                let angle1 = rand::random::<f32>() * std::f32::consts::TAU;
                let angle2 = rand::random::<f32>() * std::f32::consts::TAU;

                let mut pos = [
                    radius * angle1.cos() * angle2.sin(),
                    radius * angle1.sin() * angle2.sin(),
                    radius * angle2.cos(),
                ];

                // Fake forensics pattern
                let hardness = 0.1 + (rand::random::<f32>() * 0.9);
                let roughness = rand::random::<f32>() * 0.5;
                let intensity = 0.5 + (rand::random::<f32>() * 1.0);

                // Reddish hue for mock attack patterns
                let r = 0.5 + (rand::random::<f32>() * 0.5);
                let g = rand::random::<f32>() * 0.3;
                let b = rand::random::<f32>() * 0.3;

                new_particles.push(ParticleGPU {
                    position: pos,
                    color: [r, g, b, 1.0],
                    intensity,
                    hardness,
                    roughness,
                    wetness: 0.0,
                });
            }

            let mut lock = particles.lock().await;
            *lock = new_particles;

            let mut l = is_loading.lock().await;
            *l = false;
        });
    }

    pub async fn get_particles(&self) -> Vec<ParticleGPU> {
        let lock = self.particles.lock().await;
        lock.clone() // Simple copy for mock to upload to GPU
    }
}
