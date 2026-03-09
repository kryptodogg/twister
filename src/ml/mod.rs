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

pub mod timegnn;
pub mod wav2vec2_loader;
pub mod multimodal_fusion;
pub mod event_corpus;
pub mod timegnn_trainer;
pub mod pattern_discovery;
pub mod losses;
pub mod pointnet_encoder;
pub mod mamba_block;
pub mod point_decoder;
pub mod point_mamba;
pub mod point_mamba_trainer;

pub use timegnn::TimeGnnModel;
pub use wav2vec2_loader::{Wav2Vec2Model, load_wav2vec2, infer_wav2vec2_embedding};
pub use multimodal_fusion::{fuse_multimodal, MultimodalFeatures, ModalityStats, compute_modality_stats};
pub use event_corpus::{prepare_event_corpus, load_forensic_events, CorpusStats, ForensicEventData};
pub use timegnn_trainer::{
    train_timegnn, ContrastiveLossConfig, TimeGnnTrainingConfig, TrainingEvent, TrainingMetrics,
    cosine_similarity, compute_nt_xent_loss,
};
pub use pattern_discovery::{
    discover_patterns, Pattern, Event, KMeansConfig, ClusteringResult, kmeans,
    compute_silhouette_score, compute_temporal_frequency, generate_pattern_label,
};
pub use losses::chamfer_distance::{ChamferDistance, HuberLoss};

pub mod modular_features;
