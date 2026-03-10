//! Digital filter implementations

/// FIR filter
pub struct FIRFilter {
    coefficients: Vec<f32>,
    delay_line: Vec<f32>,
}

impl FIRFilter {
    pub fn lowpass(cutoff: f32, sample_rate: u32, order: usize) -> Self {
        let fc = cutoff / (sample_rate as f32 / 2.0);
        let m = order as f32;
        
        let mut coeffs = Vec::with_capacity(order + 1);
        for i in 0..=order {
            let n = i as f32 - m / 2.0;
            let h = if n == 0.0 {
                2.0 * fc
            } else {
                (2.0 * fc * (std::f32::consts::PI * fc * n).sin()) / (std::f32::consts::PI * n)
            };
            let window = 0.54 - 0.46 * (2.0 * std::f32::consts::PI * i as f32 / m).cos();
            coeffs.push(h * window);
        }
        
        let sum: f32 = coeffs.iter().sum();
        if sum > 0.0 {
            coeffs.iter_mut().for_each(|c| *c /= sum);
        }
        
        Self {
            coefficients: coeffs,
            delay_line: vec![0.0f32; order + 1],
        }
    }
    
    pub fn process(&mut self, input: f32) -> f32 {
        for i in (1..self.delay_line.len()).rev() {
            self.delay_line[i] = self.delay_line[i - 1];
        }
        self.delay_line[0] = input;
        
        let mut output = 0.0f32;
        for (i, &coeff) in self.coefficients.iter().enumerate() {
            output += coeff * self.delay_line[i];
        }
        output
    }
    
    pub fn process_block(&mut self, input: &[f32]) -> Vec<f32> {
        input.iter().map(|&s| self.process(s)).collect()
    }
    
    pub fn reset(&mut self) {
        self.delay_line.fill(0.0);
    }
}

/// IIR filter (biquad)
pub struct IIRFilter {
    b: [f32; 3],
    a: [f32; 3],
    x_delay: [f32; 2],
    y_delay: [f32; 2],
}

impl IIRFilter {
    pub fn lowpass(cutoff: f32, sample_rate: u32, _q: f32) -> Self {
        let fc = cutoff / sample_rate as f32;
        let w0 = 2.0 * std::f32::consts::PI * fc;
        let cos_w0 = w0.cos();
        
        let a0 = 1.0;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0;
        let b0 = (1.0 - cos_w0) / 2.0;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) / 2.0;
        
        Self {
            b: [b0/a0, b1/a0, b2/a0],
            a: [1.0, a1/a0, a2/a0],
            x_delay: [0.0; 2],
            y_delay: [0.0; 2],
        }
    }
    
    pub fn process(&mut self, input: f32) -> f32 {
        let output = self.b[0] * input
            + self.b[1] * self.x_delay[0]
            + self.b[2] * self.x_delay[1]
            - self.a[1] * self.y_delay[0]
            - self.a[2] * self.y_delay[1];
        
        self.x_delay[1] = self.x_delay[0];
        self.x_delay[0] = input;
        self.y_delay[1] = self.y_delay[0];
        self.y_delay[0] = output;
        
        output
    }
    
    pub fn process_block(&mut self, input: &[f32]) -> Vec<f32> {
        input.iter().map(|&s| self.process(s)).collect()
    }
    
    pub fn reset(&mut self) {
        self.x_delay.fill(0.0);
        self.y_delay.fill(0.0);
    }
}
