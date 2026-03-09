use crate::knowledge_graph::cognee_schema::{
    EventNode, FrequencyNode, PatternNode, SpatialLocationNode,
};
use neo4rs::*;
use std::sync::Arc;

/// A Neo4j-backed Knowledge Graph client for Track E.
/// Replaces the dummy in-memory graph to support millions of nodes.
#[derive(Clone)]
pub struct KnowledgeGraphClient {
    driver: Arc<Graph>,
}

impl KnowledgeGraphClient {
    /// Initialize connection to Neo4j and create schema constraints.
    pub async fn new(uri: &str, user: &str, pass: &str) -> anyhow::Result<Self> {
        let graph = Arc::new(Graph::new(uri, user, pass).await?);

        let client = Self { driver: graph };
        client.init_schema().await?;

        Ok(client)
    }

    /// Cypher DDL for schema constraints (Minimum viable schema for E)
    async fn init_schema(&self) -> anyhow::Result<()> {
        let queries = vec![
            "CREATE CONSTRAINT event_id IF NOT EXISTS FOR (e:Event) REQUIRE e.event_id IS UNIQUE",
            "CREATE CONSTRAINT pattern_id IF NOT EXISTS FOR (p:Pattern) REQUIRE p.pattern_id IS UNIQUE",
            "CREATE CONSTRAINT location_id IF NOT EXISTS FOR (l:SpatialLocation) REQUIRE l.location_id IS UNIQUE",
            "CREATE CONSTRAINT freq_id IF NOT EXISTS FOR (f:Frequency) REQUIRE f.frequency_hz IS UNIQUE",
            "CREATE CONSTRAINT device_id IF NOT EXISTS FOR (d:Device) REQUIRE d.device_name IS UNIQUE",
        ];

        for query in queries {
            let q = Query::new(query.to_string());
            let _ = self.driver.run(q).await;
        }

        Ok(())
    }

    /// Mode 2: Real-time event ingestion. Adds an Event node and links to a Device node.
    pub async fn create_event_node(&self, event: &EventNode) -> anyhow::Result<()> {
        let q = query(
            "
            MERGE (d:Device {device_name: $device_source})
            CREATE (e:Event {
                event_id: $id,
                timestamp_iso: $ts,
                timestamp_us: $ts_us,
                audio_rms_db: $audio_rms,
                rf_frequency_hz: $rf_freq,
                rf_peak_dbfs: $rf_peak,
                anomaly_score: $score
            })
            CREATE (e)-[:DetectedBy]->(d)
        ",
        )
        .param("device_source", event.device_source.clone())
        .param("id", event.event_id as i64)
        .param("ts", event.timestamp_iso.clone())
        .param("ts_us", event.timestamp_us as i64)
        .param("audio_rms", event.audio_rms_db as f64)
        .param("rf_freq", event.rf_frequency_hz as f64)
        .param("rf_peak", event.rf_peak_dbfs as f64)
        .param("score", event.anomaly_score as f64);

        self.driver.run(q).await?;
        Ok(())
    }

    /// Link an event to a specific pattern in the graph
    pub async fn link_event_to_pattern(
        &self,
        event_id: u64,
        pattern_id: usize,
    ) -> anyhow::Result<()> {
        let q = query(
            "
            MATCH (e:Event {event_id: $event_id})
            MATCH (p:Pattern {pattern_id: $pattern_id})
            MERGE (e)-[:HasPattern]->(p)
        ",
        )
        .param("event_id", event_id as i64)
        .param("pattern_id", pattern_id as i64);

        self.driver.run(q).await?;
        Ok(())
    }

