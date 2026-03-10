//! Recursive Least Squares (RLS) adaptive filter

use crate::bss::traits::AdaptiveFilter;
use ndarray::{Array1, Array2};

/// RLS adaptive filter configuration
#[derive(Debug, Clone)]
pub struct RLSConfig {
    pub order: usize,
    pub forgetting_factor: f32,
    pub delta: f32,
}

impl Default for RLSConfig {
    fn default() -> Self {
        Self {
            order: 32,
            forgetting_factor: 0.99,
            delta: 0.001,
        }
    }
}

/// Recursive Least Squares adaptive filter
pub struct RLSFilter {
    order: usize,
    lambda: f32,
    p: Array2<f32>,
    w: Array1<f32>,
    x_buf: Array1<f32>,
    n: usize,
}

impl RLSFilter {
    pub fn with_config(config: RLSConfig) -> Self {
        let order = config.order;
        let lambda = config.forgetting_factor;
        let delta = config.delta;
        
        let mut p = Array2::zeros((order, order));
        for i in 0..order {
            p[[i, i]] = 1.0 / delta;
        }
        
        Self {
            order,
            lambda,
            p,
            w: Array1::zeros(order),
            x_buf: Array1::zeros(order),
            n: 0,
        }
    }
    
    pub fn new(order: usize, forgetting_factor: f32) -> Self {
        Self::with_config(RLSConfig {
            order,
            forgetting_factor,
            delta: 0.001,
        })
    }
    
    pub fn for_anc() -> Self {
        Self::with_config(RLSConfig {
            order: 64,
            forgetting_factor: 0.995,
            delta: 0.0001,
        })
    }
    
    pub fn process_sample(&mut self, input: f32, reference: f32) -> f32 {
        // Shift delay line
        for i in (1..self.order).rev() {
            self.x_buf[i] = self.x_buf[i - 1];
        }
        self.x_buf[0] = reference;
        
        // Compute a priori error
        let y = self.w.dot(&self.x_buf);
        let error = input - y;
        
        // Compute gain vector
        let px = self.p.dot(&self.x_buf);
        let denominator = self.lambda + self.x_buf.dot(&px);
        
        if denominator.abs() < 1e-10 {
            return error;
        }
        
        let k = px / denominator;
        
        // Update weights
        for i in 0..self.order {
            self.w[i] += k[i] * error;
        }
        
        // Update inverse correlation matrix (simplified)
        for i in 0..self.order {
            for j in 0..self.order {
                self.p[[i, j]] = (self.p[[i, j]] - k[i] * self.x_buf[j]) / self.lambda;
            }
        }
        
        self.n += 1;
        error
    }
}

impl AdaptiveFilter for RLSFilter {
    fn new(order: usize, _learning_rate: f32) -> Self {
        Self::new(order, 0.99)
    }
    
    fn process(&mut self, input: f32, reference: f32) -> f32 {
        self.process_sample(input, reference)
    }
    
    fn process_block(&mut self, input: &[f32], reference: &[f32]) -> Vec<f32> {
        let len = input.len().min(reference.len());
        let mut output = Vec::with_capacity(len);
        
        for i in 0..len {
            output.push(self.process_sample(input[i], reference[i]));
        }
        
        output
    }
    
    fn weights(&self) -> Array1<f32> {
        self.w.clone()
    }
    
    fn reset(&mut self) {
        for i in 0..self.order {
            for j in 0..self.order {
                self.p[[i, j]] = if i == j { 1.0 / 0.001 } else { 0.0 };
            }
        }
        self.w.fill(0.0);
        self.x_buf.fill(0.0);
        self.n = 0;
    }
    
    fn order(&self) -> usize {
        self.order
    }
}
