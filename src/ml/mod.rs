pub mod event_corpus;
pub mod fold_frequency_harmonics;
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
pub mod wideband_harmonic_analysis;

pub use event_corpus::{
    load_forensic_events, prepare_event_corpus, CorpusStats, ForensicEventData,
};
pub use losses::chamfer_distance::{ChamferDistance, HuberLoss};
pub use multimodal_fusion::{
    compute_modality_stats, fuse_multimodal, ModalityStats, MultimodalFeatures,
};
pub use pattern_discovery::{
    compute_silhouette_score, compute_temporal_frequency, discover_patterns,
    generate_pattern_label, kmeans, ClusteringResult, Event, KMeansConfig, Pattern,
};
pub use losses::chamfer_distance::{ChamferDistance, HuberLoss};

pub mod impulse_coherence;
pub mod modular_features;
pub use timegnn::TimeGnnModel;
pub use timegnn_trainer::{
    compute_nt_xent_loss, cosine_similarity, train_timegnn, ContrastiveLossConfig,
    TimeGnnTrainingConfig, TrainingEvent, TrainingMetrics,
};
pub use wav2vec2_loader::{infer_wav2vec2_embedding, load_wav2vec2, Wav2Vec2Model};
