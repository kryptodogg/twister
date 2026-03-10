//! Neo4j graph database for forensic relationships

use neo4rs::*;
use crate::forensics::event::{ForensicEvent, EventType, ControlMode};
use std::sync::Arc;

/// Neo4j configuration
#[derive(Debug, Clone)]
pub struct Neo4jConfig {
    /// Connection URL (bolt://...)
    pub url: String,
    /// Username
    pub user: String,
    /// Password
    pub password: String,
    /// Database name
    pub database: String,
}

impl Default for Neo4jConfig {
    fn default() -> Self {
        Self {
            url: "neo4j://localhost:7687".to_string(),
            user: "neo4j".to_string(),
            password: "password".to_string(),
            database: "neo4j".to_string(),
        }
    }
}

/// Graph relationship type
#[derive(Debug, Clone)]
pub enum GraphRelationship {
    /// Event occurred at location
    OccurredAt { event_id: String, location_id: String },
    /// Event associated with RF source
    FromRFSource { event_id: String, source_id: String },
    /// Event has noise profile
    HasNoiseProfile { event_id: String, profile_id: String },
    /// Mode transition
    TransitionedTo { from_event: String, to_event: String },
    /// Session contains event
    SessionContains { session_id: String, event_id: String },
    /// Music program associated
    MusicProgram { event_id: String, program_id: String },
}

/// Neo4j forensics graph
pub struct Neo4jForensics {
    graph: Arc<Graph>,
    config: Neo4jConfig,
}

impl Neo4jForensics {
    /// Create a new Neo4j forensics graph
    pub async fn new(config: Neo4jConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let graph = Graph::new(
            &config.url,
            &config.user,
            &config.password,
        ).await?;

        Ok(Self {
            graph: Arc::new(graph),
            config,
        })
    }

    /// Create with mock (for testing)
    pub fn new_mock(config: Neo4jConfig) -> Self {
        // In production, would create actual graph connection
        // For testing, we'll use a placeholder approach
        Self {
            graph: Arc::new(Graph::new("neo4j://mock", "mock", "mock").await.unwrap()),
            config,
        }
    }

    /// Initialize database schema
    pub async fn init_schema(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Create constraints and indexes
        let queries = [
            "CREATE CONSTRAINT IF NOT EXISTS FOR (e:Event) REQUIRE e.id IS UNIQUE",
            "CREATE CONSTRAINT IF NOT EXISTS FOR (l:Location) REQUIRE l.id IS UNIQUE",
            "CREATE CONSTRAINT IF NOT EXISTS FOR (s:RFSource) REQUIRE s.id IS UNIQUE",
            "CREATE CONSTRAINT IF NOT EXISTS FOR (p:NoiseProfile) REQUIRE p.id IS UNIQUE",
            "CREATE CONSTRAINT IF NOT EXISTS FOR (sess:Session) REQUIRE sess.id IS UNIQUE",
            "CREATE INDEX IF NOT EXISTS FOR (e:Event) ON (e.timestamp)",
            "CREATE INDEX IF NOT EXISTS FOR (e:Event) ON (e.event_type)",
        ];

        for query in queries {
            self.graph.run(query(query)).await?;
        }

        Ok(())
    }

    /// Store a forensic event as a node
    pub async fn store_event(&self, event: &ForensicEvent) -> Result<(), Box<dyn std::error::Error>> {
        let query_str = r#"
            MERGE (e:Event {id: $id})
            SET e.timestamp = $timestamp,
                e.event_type = $event_type,
                e.session_id = $session_id,
                e.sequence = $sequence,
                e.tags = $tags,
                e.mode = $mode,
                e.target_snr_db = $target_snr_db,
                e.rf_total_power_db = $rf_power,
                e.rf_snr_db = $rf_snr,
                e.rf_rfi_detected = $rf_rfi,
                e.audio_ambient_noise_db = $audio_noise,
                e.audio_tdoa = $audio_tdoa,
                e.pipeline_latency_ms = $latency
        "#;

        self.graph
            .run(
                query(query_str)
                    .param("id", &event.id)
                    .param("timestamp", event.timestamp.to_rfc3339())
                    .param("event_type", format!("{:?}", event.metadata.event_type))
                    .param("session_id", &event.metadata.session_id)
                    .param("sequence", event.metadata.sequence as i64)
                    .param("tags", &event.metadata.tags)
                    .param("mode", format!("{:?}", event.control.mode))
                    .param("target_snr_db", event.control.target_snr_db)
                    .param("rf_power", event.rf_context.total_power_db)
                    .param("rf_snr", event.rf_context.snr_db)
                    .param("rf_rfi", event.rf_context.rfi_detected)
                    .param("audio_noise", event.audio_context.ambient_noise_db)
                    .param("audio_tdoa", event.audio_context.tdoa_estimate)
                    .param("latency", event.system_state.pipeline_latency_ms),
            )
            .await?;

        Ok(())
    }

