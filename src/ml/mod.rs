//! ML Pipeline — V3 Foundation
//!
//! # V3 Architecture Notes
//! - timegnn_trainer deleted — Track B6 being rewritten
//! - pose_estimator deleted — was stub, Track G0 rewrites
//! - spectral_frame module reference removed (check if exists)
//! - pose_materials deleted — depended on deleted pose_estimator

pub mod anomaly_gate;
pub mod chronos_bridge;
pub mod event_corpus;
pub mod fold_frequency_harmonics;
pub mod impulse_coherence;
pub mod impulse_modulation;
pub mod losses;
pub mod mamba_block;
pub mod modular_features;
pub mod multimodal_fusion;
pub mod pattern_discovery;
pub mod point_decoder;
pub mod point_mamba;
pub mod point_mamba_trainer;
pub mod pointnet_encoder;
pub mod timegnn;
// timegnn_trainer, timegnn_ui_bridge deleted — Track B6 rewrite
pub mod wav2vec2_loader;
pub mod wideband_harmonic_analysis;

// Data contracts
pub mod data_contracts;
pub mod field_particle;

// V3 UnifiedFieldMamba (replaces SSAMBA)
pub mod unified_field_mamba;

// Waveshape projection (Track A)
pub mod waveshape_projection;

// Pose modules (being rewritten — Track G0)
// pose_estimator, pose_materials deleted

// Exports
pub use losses::chamfer_distance::{ChamferDistance, HuberLoss};
pub use mamba_block::MambaBlock;
pub use modular_features::{FeatureFlags, ModularFeatureEncoder, SignalFeaturePayload, VideoFrame};
pub use pattern_discovery::{
    ClusteringResult, Event, KMeansConfig, Pattern, compute_silhouette_score,
    compute_temporal_frequency, discover_patterns, generate_pattern_label, kmeans,
};
pub use point_decoder::PointDecoder;
pub use point_mamba::PointMamba;
pub use point_mamba_trainer::{PointMambaTrainer, TrainerConfig as PointMambaTrainingConfig};
pub use pointnet_encoder::PointNetEncoder;
pub use timegnn::TimeGnnModel;
// timegnn_trainer exports deleted
pub use unified_field_mamba::UnifiedFieldMamba;
pub use waveshape_projection::{NeuralWaveshapeParams, project_latent_to_waveshape};
pub use field_particle::FieldParticle;
// PoseEstimator deleted with pose_estimator module
// SpectralFrame — check if module exists before exporting
