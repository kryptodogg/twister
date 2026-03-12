// Increase recursion limit for complex generic types (Autodiff<Wgpu> trait resolution)
#![recursion_limit = "256"]

// ═══════════════════════════════════════════════════════════════════════════════
// PROJECT SYNESTHESIA V3 — CORE MODULES
// Only modules that compile without deleted dependencies.
// Track 0-A foundation only. Everything else is being rewritten.
// ═══════════════════════════════════════════════════════════════════════════════

// Core infrastructure
pub mod af32;
pub mod app_state;
pub mod utils;

// Hardware abstraction (Track 0-B)
pub mod hardware;
pub mod hardware_io;

// DSP pipeline (V3 — ingestion only, post-processing moves downstream)
pub mod dsp;

// Mamba core (V3 UnifiedFieldMamba replaces old SSAMBA)
pub mod mamba;

// ML pipeline (Track B — timegnn_trainer deleted, being rewritten)
pub mod ml;

// Physics simulation (Track G-SPH)
pub mod physics;

// GPU compute (wgpu v28)
pub mod gpu;
pub mod gpu_shared;

// Particle system (Track G)
pub mod particle_system;

// Forensic corpus (Track C — Track 0-D foundation)
pub mod forensic;

// Dispatch loop (Track A)
pub mod dispatch;

// UI applets (Track E/F)
pub mod ui;

// Legacy/ANC modules (keep for backward compatibility)
pub mod anc;
pub mod anc_calibration;
pub mod anc_recording;
pub mod audio;
pub mod pdm;
pub mod sdr;
#[cfg(feature = "rtlsdr")]
pub mod rtlsdr;
#[cfg(feature = "rtlsdr")]
pub mod rtlsdr_ffi;
#[cfg(feature = "rtlsdr")]
pub mod safe_sdr_wrapper;

// Visualization
pub mod waterfall;
pub mod vbuffer;
pub mod visualization;

// AI reasoning (Track D — evidence_chain deleted, being rewritten)
pub mod ai;

// Knowledge graph (Track C3)
pub mod knowledge_graph;
pub mod graph;

// Spatial processing
pub mod spatial;

// Active denial (Track I/H)
pub mod active_denial;

// Fusion engine
pub mod fusion;

// Harmony/Twister
pub mod harmony;
pub mod twister;

// Embeddings (Qdrant)
pub mod embeddings;

// Evidence export
// pub mod evidence_export; — deleted, Track C1 forensic module replaces

// Parametric speaker
pub mod parametric;

// Reconstruction (Crystal Ball)
pub mod reconstruct;

// Resample
pub mod resample;

// Ridge plot
pub mod ridge_plot;

// Bispectrum
pub mod bispectrum;

// Detection
pub mod detection;

// State
pub mod state;

// Dispatch kernel
pub mod dispatch_kernel;

// Async event handler
pub mod async_event_handler;

// Analysis mock data (test only)
pub mod analysis_mock_data;

// GPU memory
pub mod gpu_memory;

// Testing
pub mod testing;
pub mod training_tests;

// Slint UI
slint::include_modules!();
