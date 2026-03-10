// Increase recursion limit for complex generic types (Autodiff<Wgpu> trait resolution)
#![recursion_limit = "256"]

pub mod active_denial;
pub mod physics;
pub mod spatial;
// src/lib.rs — Library exports for forensic queries tests

// Core modules needed for forensic query API
pub mod af32;
pub mod analysis_mock_data;
pub mod anc;
pub mod anc_calibration;
pub mod anc_recording;
pub mod async_event_handler;
pub mod audio;
pub mod bispectrum;
pub mod detection;
pub mod dispatch_kernel;
pub mod embeddings;
pub mod evidence_export;
pub mod features;
pub mod forensic;
pub mod fusion;
pub mod gpu;
pub mod gpu_memory;
pub mod gpu_shared;
pub mod graph;
pub mod harmony;
pub mod mamba;
pub mod parametric;
pub mod particle_system;
pub mod pdm;
pub mod reconstruct;
pub mod resample;
pub mod ridge_plot;
#[cfg(feature = "rtlsdr")]
pub mod rtlsdr;
#[cfg(feature = "rtlsdr")]
pub mod rtlsdr_ffi;
#[cfg(feature = "rtlsdr")]
pub mod safe_sdr_wrapper;
pub mod sdr;
pub mod state;
pub mod testing;
pub mod trainer;
pub mod training;
pub mod training_tests;
pub mod twister;
pub mod ui;
pub mod vbuffer;
pub mod visualization;
pub mod waterfall;

// Re-export commonly used types
pub use forensic_queries::{AttackPatternReport, CorrelationEvidence, DetectionWithContext};
pub mod app_state;
pub mod hardware_io;

pub mod ai;
pub mod computer_vision;
pub mod forensic_queries;
pub mod ml;
pub mod hardware;
pub mod utils;
pub mod dsp;

pub mod knowledge_graph;

pub mod dispatch;

slint::include_modules!();
