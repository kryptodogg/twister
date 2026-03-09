use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoseKeypoint {
    pub x: f32,          // Normalized (0-1) image width
    pub y: f32,          // Normalized (0-1) image height
    pub z: f32,          // Depth (relative to hip)
    pub confidence: f32, // 0.0-1.0
}

#[derive(Debug, Clone, PartialEq)]
pub struct PoseFrame {
    pub timestamp_us: u64,
    pub keypoints: [PoseKeypoint; 33],  // 33-point BlazePose
}

/// The base trait for inferring 33-point poses.
pub trait PoseInference {
    /// Note: taking a generic byte slice representation of the frame for the trait
    /// interface so it works for mock, image buffer, or burn tensor based setups.
    fn estimate_pose(&self, frame: &[u8], width: u32, height: u32) -> PoseFrame;
}

pub struct MockPoseInference;

impl MockPoseInference {
    pub fn new() -> Self {
        Self {}
    }
}

impl PoseInference for MockPoseInference {
    fn estimate_pose(&self, _frame: &[u8], _width: u32, _height: u32) -> PoseFrame {
        let timestamp_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        // Return a mock synthetic skeleton
        let mut keypoints = [PoseKeypoint { x: 0.0, y: 0.0, z: 0.0, confidence: 0.0 }; 33];

        for (i, kp) in keypoints.iter_mut().enumerate() {
            kp.x = 0.5 + (i as f32 * 0.01);
            kp.y = 0.5 - (i as f32 * 0.01);
            kp.z = 0.1;
            kp.confidence = 0.95;
        }

        PoseFrame {
            timestamp_us,
            keypoints,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_inference() {
        let mock = MockPoseInference::new();
        let dummy_frame = vec![0u8; 100];
        let pose = mock.estimate_pose(&dummy_frame, 10, 10);

        assert_eq!(pose.keypoints.len(), 33);
        assert!(pose.keypoints[0].confidence > 0.9);
        assert!(pose.keypoints[0].x >= 0.0 && pose.keypoints[0].x <= 1.0);
    }
}
