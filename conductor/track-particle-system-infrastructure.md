# Track: Particle System Infrastructure

**Ownership**: Graphics/Physics Engineer (exclusive ownership of src/particle_system/)
**Duration**: 1.5 days (2-3 hours parallel work, not sequential)
**Integration Point**: Feeds Track D.4 (Gaussian splatting), Track I.5 (physics simulation), Track VI (visualization)
**Critical Dependency**: None (can be implemented independently)

---

## Strategic Overview

**Problem**: Three different tracks need particle system primitives simultaneously:
- **Track D.4** (Temporal Rewind UI): Needs Gaussian splatting (render 3D point clouds as particles)
- **Track I.5** (Physics Particle System): Needs physics engine (bounce, friction, drag)
- **Track VI** (Aether Visualization): Needs particle rendering (final visualization)

**Solution**: Extract particle system into shared infrastructure track owned by one graphics engineer.

**Why this works**: All three tracks import from `src/particle_system/` module; no conflicts, clear interfaces.

---

## PS.1: Particle Primitive Definitions (30 minutes)

### File: `src/particle_system/particle.rs` (150 lines)

```rust
/// Core particle data structure
#[derive(Clone, Copy, Debug)]
pub struct Particle {
    // Position & velocity
    pub position: Vec3,           // (x, y, z) in world space
    pub velocity: Vec3,           // (vx, vy, vz) velocity vector
    pub acceleration: Vec3,       // (ax, ay, az) acceleration (gravity, etc.)

    // Visual properties
    pub color: Vec4,              // (r, g, b, a) RGBA
    pub scale: f32,               // Size multiplier [0.1, 10.0]
    pub intensity: f32,           // Brightness [0, 1]

    // Physical properties
    pub mass: f32,                // Mass (for physics)
    pub lifetime_s: f32,          // Total lifetime in seconds
    pub age_s: f32,               // Current age [0, lifetime_s]

    // Material properties (RF-BSDF)
    pub hardness: f32,            // [0, 1] bounce/reflection
    pub roughness: f32,           // [0, 1] surface texture
    pub wetness: f32,             // [0, 1] absorption/drag

    // Emission (for light-emitting particles)
    pub emits_light: bool,
    pub light_radius: f32,
}

impl Particle {
    /// Check if particle is alive
    pub fn is_alive(&self) -> bool {
        self.age_s < self.lifetime_s
    }

    /// Get alpha value based on age (fade out near end of life)
    pub fn get_alpha(&self) -> f32 {
        let fade_start = self.lifetime_s * 0.8;  // Start fading at 80% age
        if self.age_s > fade_start {
            let fade_duration = self.lifetime_s * 0.2;
            let fade_progress = (self.age_s - fade_start) / fade_duration;
            (1.0 - fade_progress).max(0.0)
        } else {
            1.0
        }
    }

    /// Update particle state (age)
    pub fn update(&mut self, delta_time_s: f32) {
        self.age_s += delta_time_s;
    }
}

/// Particle emitter configuration
pub struct ParticleEmitterConfig {
    pub emission_rate: f32,       // Particles per second
    pub particle_lifetime_s: f32, // How long each particle lives
    pub initial_velocity: Vec3,   // Base velocity
    pub velocity_variance: Vec3,  // Random variance in each direction
    pub initial_scale: f32,
    pub color: Vec4,
    pub material: ParticleMaterial,
}

#[derive(Clone, Copy)]
pub struct ParticleMaterial {
    pub hardness: f32,
    pub roughness: f32,
    pub wetness: f32,
}
```

---

## PS.2: ParticleEmitter (Spawning) (30 minutes)

### File: `src/particle_system/emitter.rs` (180 lines)

