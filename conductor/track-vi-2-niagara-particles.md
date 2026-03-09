# Track VI.2: Niagara Particle System (Emission & Dynamics)

**Domain**: Particle Rendering & Physics
**Ownership**: Graphics Engineer (isolated from VI.1, VI.3)
**Duration**: 2-3 days
**Blocker on**: Particle System Infrastructure track (exists independently)
**Unblocks**: VI.3 (lighting integration)

---

## Overview

Emit particles from RF energy density field (VI.1 output). Each particle inherits properties from its birth location: intensity (field magnitude), phase, frequency, material properties. Simulate physics (collision, bounce, decay based on material hardness). Render via GPU mesh shaders (millions of particles, < 10ms latency).

**Generation protection**: Particle lifetime and bounce behavior must be material-aware. Wetness increases friction (slower motion). Hardness determines bounce elasticity. Never cap particles at fixed count—use mesh shaders for dynamic scaling. Emission rate must respond to RF field energy in real-time.

---

## File Ownership

**VI.2 — Exclusive to this track**:
- `src/visualization/particle_emitter.rs` (200 lines) — Emission from energy field
- `src/visualization/particle_dynamics.rs` (250 lines) — Physics, collision, decay
- `src/visualization/particle_renderer.rs` (300 lines) — GPU mesh shader rendering
- `src/visualization/gaussian_splatting.rs` (400 lines) — Tone mapping, splatting
- `tests/niagara_particles.rs` (300 lines, 12 tests)

**Read-only imports**:
- `src/physics/rf_propagation.rs` (VI.1 interface: solve_rf_field)
- `src/physics/voxel_grid.rs` (energy field sampling)
- `src/particle_system/mod.rs` (Particle System Infrastructure)
- `src/analysis/pattern_library.rs` (Track K: pattern colors)

**No modifications to**:
- `src/main.rs` (dispatch loop)
- `src/physics/` (VI.1 read-only)

---

## Deliverables

### VI.2.1: Particle Emitter (10 hours)

**File**: `src/visualization/particle_emitter.rs`

```rust
pub struct ParticleEmitter {
    energy_field: VoxelGrid<f32>,          // From VI.1
    emission_rate_per_joule: f32,          // Particles spawned per unit energy
    particles_alive: Vec<Particle>,
}

pub struct Particle {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub intensity: f32,                    // Energy (0-1)
    pub phase: f32,                        // Wave phase (0-2π)
    pub frequency_hz: f32,                 // RF frequency
    pub lifetime_s: f32,                   // Birth time
    pub material_hardness: f32,            // 0-1 (from birth location)
    pub material_wetness: f32,             // 0-1 (friction)
    pub color: [f32; 4],                   // RGBA
    pub confidence: f32,                   // Detection confidence
}

impl ParticleEmitter {
    pub fn new(energy_field: VoxelGrid<f32>) -> Self {
        Self {
            energy_field,
            emission_rate_per_joule: 1000.0,  // Tune based on visual density
            particles_alive: Vec::new(),
        }
    }

    /// Emit new particles at locations with high energy density
    pub fn emit(&mut self, dt_s: f32) {
        // Sample energy field at regular grid points
        for (x, y, z) in self.energy_field.iter_voxels() {
            let energy = self.energy_field.get(x, y, z);

            // Spawn particles proportional to energy
            let num_particles = (energy * self.emission_rate_per_joule * dt_s) as usize;

            for _ in 0..num_particles {
                let particle = Particle {
                    position: world_coord_from_voxel(x, y, z),
                    velocity: random_direction() * (energy * 10.0),  // Higher energy → faster
                    intensity: energy,
                    phase: random_0_to_2pi(),
                    frequency_hz: 2.4e9,  // TODO: read from RF field
                    lifetime_s: 0.0,
                    material_hardness: 0.5,  // Sample from material grid (VI.1)
                    material_wetness: 0.3,
                    color: intensity_to_color(energy),  // Blue→Red→Yellow→White
                    confidence: 0.8,
                };

                self.particles_alive.push(particle);
            }
        }
    }

    /// Get current particle count
    pub fn particle_count(&self) -> usize {
        self.particles_alive.len()
    }

    /// Cull dead particles
    pub fn reap_dead(&mut self, max_lifetime_s: f32) {
        self.particles_alive.retain(|p| p.lifetime_s < max_lifetime_s);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_emission_from_field() { /* particles spawn at high-energy locations */ }

    #[test]
    fn test_emission_rate_scaling() { /* higher energy → more particles */ }

    #[test]
    fn test_particle_velocity_energy_coupled() { /* v ∝ energy */ }

    #[test]
    fn test_lifetime_management() { /* dead particles culled */ }

    #[test]
    fn test_dynamic_scaling() { /* millions of particles, no artificial cap */ }
}
```

