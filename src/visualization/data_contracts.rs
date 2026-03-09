#[derive(Clone, Debug, Default)]
pub struct PoseFrame {
    pub keypoints: Vec<(f32, f32, f32)>, // 33 keypoints
}

#[derive(Clone, Debug)]
pub struct RoomGeometry {
    pub min_bound: (f32, f32, f32),
    pub max_bound: (f32, f32, f32),
}

impl Default for RoomGeometry {
    fn default() -> Self {
        Self {
            min_bound: (-5.0, 0.0, -5.0),
            max_bound: (5.0, 3.0, 5.0),
        }
    }
}