```rust
pub struct ParticleEmitter {
    config: ParticleEmitterConfig,
    particles: Vec<Particle>,
    emission_accumulator: f32,      // Fractional particle counter
    max_particles: usize,            // Cap to prevent runaway
}

impl ParticleEmitter {
    pub fn new(config: ParticleEmitterConfig, max_particles: usize) -> Self {
        Self {
            config,
            particles: Vec::with_capacity(max_particles),
            emission_accumulator: 0.0,
            max_particles,
        }
    }

    /// Update emitter: spawn new particles, remove dead ones
    pub fn update(&mut self, delta_time_s: f32) {
        // Spawn new particles
        self.emission_accumulator += self.config.emission_rate * delta_time_s;

        while self.emission_accumulator >= 1.0 && self.particles.len() < self.max_particles {
            self.spawn_particle();
            self.emission_accumulator -= 1.0;
        }

        // Update existing particles
        for particle in &mut self.particles {
            particle.update(delta_time_s);
        }

        // Remove dead particles
        self.particles.retain(|p| p.is_alive());
    }

    fn spawn_particle(&mut self) {
        let mut rng = rand::thread_rng();

        let velocity = self.config.initial_velocity
            + Vec3::new(
                (rng.gen::<f32>() - 0.5) * 2.0 * self.config.velocity_variance.x,
                (rng.gen::<f32>() - 0.5) * 2.0 * self.config.velocity_variance.y,
                (rng.gen::<f32>() - 0.5) * 2.0 * self.config.velocity_variance.z,
            );

        self.particles.push(Particle {
            position: Vec3::ZERO,
            velocity,
            acceleration: Vec3::ZERO,
            color: self.config.color,
            scale: self.config.initial_scale,
            intensity: 1.0,
            mass: 1.0,
            lifetime_s: self.config.particle_lifetime_s,
            age_s: 0.0,
            hardness: self.config.material.hardness,
            roughness: self.config.material.roughness,
            wetness: self.config.material.wetness,
            emits_light: false,
            light_radius: 0.0,
        });
    }

    pub fn get_particles(&self) -> &[Particle] {
        &self.particles
    }

    pub fn get_particles_mut(&mut self) -> &mut [Particle] {
        &mut self.particles
    }

    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }
}
```

---

## PS.3: ParticlePhysics (Simulation) (30 minutes)

### File: `src/particle_system/physics.rs` (200 lines)

```rust
pub struct ParticlePhysicsSimulator {
    gravity: Vec3,                    // (0, -9.81, 0) typically
    air_resistance: f32,              // Drag coefficient
}

impl ParticlePhysicsSimulator {
    pub fn new(gravity: Vec3, air_resistance: f32) -> Self {
        Self {
            gravity,
            air_resistance,
        }
    }

    /// Update particle physics: velocity, position, collisions
    pub fn simulate_step(
        &self,
        particles: &mut [Particle],
        delta_time_s: f32,
        colliders: &[Collider],  // Static geometry (floor, walls, etc.)
    ) {
        for particle in particles {
            if !particle.is_alive() {
                continue;
            }

            // Apply forces
            let drag = self.compute_drag(particle);
            particle.acceleration = self.gravity + drag;

            // Euler integration
            particle.velocity += particle.acceleration * delta_time_s;
            particle.position += particle.velocity * delta_time_s;

            // Collision detection & response
            for collider in colliders {
                if let Some(collision) = collider.test_collision(particle.position) {
                    self.resolve_collision(particle, &collision);
                }
            }
        }
    }

    fn compute_drag(&self, particle: &Particle) -> Vec3 {
        // Drag force proportional to velocity and wetness
        // Wetness acts like air resistance (higher = more drag)
        let drag_coeff = self.air_resistance * (1.0 + particle.wetness);
        -particle.velocity * drag_coeff
    }

    fn resolve_collision(
        &self,
        particle: &mut Particle,
        collision: &CollisionInfo,
    ) {
        // Bounce based on hardness (harder = more bounce)
        let elasticity = particle.hardness;  // [0, 1]

        // Reflect velocity around surface normal
        let normal = collision.surface_normal;
        let velocity_along_normal = particle.velocity.dot(&normal);

        if velocity_along_normal < 0.0 {  // Moving into surface
            particle.velocity -= normal * velocity_along_normal * (1.0 + elasticity);

            // Friction based on roughness (rougher = more friction)
            let friction = particle.roughness;  // [0, 1]
            particle.velocity *= (1.0 - friction * 0.5);  // Reduce tangential velocity

            // Push particle outside of collider (prevent tunneling)
            particle.position += normal * collision.penetration_depth * 1.1;
        }
    }
}

pub struct Collider {
    pub geometry: ColliderGeometry,  // Sphere, Box, Plane, etc.
}

pub enum ColliderGeometry {
    Plane { normal: Vec3, distance: f32 },
    Sphere { center: Vec3, radius: f32 },
    Box { center: Vec3, half_extents: Vec3 },
}

pub struct CollisionInfo {
    pub surface_normal: Vec3,
    pub penetration_depth: f32,
}

impl Collider {
    pub fn test_collision(&self, point: Vec3) -> Option<CollisionInfo> {
        match &self.geometry {
            ColliderGeometry::Plane { normal, distance } => {
                let dist = point.dot(normal) - distance;
                if dist < 0.0 {
                    Some(CollisionInfo {
                        surface_normal: *normal,
                        penetration_depth: dist.abs(),
                    })
                } else {
                    None
                }
            }
            // ... other geometries
        }
    }
}
```

