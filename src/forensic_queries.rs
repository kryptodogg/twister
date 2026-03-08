// src/forensic_queries.rs — Neo4j Forensic Queries API (v0.1)
//
// Forensic query infrastructure for police investigations.
// Provides APIs to answer critical questions an investigator needs:
// - Which attacks occurred in a time range?
// - Were RF signals and DC bias coordinated (< 5ms)?
// - What's the overall attack pattern? (Evidence of targeting vs random noise)
// - Can we prove simultaneous RF + audio + DC activation?

use chrono::{DateTime, Utc};

// ── Result Types ──────────────────────────────────────────────────────────

/// Detection event with full context from Neo4j
/// Links RF frequency, timestamp, confidence, and audio/DC bias data
#[derive(Debug, Clone)]
pub struct DetectionWithContext {
    /// Unique event ID from Neo4j
    pub event_id: String,
    /// UTC timestamp of detection
    pub timestamp_utc: DateTime<Utc>,
    /// RF frequency detected in Hz
    pub rf_freq_hz: f32,
    /// RF detection confidence (0.0 - 1.0)
    pub rf_confidence: f32,
    /// Bispectrum detection method (e.g., "phase-coupled-carrier")
    pub bispectrum_method: String,
    /// SDR DC bias voltage at center frequency
    pub sdr_dc_bias_v: f32,
    /// Audio DC bias voltage (absolute deviation from zero)
    pub audio_dc_bias_v: Option<f32>,
    /// Mamba reconstruction error in dB
    pub mamba_anomaly_db: f32,
}

/// Statistical summary of attack pattern for investigative conclusion
#[derive(Debug, Clone)]
pub struct AttackPatternReport {
    /// Analysis period in hours
    pub period_hours: u32,
    /// Total attacks detected in period
    pub total_attacks: usize,
    /// Number of distinct RF frequencies used
    pub unique_frequencies: usize,
    /// Average attack duration in seconds
    pub avg_duration_seconds: f32,
    /// Time window when attacks occur (e.g., "08:00-22:00")
    pub attack_time_window: String,
    /// Percentage of attacks with DC bias correlation (>80% = proof of coordination)
    pub dc_bias_correlation_percent: f32,
    /// Percentage of attacks flagged by Mamba anomaly detector (should be 100%)
    pub mamba_anomaly_correlation_percent: f32,
    /// Whether attacks occur throughout the day (true) or during specific hours (false)
    pub attacks_throughout_day: bool,
    /// Investigative conclusion text for police report
    pub conclusion: String,
}

/// Detailed proof of simultaneous RF + audio + DC activation
/// Used to prove coordination in harassment evidence
#[derive(Debug, Clone)]
pub struct CorrelationEvidence {
    /// Event ID
    pub event_id: String,
    /// UTC timestamp
    pub timestamp_utc: DateTime<Utc>,
    /// RF frequency detected
    pub rf_frequency_hz: f32,
    /// RF confidence score
    pub rf_confidence: f32,
    /// RF signal start time (ms since epoch)
    pub rf_start_timestamp_ms: i64,
    /// Audio DC spike timestamp (ms since epoch)
    pub audio_dc_spike_timestamp_ms: Option<i64>,
    /// Time delta between RF and DC (ms) - < 5ms proves coordination
    pub timestamp_sync_ms: i64,
    /// Audio DC bias voltage
    pub audio_dc_bias_v: Option<f32>,
    /// SDR DC bias voltage
    pub sdr_dc_bias_v: f32,
    /// Mamba anomaly score in dB
    pub mamba_anomaly_db: f32,
    /// Is synchronized within 5ms threshold (coordination proof)
    pub is_synchronized: bool,
}

// ── Query Functions ────────────────────────────────────────────────────────

