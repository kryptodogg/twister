//! Tensor conversion utilities

use ndarray::{Array1, Array2, Array3, Array4, Ix1, Ix2, Ix3, Ix4};

/// Tensor shape descriptor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TensorShape {
    pub dims: [usize; 4],
    pub ndim: usize,
}

impl TensorShape {
    /// Create 1D shape
    pub fn from_1d(n: usize) -> Self {
        Self {
            dims: [n, 1, 1, 1],
            ndim: 1,
        }
    }
    
    /// Create 2D shape
    pub fn from_2d(rows: usize, cols: usize) -> Self {
        Self {
            dims: [rows, cols, 1, 1],
            ndim: 2,
        }
    }
    
    /// Create 3D shape
    pub fn from_3d(d0: usize, d1: usize, d2: usize) -> Self {
        Self {
            dims: [d0, d1, d2, 1],
            ndim: 3,
        }
    }
    
    /// Create 4D shape (NCHW)
    pub fn from_4d(n: usize, c: usize, h: usize, w: usize) -> Self {
        Self {
            dims: [n, c, h, w],
            ndim: 4,
        }
    }
    
    /// Get total number of elements
    pub fn numel(&self) -> usize {
        self.dims.iter().take(self.ndim).product()
    }
}

/// Tensor conversion utilities for ndarray
pub struct TensorConverter;

impl TensorConverter {
    /// Convert 1D array to 2D (batch, features)
    pub fn to_batched(array: Array1<f32>) -> Array2<f32> {
        let len = array.len();
        array.into_shape_with_order((1, len)).unwrap()
    }
    
    /// Convert 2D array to 3D (batch, seq, features)
    pub fn to_sequence(array: Array2<f32>) -> Array3<f32> {
        let (rows, cols) = array.dim();
        array.into_shape_with_order((1, rows, cols)).unwrap()
    }
    
    /// Convert audio samples to spectrogram-like tensor
    pub fn samples_to_tensor(samples: &[f32], seq_len: usize, n_features: usize) -> Array3<f32> {
        let n_frames = samples.len() / (seq_len * n_features);
        let mut tensor = Array3::zeros((n_frames, seq_len, n_features));
        
        for i in 0..n_frames {
            for j in 0..seq_len {
                for k in 0..n_features {
                    let idx = i * seq_len * n_features + j * n_features + k;
                    if idx < samples.len() {
                        tensor[[i, j, k]] = samples[idx];
                    }
                }
            }
        }
        
        tensor
    }
    
    /// Flatten 3D tensor to 1D
    pub fn flatten_3d(tensor: Array3<f32>) -> Array1<f32> {
        let len = tensor.len();
        tensor.into_shape_with_order((len,)).unwrap()
    }
    
    /// Normalize tensor (zero mean, unit variance)
    pub fn normalize(mut tensor: Array3<f32>) -> Array3<f32> {
        let mean = tensor.mean().unwrap_or(0.0);
        let std = tensor.mapv(|x| (x - mean).powi(2)).mean().unwrap_or(1.0).sqrt();
        
        if std > 1e-10 {
            tensor.mapv_inplace(|x| (x - mean) / std);
        }
        
        tensor
    }
    
    /// Apply min-max normalization
    pub fn normalize_minmax(mut tensor: Array3<f32>) -> Array3<f32> {
        let min = tensor.fold(f32::INFINITY, |a, &b| a.min(b));
        let max = tensor.fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        
        let range = max - min;
        if range > 1e-10 {
            tensor.mapv_inplace(|x| (x - min) / range);
        }
        
        tensor
    }
    
    /// Convert stereo to mono
    pub fn stereo_to_mono(left: &[f32], right: &[f32]) -> Vec<f32> {
        left.iter()
            .zip(right.iter())
            .map(|(&l, &r)| (l + r) / 2.0)
            .collect()
    }
    
    /// Convert mono to stereo
    pub fn mono_to_stereo(mono: &[f32]) -> Vec<f32> {
        mono.iter()
            .flat_map(|&m| vec![m, m])
            .collect()
    }
    
    /// Interleave channels
    pub fn interleave(channels: &[&[f32]]) -> Vec<f32> {
        if channels.is_empty() {
            return vec![];
        }
        
        let n_channels = channels.len();
        let n_samples = channels.iter().map(|c| c.len()).min().unwrap_or(0);
        
        let mut interleaved = Vec::with_capacity(n_channels * n_samples);
        
        for i in 0..n_samples {
            for channel in channels {
                interleaved.push(channel[i]);
            }
        }
        
        interleaved
    }
    
    /// Deinterleave channels
    pub fn deinterleave(interleaved: &[f32], n_channels: usize) -> Vec<Vec<f32>> {
        let n_samples = interleaved.len() / n_channels;
        let mut channels = vec![Vec::with_capacity(n_samples); n_channels];
        
        for (i, &sample) in interleaved.iter().enumerate() {
            channels[i % n_channels].push(sample);
        }
        
        channels
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_stereo_to_mono() {
        let left = vec![1.0, 2.0, 3.0];
        let right = vec![4.0, 5.0, 6.0];
        
        let mono = TensorConverter::stereo_to_mono(&left, &right);
        
        assert_eq!(mono, vec![2.5, 3.5, 4.5]);
    }
    
    #[test]
    fn test_mono_to_stereo() {
        let mono = vec![1.0, 2.0, 3.0];
        
        let stereo = TensorConverter::mono_to_stereo(&mono);
        
        assert_eq!(stereo, vec![1.0, 1.0, 2.0, 2.0, 3.0, 3.0]);
    }
    
    #[test]
    fn test_interleave() {
        let ch1 = vec![1.0, 2.0];
        let ch2 = vec![3.0, 4.0];
        
        let interleaved = TensorConverter::interleave(&[&ch1, &ch2]);
        
        assert_eq!(interleaved, vec![1.0, 3.0, 2.0, 4.0]);
    }
    
    #[test]
    fn test_deinterleave() {
        let interleaved = vec![1.0, 3.0, 2.0, 4.0];
        
        let channels = TensorConverter::deinterleave(&interleaved, 2);
        
        assert_eq!(channels[0], vec![1.0, 2.0]);
        assert_eq!(channels[1], vec![3.0, 4.0]);
    }
}
