use num_complex::Complex;
use crate::physics::voxel_grid::VoxelGrid;
use crate::visualization::data_contracts::RoomGeometry;

#[derive(Clone, Debug, Default)]
pub struct Material {
    pub name: String,
    pub hardness: f32,           // 0.0-1.0 (0 = soft/absorbent, 1 = hard/reflective)
    pub roughness: f32,          // 0.0-1.0 (diffuse scattering)
    pub wetness: f32,            // 0.0-1.0 (water content, affects permittivity)
    pub permittivity: f32,       // ε_r (relative permittivity)
    pub conductivity: f32,       // σ (siemens per meter)
}

impl Material {
    /// Attenuation coefficient: α = ω * sqrt(εμ/2) * sqrt(1 + (σ/ωε)² - 1)
    /// Based on material loss tangent: tan(δ) = σ / (ωε)
    pub fn attenuation_coeff(&self, frequency_hz: f32) -> f32 {
        if self.permittivity <= 0.0 {
            return 0.0;
        }

        let omega = 2.0 * std::f32::consts::PI * frequency_hz;
        let epsilon_0 = 8.854e-12;
        let mu_0 = 4.0 * std::f32::consts::PI * 1e-7;

        let permittivity = self.permittivity * epsilon_0;
        let tan_delta = self.conductivity / (omega * permittivity);

        // Simplified: higher wetness → higher conductivity → more loss
        omega * (permittivity * mu_0).sqrt() * ((1.0 + tan_delta * tan_delta).sqrt() - 1.0).sqrt() / 2.0_f32.sqrt()
    }

    /// Reflection coefficient: R = |E_reflected / E_incident|
    /// Fresnel equations for normal incidence
    pub fn reflection_coeff(&self, _frequency_hz: f32) -> f32 {
        // Simplified: hardness scales R linearly
        self.hardness.min(1.0).max(0.0)
    }

    /// Scattering coefficient: depends on roughness
    pub fn scattering_coeff(&self) -> f32 {
        self.roughness.min(1.0).max(0.0)  // 0 = specular, 1 = diffuse
    }

    pub fn air() -> Self {
        Self {
            name: "Air".to_string(),
            hardness: 0.0,
            roughness: 0.0,
            wetness: 0.0,
            permittivity: 1.0,
            conductivity: 0.0,
        }
    }

    pub fn drywall() -> Self {
        Self {
            name: "Drywall".to_string(),
            hardness: 0.6,
            roughness: 0.5,
            wetness: 0.05,
            permittivity: 2.8,
            conductivity: 0.01,
        }
    }

    pub fn concrete() -> Self {
        Self {
            name: "Concrete".to_string(),
            hardness: 0.9,
            roughness: 0.7,
            wetness: 0.1,
            permittivity: 4.5,
            conductivity: 0.03,
        }
    }

    pub fn water() -> Self {
        Self {
            name: "Water".to_string(),
            hardness: 0.1,
            roughness: 0.0,
            wetness: 1.0,
            permittivity: 80.0,
            conductivity: 1.5,
        }
    }
}

pub struct MaterialGrid {
    pub grid: VoxelGrid<Material>,
    pub frequency_hz: f32,
}

impl MaterialGrid {
    pub fn new(size: usize, freq: f32) -> Self {
        Self {
            grid: VoxelGrid::new(size),
            frequency_hz: freq,
        }
    }

    pub fn from_room_geometry(room: &RoomGeometry, frequency_hz: f32, voxel_size_m: f32) -> Self {
        // Calculate dimensions based on bounds
        let width = (room.max_bound.0 - room.min_bound.0) / voxel_size_m;
        let height = (room.max_bound.1 - room.min_bound.1) / voxel_size_m;
        let depth = (room.max_bound.2 - room.min_bound.2) / voxel_size_m;

        let dim_x = width.ceil() as usize;
        let dim_y = height.ceil() as usize;
        let dim_z = depth.ceil() as usize;

        let mut grid = VoxelGrid::with_dimensions_and_size(dim_x, dim_y, dim_z, voxel_size_m, room.min_bound);

        // Fill with air initially
        for x in 0..dim_x {
            for y in 0..dim_y {
                for z in 0..dim_z {
                    grid.set(x, y, z, Material::air());
                }
            }
        }

        // Walls (drywall) - simple AABB
        for x in 0..dim_x {
            for y in 0..dim_y {
                // Front and back walls
                grid.set(x, y, 0, Material::drywall());
                grid.set(x, y, dim_z.saturating_sub(1), Material::drywall());
            }
        }

        for z in 0..dim_z {
            for y in 0..dim_y {
                // Left and right walls
                grid.set(0, y, z, Material::drywall());
                grid.set(dim_x.saturating_sub(1), y, z, Material::drywall());
            }
        }

        // Floor and ceiling (concrete)
        for x in 0..dim_x {
            for z in 0..dim_z {
                grid.set(x, 0, z, Material::concrete());
                grid.set(x, dim_y.saturating_sub(1), z, Material::concrete());
            }
        }

        Self {
            grid,
            frequency_hz,
        }
    }

    pub fn attenuate_wave(&self, wave: Complex<f32>, distance: f32, material: &Material) -> Complex<f32> {
        let alpha = material.attenuation_coeff(self.frequency_hz);
        let loss = (-alpha * distance).exp();
        wave * loss
    }
}