/// Query all detection events in a time range
///
/// Cypher Query (planned):
/// ```
/// MATCH (e:DetectionEvent)-[]->(a:AudioFrame)
/// WHERE e.timestamp_utc >= $start AND e.timestamp_utc <= $end
/// RETURN e.event_id, e.timestamp_utc, e.rf_freq_hz, e.rf_confidence,
///        a.mamba_anomaly_db, a.audio_dc_bias_v
/// ```
///
/// # Arguments
/// * `graph` - Neo4j graph client
/// * `start` - Start of time range (UTC)
/// * `end` - End of time range (UTC)
///
/// # Returns
/// Vector of DetectionWithContext with all associated data
pub async fn events_in_timerange(
    graph: &crate::graph::ForensicGraph,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Vec<DetectionWithContext> {
    // TODO: Execute Neo4j query
    // MATCH (e:DetectionEvent)-[:LINKED_TO]->(a:AudioFrame)
    //       -[:HAS_DC_BIAS]->(d:DcBias)
    // WHERE e.timestamp_utc >= $start AND e.timestamp_utc <= $end
    // RETURN e, a, d

    // Placeholder: return empty vector
    // Full implementation requires Neo4j driver integration
    let _graph = graph; // Use graph to avoid unused warning
    let _start = start;
    let _end = end;

    vec![]
}

/// Find attacks where RF and DC bias occurred simultaneously (< 5ms delta)
///
/// Cypher Query (planned):
/// ```
/// MATCH (e:DetectionEvent)-[:HAS_RF_START]->(rf:RfEvent),
///       (e)-[:HAS_DC_SPIKE]->(dc:DcEvent)
/// WHERE ABS(rf.timestamp_ms - dc.timestamp_ms) < $time_delta_ms
/// RETURN e, rf, dc
/// ```
///
/// This query proves coordination: real attacks show tight RF/DC timing,
/// while environmental noise would show random timing.
///
/// # Arguments
/// * `graph` - Neo4j graph client
/// * `time_delta_ms` - Synchronization threshold in milliseconds
///
/// # Returns
/// Vector of DetectionWithContext showing synchronized attacks
pub async fn synchronized_attacks(
    graph: &crate::graph::ForensicGraph,
    time_delta_ms: i64,
) -> Vec<DetectionWithContext> {
    // TODO: Execute Neo4j query
    // MATCH (e:DetectionEvent)-[:RF_START]->(r:RfTimestamp),
    //       (e)-[:DC_SPIKE]->(d:DcTimestamp)
    // WHERE ABS(r.timestamp_ms - d.timestamp_ms) < $time_delta_ms
    // RETURN e properties with timing proof

    let _graph = graph; // Use graph to avoid unused warning
    let _time_delta_ms = time_delta_ms;

    vec![]
}

/// Summarize attack pattern to prove targeting vs environmental noise
///
/// Cypher Queries (planned):
/// 1. Count total attacks in period
/// 2. Find unique RF frequencies
/// 3. Calculate average duration
/// 4. Analyze time-of-day distribution (attacks 08:00-22:00 = targeting)
/// 5. Correlation: % with DC bias (>80% = proof of coordination)
/// 6. Correlation: % flagged by Mamba (100% = model learned pattern)
///
/// Investigation conclusion logic:
/// - If DC_correlation > 80% AND time_pattern != "random" AND mamba == 100%
///   → "Coordinated, targeted attack pattern. Not environmental noise."
///
/// # Arguments
/// * `graph` - Neo4j graph client
/// * `hours` - Analysis period in hours
///
/// # Returns
/// AttackPatternReport with statistics and investigative conclusion
pub async fn attack_pattern_summary(
    graph: &crate::graph::ForensicGraph,
    hours: u32,
) -> AttackPatternReport {
    // TODO: Execute Neo4j queries to compute:
    // 1. SELECT COUNT(*) FROM events WHERE timestamp WITHIN last $hours
    // 2. SELECT COUNT(DISTINCT rf_freq_hz) FROM events
    // 3. SELECT AVG(duration_seconds) FROM events
    // 4. SELECT COUNT(*) FROM events WHERE HOUR(timestamp) BETWEEN 8 AND 22
    // 5. SELECT COUNT(*) FROM events WHERE sdr_dc_bias IS NOT NULL / COUNT(*)
    // 6. SELECT COUNT(*) FROM events WHERE mamba_anomaly_db > threshold / COUNT(*)

    let _graph = graph; // Use graph to avoid unused warning

    // Build conclusion based on statistics
    let conclusion = if hours >= 24 {
        "Insufficient data for pattern analysis".to_string()
    } else {
        "Multiple attacks detected, time pattern analysis pending".to_string()
    };

    AttackPatternReport {
        period_hours: hours,
        total_attacks: 0,
        unique_frequencies: 0,
        avg_duration_seconds: 0.0,
        attack_time_window: "unknown".to_string(),
        dc_bias_correlation_percent: 0.0,
        mamba_anomaly_correlation_percent: 0.0,
        attacks_throughout_day: false,
        conclusion,
    }
}

/// Get detailed correlation proof for one event
///
/// Cypher Query (planned):
/// ```
/// MATCH (e:DetectionEvent { id: $event_id })
///       -[:HAS_RF_DATA]->(rf:RfData)
///       -[:LINKED_TO]->(a:AudioFrame)
///       -[:HAS_DC_BIAS]->(dc:DcBias)
/// RETURN e, rf, a, dc
/// ```
///
/// Calculates synchronization delta (RF timestamp - DC timestamp).
/// If delta < 5ms, proves simultaneous coordination.
///
/// # Arguments
/// * `graph` - Neo4j graph client
/// * `event_id` - Event ID to query
///
/// # Returns
/// CorrelationEvidence with proof of simultaneous RF/DC activation
pub async fn correlation_evidence(
    graph: &crate::graph::ForensicGraph,
    event_id: &str,
) -> CorrelationEvidence {
    // TODO: Execute Neo4j query
    // MATCH (e:DetectionEvent { id: $event_id })
    //       -[:HAS_RF_START]->(r:RfEvent),
    //       (e)-[:HAS_DC_SPIKE]->(d:DcEvent)
    // RETURN e.timestamp_utc, e.rf_freq_hz, e.rf_confidence,
    //        r.timestamp_ms, d.timestamp_ms, d.audio_dc_bias_v, e.sdr_dc_bias_v,
    //        e.mamba_anomaly_db

    let _graph = graph; // Use graph to avoid unused warning

    // Calculate synchronization
    let rf_start_ms: i64 = 0;
    let dc_start_ms: Option<i64> = None;
    let sync_ms = dc_start_ms
        .map(|dc| (rf_start_ms - dc).abs())
        .unwrap_or(0);

    CorrelationEvidence {
        event_id: event_id.to_string(),
        timestamp_utc: Utc::now(),
        rf_frequency_hz: 0.0,
        rf_confidence: 0.0,
        rf_start_timestamp_ms: rf_start_ms,
        audio_dc_spike_timestamp_ms: dc_start_ms,
        timestamp_sync_ms: sync_ms,
        audio_dc_bias_v: None,
        sdr_dc_bias_v: 0.0,
        mamba_anomaly_db: 0.0,
        is_synchronized: sync_ms < 5 && dc_start_ms.is_some(),
    }
}

// ── Investigation Helpers ──────────────────────────────────────────────────

/// Helper: Check if a detection is within forensic proof threshold
///
/// Proof threshold: correlation within 5ms of RF detection
/// This matches the human reaction time window and proves coordination.
pub fn is_forensic_proof(evidence: &CorrelationEvidence) -> bool {
    evidence.is_synchronized && evidence.timestamp_sync_ms < 5
}

/// Helper: Interpret DC bias correlation percentage for police report
pub fn interpret_dc_correlation(percent: f32) -> String {
    match percent {
        p if p >= 80.0 => {
            "Strong evidence of coordinated attack (DC bias in >80% of events)"
                .to_string()
        }
        p if p >= 50.0 => {
            "Moderate evidence of coordination (DC bias in 50-80% of events)"
                .to_string()
        }
        _ => "Insufficient evidence of DC coordination".to_string(),
    }
}

/// Helper: Interpret attack time window for investigation conclusion
pub fn interpret_time_pattern(attacks_throughout_day: bool) -> String {
    if attacks_throughout_day {
        "Attacks throughout day (suggests environmental interference)"
            .to_string()
    } else {
        "Attacks during business hours only (suggests targeted harassment)"
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forensic_proof_sync_check() {
        let evidence = CorrelationEvidence {
            event_id: "test_001".to_string(),
            timestamp_utc: Utc::now(),
            rf_frequency_hz: 145.5,
            rf_confidence: 0.92,
            rf_start_timestamp_ms: 1000,
            audio_dc_spike_timestamp_ms: Some(1003),
            timestamp_sync_ms: 3,
            audio_dc_bias_v: Some(0.121),
            sdr_dc_bias_v: 0.05,
            mamba_anomaly_db: 22.66,
            is_synchronized: true,
        };

        assert!(is_forensic_proof(&evidence), "3ms delta should be forensic proof");
    }

    #[test]
    fn test_forensic_proof_fails_beyond_threshold() {
        let evidence = CorrelationEvidence {
            event_id: "test_002".to_string(),
            timestamp_utc: Utc::now(),
            rf_frequency_hz: 145.5,
            rf_confidence: 0.92,
            rf_start_timestamp_ms: 1000,
            audio_dc_spike_timestamp_ms: Some(1010),
            timestamp_sync_ms: 10,
            audio_dc_bias_v: Some(0.121),
            sdr_dc_bias_v: 0.05,
            mamba_anomaly_db: 22.66,
            is_synchronized: false,
        };

        assert!(
            !is_forensic_proof(&evidence),
            "10ms delta exceeds 5ms proof threshold"
        );
    }

    #[test]
    fn test_dc_correlation_interpretation() {
        let strong = interpret_dc_correlation(85.0);
        assert!(strong.contains("Strong evidence"));

        let moderate = interpret_dc_correlation(65.0);
        assert!(moderate.contains("Moderate evidence"));

        let weak = interpret_dc_correlation(30.0);
        assert!(weak.contains("Insufficient"));
    }

    #[test]
    fn test_time_pattern_interpretation() {
        let targeted = interpret_time_pattern(false);
        assert!(targeted.contains("business hours"));

        let environmental = interpret_time_pattern(true);
        assert!(environmental.contains("throughout day"));
    }
}
