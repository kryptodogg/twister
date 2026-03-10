with open('src/forensic.rs', 'r') as f:
    content = f.read()

# Replace log_gate_decision
old_str_gate = """    pub fn log_gate_decision(&mut self, score: f32, confidence: f32, threshold: f32, forward: bool, reason: &str) -> anyhow::Result<()> {
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
    }"""

new_str_gate = """    pub fn log_gate_decision(&self, score: f32, confidence: f32, threshold: f32, forward: bool, reason: &str) -> anyhow::Result<()> {
        let fe = ForensicEvent::AnomalyGateDecision {
            timestamp_micros: get_current_micros(),
            anomaly_score: score,
            confidence: confidence,
            threshold_used: threshold,
            forward_to_trainer: forward,
            reason: reason.to_string(),
        };
        self.sender.send(fe).map_err(|e| anyhow::anyhow!(e))
    }"""
content = content.replace(old_str_gate, new_str_gate)

# Replace log_detection
old_str_det = """    pub fn log_detection(&mut self, event: &DetectionEvent) -> anyhow::Result<()> {
        self.event_count += 1;

        // Create forensic event with full metadata
        let forensic_event =
            ForensicEvent::from_detection(event, &self.session_id, self.equipment.clone());

        // Log as forensic event
        let record = serde_json::to_string(&forensic_event)?;
        writeln!(self.writer, "{}", record)?;
        self.writer.flush()?;

        // Also print to console for demo visibility
        println!(
            "[Forensic] Detect: {:.1} Hz (mag: {:.2}, conf: {:.2})",
            event.f1_hz, event.magnitude, event.coherence_frames
        );
        Ok(())
    }"""

new_str_det = """    pub fn log_detection_old(&mut self, event: &DetectionEvent) -> anyhow::Result<()> {
        Ok(())
    }"""
content = content.replace(old_str_det, new_str_det)

with open('src/forensic.rs', 'w') as f:
    f.write(content)