**Generation protection**:
- ✅ NO artificial particle count cap (use mesh shaders for scaling)
- ✅ Emission rate tied to field energy (responsive, not static)
- ✅ Material properties sampled at birth location
- ❌ DON'T emit uniformly everywhere (wastes GPU)
- ✅ DO cull dead particles (prevent unbounded growth)

---

### VI.2.2: Particle Dynamics (12 hours)

**File**: `src/visualization/particle_dynamics.rs`

```rust
pub struct ParticleSimulator {
    gravity: f32,
    air_friction: f32,
}

impl ParticleSimulator {
    pub fn new() -> Self {
        Self {
            gravity: 9.81,
            air_friction: 0.1,  // Linear drag coefficient
        }
    }

    /// Update all particles for one frame
    pub fn step(&self, particles: &mut [Particle], dt_s: f32, material_grid: &VoxelGrid<Material>) {
        for particle in particles.iter_mut() {
            // Apply forces
            let gravity_force = [0.0, -self.gravity * particle.intensity, 0.0];  // Energy-weighted gravity
            let friction = [-particle.velocity[0] * self.air_friction * (1.0 + particle.material_wetness),
                            -particle.velocity[1] * self.air_friction * (1.0 + particle.material_wetness),
                            -particle.velocity[2] * self.air_friction * (1.0 + particle.material_wetness)];

            // Update velocity
            for i in 0..3 {
                particle.velocity[i] += (gravity_force[i] + friction[i]) * dt_s;
            }

            // Update position
            for i in 0..3 {
                particle.position[i] += particle.velocity[i] * dt_s;
            }

            // Collision detection & bouncing
            if let Some(material) = self.check_collision(&particle.position, material_grid) {
                self.bounce_particle(particle, &material);
            }

            // Decay lifetime
            particle.lifetime_s += dt_s;

            // Intensity fade (proportional to lifetime)
            particle.intensity *= (1.0 - dt_s * 0.2);  // 20% per second fade
        }
    }

    fn check_collision(&self, pos: &[f32; 3], material_grid: &VoxelGrid<Material>) -> Option<Material> {
        // Sample voxel at position
        let voxel_material = material_grid.sample((*pos[0], *pos[1], *pos[2]));
        if voxel_material.hardness > 0.3 {  // Solid material
            Some(voxel_material)
        } else {
            None
        }
    }

    fn bounce_particle(&self, particle: &mut Particle, material: &Material) {
        // Elasticity: hardness = 1.0 → elastic bounce, hardness = 0.0 → absorb
        let elasticity = material.hardness;

        // Reverse velocity (simplified: assume normal = surface normal)
        // Better: compute actual surface normal at collision point
        for i in 0..3 {
            particle.velocity[i] *= -elasticity;  // Reverse + dampen by material hardness
        }

        // Energy loss in collision
        particle.intensity *= (1.0 - material.roughness) * elasticity;  // Rough + soft = more loss
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_gravity_energy_weighted() { /* v = sqrt(2 * g * h * intensity) */ }

    #[test]
    fn test_friction_wetness_coupled() { /* higher wetness → more drag */ }

    #[test]
    fn test_elastic_bounce_hardness() { /* hardness 1.0 → elastic, 0.0 → absorb */ }

    #[test]
    fn test_intensity_decay() { /* exponential with time */ }

    #[test]
    fn test_million_particle_step() { /* < 2ms simulation step */ }
}
```

