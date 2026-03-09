use anyhow::{Context, Result};
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{
    CreateCollectionBuilder, Distance, PointStruct, SearchPointsBuilder, UpsertPointsBuilder,
    Value, VectorParams, VectorsConfig, value::Kind,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const EMBEDDING_DIM: usize = 32; // Mamba latent dimension

/// Forensic signal embedding metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignalMetadata {
    pub timestamp_ms: i64,
    pub channel_id: u32,
    pub frequency_hz: f32,
    pub confidence: f32,
    pub graph_node_id: Option<String>,
    pub mode: String,
    pub sample_rate: u32,
}

/// Qdrant client for embedding storage and similarity search
#[allow(dead_code)]
pub struct QdrantClient {
    client: Qdrant,
    collection_name: String,
}

#[allow(dead_code)]
impl QdrantClient {
    /// Connect to Qdrant server and ensure collection exists
    pub async fn new(url: &str, collection: &str) -> Result<Self> {
        let client = Qdrant::from_url(url)
            .build()
            .context("Failed to connect to Qdrant")?;

        let collection_name = collection.to_string();

        // Create collection if it doesn't exist
        if !client
            .collection_exists(&collection_name)
            .await
            .context("Failed to check collection existence")?
        {
            let vector_params = VectorParams {
                size: EMBEDDING_DIM as u64,
                distance: Distance::Cosine as i32,
                datatype: None,
                on_disk: Some(false),
                quantization_config: None,
                hnsw_config: None,
                multivector_config: None,
            };

            client
                .create_collection(
                    CreateCollectionBuilder::new(&collection_name)
                        .vectors_config(VectorsConfig {
                            config: Some(qdrant_client::qdrant::vectors_config::Config::Params(
                                vector_params,
                            )),
                        })
                        .build(),
                )
                .await
                .context("Failed to create Qdrant collection")?;
        }

        Ok(QdrantClient {
            client,
            collection_name,
        })
    }

    /// Health check
    pub async fn health(&self) -> Result<bool> {
        self.client
            .health_check()
            .await
            .context("Qdrant health check failed")
            .map(|_| true)
    }

    /// Store embeddings with forensic metadata
    pub async fn upsert_embeddings(
        &self,
        points: Vec<(u64, Vec<f32>, SignalMetadata)>,
    ) -> Result<()> {
        let qdrant_points: Vec<PointStruct> = points
            .into_iter()
            .map(|(id, embedding, metadata)| {
                let mut payload = HashMap::new();
                payload.insert(
                    "timestamp_ms".to_string(),
                    Value {
                        kind: Some(Kind::IntegerValue(metadata.timestamp_ms)),
                    },
                );
                payload.insert(
                    "channel_id".to_string(),
                    Value {
                        kind: Some(Kind::IntegerValue(metadata.channel_id as i64)),
                    },
                );
                payload.insert(
                    "frequency_hz".to_string(),
                    Value {
                        kind: Some(Kind::DoubleValue(metadata.frequency_hz as f64)),
                    },
                );
                payload.insert(
                    "confidence".to_string(),
                    Value {
                        kind: Some(Kind::DoubleValue(metadata.confidence as f64)),
                    },
                );
                if let Some(node_id) = metadata.graph_node_id {
                    payload.insert(
                        "graph_node_id".to_string(),
                        Value {
                            kind: Some(Kind::StringValue(node_id)),
                        },
                    );
                }
                payload.insert(
                    "mode".to_string(),
                    Value {
                        kind: Some(Kind::StringValue(metadata.mode)),
                    },
                );
                payload.insert(
                    "sample_rate".to_string(),
                    Value {
                        kind: Some(Kind::IntegerValue(metadata.sample_rate as i64)),
                    },
                );

                PointStruct::new(id, embedding, payload)
            })
            .collect();

        self.client
            .upsert_points(UpsertPointsBuilder::new(
                &self.collection_name,
                qdrant_points,
            ))
            .await
            .context("Failed to upsert embeddings to Qdrant")?;

        Ok(())
    }

