//! Burn ML backend for native Rust inference

use burn::backend::NdArray;
use burn::tensor::{Tensor, TensorData, Shape};
use burn::tensor::activation::{relu, gelu, silu};
use burn::nn::LayerNorm;
use burn::nn::layer_norm::LayerNormConfig;
use ndarray::{Array1, Array2, Array3};
use crate::utils::error::{Error, Result};

/// Burn backend wrapper
pub struct BurnBackend {
    device: burn_ndarray::NdArrayDevice,
}

impl BurnBackend {
    /// Create new Burn backend
    pub fn new() -> Self {
        Self {
            device: Default::default(),
        }
    }

    /// Convert ndarray to Burn tensor
    pub fn array_to_tensor_1d(&self, array: Array1<f32>) -> Tensor<NdArray, 1> {
        let shape = Shape::from([array.len()]);
        let data: Vec<f32> = array.into_raw_vec_and_offset().0;
        let tensor_data = TensorData::new(data, shape);
        Tensor::from_data(tensor_data, &self.device)
    }

    /// Convert ndarray to Burn tensor (2D)
    pub fn array_to_tensor_2d(&self, array: Array2<f32>) -> Tensor<NdArray, 2> {
        let (rows, cols) = array.dim();
        let shape = Shape::from([rows, cols]);
        let data: Vec<f32> = array.into_raw_vec_and_offset().0;
        let tensor_data = TensorData::new(data, shape);
        Tensor::from_data(tensor_data, &self.device)
    }

    /// Convert ndarray to Burn tensor (3D)
    pub fn array_to_tensor_3d(&self, array: Array3<f32>) -> Tensor<NdArray, 3> {
        let (d0, d1, d2) = array.dim();
        let shape = Shape::from([d0, d1, d2]);
        let data: Vec<f32> = array.into_raw_vec_and_offset().0;
        let tensor_data = TensorData::new(data, shape);
        Tensor::from_data(tensor_data, &self.device)
    }

    /// Convert Burn tensor to ndarray (1D)
    pub fn tensor_to_array_1d(&self, tensor: Tensor<NdArray, 1>) -> Array1<f32> {
        let data = tensor.into_data();
        let shape = data.shape();
        let vec: Vec<f32> = data.as_slice().expect("Burn tensor data should be contiguous").to_vec();
        Array1::from_shape_vec(shape.dims(), vec).expect("Shape and vector length mismatch")
    }

    /// Convert Burn tensor to ndarray (2D)
    pub fn tensor_to_array_2d(&self, tensor: Tensor<NdArray, 2>) -> Array2<f32> {
        let data = tensor.into_data();
        let shape = data.shape();
        let vec: Vec<f32> = data.as_slice().expect("Burn tensor data should be contiguous").to_vec();
        Array2::from_shape_vec(shape.dims(), vec).expect("Shape and vector length mismatch")
    }

    /// Convert Burn tensor to ndarray (3D)
    pub fn tensor_to_array_3d(&self, tensor: Tensor<NdArray, 3>) -> Array3<f32> {
        let data = tensor.into_data();
        let shape = data.shape();
        let vec: Vec<f32> = data.as_slice().expect("Burn tensor data should be contiguous").to_vec();
        Array3::from_shape_vec(shape.dims(), vec).expect("Shape and vector length mismatch")
    }

    /// Apply layer normalization
    pub fn layer_norm(&self, tensor: Tensor<NdArray, 3>) -> Tensor<NdArray, 3> {
        let d_model = tensor.dims()[2];
        let ln = LayerNormConfig::new(d_model).init(&self.device);
        ln.forward(tensor)
    }

    /// Apply GELU activation
    pub fn gelu(&self, tensor: Tensor<NdArray, 3>) -> Tensor<NdArray, 3> {
        gelu(tensor)
    }

    /// Apply ReLU activation
    pub fn relu(&self, tensor: Tensor<NdArray, 3>) -> Tensor<NdArray, 3> {
        relu(tensor)
    }

    /// Apply SiLU activation
    pub fn silu(&self, tensor: Tensor<NdArray, 3>) -> Tensor<NdArray, 3> {
        silu(tensor)
    }

    /// Matrix multiplication
    pub fn matmul(&self, a: Tensor<NdArray, 3>, b: Tensor<NdArray, 3>) -> Tensor<NdArray, 3> {
        a.matmul(b)
    }

    /// Get device
    pub fn device(&self) -> &burn_ndarray::NdArrayDevice {
        &self.device
    }
}

impl Default for BurnBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Model runner for Burn models
pub struct ModelRunner {
    backend: BurnBackend,
}

impl ModelRunner {
    /// Create new model runner
    pub fn new() -> Self {
        Self {
            backend: BurnBackend::new(),
        }
    }

    /// Run inference on input
    pub fn run(&self, input: Array3<f32>) -> Result<Array3<f32>> {
        let tensor = self.backend.array_to_tensor_3d(input);

        // Placeholder: identity pass-through
        // In practice, this would run the actual model
        let output = tensor;

        Ok(self.backend.tensor_to_array_3d(output))
    }
}

impl Default for ModelRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_creation() {
        let backend = BurnBackend::new();
        // Just verify it compiles and creates
        assert!(backend.device().is_cpu());
    }

    #[test]
    fn test_tensor_conversion() {
        let backend = BurnBackend::new();

        let array = Array3::from_shape_vec((2, 3, 4), vec![1.0f32; 24]).unwrap();
        let tensor = backend.array_to_tensor_3d(array.clone());
        let back = backend.tensor_to_array_3d(tensor);

        assert_eq!(array.dim(), back.dim());
    }
}
