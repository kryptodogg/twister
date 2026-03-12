//! Digital Signal Processing — V3 Ingestion Pipeline
//!
//! # V3 Architecture Note
//! PSD, TDOA, and feature extraction moved downstream of inference.
//! Ingestion now does minimal preprocessing — FFT and wavelets only.
//! Post-processing happens on the 3D point cloud (Track G).

pub mod bss;
pub mod fft;
pub mod filters;
pub mod resample;
pub mod wavelets;
pub mod window;
// tdoa, features, psd deleted — V3 moves downstream of inference

pub use bss::BSSProcessor;
pub use fft::FFTProcessor;
pub use wavelets::{WaveletProcessor, WaveletFamily};
// TDOA, FeatureVector, WelchPSD, PSDConfig deleted with modules