    /// Create a relationship
    pub async fn create_relationship(
        &self,
        relationship: GraphRelationship,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match relationship {
            GraphRelationship::OccurredAt { event_id, location_id } => {
                let query_str = r#"
                    MATCH (e:Event {id: $event_id})
                    MATCH (l:Location {id: $location_id})
                    MERGE (e)-[:OCCURRED_AT]->(l)
                "#;
                self.graph
                    .run(
                        query(query_str)
                            .param("event_id", event_id)
                            .param("location_id", location_id),
                    )
                    .await?;
            }
            GraphRelationship::FromRFSource { event_id, source_id } => {
                let query_str = r#"
                    MATCH (e:Event {id: $event_id})
                    MATCH (s:RFSource {id: $source_id})
                    MERGE (e)-[:FROM_SOURCE]->(s)
                "#;
                self.graph
                    .run(
                        query(query_str)
                            .param("event_id", event_id)
                            .param("source_id", source_id),
                    )
                    .await?;
            }
            GraphRelationship::HasNoiseProfile { event_id, profile_id } => {
                let query_str = r#"
                    MATCH (e:Event {id: $event_id})
                    MATCH (p:NoiseProfile {id: $profile_id})
                    MERGE (e)-[:HAS_PROFILE]->(p)
                "#;
                self.graph
                    .run(
                        query(query_str)
                            .param("event_id", event_id)
                            .param("profile_id", profile_id),
                    )
                    .await?;
            }
            GraphRelationship::TransitionedTo { from_event, to_event } => {
                let query_str = r#"
                    MATCH (e1:Event {id: $from_event})
                    MATCH (e2:Event {id: $to_event})
                    MERGE (e1)-[:TRANSITIONED_TO]->(e2)
                "#;
                self.graph
                    .run(
                        query(query_str)
                            .param("from_event", from_event)
                            .param("to_event", to_event),
                    )
                    .await?;
            }
            GraphRelationship::SessionContains { session_id, event_id } => {
                let query_str = r#"
                    MATCH (sess:Session {id: $session_id})
                    MATCH (e:Event {id: $event_id})
                    MERGE (sess)-[:CONTAINS]->(e)
                "#;
                self.graph
                    .run(
                        query(query_str)
                            .param("session_id", session_id)
                            .param("event_id", event_id),
                    )
                    .await?;
            }
            GraphRelationship::MusicProgram { event_id, program_id } => {
                let query_str = r#"
                    MATCH (e:Event {id: $event_id})
                    MATCH (p:MusicProgram {id: $program_id})
                    MERGE (e)-[:MUSIC_PROGRAM]->(p)
                "#;
                self.graph
                    .run(
                        query(query_str)
                            .param("event_id", event_id)
                            .param("program_id", program_id),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    /// Query events by type
    pub async fn query_by_type(
        &self,
        event_type: EventType,
        limit: usize,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let query_str = r#"
            MATCH (e:Event)
            WHERE e.event_type = $event_type
            RETURN e.id as id
            ORDER BY e.timestamp DESC
            LIMIT $limit
        "#;

        let mut result = self
            .graph
            .run(
                query(query_str)
                    .param("event_type", format!("{:?}", event_type))
                    .param("limit", limit as i64),
            )
            .await?;

        let mut ids = Vec::new();
        while let Some(row) = result.next().await? {
            if let Some(id) = row.get::<String>("id") {
                ids.push(id);
            }
        }

        Ok(ids)
    }

    /// Query events by mode
    pub async fn query_by_mode(
        &self,
        mode: ControlMode,
        limit: usize,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let query_str = r#"
            MATCH (e:Event)
            WHERE e.mode = $mode
            RETURN e.id as id
            ORDER BY e.timestamp DESC
            LIMIT $limit
        "#;

        let mut result = self
            .graph
            .run(
                query(query_str)
                    .param("mode", format!("{:?}", mode))
                    .param("limit", limit as i64),
            )
            .await?;

        let mut ids = Vec::new();
        while let Some(row) = result.next().await? {
            if let Some(id) = row.get::<String>("id") {
                ids.push(id);
            }
        }

        Ok(ids)
    }

    /// Find related events
    pub async fn find_related(
        &self,
        event_id: &str,
        max_depth: usize,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let query_str = r#"
            MATCH (e:Event {id: $event_id})
            MATCH path = (e)-[*1..$max_depth]-(related:Event)
            RETURN DISTINCT related.id as id
        "#;

        let mut result = self
            .graph
            .run(
                query(query_str)
                    .param("event_id", event_id)
                    .param("max_depth", max_depth as i64),
            )
            .await?;

        let mut ids = Vec::new();
        while let Some(row) = result.next().await? {
            if let Some(id) = row.get::<String>("id") {
                ids.push(id);
            }
        }

        Ok(ids)
    }

    /// Get graph statistics
    pub async fn stats(&self) -> Result<GraphStats, Box<dyn std::error::Error>> {
        let query_str = r#"
            MATCH (e:Event)
            RETURN count(e) as event_count
        "#;

        let mut result = self.graph.run(query(query_str)).await?;
        let event_count = result
            .next()
            .await?
            .and_then(|r| r.get::<i64>("event_count"))
            .unwrap_or(0) as usize;

        Ok(GraphStats { event_count })
    }

    /// Clear all data
    pub async fn clear(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.graph
            .run(query("MATCH (n) DETACH DELETE n"))
            .await?;
        Ok(())
    }
}

/// Graph statistics
#[derive(Debug, Clone)]
pub struct GraphStats {
    pub event_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_neo4j_mock() {
        let config = Neo4jConfig::default();
        let graph = Neo4jForensics::new_mock(config);

        // Mock should not panic on creation
        // Operations will fail but shouldn't panic
        let result = graph.init_schema().await;
        assert!(result.is_err()); // Expected to fail with mock
    }

    #[test]
    fn test_config_default() {
        let config = Neo4jConfig::default();
        assert_eq!(config.url, "neo4j://localhost:7687");
        assert_eq!(config.user, "neo4j");
        assert_eq!(config.database, "neo4j");
    }

    #[test]
    fn test_graph_relationship() {
        let rel = GraphRelationship::OccurredAt {
            event_id: "e1".into(),
            location_id: "loc1".into(),
        };
        
        match rel {
            GraphRelationship::OccurredAt { event_id, location_id } => {
                assert_eq!(event_id, "e1");
                assert_eq!(location_id, "loc1");
            }
            _ => panic!("Wrong variant"),
        }
    }
}
