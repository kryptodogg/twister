// ─────────────────────────────────────────────────────────────────────
// UNIFIED FIELD PARTICLE STRUCTURES (Phase 1: Wavelet-Particle Unification)
// ─────────────────────────────────────────────────────────────────────

/// **FieldParticle**: The fundamental unit of the unified phase-space field.
/// Every detection—RF, audio, spatial—is a particle in Cartesian voxel coordinates.
/// Size: 48 bytes (GPU-optimized for alignment)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FieldParticle {
    /// 3D position in voxel grid (Cartesian, NOT spherical TDOA)
    /// Integer-like coordinates map to RF field solver grid
    pub position: [f32; 3],

    /// Phase & amplitude (IQ data, learned by Mamba)
    /// [phase_in_quadrature, phase_in_phase] → phase prediction from neural network
    pub phase_amp: [f32; 2],

    /// Material properties (inverse-problem estimated by Mamba)
    /// [hardness, roughness, wetness] ∈ [0,1]
    /// Hardness: reflection coefficient (0=absorbing, 1=metal)
    /// Roughness: scattering ratio (0=specular, 1=diffuse)
    /// Wetness: water content affecting permittivity (0=dry, 1=water)
    pub material: [f32; 3],

    /// Energy gradient magnitude ∇|E|² (force field for dynamics)
    /// Predicted by Mamba as output; used for particle advection and rendering glow
    pub energy_gradient: f32,

    /// Reserved for alignment (future: particle ID, confidence, etc.)
    pub _padding: f32,
}

impl FieldParticle {
    /// Create a neutral particle at voxel center
    pub fn new(position: [f32; 3]) -> Self {
        FieldParticle {
            position,
            phase_amp: [0.0, 0.0],
            material: [0.5, 0.5, 0.0],  // Neutral material (drywall)
            energy_gradient: 0.0,
            _padding: 0.0,
        }
    }

    /// Tensor layout: 9 floats per particle
    /// [x, y, z, phase_i, phase_q, hardness, roughness, wetness, energy_gradient]
    pub const FLOATS_PER_PARTICLE: usize = 9;
}

// ─────────────────────────────────────────────────────────────────────
// LEGACY STRUCTURES (Kept for compatibility with existing code)
// ─────────────────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PointMambaEncoderOutput {
    pub embedding: [f32; 256],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RFDetection {
    pub azimuth: f32,
    pub elevation: f32,
    pub frequency: f32,
    pub intensity: f32,
    pub timestamp: u64,
    pub confidence: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IMUSample {
    pub accel: [f32; 3],
    pub gyro: [f32; 3],
    pub timestamp: u64,
}

// ─────────────────────────────────────────────────────────────────────
// CONTAINER STRUCTURES FOR GPU BUFFERS
// ─────────────────────────────────────────────────────────────────────

/// **PointCloud**: Spatial positions + feature vectors (from PointNet or Mamba)
#[derive(Debug, Clone)]
pub struct PointCloud {
    /// N points × 3 coordinates
    pub positions: Vec<[f32; 3]>,
    /// N points × 256 features (from PointNet encoder or Mamba output)
    pub features: Vec<[f32; 256]>,
}

impl PointCloud {
    pub fn new(num_points: usize) -> Self {
        PointCloud {
            positions: vec![[0.0; 3]; num_points],
            features: vec![[0.0; 256]; num_points],
        }
    }

    pub fn num_points(&self) -> usize {
        self.positions.len()
    }
}

/// **FieldParticleCloud**: GPU-ready collection of unified particles
#[derive(Debug, Clone)]
pub struct FieldParticleCloud {
    /// N particles with full FieldParticle data
    pub particles: Vec<FieldParticle>,
    /// Hilbert indices for sorting (pre-computed or updated during accumulation)
    pub hilbert_indices: Vec<u32>,
    /// Timestamps for each particle (microseconds since epoch)
    pub timestamps: Vec<u64>,
}

impl FieldParticleCloud {
    pub fn new(capacity: usize) -> Self {
        FieldParticleCloud {
            particles: Vec::with_capacity(capacity),
            hilbert_indices: Vec::with_capacity(capacity),
            timestamps: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, particle: FieldParticle, hilbert_idx: u32, timestamp: u64) {
        self.particles.push(particle);
        self.hilbert_indices.push(hilbert_idx);
        self.timestamps.push(timestamp);
    }

    pub fn len(&self) -> usize {
        self.particles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.hilbert_indices.clear();
        self.timestamps.clear();
    }

    /// Export as flat tensor [N, 9] for Mamba input
    /// [x, y, z, phase_i, phase_q, hardness, roughness, wetness, energy_gradient]
    pub fn as_tensor(&self) -> Vec<f32> {
        let mut tensor = Vec::with_capacity(self.particles.len() * FieldParticle::FLOATS_PER_PARTICLE);
        for p in &self.particles {
            tensor.push(p.position[0]);
            tensor.push(p.position[1]);
            tensor.push(p.position[2]);
            tensor.push(p.phase_amp[0]);
            tensor.push(p.phase_amp[1]);
            tensor.push(p.material[0]);
            tensor.push(p.material[1]);
            tensor.push(p.material[2]);
            tensor.push(p.energy_gradient);
        }
        tensor
    }

    /// Import from flat tensor [N, 9]
    pub fn from_tensor(data: &[f32]) -> Option<Self> {
        if data.len() % FieldParticle::FLOATS_PER_PARTICLE != 0 {
            return None;
        }

        let num_particles = data.len() / FieldParticle::FLOATS_PER_PARTICLE;
        let mut cloud = FieldParticleCloud::new(num_particles);

        for i in 0..num_particles {
            let base = i * FieldParticle::FLOATS_PER_PARTICLE;
            let particle = FieldParticle {
                position: [data[base], data[base + 1], data[base + 2]],
                phase_amp: [data[base + 3], data[base + 4]],
                material: [data[base + 5], data[base + 6], data[base + 7]],
                energy_gradient: data[base + 8],
                _padding: 0.0,
            };
            cloud.push(particle, 0, 0);
        }

        Some(cloud)
    }
}

/// **EnergyGradientField**: GPU buffer containing ∇|E|² predictions
/// Output from Mamba Neural Operator; input to ray-marcher
#[derive(Debug, Clone)]
pub struct EnergyGradientField {
    /// Per-voxel magnitude of energy gradient
    pub magnitude: Vec<f32>,
    /// Per-voxel gradient direction (3D unit vector)
    pub direction: Vec<[f32; 3]>,
}