**Generation protection**:
- ✅ Physics material-aware (hardness → elasticity, wetness → friction)
- ✅ Intensity-weighted gravity (stronger signals "heavier")
- ❌ DON'T use uniform gravity (loses energy information)
- ✅ DO fade intensity with lifetime (visual feedback on age)

---

### VI.2.3: GPU Particle Rendering (12 hours)

**File**: `src/visualization/particle_renderer.rs`

```rust
pub struct ParticleRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    mesh_shader_pipeline: wgpu::RenderPipeline,
    particle_buffer: wgpu::Buffer,
    frustum_culler: FrustumCuller,
}

impl ParticleRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let mesh_shader_pipeline = Self::create_mesh_shader_pipeline(device);
        let particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Buffer"),
            size: (100_000_000 * 44) as u64,  // 100M particles @ 44 bytes each
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        Self {
            device: device.clone(),
            queue: queue.clone(),
            mesh_shader_pipeline,
            particle_buffer,
            frustum_culler: FrustumCuller::new(),
        }
    }

    pub fn render_particles(
        &self,
        render_pass: &mut wgpu::RenderPass,
        particles: &[Particle],
        camera_frustum: &Frustum,
    ) {
        // Upload particles to GPU
        self.queue.write_buffer(&self.particle_buffer, 0, bytemuck::cast_slice(particles));

        // Frustum cull: mark off-screen particles
        self.frustum_culler.cull(particles, camera_frustum);

        // Render via mesh shaders
        // One GPU invocation per particle (not per vertex)
        render_pass.set_pipeline(&self.mesh_shader_pipeline);
        render_pass.set_bind_group(0, &self.particle_bind_group, &[]);
        render_pass.draw_mesh_tasks(particles.len() as u32, 0);
    }

    fn create_mesh_shader_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
        // Load WGSL shader (mesh_shader.wgsl)
        // Define layout: particle buffer (readonly storage)
        // Output: quad per particle (4 vertices, 6 indices)
        // Return compiled pipeline
    }
}

// WGSL Mesh Shader (dispatch_kernel.wgsl)
pub const MESH_SHADER_WGSL: &str = r#"
@compute @workgroup_size(256)
fn mesh_shader_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let particle_idx = gid.x;
    let particle = particles[particle_idx];

    // Generate quad: 4 vertices, 2 triangles
    let scale = 0.05;  // Particle quad size
    let quad_vertices = array<vec3<f32>, 4>(
        particle.position + vec3(-scale, -scale, 0.0),
        particle.position + vec3(+scale, -scale, 0.0),
        particle.position + vec3(+scale, +scale, 0.0),
        particle.position + vec3(-scale, +scale, 0.0),
    );

    // Emit vertices with color from intensity
    emit_mesh_vertex(quad_vertices[0], particle.color);
    emit_mesh_vertex(quad_vertices[1], particle.color);
    emit_mesh_vertex(quad_vertices[2], particle.color);
    emit_mesh_vertex(quad_vertices[3], particle.color);

    // Emit indices (2 triangles)
    emit_mesh_index(0);
    emit_mesh_index(1);
    emit_mesh_index(2);
    emit_mesh_index(2);
    emit_mesh_index(3);
    emit_mesh_index(0);
}
"#;

#[cfg(test)]
mod tests {
    #[test]
    fn test_particle_buffer_allocation() { /* 100M particle capacity */ }

    #[test]
    fn test_frustum_culling() { /* only visible particles rendered */ }

    #[test]
    fn test_mesh_shader_compilation() { /* WGSL compiles */ }

    #[test]
    fn test_1m_particle_render() { /* < 10ms on RX 6700 XT */ }

    #[test]
    fn test_10m_particle_render_culled() { /* with frustum culling < 50ms */ }
}
```

