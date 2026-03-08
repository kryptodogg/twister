// src/embeddings.rs — Embedding Store Facade  (v0.5)
//
// Wraps Qdrant vector database for forensic evidence retrieval.
// All operations are async (tokio).  The store connects at startup;
// if Qdrant is unavailable the Option<EmbeddingStore> in main.rs is None
// and all store calls are silently skipped — the app runs without a DB.

use anyhow::Result;
use crate::detection::DetectionEvent;

/// A detection retrieved from a vector similarity search.
/// Fields match the access pattern in main.rs:
///   sim.score          — cosine similarity [0.0, 1.0]
///   sim.event.f1_hz    — primary frequency of the similar past event
///   sim.event.timestamp — when the similar event was recorded
pub struct SimilarDetection {
    pub score: f32,
    pub event: DetectionEvent,
}

/// Embedding store backed by Qdrant.
/// Clone-able so multiple tasks can hold a reference without Arc wrapping.
#[derive(Clone)]
pub struct EmbeddingStore {
    // TODO: Replace with live qdrant_client::Qdrant once connection pooling
    // is wired.  The stub implementation compiles cleanly and lets the rest
    // of the pipeline run.
    _marker: (),
}

impl EmbeddingStore {
    /// Connect to Qdrant at the default local address.
    pub async fn new() -> Result<Self> {
        // TODO: let client = qdrant_client::Qdrant::from_url("http://localhost:6334").build()?;
        // Attempt a health-check here and propagate any connection error so
        // main.rs can disable the DB path gracefully.
        Ok(Self { _marker: () })
    }

    /// Persist a bispectrum / Mamba detection event embedding.
    pub async fn store_detection(&self, _event: &DetectionEvent) -> Result<()> {
        // TODO: convert event fields → f32 embedding vector, upsert to Qdrant
        Ok(())
    }

    /// Find the top-k most similar past detections by embedding distance.
    /// Returns a vec of (similarity_score, event) pairs sorted descending by score.
    pub async fn find_similar(
        &self,
        _event: &DetectionEvent,
        _limit: usize,
    ) -> Result<Vec<SimilarDetection>> {
        // TODO: query Qdrant with the event's embedding, map results back to
        // SimilarDetection.  Until then, return empty so the forensic pipeline
        // degrades gracefully.
        Ok(Vec::new())
    }

    /// Persist Mamba latent vectors for downstream anomaly pattern mining.
    pub async fn store_latents(
        &self,
        _latents: &[f32],
        _events: &[DetectionEvent],
    ) -> Result<()> {
        // TODO: batch upsert latent embeddings with event metadata
        Ok(())
    }
}
