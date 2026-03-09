// src/features/mod.rs — Multimodal Feature Extraction
//
// Phase 2 ANALYSIS tab infrastructure: Extraction of multimodal features
// for TimeGNN training and harassment signature discovery.
//
// Components:
// - Audio: 196-D (STFT Mel + TDOA + Sparse PDM + Bispectrum + Wave Coherence + Musical)
// - RF: 128-D (placeholder, Task A.2)
// - Visual: 64-D (placeholder, Task A.3)
// - Concatenated: 448-D (placeholder, Task A.4)

pub mod audio;

pub use audio::{AudioFeatures, extract_audio_features};