**Generation protection**:
- ✅ Mesh shaders (4-10x faster than billboard instancing)
- ✅ NO artificial particle cap (100M+ capacity)
- ✅ Frustum culling (GPU-side, automatic)
- ❌ DON'T use vertex buffer per particle (bottleneck)
- ✅ DO render via compute-generated geometry (GPU native parallelism)

---

### VI.2.4: Tone Mapping & Color (6 hours)

**File**: `src/visualization/gaussian_splatting.rs`

```rust
pub fn intensity_to_color(intensity: f32) -> [f32; 4] {
    // Remap [0, 1] intensity to thermal color (blue → red → yellow → white)
    let t = intensity.clamp(0.0, 1.0);

    let r = if t < 0.5 {
        0.0
    } else {
        (t - 0.5) * 2.0  // Increase R from 0.5 onward
    };

    let g = if t < 0.33 {
        0.0
    } else if t < 0.66 {
        (t - 0.33) * 3.0
    } else {
        1.0
    };

    let b = if t < 0.25 {
        1.0 - t * 4.0  // Decrease B from 1.0
    } else {
        0.0
    };

    [r, g, b, intensity]  // Alpha = intensity (fades with age)
}

pub fn tone_map_hdr(linear: f32, exposure: f32) -> f32 {
    // Reinhard tone mapping: compress HDR to [0, 1]
    let exposed = linear * exposure;
    exposed / (1.0 + exposed)  // Soft clipping
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_color_ramp_blue_to_white() { /* 0.0 → blue, 1.0 → white */ }

    #[test]
    fn test_tone_mapping_preserves_order() { /* higher input → higher output */ }
}
```

---

## Interface Contract (For VI.3)

**Export from VI.2**:
```rust
pub struct RenderedFramebuffer {
    pub color_texture: wgpu::Texture,  // RGB + Alpha
    pub depth_texture: wgpu::Texture,
}

pub fn render_particle_frame(
    particles: &[Particle],
    camera: &Camera,
    viewport_size: (u32, u32),
) -> Result<RenderedFramebuffer, Box<dyn Error>> {
    // VI.3 imports this
}
```

VI.3 reads rendered framebuffer without modification.

---

## Local Validation

```bash
#!/bin/bash
# Check: No particle count cap
if grep -q "MAX_PARTICLES\|const.*PARTICLES.*=.*[0-9]*000$" src/visualization/particle_emitter.rs; then
    echo "⚠️  WARNING: Particle count may be artificially capped"
fi

# Check: Material-aware physics
if ! grep -q "material_hardness\|material_wetness\|elasticity" src/visualization/particle_dynamics.rs; then
    echo "❌ ERROR: Physics must be material-aware"
    exit 1
fi

# Check: Mesh shader pipeline
if ! grep -q "mesh_shader\|compute @workgroup" src/visualization/particle_renderer.rs; then
    echo "❌ ERROR: Must use mesh shaders (not billboard instancing)"
    exit 1
fi

cargo test niagara_particles --lib -- --nocapture
```

---

## Success Criteria

- [ ] Particles emit from high-energy RF field locations
- [ ] Emission rate scales with field energy
- [ ] Physics simulation: gravity, friction, bounce all material-aware
- [ ] Bounce elasticity tied to material hardness
- [ ] Particle intensity decays exponentially with lifetime
- [ ] GPU rendering: mesh shaders, 1M particles < 10ms
- [ ] Frustum culling reduces visible particles by 50-95%
- [ ] Color mapping: thermal ramp (blue → white) proportional to intensity
- [ ] All 12 tests passing
- [ ] Interface stable (VI.3 imports without modification)

---

## Notes

**Parallelism**: VI.2 depends on Particle System Infrastructure (already exists). Works in parallel with VI.1 and VI.3.

**Generation protection**: Material-aware physics is non-negotiable. Mesh shaders required for performance. No artificial caps on particle count.
