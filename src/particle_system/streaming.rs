use std::sync::Arc;
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
            use rand::distr::{Distribution, Uniform};
            let mut new_particles = Vec::with_capacity(max_particles);

            // Create rng inside the async block (ThreadRng is not Send across await boundaries)
            let mut rng = rand::rng();

            // Say time affects the "attack pattern"
            let _duration = end_ms.saturating_sub(start_ms) as f32;
            let time_factor = (start_ms as f64 % 1000000.0) as f32 / 100000.0;

            let radius_dist = Uniform::new(5.0, 100.0).unwrap();
            let angle_dist = Uniform::new(0.0, std::f32::consts::TAU).unwrap();
            let hardness_dist = Uniform::new(0.1, 1.0).unwrap();
            let roughness_dist = Uniform::new(0.0, 0.5).unwrap();
            let intensity_dist = Uniform::new(0.5, 1.5).unwrap();
            let color_r_dist = Uniform::new(0.5, 1.0).unwrap();
            let color_g_dist = Uniform::new(0.0, 0.3).unwrap();
            let color_b_dist = Uniform::new(0.0, 0.3).unwrap();

            for _i in 0..max_particles {
                let radius = radius_dist.sample(&mut rng);
                let angle1 = angle_dist.sample(&mut rng);
                let angle2 = angle_dist.sample(&mut rng);

                let px = radius * angle1.cos() * angle2.sin();
                let py = radius * angle1.sin() * angle2.sin() + (time_factor * 10.0);
                let pz = radius * angle2.cos();

                // Fake forensics pattern
                let hardness = hardness_dist.sample(&mut rng);
                let roughness = roughness_dist.sample(&mut rng);
                let intensity = intensity_dist.sample(&mut rng);

                // Reddish hue for mock attack patterns
                let r = color_r_dist.sample(&mut rng);
                let g = color_g_dist.sample(&mut rng);
                let b = color_b_dist.sample(&mut rng);

                new_particles.push(ParticleGPU {
                    position: [px, py, pz],
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