---

## PS.4: ParticleRenderer (GPU Rendering) (30 minutes)

### File: `src/particle_system/renderer.rs` (250 lines)

```rust
pub struct ParticleRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl ParticleRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particle.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/particle.wgsl").into()),
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("particle_pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("particle_layout"),
                bind_group_layouts: &[&/* bind group layout */],
                push_constant_ranges: &[],
            })),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<ParticleGPU>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3,   // position
                        1 => Float32x4,   // color
                        2 => Float32,     // scale
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        Self {
            device: device.clone(),
            queue: queue.clone(),
            render_pipeline,
            vertex_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("particle_vertices"),
                size: 1024 * 1024,  // 1MB buffer for particle data
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            index_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("particle_indices"),
                size: 512 * 1024,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            bind_group: /* ... */,
        }
    }

    /// Render particles to screen
    pub fn render(
        &self,
        render_pass: &mut wgpu::RenderPass,
        particles: &[Particle],
    ) {
        // Convert CPU particles to GPU-friendly format
        let gpu_particles: Vec<ParticleGPU> = particles.iter()
            .map(|p| ParticleGPU {
                position: p.position.into(),
                color: p.color.into(),
                scale: p.scale * (0.5 + p.intensity * 0.5),  // Scale with intensity
            })
            .collect();

        // Upload to GPU
        self.queue.write_buffer(
            &self.vertex_buffer,
            0,
            bytemuck::cast_slice(&gpu_particles),
        );

        // Render
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..6, 0..gpu_particles.len() as u32);
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleGPU {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub scale: f32,
}
```

### File: `src/shaders/particle.wgsl` (80 lines)

```wgsl
// Vertex shader
struct ParticleData {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) scale: f32,
}

@vertex
fn vs_main(particle: ParticleData) -> @builtin(position) vec4<f32> {
    // Billboard quad (always faces camera)
    let quad_pos = vec2<f32>(-1.0, -1.0);  // or (1, 1), (-1, 1), (1, -1) depending on instance
    let offset = quad_pos * particle.scale * 0.1;  // Screen-space offset
    return vec4<f32>(particle.position.xy + offset, particle.position.z, 1.0);
}

// Fragment shader
@fragment
fn fs_main(particle: ParticleData) -> @location(0) vec4<f32> {
    let dist_from_center = length(particle.position);
    let alpha = 1.0 - (dist_from_center / particle.scale);  // Gaussian falloff
    return vec4<f32>(particle.color.rgb, particle.color.a * alpha.max(0.0));
}
```

---

## PS.5: Module Integration (30 minutes)

### File: `src/particle_system/mod.rs` (100 lines)

```rust
pub mod particle;
pub mod emitter;
pub mod physics;
pub mod renderer;

pub use particle::{Particle, ParticleEmitterConfig, ParticleMaterial};
pub use emitter::ParticleEmitter;
pub use physics::ParticlePhysicsSimulator;
pub use renderer::ParticleRenderer;

/// Unified particle system interface
pub struct ParticleSystem {
    pub emitter: ParticleEmitter,
    pub physics: ParticlePhysicsSimulator,
    pub renderer: ParticleRenderer,
}

impl ParticleSystem {
    pub fn new(
        emitter_config: ParticleEmitterConfig,
        physics_gravity: Vec3,
        renderer_device: &wgpu::Device,
        renderer_queue: &wgpu::Queue,
    ) -> Self {
        Self {
            emitter: ParticleEmitter::new(emitter_config, 10000),
            physics: ParticlePhysicsSimulator::new(physics_gravity, 0.1),
            renderer: ParticleRenderer::new(renderer_device, renderer_queue),
        }
    }

    /// Update frame: spawn, simulate, update age
    pub fn update(&mut self, delta_time_s: f32, colliders: &[physics::Collider]) {
        self.emitter.update(delta_time_s);
        self.physics.simulate_step(
            self.emitter.get_particles_mut(),
            delta_time_s,
            colliders,
        );
    }

    /// Render to screen
    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        self.renderer.render(render_pass, self.emitter.get_particles());
    }
}
```

