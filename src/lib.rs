// src/lib.rs — Library exports for forensic queries tests

// Core modules needed for forensic query API
pub mod af32;
pub mod analysis_mock_data;
pub mod features;
pub mod ml;
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
pub mod forensic;
pub mod forensic_queries;
pub mod fusion;
pub mod gpu;
pub mod gpu_shared;
pub mod gpu_memory;
pub mod harmony;
pub mod graph;
pub mod mamba;
pub mod parametric;
pub mod pdm;
pub mod resample;
pub mod ridge_plot;
pub mod rtlsdr_ffi;
pub mod rtlsdr;
pub mod sdr;
pub mod state;
pub mod testing;
pub mod trainer;
pub mod training;
pub mod training_tests;
pub mod twister;
pub mod vbuffer;
pub mod visualization;
pub mod waterfall;

// Re-export commonly used types
pub use forensic_queries::{
    AttackPatternReport,
    CorrelationEvidence,
    DetectionWithContext,
};
