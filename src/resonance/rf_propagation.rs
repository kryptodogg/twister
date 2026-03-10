use std::error::Error;
use num_complex::Complex;
use crate::resonance::voxel_grid::VoxelGrid;
use crate::resonance::material_absorption::Material;

pub struct RFWavePropagation {
    pub grid: VoxelGrid<Complex<f32>>,  // Complex amplitude per voxel (phase-aware)
    pub frequency_hz: f32,
    pub wavelength_m: f32,
    pub speed_of_light: f32,
}

impl RFWavePropagation {
    pub fn new(grid_size: usize, freq_hz: f32) -> Self {
        let speed_of_light = 3e8;
        let wavelength = speed_of_light / freq_hz;
        Self {
            grid: VoxelGrid::new(grid_size),
            frequency_hz: freq_hz,
            wavelength_m: wavelength,
            speed_of_light,
        }
    }

    /// Solve wave equation: ∇²E = -k²E (Helmholtz equation)
    /// Using Gauss-Seidel iterative relaxation (FDFD approach) for performance constraints
    pub fn solve_wave_equation(
        &mut self,
        source_position: (f32, f32, f32),
        source_amplitude: f32,
        material_grid: &VoxelGrid<Material>,
    ) -> Result<(), Box<dyn Error>> {
        let k = 2.0 * std::f32::consts::PI / self.wavelength_m;  // Wave number
        let k2 = k * k;
        let h = self.grid.voxel_size_m;
        let h2 = h * h;

        let dim_x = self.grid.dimensions.0;
        let dim_y = self.grid.dimensions.1;
        let dim_z = self.grid.dimensions.2;

        // Initial setup - plane/spherical wave source boundary condition
        let src_grid_pos = self.grid.world_to_grid(source_position);
        let sx = src_grid_pos.0.floor() as usize;
        let sy = src_grid_pos.1.floor() as usize;
        let sz = src_grid_pos.2.floor() as usize;

        if sx < dim_x && sy < dim_y && sz < dim_z {
            self.grid.set(sx, sy, sz, Complex::new(source_amplitude, 0.0));
        }

        // Gauss-Seidel relaxation loop
        let max_iterations = 20; // Bound for 169fps constraints
        let omega = 1.6; // Successive over-relaxation factor

        for _ in 0..max_iterations {
            for z in 1..dim_z-1 {
                for y in 1..dim_y-1 {
                    for x in 1..dim_x-1 {
                        // Skip source node to maintain injection boundary
                        if x == sx && y == sy && z == sz {
                            continue;
                        }

                        let material = material_grid.get(x, y, z);

                        // Local wave number taking material permittivity into account
                        let local_k = k * material.permittivity.sqrt();
                        let local_k2 = local_k * local_k;

                        // Attenuation term (absorption)
                        let alpha = material.attenuation_coeff(self.frequency_hz);
                        // Complex wave number for lossy media: k_complex = k - i*alpha
                        let k_complex = Complex::new(local_k, -alpha);
                        let k_complex2 = k_complex * k_complex;

                        let e_x_plus = self.grid.get(x + 1, y, z);
                        let e_x_minus = self.grid.get(x - 1, y, z);
                        let e_y_plus = self.grid.get(x, y + 1, z);
                        let e_y_minus = self.grid.get(x, y - 1, z);
                        let e_z_plus = self.grid.get(x, y, z + 1);
                        let e_z_minus = self.grid.get(x, y, z - 1);

                        // Finite difference approximation of Laplacian
                        let sum_neighbors = e_x_plus + e_x_minus +
                                          e_y_plus + e_y_minus +
                                          e_z_plus + e_z_minus;

                        // ∇²E = (E_x+1 + E_x-1 + E_y+1 + E_y-1 + E_z+1 + E_z-1 - 6*E_xyz) / h²
                        // ∇²E + k²E = 0
                        // (sum_neighbors - 6*E) / h² + k_c²*E = 0
                        // sum_neighbors / h² = E * (6/h² - k_c²)
                        // E = sum_neighbors / (6 - k_c² * h²)

                        let denominator = Complex::new(6.0, 0.0) - k_complex2 * Complex::new(h2, 0.0);

                        if denominator.norm_sqr() > 1e-10 {
                            let new_e = sum_neighbors / denominator;
                            let current_e = self.grid.get(x, y, z);

                            // SOR update
                            let updated_e = current_e * Complex::new(1.0 - omega, 0.0) + new_e * Complex::new(omega, 0.0);
                            self.grid.set(x, y, z, updated_e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get field magnitude (energy density) and phase at position
    pub fn field_at(&self, pos: (f32, f32, f32)) -> (f32, f32) {
        let complex = self.grid.sample(pos);
        (complex.norm(), complex.arg())
    }
}
