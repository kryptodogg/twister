//! Least Mean Squares (LMS) adaptive filter

use crate::bss::traits::AdaptiveFilter;
use ndarray::Array1;

/// LMS adaptive filter
pub struct LMSFilter {
    order: usize,
    mu: f32,
    w: Array1<f32>,
    x_buf: Array1<f32>,
}

impl LMSFilter {
    pub fn new(order: usize, learning_rate: f32) -> Self {
        Self {
            order,
            mu: learning_rate,
            w: Array1::zeros(order),
            x_buf: Array1::zeros(order),
        }
    }
    
    pub fn for_anc() -> Self {
        Self::new(64, 0.005)
    }
    
    pub fn process_sample(&mut self, input: f32, reference: f32) -> f32 {
        for i in (1..self.order).rev() {
            self.x_buf[i] = self.x_buf[i - 1];
        }
        self.x_buf[0] = reference;
        
        let y = self.w.dot(&self.x_buf);
        let error = input - y;
        
        for i in 0..self.order {
            self.w[i] += self.mu * error * self.x_buf[i];
        }
        
        error
    }
}

impl AdaptiveFilter for LMSFilter {
    fn new(order: usize, learning_rate: f32) -> Self {
        Self::new(order, learning_rate)
    }
    
    fn process(&mut self, input: f32, reference: f32) -> f32 {
        self.process_sample(input, reference)
    }
    
    fn process_block(&mut self, input: &[f32], reference: &[f32]) -> Vec<f32> {
        let len = input.len().min(reference.len());
        (0..len).map(|i| self.process_sample(input[i], reference[i])).collect()
    }
    
    fn weights(&self) -> Array1<f32> {
        self.w.clone()
    }
    
    fn reset(&mut self) {
        self.w.fill(0.0);
        self.x_buf.fill(0.0);
    }
    
    fn order(&self) -> usize {
        self.order
    }
}
