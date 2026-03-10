//! Sample rate conversion (resampling)

/// Linear interpolator for simple resampling
pub struct LinearResampler {
    input_rate: u32,
    output_rate: u32,
    ratio: f64,
    phase: f64,
    last_sample: f32,
}

impl LinearResampler {
    /// Create new linear resampler
    pub fn new(input_rate: u32, output_rate: u32) -> Self {
        let ratio = output_rate as f64 / input_rate as f64;
        
        Self {
            input_rate,
            output_rate,
            ratio,
            phase: 0.0,
            last_sample: 0.0,
        }
    }
    
    /// Process one sample (returns Some if output sample available)
    pub fn process(&mut self, input: f32) -> Option<f32> {
        // Linear interpolation
        let phase_f32 = self.phase as f32;
        let output = self.last_sample * (1.0 - phase_f32) + input * phase_f32;
        
        self.phase += self.ratio;
        
        if self.phase >= 1.0 {
            self.phase -= 1.0;
            self.last_sample = input;
            Some(output)
        } else {
            None
        }
    }
    
    /// Resample a block of samples
    pub fn process_block(&mut self, input: &[f32]) -> Vec<f32> {
        let mut output = Vec::new();
        
        for &sample in input {
            if let Some(out) = self.process(sample) {
                output.push(out);
            }
        }
        
        output
    }
    
    /// Reset resampler state
    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.last_sample = 0.0;
    }
    
    /// Get input rate
    pub fn input_rate(&self) -> u32 {
        self.input_rate
    }
    
    /// Get output rate
    pub fn output_rate(&self) -> u32 {
        self.output_rate
    }
    
    /// Get resampling ratio
    pub fn ratio(&self) -> f64 {
        self.ratio
    }
}

/// Polyphase resampler for higher quality
pub struct PolyphaseResampler {
    input_rate: u32,
    output_rate: u32,
    /// Upsampling factor
    l: usize,
    /// Downsampling factor
    m: usize,
    /// Filter coefficients (polyphase)
    coefficients: Vec<Vec<f32>>,
    /// Delay line
    delay_line: Vec<f32>,
    /// Current position
    position: usize,
}

impl PolyphaseResampler {
    /// Create new polyphase resampler
    pub fn new(input_rate: u32, output_rate: u32, filter_order: usize) -> Self {
        // Find L and M such that output/input = L/M
        let gcd = Self::gcd(input_rate, output_rate);
        let l = (output_rate / gcd) as usize;
        let m = (input_rate / gcd) as usize;
        
        // Design low-pass filter for anti-aliasing
        let cutoff = (1.0 / l.max(m) as f32 * 0.9).min(0.45);
        let coefficients = Self::design_polyphase_filter(filter_order, l, cutoff);
        
        Self {
            input_rate,
            output_rate,
            l,
            m,
            coefficients,
            delay_line: vec![0.0f32; filter_order + 1],
            position: 0,
        }
    }
    
    /// Greatest common divisor
    fn gcd(a: u32, b: u32) -> u32 {
        if b == 0 {
            a
        } else {
            Self::gcd(b, a % b)
        }
    }
    
    /// Design polyphase filter
    fn design_polyphase_filter(order: usize, l: usize, cutoff: f32) -> Vec<Vec<f32>> {
        // Simplified: use windowed-sinc
        let m = order;
        let fc = cutoff;
        
        let mut coeffs = Vec::with_capacity(l);
        
        for phase in 0..l {
            let mut phase_coeffs = Vec::with_capacity(m + 1);
            
            for i in 0..=m {
                let n = i as f32 - m as f32 / 2.0 + phase as f32 / l as f32;
                
                let h = if n.abs() < 1e-10 {
                    2.0 * fc
                } else {
                    (2.0 * fc * (std::f32::consts::PI * fc * n).sin()) / (std::f32::consts::PI * n)
                };
                
                // Hamming window
                let window = 0.54 - 0.46 * (2.0 * std::f32::consts::PI * i as f32 / m as f32).cos();
                phase_coeffs.push(h * window);
            }
            
            coeffs.push(phase_coeffs);
        }
        
        coeffs
    }
    
    /// Process one sample
    pub fn process(&mut self, input: f32) -> Vec<f32> {
        // Shift delay line
        for i in (1..self.delay_line.len()).rev() {
            self.delay_line[i] = self.delay_line[i - 1];
        }
        self.delay_line[0] = input;
        
        let mut outputs = Vec::new();
        
        // Generate L output samples, keep every M-th
        for i in 0..self.l {
            let phase_idx = (self.position + i) % self.l;
            let coeffs = &self.coefficients[phase_idx];
            
            let mut output = 0.0f32;
            for (j, &coeff) in coeffs.iter().enumerate() {
                if j < self.delay_line.len() {
                    output += coeff * self.delay_line[j];
                }
            }
            
            // Output every M-th sample
            if (self.position + i) % self.m == 0 {
                outputs.push(output);
            }
        }
        
        self.position = (self.position + self.l) % (self.l * self.m);
        
        outputs
    }
    
    /// Reset resampler state
    pub fn reset(&mut self) {
        self.delay_line.fill(0.0);
        self.position = 0;
    }
}

/// Sample rate converter with automatic rate detection
pub struct Resampler {
    inner: ResamplerInner,
}

enum ResamplerInner {
    Linear(LinearResampler),
    Polyphase(PolyphaseResampler),
}

impl Resampler {
    /// Create linear resampler (fast, lower quality)
    pub fn linear(input_rate: u32, output_rate: u32) -> Self {
        Self {
            inner: ResamplerInner::Linear(LinearResampler::new(input_rate, output_rate)),
        }
    }
    
    /// Create polyphase resampler (slower, higher quality)
    pub fn polyphase(input_rate: u32, output_rate: u32) -> Self {
        Self {
            inner: ResamplerInner::Polyphase(PolyphaseResampler::new(input_rate, output_rate, 32)),
        }
    }
    
    /// Resample a block of samples
    pub fn process_block(&mut self, input: &[f32]) -> Vec<f32> {
        match &mut self.inner {
            ResamplerInner::Linear(r) => r.process_block(input),
            ResamplerInner::Polyphase(r) => {
                let mut output = Vec::new();
                for &sample in input {
                    output.extend(r.process(sample));
                }
                output
            }
        }
    }
    
    /// Reset resampler state
    pub fn reset(&mut self) {
        match &mut self.inner {
            ResamplerInner::Linear(r) => r.reset(),
            ResamplerInner::Polyphase(r) => r.reset(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_linear_upsample() {
        let mut resampler = LinearResampler::new(1000, 2000);
        
        let input = vec![1.0f32, 2.0, 3.0];
        let output = resampler.process_block(&input);
        
        // Upsampling by 2x should produce ~6 samples
        assert!(output.len() >= 5);
    }
    
    #[test]
    fn test_linear_downsample() {
        let mut resampler = LinearResampler::new(2000, 1000);
        
        let input = vec![1.0f32, 2.0, 3.0, 4.0];
        let output = resampler.process_block(&input);
        
        // Downsampling by 2x should produce ~2 samples
        assert!(output.len() >= 1);
    }
}
