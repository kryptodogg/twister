use burn::tensor::backend::Backend;
use burn::nn::{Linear, LinearConfig};
use burn::module::Module;

#[derive(Module, Debug)]
pub struct MyModel<B: Backend> {
    fc1: Linear<B>,
}

impl<B: Backend> MyModel<B> {
    pub fn new(device: &B::Device) -> Self {
        Self {
            fc1: LinearConfig::new(196, 256).init(device),
        }
    }
}
