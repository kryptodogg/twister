with open('src/forensic.rs', 'r') as f:
    content = f.read()

# Let's fix the specific error with ForensicEvent initialization.
old_event_init = """        let event = ForensicEvent {
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
        };"""

# Wait, `ForensicEvent` is an enum, not a struct! Let's check `ForensicEvent` definition.