    /// Link an event to a spatial location (Track D integration)
    pub async fn link_event_to_location(
        &self,
        event_id: u64,
        loc: &SpatialLocationNode,
    ) -> anyhow::Result<()> {
        let q = query(
            "
            MATCH (e:Event {event_id: $event_id})
            MERGE (l:SpatialLocation {location_id: $loc_id})
            ON CREATE SET
                l.azimuth_rad = $az,
                l.elevation_rad = $el,
                l.physical_interpretation = $interp
            MERGE (e)-[:OccurredAt]->(l)
        ",
        )
        .param("event_id", event_id as i64)
        .param("loc_id", loc.location_id as i64)
        .param("az", loc.azimuth_rad as f64)
        .param("el", loc.elevation_rad as f64)
        .param("interp", loc.physical_interpretation.clone());

        self.driver.run(q).await?;
        Ok(())
    }

    /// Link an event to a frequency (Track C integration)
    pub async fn link_event_to_frequency(
        &self,
        event_id: u64,
        freq: &FrequencyNode,
    ) -> anyhow::Result<()> {
        let q = query(
            "
            MATCH (e:Event {event_id: $event_id})
            MERGE (f:Frequency {frequency_hz: $freq_hz})
            ON CREATE SET
                f.band_name = $band,
                f.typical_confidence = $conf
            MERGE (e)-[:AtFrequency]->(f)
        ",
        )
        .param("event_id", event_id as i64)
        .param("freq_hz", freq.frequency_hz as f64)
        .param("band", freq.band_name.clone())
        .param("conf", freq.typical_confidence as f64);

        self.driver.run(q).await?;
        Ok(())
    }

    /// Mode 1: Create a Pattern Node
    pub async fn create_pattern_node(&self, pattern: &PatternNode) -> anyhow::Result<()> {
        let q = query(
            "
            MERGE (p:Pattern {pattern_id: $id})
            ON CREATE SET
                p.name = $name,
                p.frequency_hz = $freq,
                p.confidence = $conf,
                p.temporal_signature = $tsig
            ON MATCH SET
                p.name = $name,
                p.frequency_hz = $freq,
                p.confidence = $conf,
                p.temporal_signature = $tsig
        ",
        )
        .param("id", pattern.pattern_id as i64)
        .param("name", pattern.name.clone())
        .param("freq", pattern.frequency_hz as f64)
        .param("conf", pattern.confidence as f64)
        .param("tsig", pattern.temporal_signature.clone());

        self.driver.run(q).await?;
        Ok(())
    }

    /// Find patterns that match an event's attributes (e.g. frequency match)
    pub async fn find_matching_patterns(&self, event: &EventNode) -> anyhow::Result<Vec<usize>> {
        // Simplified heuristic: Find patterns where frequency matches within 1%
        let q = query(
            "
            MATCH (p:Pattern)
            WHERE abs(p.frequency_hz - $freq) / $freq < 0.01
            RETURN p.pattern_id as pattern_id
        ",
        )
        .param("freq", event.rf_frequency_hz as f64);

        let mut txn = self.driver.start_txn().await?;
        let mut stream = txn.execute(q).await?;
        let mut matching_patterns = Vec::new();
        while let Ok(Some(row)) = stream.next(&mut txn).await {
            let id: i64 = row.get("pattern_id")?;
            matching_patterns.push(id as usize);
        }
        Ok(matching_patterns)
    }

    /// Backfill links between a newly discovered pattern and all past events
    pub async fn backfill_pattern_links(&self, pattern: &PatternNode) -> anyhow::Result<()> {
        // Find events that match this pattern's frequency and link them
        let q = query(
            "
            MATCH (p:Pattern {pattern_id: $pid})
            MATCH (e:Event)
            WHERE abs(e.rf_frequency_hz - p.frequency_hz) / p.frequency_hz < 0.01
            MERGE (e)-[:HasPattern]->(p)
        ",
        )
        .param("pid", pattern.pattern_id as i64);

        self.driver.run(q).await?;
        Ok(())
    }

    /// Execute an arbitrary read-only Cypher query (used by Copilot tools)
    pub async fn execute_query(
        &self,
        q: neo4rs::Query,
    ) -> anyhow::Result<(neo4rs::Txn, neo4rs::RowStream)> {
        let mut txn = self.driver.start_txn().await?;
        let stream = txn.execute(q).await?;
        Ok((txn, stream))
    }
}