    /// Search for similar embeddings by cosine distance
    pub async fn search_similar(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(u64, f32, SignalMetadata)>> {
        let search_result = self
            .client
            .search_points(
                SearchPointsBuilder::new(&self.collection_name, embedding.to_vec(), limit as u64)
                    .with_payload(true)
                    .build(),
            )
            .await
            .context("Failed to search Qdrant")?;

        let mut results = Vec::new();
        for scored_point in search_result.result {
            let point_id = match scored_point.id.unwrap().point_id_options.unwrap() {
                qdrant_client::qdrant::point_id::PointIdOptions::Num(n) => n,
                qdrant_client::qdrant::point_id::PointIdOptions::Uuid(uuid) => {
                    // For UUID, hash it to u64
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    uuid.hash(&mut hasher);
                    hasher.finish()
                }
            };
            let score = scored_point.score;
            let payload = scored_point.payload;

            // Extract metadata from payload
            let timestamp_ms = payload
                .get("timestamp_ms")
                .and_then(|v| v.kind.as_ref())
                .and_then(|k| {
                    if let Kind::IntegerValue(v) = k {
                        Some(*v)
                    } else {
                        None
                    }
                })
                .unwrap_or(0);

            let channel_id = payload
                .get("channel_id")
                .and_then(|v| v.kind.as_ref())
                .and_then(|k| {
                    if let Kind::IntegerValue(v) = k {
                        Some(*v as u32)
                    } else {
                        None
                    }
                })
                .unwrap_or(0);

            let frequency_hz = payload
                .get("frequency_hz")
                .and_then(|v| v.kind.as_ref())
                .and_then(|k| {
                    if let Kind::DoubleValue(v) = k {
                        Some(*v as f32)
                    } else {
                        None
                    }
                })
                .unwrap_or(0.0);

            let confidence = payload
                .get("confidence")
                .and_then(|v| v.kind.as_ref())
                .and_then(|k| {
                    if let Kind::DoubleValue(v) = k {
                        Some(*v as f32)
                    } else {
                        None
                    }
                })
                .unwrap_or(0.0);

            let graph_node_id = payload
                .get("graph_node_id")
                .and_then(|v| v.kind.as_ref())
                .and_then(|k| {
                    if let Kind::StringValue(v) = k {
                        Some(v.clone())
                    } else {
                        None
                    }
                });

            let mode = payload
                .get("mode")
                .and_then(|v| v.kind.as_ref())
                .and_then(|k| {
                    if let Kind::StringValue(v) = k {
                        Some(v.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_default();

            let sample_rate = payload
                .get("sample_rate")
                .and_then(|v| v.kind.as_ref())
                .and_then(|k| {
                    if let Kind::IntegerValue(v) = k {
                        Some(*v as u32)
                    } else {
                        None
                    }
                })
                .unwrap_or(0);

            let metadata = SignalMetadata {
                timestamp_ms,
                channel_id,
                frequency_hz,
                confidence,
                graph_node_id,
                mode,
                sample_rate,
            };

            results.push((point_id, score, metadata));
        }

        Ok(results)
    }

    /// Update point metadata (e.g., link to Neo4j graph node)
    pub async fn update_metadata(&self, point_id: u64, graph_node_id: String) -> Result<()> {
        let mut payload = HashMap::new();
        payload.insert(
            "graph_node_id".to_string(),
            Value {
                kind: Some(Kind::StringValue(graph_node_id)),
            },
        );

        self.client
            .upsert_points(UpsertPointsBuilder::new(
                &self.collection_name,
                vec![PointStruct::new(point_id, vec![], payload)],
            ))
            .await
            .context("Failed to update point metadata")?;

        Ok(())
    }

    /// Delete points by ID
    pub async fn delete_points(&self, point_ids: &[u64]) -> Result<()> {
        use qdrant_client::qdrant::{DeletePointsBuilder, PointId, PointsIdsList};

        let ids: Vec<PointId> = point_ids
            .iter()
            .map(|&id| PointId {
                point_id_options: Some(qdrant_client::qdrant::point_id::PointIdOptions::Num(id)),
            })
            .collect();

        let request = DeletePointsBuilder::new(&self.collection_name)
            .points(PointsIdsList { ids })
            .build();

        self.client
            .delete_points(request)
            .await
            .context("Failed to delete points from Qdrant")?;

        Ok(())
    }

    /// Scroll all points (for batch export/analysis)
    pub async fn scroll_all(&self, limit: u64) -> Result<Vec<(u64, SignalMetadata)>> {
        use qdrant_client::qdrant::ScrollPointsBuilder;

        let scroll_result = self
            .client
            .scroll(
                ScrollPointsBuilder::new(&self.collection_name)
                    .limit(limit as u32)
                    .with_payload(true)
                    .build(),
            )
            .await
            .context("Failed to scroll Qdrant")?;

        let mut results = Vec::new();
        for point in scroll_result.result {
            let point_id = match point.id.unwrap().point_id_options.unwrap() {
                qdrant_client::qdrant::point_id::PointIdOptions::Num(n) => n,
                qdrant_client::qdrant::point_id::PointIdOptions::Uuid(uuid) => {
                    // For UUID, hash it to u64
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    uuid.hash(&mut hasher);
                    hasher.finish()
                }
            };
            let payload = point.payload;

            // Same metadata extraction as search_similar
            let timestamp_ms = payload
                .get("timestamp_ms")
                .and_then(|v| v.kind.as_ref())
                .and_then(|k| {
                    if let Kind::IntegerValue(v) = k {
                        Some(*v)
                    } else {
                        None
                    }
                })
                .unwrap_or(0);

            let channel_id = payload
                .get("channel_id")
                .and_then(|v| v.kind.as_ref())
                .and_then(|k| {
                    if let Kind::IntegerValue(v) = k {
                        Some(*v as u32)
                    } else {
                        None
                    }
                })
                .unwrap_or(0);

            let frequency_hz = payload
                .get("frequency_hz")
                .and_then(|v| v.kind.as_ref())
                .and_then(|k| {
                    if let Kind::DoubleValue(v) = k {
                        Some(*v as f32)
                    } else {
                        None
                    }
                })
                .unwrap_or(0.0);

            let confidence = payload
                .get("confidence")
                .and_then(|v| v.kind.as_ref())
                .and_then(|k| {
                    if let Kind::DoubleValue(v) = k {
                        Some(*v as f32)
                    } else {
                        None
                    }
                })
                .unwrap_or(0.0);

            let graph_node_id = payload
                .get("graph_node_id")
                .and_then(|v| v.kind.as_ref())
                .and_then(|k| {
                    if let Kind::StringValue(v) = k {
                        Some(v.clone())
                    } else {
                        None
                    }
                });

            let mode = payload
                .get("mode")
                .and_then(|v| v.kind.as_ref())
                .and_then(|k| {
                    if let Kind::StringValue(v) = k {
                        Some(v.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_default();

            let sample_rate = payload
                .get("sample_rate")
                .and_then(|v| v.kind.as_ref())
                .and_then(|k| {
                    if let Kind::IntegerValue(v) = k {
                        Some(*v as u32)
                    } else {
                        None
                    }
                })
                .unwrap_or(0);

            let metadata = SignalMetadata {
                timestamp_ms,
                channel_id,
                frequency_hz,
                confidence,
                graph_node_id,
                mode,
                sample_rate,
            };

            results.push((point_id, metadata));
        }

        Ok(results)
    }
}
