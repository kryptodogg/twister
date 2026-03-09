use neo4rs::{Graph, query};

// src/graph.rs — Graph DB Facade  (v0.4)

#[derive(Clone)]
pub struct ForensicGraph {
    pub client: Graph,
}

impl ForensicGraph {
    pub async fn new(uri: &str, user: &str, pass: &str) -> anyhow::Result<Self> {
        let client = Graph::new(uri, user, pass).await?;
        Ok(Self { client })
    }

    pub async fn store_detection(
        &self,
        _event: &crate::detection::DetectionEvent,
    ) -> anyhow::Result<()> {
        // Implementation for general detection storing
        Ok(())
    }

    pub async fn link_detection(
        &self,
        event_id: &str,
        audio_hz: f32,
        rf_hz: f32,      // The RTL-SDR reference tuner freq
        true_rf_hz: f32, // The aliased RF harmonic reconstructed from wideband
        audio_dc_bias: f32,
        rf_dc_bias: f32,
    ) -> anyhow::Result<()> {
        let q = query(
            "
            MERGE (e:Event { id: $event_id })
            SET e.audio_hz = $audio_hz,
                e.rf_hz = $rf_hz,
                e.true_rf_hz = $true_rf_hz,
                e.timestamp = datetime()
            
            MERGE (a:AudioFreq { hz: $audio_hz })
            MERGE (r:RfFreq { hz: $rf_hz })
            MERGE (tr:RfFreq { hz: $true_rf_hz })
            
            MERGE (e)-[:HAS_AUDIO { dc_bias: $audio_dc_bias }]->(a)
            MERGE (e)-[:HAS_RF { dc_bias: $rf_dc_bias }]->(r)
            MERGE (e)-[:HAS_TRUE_HARMONIC]->(tr)
        ",
        )
        .param("event_id", event_id)
        .param("audio_hz", audio_hz as f64)
        .param("rf_hz", rf_hz as f64)
        .param("true_rf_hz", true_rf_hz as f64)
        .param("audio_dc_bias", audio_dc_bias as f64)
        .param("rf_dc_bias", rf_dc_bias as f64);

        self.client.run(q).await?;
        Ok(())
    }
}
