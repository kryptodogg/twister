use num_complex::Complex;
use std::fmt::Debug;

#[derive(Clone, Debug)]
#[repr(C)] // Ensures memory alignment for future GPU mapping
pub struct VoxelGrid<T: Clone> {
    pub data: Vec<T>,
    pub dimensions: (usize, usize, usize), // (X, Y, Z)
    pub voxel_size_m: f32,
    pub origin: (f32, f32, f32),
}

impl<T: Clone + Default> VoxelGrid<T> {
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![T::default(); size * size * size],
            dimensions: (size, size, size),
            voxel_size_m: 0.1, // 10cm default
            origin: (0.0, 0.0, 0.0),
        }
    }

    pub fn with_dimensions_and_size(
        dim_x: usize,
        dim_y: usize,
        dim_z: usize,
        voxel_size_m: f32,
        origin: (f32, f32, f32),
    ) -> Self {
        Self {
            data: vec![T::default(); dim_x * dim_y * dim_z],
            dimensions: (dim_x, dim_y, dim_z),
            voxel_size_m,
            origin,
        }
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, value: T) {
        if x < self.dimensions.0 && y < self.dimensions.1 && z < self.dimensions.2 {
            let idx = self.index(x, y, z);
            self.data[idx] = value;
        }
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> T {
        if x < self.dimensions.0 && y < self.dimensions.1 && z < self.dimensions.2 {
            let idx = self.index(x, y, z);
            self.data[idx].clone()
        } else {
            T::default()
        }
    }

    pub fn get_mut(&mut self, x: usize, y: usize, z: usize) -> Option<&mut T> {
        if x < self.dimensions.0 && y < self.dimensions.1 && z < self.dimensions.2 {
            let idx = self.index(x, y, z);
            Some(&mut self.data[idx])
        } else {
            None
        }
    }

    pub fn get_ref(&self, x: usize, y: usize, z: usize) -> Option<&T> {
        if x < self.dimensions.0 && y < self.dimensions.1 && z < self.dimensions.2 {
            let idx = self.index(x, y, z);
            Some(&self.data[idx])
        } else {
            None
        }
    }

    pub fn index(&self, x: usize, y: usize, z: usize) -> usize {
        x + y * self.dimensions.0 + z * self.dimensions.0 * self.dimensions.1
    }

    pub fn world_to_grid(&self, pos: (f32, f32, f32)) -> (f32, f32, f32) {
        (
            (pos.0 - self.origin.0) / self.voxel_size_m,
            (pos.1 - self.origin.1) / self.voxel_size_m,
            (pos.2 - self.origin.2) / self.voxel_size_m,
        )
    }
}

// Implement specifically for Complex<f32> and f32 since those need lerping

impl VoxelGrid<Complex<f32>> {
    pub fn sample(&self, pos: (f32, f32, f32)) -> Complex<f32> {
        let (gx, gy, gz) = self.world_to_grid(pos);

        let x0 = gx.floor() as isize;
        let y0 = gy.floor() as isize;
        let z0 = gz.floor() as isize;

        let x1 = x0 + 1;
        let y1 = y0 + 1;
        let z1 = z0 + 1;

        let xd = gx - x0 as f32;
        let yd = gy - y0 as f32;
        let zd = gz - z0 as f32;

        // Trilinear interpolation for Complex<f32>
        let get_val = |x: isize, y: isize, z: isize| -> Complex<f32> {
            if x >= 0
                && x < self.dimensions.0 as isize
                && y >= 0
                && y < self.dimensions.1 as isize
                && z >= 0
                && z < self.dimensions.2 as isize
            {
                self.get(x as usize, y as usize, z as usize)
            } else {
                Complex::new(0.0, 0.0)
            }
        };

        let c000 = get_val(x0, y0, z0);
        let c100 = get_val(x1, y0, z0);
        let c010 = get_val(x0, y1, z0);
        let c110 = get_val(x1, y1, z0);
        let c001 = get_val(x0, y0, z1);
        let c101 = get_val(x1, y0, z1);
        let c011 = get_val(x0, y1, z1);
        let c111 = get_val(x1, y1, z1);

        let c00 = c000 * (1.0 - xd) + c100 * xd;
        let c10 = c010 * (1.0 - xd) + c110 * xd;
        let c01 = c001 * (1.0 - xd) + c101 * xd;
        let c11 = c011 * (1.0 - xd) + c111 * xd;

        let c0 = c00 * (1.0 - yd) + c10 * yd;
        let c1 = c01 * (1.0 - yd) + c11 * yd;

        c0 * (1.0 - zd) + c1 * zd
    }
}

impl VoxelGrid<f32> {
    pub fn sample(&self, pos: (f32, f32, f32)) -> f32 {
        let (gx, gy, gz) = self.world_to_grid(pos);

        let x0 = gx.floor() as isize;
        let y0 = gy.floor() as isize;
        let z0 = gz.floor() as isize;

        let x1 = x0 + 1;
        let y1 = y0 + 1;
        let z1 = z0 + 1;

        let xd = gx - x0 as f32;
        let yd = gy - y0 as f32;
        let zd = gz - z0 as f32;

        let get_val = |x: isize, y: isize, z: isize| -> f32 {
            if x >= 0
                && x < self.dimensions.0 as isize
                && y >= 0
                && y < self.dimensions.1 as isize
                && z >= 0
                && z < self.dimensions.2 as isize
            {
                self.get(x as usize, y as usize, z as usize)
            } else {
                0.0
            }
        };

        let c000 = get_val(x0, y0, z0);
        let c100 = get_val(x1, y0, z0);
        let c010 = get_val(x0, y1, z0);
        let c110 = get_val(x1, y1, z0);
        let c001 = get_val(x0, y0, z1);
        let c101 = get_val(x1, y0, z1);
        let c011 = get_val(x0, y1, z1);
        let c111 = get_val(x1, y1, z1);

        let c00 = c000 * (1.0 - xd) + c100 * xd;
        let c10 = c010 * (1.0 - xd) + c110 * xd;
        let c01 = c001 * (1.0 - xd) + c101 * xd;
        let c11 = c011 * (1.0 - xd) + c111 * xd;

        let c0 = c00 * (1.0 - yd) + c10 * yd;
        let c1 = c01 * (1.0 - yd) + c11 * yd;

        c0 * (1.0 - zd) + c1 * zd
    }

    pub fn average_along_path(&self, _distance: f32) -> f32 {
        // Average occupancy
        1.0
    }
}
