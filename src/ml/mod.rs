pub mod anomaly_gate;
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
pub mod spectral_frame;
pub mod spectral_frame;
pub mod timegnn;
pub mod timegnn_trainer;
pub mod wav2vec2_loader;
pub mod wideband_harmonic_analysis;

pub use modular_features::{FeatureFlags, ModularFeatureEncoder, SignalFeaturePayload, VideoFrame};

pub use anomaly_gate::{AnomalyGateConfig, AnomalyGateDecision, evaluate_anomaly_gate};
/// src/ml/mod.rs
/// ML module: burn-wgpu graph neural network for event embedding
/// Orchestrates TimeGNN model for GPU-accelerated inference
/// Includes Point Mamba cascade (PointNet → Mamba → Decoder)
///
/// Submodules:
/// - timegnn: TimeGNN model (1092-D → 128-D event embeddings)
/// - wav2vec2_loader: facebook/wav2vec2-base-960h frozen speech encoder
/// - multimodal_fusion: Audio + Ray + Wav2vec2 feature fusion (1092-D)
/// - event_corpus: Forensic log → HDF5 training corpus generation
/// - pointnet_encoder: (N, 6) → (N, 256) point feature extraction
/// - mamba_block: Selective scan state-space for temporal dynamics
/// - point_decoder: (N, 128) → (N, 3) 3D offset reconstruction
/// - point_mamba_trainer: Training with Chamfer-Huber loss fusion
/// - losses: Chamfer distance + Huber outlier robustness
/// - fold_frequency_harmonics: Harmonic analysis for periodic signals
/// - impulse_modulation: Detects impulsive patterns in waveforms
/// - wideband_harmonic_analysis: Global harmonic structure analysis
pub use event_corpus::{
    CorpusStats, ForensicEventData, load_forensic_events, prepare_event_corpus,
};
pub use losses::chamfer_distance::{ChamferDistance, HuberLoss};
pub use mamba_block::MambaBlock;
pub use multimodal_fusion::{
    ModalityStats, MultimodalFeatures, compute_modality_stats, fuse_multimodal,
};
pub use pattern_discovery::{
    ClusteringResult, Event, KMeansConfig, Pattern, compute_silhouette_score,
    compute_temporal_frequency, discover_patterns, generate_pattern_label, kmeans,
};
pub use point_decoder::PointDecoder;
pub use point_mamba::PointMamba;
pub use point_mamba_trainer::{PointMambaTrainer, TrainerConfig as PointMambaTrainingConfig};
pub use pointnet_encoder::PointNetEncoder;
pub use spectral_frame::SpectralFrame;
pub use timegnn::TimeGnnModel;
pub use timegnn_trainer::{
    ContrastiveLossConfig, TimeGnnTrainingConfig, TrainingEvent, TrainingMetrics,
    compute_nt_xent_loss, cosine_similarity, train_timegnn,
};
pub use wav2vec2_loader::{Wav2Vec2Model, infer_wav2vec2_embedding, load_wav2vec2};
pub mod material_learning;