---

## Integration Pattern (How D.4, I.5, VI Use This)

### Track D.4 (Gaussian Splatting):
```rust
use crate::particle_system::*;

let emitter_config = ParticleEmitterConfig {
    emission_rate: 100.0,  // 100 particles/sec
    particle_lifetime_s: 5.0,
    initial_velocity: Vec3::ZERO,
    velocity_variance: Vec3::ZERO,
    initial_scale: 0.1,
    color: Vec4::new(1.0, 0.0, 0.0, 0.8),  // Red, semi-transparent
    material: ParticleMaterial {
        hardness: 0.8,
        roughness: 0.2,
        wetness: 0.1,
    },
};

let mut ps = ParticleSystem::new(emitter_config, Vec3::new(0.0, -9.81, 0.0), device, queue);

// Each frame:
ps.update(delta_time_s, &[]);  // No colliders for D.4
ps.render(&mut render_pass);
```

### Track I.5 (Physics Particle System):
```rust
use crate::particle_system::*;

let emitter_config = ParticleEmitterConfig {
    emission_rate: 50.0,
    particle_lifetime_s: 10.0,
    initial_velocity: Vec3::new(0.0, 2.0, 0.0),
    velocity_variance: Vec3::new(1.0, 0.5, 1.0),
    material: ParticleMaterial {
        hardness: 0.6,   // Bouncy
        roughness: 0.4,  // Moderate texture
        wetness: 0.3,    // Some drag
    },
};

let mut ps = ParticleSystem::new(emitter_config, Vec3::new(0.0, -9.81, 0.0), device, queue);

// Define colliders (ground, walls, etc.)
let colliders = vec![
    Collider { geometry: ColliderGeometry::Plane {
        normal: Vec3::new(0.0, 1.0, 0.0),
        distance: 0.0,  // Ground plane at y=0
    }},
];

// Each frame:
ps.update(delta_time_s, &colliders);  // Physics simulation with bouncing
ps.render(&mut render_pass);
```

---

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| **Particle Count** | 50,000 | Max before GPU memory pressure |
| **Spawn Rate** | 10,000/sec | Typical peak |
| **Update Latency** | < 5ms | Per-frame CPU side |
| **Render Time** | < 8ms | GPU side (1024×1024 viewport) |
| **Total Frame Budget** | < 16ms | 60 fps target |

---

## Tests (8 tests)

- Test 1: Particle creation and destruction
- Test 2: Lifetime and fade-out
- Test 3: Physics gravity and velocity
- Test 4: Collision detection
- Test 5: Bounce based on hardness
- Test 6: Friction based on roughness
- Test 7: Drag based on wetness
- Test 8: Render 10k particles in < 10ms

---

## File Ownership (Exclusive)

```
src/particle_system/
├── particle.rs              (Particle primitive)
├── emitter.rs              (ParticleEmitter)
├── physics.rs              (Physics simulation)
├── renderer.rs             (GPU rendering)
├── mod.rs                  (Module interface)
└── ../shaders/particle.wgsl (WGSL shader)

tests/particle_system_integration.rs  (All 8 tests)
```

**No conflicts**: Only D.4, I.5, and VI import from this module. No shared writes.

---

## Success Criteria

✅ Particle primitive defined
✅ Emitter spawns particles at target rate
✅ Physics simulation with gravity, drag, collision
✅ Renderer outputs particles to GPU in < 8ms
✅ Hardness → bounce coefficient
✅ Roughness → friction coefficient
✅ Wetness → drag coefficient
✅ All 8 tests passing

---

## Notes

**GPU Optimization**: Uses instanced rendering (one quad per particle, GPU-side transformation). Avoids per-particle draw call overhead.

**Material Properties**: Hardness/Roughness/Wetness directly map to physics behavior, enabling RF-BSDF visualization (same material properties as Track I).

**Extensibility**: Easy to add new geometry types (mesh particles, trails, light emissions) without breaking existing code.

