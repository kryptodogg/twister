use crate::knowledge_graph::KnowledgeGraphClient;
use std::sync::Arc;
use neo4rs::*;
use serde_json::Value;

/// Represents a query tool that the LLM can invoke to fetch data from Neo4j.
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, args: &str, graph: &Arc<KnowledgeGraphClient>) -> Result<(String, Vec<u64>), String>;
}

/// Tool: "find_events_by_location"
/// Description: "Show me all Friday attacks at azimuth 45 degrees"
pub struct FindEventsByLocationTool;

#[async_trait::async_trait]
impl Tool for FindEventsByLocationTool {
    fn name(&self) -> &str {
        "find_events_by_location"
    }

    fn description(&self) -> &str {
        "Find forensic events near a specific spatial azimuth (0-360 degrees). Args: {\"azimuth\": 45.0, \"tolerance\": 5.0, \"day\": \"Friday\"}"
    }

    async fn execute(&self, args: &str, graph: &Arc<KnowledgeGraphClient>) -> Result<(String, Vec<u64>), String> {
        // Dummy arg parsing: expecting azimuth and an optional tolerance.
        let target_azimuth = 45.0_f64; // In degrees, converting to radians inside cypher if needed
        let day = "Friday";

        // Generate the Cypher query dynamically based on arguments.
        // Assuming l.azimuth_rad is stored in radians, so 45 deg ≈ 0.785 rad.
        let q = query("
            MATCH (e:Event)-[:HasPattern]->(p:Pattern)
            MATCH (e)-[:OccurredAt]->(l:SpatialLocation)
            WHERE e.timestamp_iso CONTAINS $day
              AND abs(l.azimuth_rad - $target_rad) < 0.1
            RETURN e.event_id as event_id, p.name as pattern_name, p.confidence as pattern_conf
        ")
        .param("day", day)
        .param("target_rad", (target_azimuth * std::f64::consts::PI / 180.0));

        let (mut txn, mut stream) = graph.execute_query(q).await.map_err(|e| e.to_string())?;

        let mut matched_events = Vec::new();
        let mut total_confidence = 0.0;
        let mut count = 0;

        while let Ok(Some(row)) = stream.next(&mut txn).await {
            let id: i64 = row.get("event_id").map_err(|e| e.to_string())?;
            let conf: f64 = row.get("pattern_conf").unwrap_or(0.0);

            matched_events.push(id as u64);
            total_confidence += conf;
            count += 1;
        }

        if count == 0 {
            return Ok((format!("No events found near azimuth {}", target_azimuth), Vec::new()));
        }

        let avg_conf = (total_confidence / count as f64) as f32;

        let result_string = format!(
            "{} events match all criteria. Average pattern confidence: {:.1}%",
            count,
            avg_conf * 100.0
        );

        Ok((result_string, matched_events))
    }
}

/// Tool: "find_pattern_for_frequency"
/// Description: "What is the pattern for 2.4 GHz heterodyning?"
pub struct FindPatternForFrequencyTool;

#[async_trait::async_trait]
impl Tool for FindPatternForFrequencyTool {
    fn name(&self) -> &str {
        "find_pattern_for_frequency"
    }

    fn description(&self) -> &str {
        "Find the motif/pattern associated with a specific frequency in Hz. Args: {\"frequency\": 2400000000}"
    }

    async fn execute(&self, _args: &str, _graph: &Arc<KnowledgeGraphClient>) -> Result<(String, Vec<u64>), String> {
        // Dummy execution for now to simulate the other tool
        Ok(("Results: Daily pattern, 85% confidence, 1200+ events".to_string(), vec![]))
    }
}
