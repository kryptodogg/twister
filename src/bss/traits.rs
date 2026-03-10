//! Adaptive filter traits

use ndarray::Array1;

pub trait AdaptiveFilter {
    fn new(order: usize, learning_rate: f32) -> Self;
    fn process(&mut self, input: f32, reference: f32) -> f32;
    fn process_block(&mut self, input: &[f32], reference: &[f32]) -> Vec<f32>;
    fn weights(&self) -> Array1<f32>;
    fn reset(&mut self);
    fn order(&self) -> usize;
}
