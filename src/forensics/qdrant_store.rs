//! Qdrant vector storage for forensic events

use crate::forensics::event::{ForensicEvent, ControlMode};
use qdrant_client::{
    qdrant::{
        PointStruct, QueryPoints, SearchPoints, VectorParams, VectorsConfig,
        Distance, Value, CreateCollection,
    },
    Payload, Qdrant,
};
use std::sync::Arc;

/// Qdrant configuration
#[derive(Debug, Clone)]
pub struct QdrantConfig {
    /// Server URL
    pub url: String,
    /// Collection name
    pub collection: String,
    /// API key (optional)
    pub api_key: Option<String>,
    /// Vector dimension
    pub vector_dim: usize,
}

impl Default for QdrantConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:6333".to_string(),
            collection: "rf_forensics".to_string(),
            api_key: None,
            vector_dim: 64,
        }
    }
}

/// Qdrant forensics store
pub struct QdrantForensics {
    client: Arc<Qdrant>,
    config: QdrantConfig,
}

/// Vector search result
#[derive(Debug, Clone)]
pub struct VectorSearch {
    /// Event ID
    pub id: String,
    /// Similarity score
    pub score: f32,
    /// Payload data
    pub payload: serde_json::Value,
}

impl QdrantForensics {
    /// Create a new Qdrant forensics store
    pub async fn new(config: QdrantConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let mut client_builder = Qdrant::from_url(&config.url);
        
        if let Some(ref api_key) = config.api_key {
            client_builder = client_builder.with_api_key(api_key);
        }

        let client = Arc::new(client_builder.build()?);

        let store = Self { client, config };
        store.ensure_collection().await?;

        Ok(store)
    }

    /// Create with mock client (for testing)
    pub fn new_mock(config: QdrantConfig) -> Self {
        // In production, would create actual client
        // For now, return a placeholder
        Self {
            client: Arc::new(Qdrant::from_url("http://mock").build().unwrap()),
            config,
        }
    }

