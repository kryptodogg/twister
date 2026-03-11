pub mod anomaly_gate;
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
pub mod timegnn_trainer;
pub mod wav2vec2_loader;
pub mod wideband_harmonic_analysis;

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
pub use timegnn_trainer::{
    ContrastiveLossConfig, TimeGnnTrainingConfig, TrainingEvent, TrainingMetrics,
    compute_nt_xent_loss, cosine_similarity, train_timegnn,
};

pub use spectral_frame::SpectralFrame;

pub mod data_contracts;

pub mod body_region_classifier;
pub mod pose_materials;

pub mod pose_mamba_trainer;
pub mod unified_field_mamba;
pub use unified_field_mamba::UnifiedFieldMamba;
pub mod waveshape_projection;
pub use waveshape_projection::{NeuralWaveshapeParams, project_latent_to_waveshape};
pub mod spectral_frame;

pub mod field_particle;
pub use field_particle::FieldParticle;

pub mod field_pipeline;
pub use field_pipeline::{FieldPipeline, MambaProjections, AudioSource};

pub mod mamba;
pub use mamba::{MambaModel, MambaConfig};

pub mod waveshape;
pub use waveshape::{WaveshapeProjector, WaveshapeConfig};
