use crate::knowledge_graph::cognee_schema::{EventNode, PatternNode};
use crate::knowledge_graph::KnowledgeGraphClient;
use std::sync::Arc;
use tokio::sync::mpsc;
use chrono::Utc;

/// The Graph Ingestion Pipeline consumes Semantic Events from Tracks A-D asynchronously,
/// adding them to the Neo4j Knowledge Graph without blocking the dispatch loop.
pub struct GraphIngestionPipeline {
    rx: mpsc::UnboundedReceiver<EventNode>,
    neo4j_client: Arc<KnowledgeGraphClient>,
}

impl GraphIngestionPipeline {
    /// Initialize the pipeline with a Neo4j client and return the sender half.
    pub fn new(neo4j_client: Arc<KnowledgeGraphClient>) -> (mpsc::UnboundedSender<EventNode>, Self) {
        // Use an unbounded channel to ensure the dispatch loop never blocks
        let (tx, rx) = mpsc::unbounded_channel();
        (
            tx,
            Self {
                rx,
                neo4j_client,
            },
        )
    }

    /// Run the ingestion loop. This should be spawned in a Tokio task.
    pub async fn ingest_stream(mut self) {
        while let Some(event) = self.rx.recv().await {
            let client = Arc::clone(&self.neo4j_client);

            // Fire-and-forget task to avoid blocking the ingestion loop
            tokio::spawn(async move {
                // 1. Create the base Event node
                if let Err(e) = client.create_event_node(&event).await {
                    eprintln!("Failed to create EventNode {}: {}", event.event_id, e);
                    return; // Fail gracefully
                }

                // 2. Real-time Pattern Matching (Mode 2)
                // Check if event matches any KNOWN patterns
                match client.find_matching_patterns(&event).await {
                    Ok(patterns) => {
                        for pattern_id in patterns {
                            if let Err(e) = client.link_event_to_pattern(event.event_id, pattern_id).await {
                                eprintln!("Failed to link Event to Pattern {}: {}", pattern_id, e);
                            }
                        }
                    }
                    Err(e) => eprintln!("Failed to match patterns: {}", e),
                }

                // E.2 Latency is minimal because the ingestion task is detached
            });
        }
    }
}

/// A separate utility for Track B to ingest historical patterns (Mode 1).
pub struct PatternIngestionPipeline {
    neo4j_client: Arc<KnowledgeGraphClient>,
}

impl PatternIngestionPipeline {
    pub fn new(neo4j_client: Arc<KnowledgeGraphClient>) -> Self {
        Self { neo4j_client }
    }

    /// Mode 1: Historical pattern ingestion (batch).
    /// Track B calls this when Phase 2C discovers new patterns offline.
    pub async fn ingest_pattern_batch(&self, patterns: Vec<PatternNode>) -> anyhow::Result<()> {
        for pattern in patterns {
            // Create the pattern node
            self.neo4j_client.create_pattern_node(&pattern).await?;

            // Backfill: link all past events to this pattern
            // Note: This is expensive O(N) but happens offline
            println!("Backfilling events for pattern {}", pattern.pattern_id);
            self.neo4j_client.backfill_pattern_links(&pattern).await?;
        }
        Ok(())
    }
}

/// A Mock Generator to simulate Track A event blasts (for testing E.2 performance)
pub async fn run_mock_generator(tx: mpsc::UnboundedSender<EventNode>, rate_hz: u64) {
    let delay = std::time::Duration::from_secs_f64(1.0 / rate_hz as f64);
    let mut counter = 0;

    loop {
        let event = EventNode {
            event_id: counter,
            timestamp_iso: Utc::now().to_rfc3339(),
            timestamp_us: Utc::now().timestamp_micros() as u64,
            audio_rms_db: -40.0,
            rf_frequency_hz: 2.4e9,
            rf_peak_dbfs: -20.0,
            anomaly_score: 0.85,
            tags: vec!["MOCK".to_string()],
            device_source: "MockDevice".to_string(),
        };

        if tx.send(event).is_err() {
            eprintln!("Mock generator: receiver dropped");
            break;
        }

        counter += 1;
        tokio::time::sleep(delay).await;
    }
}
