//! Window functions

#[derive(Debug, Clone, Copy)]
pub enum WindowType {
    Rectangular,
    Hann,
    Hamming,
    Blackman,
}

pub struct WindowFunction {
    window: Vec<f32>,
    window_type: WindowType,
}

impl WindowFunction {
    pub fn new(window_type: WindowType, size: usize) -> Self {
        let window = match window_type {
            WindowType::Rectangular => vec![1.0f32; size],
            WindowType::Hann => (0..size)
                .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (size - 1) as f32).cos()))
                .collect(),
            WindowType::Hamming => (0..size)
                .map(|i| 0.54 - 0.46 * (2.0 * std::f32::consts::PI * i as f32 / (size - 1) as f32).cos())
                .collect(),
            WindowType::Blackman => (0..size)
                .map(|i| {
                    let n = i as f32;
                    let m = (size - 1) as f32;
                    0.42 - 0.5 * (2.0 * std::f32::consts::PI * n / m).cos()
                        + 0.08 * (4.0 * std::f32::consts::PI * n / m).cos()
                })
                .collect(),
        };
        
        Self { window, window_type }
    }
    
    pub fn apply(&self, signal: &[f32]) -> Vec<f32> {
        signal.iter().zip(self.window.iter()).map(|(&s, &w)| s * w).collect()
    }
    
    pub fn coefficients(&self) -> &[f32] {
        &self.window
    }
    
    pub fn size(&self) -> usize {
        self.window.len()
    }
}
