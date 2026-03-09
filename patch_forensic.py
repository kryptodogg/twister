import re

with open("src/forensic.rs", "r") as f:
    content = f.read()

if "AnomalyGateDecision {" not in content:
    replacement = """    Bispectrum {
        f1_hz: f32,
        f2_hz: f32,
        product_hz: f32,
        magnitude: f32,
        coherence_frames: u32,
    },
    AnomalyGateDecision {
        anomaly_score: f32,
        confidence: f32,
        threshold_used: f32,
        forward_to_trainer: bool,
        reason: String,
    },
}"""
    content = content.replace("    Bispectrum {\n        f1_hz: f32,\n        f2_hz: f32,\n        product_hz: f32,\n        magnitude: f32,\n        coherence_frames: u32,\n    },\n}", replacement)

    replacement2 = """    pub fn log_gate_decision(&mut self, score: f32, confidence: f32, threshold: f32, forward: bool, reason: &str) -> anyhow::Result<()> {
        let now = std::time::SystemTime::now();
        let unix_ts = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64();
        let utc_ts = chrono::DateTime::from_timestamp(unix_ts as i64, 0).unwrap_or_default().to_rfc3339();

        let event = ForensicEvent {
            id: format!("gate_{}", unix_ts),
            timestamp_utc: utc_ts,
            timestamp_unix: unix_ts,
            session_id: self.session_id.clone(),
            event_type: ForensicEventType::AnomalyGateDecision {
                anomaly_score: score,
                confidence,
                threshold_used: threshold,
                forward_to_trainer: forward,
                reason: reason.to_string(),
            },
            confidence,
            duration_seconds: 0.0,
            equipment: self.equipment.clone(),
            metadata: std::collections::HashMap::new(),
        };

        let record = serde_json::to_string(&event)?;
        writeln!(self.writer, "{}", record)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn log_detection"""

    content = content.replace("    pub fn log_detection", replacement2)

with open("src/forensic.rs", "w") as f:
    f.write(content)
