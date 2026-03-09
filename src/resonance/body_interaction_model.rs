use num_complex::Complex;
use crate::physics::voxel_grid::VoxelGrid;
use crate::visualization::data_contracts::PoseFrame;

pub struct HumanBody {
    pub skeleton: PoseFrame,  // 33 keypoints
    pub voxel_map: VoxelGrid<f32>,  // Occupancy per voxel (0.0-1.0)
}

impl HumanBody {
    pub fn from_pose(pose: &PoseFrame, grid_size: usize, voxel_size_m: f32) -> Self {
        // Voxelize skeleton + simple cylinders (arms, legs, torso)
        // Muscle tissue: ε_r ≈ 50, σ ≈ 1.0 S/m (very lossy)
        let mut voxel_map = VoxelGrid::new(grid_size);
        voxel_map.voxel_size_m = voxel_size_m;

        // Simple rasterization of keypoints into voxels
        for keypoint in &pose.keypoints {
            let (gx, gy, gz) = voxel_map.world_to_grid(*keypoint);
            let x = gx.floor() as usize;
            let y = gy.floor() as usize;
            let z = gz.floor() as usize;

            // Render basic cylinder/point
            // Set occupancy to 1.0 for human tissue, 0.0 for air
            // For now, we just set the exact keypoint voxel and immediate neighbors
            for dx in 0..=1 {
                for dy in 0..=1 {
                    for dz in 0..=1 {
                        voxel_map.set(
                            x.saturating_add(dx),
                            y.saturating_add(dy),
                            z.saturating_add(dz),
                            1.0
                        );
                    }
                }
            }
        }

        Self {
            skeleton: pose.clone(),
            voxel_map
        }
    }

    /// RF field attenuation due to human body (major effect)
    /// Human tissue @ 2.4 GHz: ~50-70% absorption per 10cm
    pub fn attenuate_rf_field(&self, field: Complex<f32>, distance: f32, _freq_hz: f32) -> Complex<f32> {
        // Muscle tissue attenuation: ~0.3-0.5 nepers/cm at 2.4 GHz
        let muscle_attenuation_per_m = 35.0;  // Empirical
        let occupancy_factor = self.voxel_map.average_along_path(distance);

        let loss = (-muscle_attenuation_per_m * distance * occupancy_factor).exp();
        field * loss
    }

    /// Position-dependent body shielding
    /// Azimuth facing away from RF source? Shield increases.
    pub fn shielding_factor(&self, rf_azimuth: f32, body_facing: f32) -> f32 {
        let angle_to_rf = (rf_azimuth - body_facing).abs();
        // Facing away (180°): maximum shielding (~0.7)
        // Facing toward (0°): minimum shielding (~0.2)
        0.2 + 0.5 * (-angle_to_rf.abs() / std::f32::consts::PI).exp()
    }
}
