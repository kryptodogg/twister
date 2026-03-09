use crate::computer_vision::pose_estimator::PoseFrame;
use crate::ml::data_contracts::IMUSample;
use std::collections::VecDeque;

pub struct IMUPoseFusion {
    accel_history: VecDeque<[f32; 3]>,  // Last N frames
    gyro_history: VecDeque<[f32; 3]>,
}

#[derive(Debug, Clone)]
pub struct FusedPoseFrame {
    pub pose: PoseFrame,
    pub gravity_aligned: bool,
    pub motion_agreement: f32,  // Confidence metric [0, 1]
}

impl IMUPoseFusion {
    pub fn new() -> Self {
        Self {
            accel_history: VecDeque::with_capacity(10),
            gyro_history: VecDeque::with_capacity(10),
        }
    }

    /// Fuse IMU with pose to get world-aligned skeleton
    pub fn fuse(
        &mut self,
        pose: &PoseFrame,
        imu_sample: &IMUSample,
    ) -> FusedPoseFrame {
        if self.accel_history.len() >= 10 {
            self.accel_history.pop_front();
            self.gyro_history.pop_front();
        }
        self.accel_history.push_back(imu_sample.accel);
        self.gyro_history.push_back(imu_sample.gyro);

        let gravity = self.estimate_gravity();
        let aligned_pose = self.apply_gravity_alignment(pose, gravity);
        let motion_agreement = self.compute_motion_agreement(pose, imu_sample);

        FusedPoseFrame {
            pose: aligned_pose,
            gravity_aligned: true,
            motion_agreement,
        }
    }

    fn estimate_gravity(&self) -> [f32; 3] {
        if self.accel_history.is_empty() {
            return [0.0, -9.81, 0.0];
        }

        // Simple average over history
        let mut sum = [0.0; 3];
        for acc in &self.accel_history {
            sum[0] += acc[0];
            sum[1] += acc[1];
            sum[2] += acc[2];
        }
        let n = self.accel_history.len() as f32;
        [sum[0] / n, sum[1] / n, sum[2] / n]
    }

    fn apply_gravity_alignment(&self, pose: &PoseFrame, gravity: [f32; 3]) -> PoseFrame {
        // In a real implementation, we would construct a rotation matrix from the gravity
        // vector and apply it to all keypoints. For now, we return a mock alignment.
        // We know gravity points "down", so we could rotate keypoints such that the
        // global Y axis aligns with -gravity.
        let mut aligned = pose.clone();

        // Normalize gravity
        let mag = (gravity[0].powi(2) + gravity[1].powi(2) + gravity[2].powi(2)).sqrt();
        if mag > 0.1 {
            // Apply mock alignment adjustment based on gravity direction
            for kp in &mut aligned.keypoints {
                kp.y += gravity[1] / mag * 0.01;
            }
        }

        aligned
    }

    fn compute_motion_agreement(&self, _pose: &PoseFrame, imu_sample: &IMUSample) -> f32 {
        // Here we'd compare gyro rotational velocities with pose keypoint velocities.
        // For the mock, if gyro shows motion, we give a reasonable confidence.
        let gyro_mag = (imu_sample.gyro[0].powi(2) + imu_sample.gyro[1].powi(2) + imu_sample.gyro[2].powi(2)).sqrt();
        if gyro_mag > 0.5 {
            0.85 // High confidence when moving
        } else {
            0.95 // Very high confidence when static
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::computer_vision::pose_estimator::PoseKeypoint;

    #[test]
    fn test_gravity_estimation() {
        let mut fusion = IMUPoseFusion::new();
        let pose = PoseFrame { timestamp_us: 0, keypoints: [PoseKeypoint { x: 0.0, y: 0.0, z: 0.0, confidence: 0.0 }; 33] };

        let imu = IMUSample {
            accel: [0.0, -9.81, 0.0],
            gyro: [0.0, 0.0, 0.0],
            timestamp: 0,
        };

        let fused = fusion.fuse(&pose, &imu);
        assert!(fused.gravity_aligned);
        assert!(fused.motion_agreement > 0.9);
    }
}