    /// Ensure collection exists
    async fn ensure_collection(&self) -> Result<(), Box<dyn std::error::Error>> {
        let collections = self.client.list_collections().await?;

        if !collections.collections.iter().any(|c| c.name == self.config.collection) {
            // Create collection
            self.client
                .create_collection(&CreateCollection {
                    collection_name: self.config.collection.clone(),
                    vectors_config: Some(VectorsConfig {
                        params: Some(VectorParams {
                            size: self.config.vector_dim as u64,
                            distance: Distance::Cosine.into(),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await?;
        }

        Ok(())
    }

    /// Store a forensic event
    pub async fn store(&self, event: &ForensicEvent) -> Result<String, Box<dyn std::error::Error>> {
        let payload = Payload::try_from(serde_json::to_value(&event)?)?;

        let point = PointStruct::new(
            event.id.clone(),
            event.latent.clone(),
            payload,
        );

        self.client
            .upsert_points(&self.config.collection, None, vec![point], None)
            .await?;

        Ok(event.id.clone())
    }

    /// Store multiple events (batch)
    pub async fn store_batch(
        &self,
        events: &[ForensicEvent],
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let points: Result<Vec<PointStruct>, Box<dyn std::error::Error>> = events
            .iter()
            .map(|event| {
                let json_val = serde_json::to_value(event)?;
                let payload = Payload::try_from(json_val)?;
                Ok(PointStruct::new(
                    event.id.clone(),
                    event.latent.clone(),
                    payload,
                ))
            })
            .collect();

        self.client
            .upsert_points(&self.config.collection, None, points?, None)
            .await?;

        Ok(events.iter().map(|e| e.id.clone()).collect())
    }

    /// Search for similar events
    pub async fn search(
        &self,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<Vec<VectorSearch>, Box<dyn std::error::Error>> {
        let result = self
            .client
            .search_points(&SearchPoints {
                collection_name: self.config.collection.clone(),
                vector: query_vector.to_vec(),
                limit: limit as u64,
                with_payload: Some(true.into()),
                with_vectors: Some(false.into()),
                ..Default::default()
            })
            .await?;

        let searches: Vec<VectorSearch> = result
            .result
            .into_iter()
            .filter_map(|r| {
                let payload: serde_json::Value = r.payload.into();
                Some(VectorSearch {
                    id: r.id.to_string(),
                    score: r.score,
                    payload,
                })
            })
            .collect();

        Ok(searches)
    }

    /// Search by event (using its latent vector)
    pub async fn search_by_event(
        &self,
        event: &ForensicEvent,
        limit: usize,
    ) -> Result<Vec<VectorSearch>, Box<dyn std::error::Error>> {
        self.search(&event.latent, limit).await
    }

    /// Get event by ID
    pub async fn get(&self, id: &str) -> Result<Option<ForensicEvent>, Box<dyn std::error::Error>> {
        let result = self
            .client
            .get_points(&self.config.collection, None, vec![id.into()], None, None)
            .await?;

        if result.result.is_empty() {
            return Ok(None);
        }

        let point = &result.result[0];
        let event: ForensicEvent = serde_json::from_value(point.payload.clone().into())?;
        Ok(Some(event))
    }

    /// Delete event by ID
    pub async fn delete(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.client
            .delete_points(&self.config.collection, None, vec![id.into()].into(), None)
            .await?;
        Ok(())
    }

    /// Get collection stats
    pub async fn stats(&self) -> Result<CollectionStats, Box<dyn std::error::Error>> {
        let info = self.client.collection_info(&self.config.collection).await?;

        Ok(CollectionStats {
            vector_count: info.result.points_count.map(|c| c.count).unwrap_or(0),
            vector_dim: self.config.vector_dim,
            collection_name: self.config.collection.clone(),
        })
    }

    /// Clear all events
    pub async fn clear(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.client
            .delete_collection(&self.config.collection)
            .await?;
        self.ensure_collection().await?;
        Ok(())
    }
}

/// Collection statistics
#[derive(Debug, Clone)]
pub struct CollectionStats {
    pub vector_count: u64,
    pub vector_dim: usize,
    pub collection_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forensics::event::*;
    use ndarray::Array1;

    #[tokio::test]
    async fn test_qdrant_mock() {
        let config = QdrantConfig::default();
        let store = QdrantForensics::new_mock(config);

        // Mock store should not panic
        let event = ForensicEvent::snapshot(
            RFContext {
                center_frequency_hz: 100_000_000,
                sample_rate_hz: 2_048_000,
                psd: vec![0.0; 256],
                total_power_db: -50.0,
                spectral_kurtosis: 0.0,
                peak_bin: 0,
                band_ratios: [0.33, 0.33, 0.34],
                rfi_detected: false,
                snr_db: 50.0,
            },
            AudioContext {
                sample_rate_hz: 192_000,
                num_channels: 3,
                psd: vec![0.0; 128],
                tdoa: vec![0.0; 16],
                tdoa_estimate: 0.0,
                correlation_peak: 0.0,
                residual_energy: 0.0,
                channel_energies: [0.0, 0.0, 0.0],
                spectral_centroid: 0.0,
                zcr: 0.0,
                ambient_noise_db: 40.0,
            },
            Array1::zeros(64),
            ControlState {
                mode: ControlMode::Anc,
                mode_probs: [1.0, 0.0, 0.0],
                target_snr_db: 108.0,
                anc_weights_version: 0,
                fade_state: 1.0,
            },
        );

        // In mock mode, store will fail but shouldn't panic
        let result = store.store(&event).await;
        // Expected to fail with mock client
        assert!(result.is_err());
    }

    #[test]
    fn test_config_default() {
        let config = QdrantConfig::default();
        assert_eq!(config.url, "http://localhost:6333");
        assert_eq!(config.collection, "rf_forensics");
        assert_eq!(config.vector_dim, 64);
    }
}
