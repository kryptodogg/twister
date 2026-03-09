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
