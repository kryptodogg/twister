// tests/evidence_export_test.rs — Evidence Export Layer Tests (TDD)
//
// Tests for converting forensic data into court-ready evidence exports.
// Includes JSON for machines, Markdown for investigators, CSV for spreadsheets.

use chrono::{DateTime, Utc};
use twister::evidence_export::{
    export_csv_timeline, export_json_evidence, export_markdown_summary,
};
use twister::{AttackPatternReport, DetectionWithContext};

#[test]
fn test_json_export_structure() {
    // Given: Sample attack events
    let events = vec![
        DetectionWithContext {
            event_id: "twister_001_4521".to_string(),
            timestamp_utc: parse_time("2025-03-06T14:23:45.123Z"),
            rf_freq_hz: 750.0,
            rf_confidence: 0.92,
            bispectrum_method: "375+375→750".to_string(),
            sdr_dc_bias_v: 2.679,
            audio_dc_bias_v: Some(0.121),
            mamba_anomaly_db: 22.66,
        },
        DetectionWithContext {
            event_id: "twister_002_5200".to_string(),
            timestamp_utc: parse_time("2025-03-06T14:24:12.456Z"),
            rf_freq_hz: 1500.0,
            rf_confidence: 0.88,
            bispectrum_method: "phase-coupled".to_string(),
            sdr_dc_bias_v: 2.543,
            audio_dc_bias_v: Some(0.087),
            mamba_anomaly_db: 19.42,
        },
    ];

    // When: Export to JSON
    let json = export_json_evidence(&events, "Police Investigation", "2025-03-06");

    // Then: Should contain all required fields
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("JSON should parse");

    // Check investigation metadata
    assert!(parsed["investigation"]["title"].is_string());
    assert_eq!(parsed["investigation"]["title"], "Police Investigation");
    assert!(parsed["investigation"]["date"].is_string());
    assert_eq!(parsed["investigation"]["date"], "2025-03-06");
    assert!(parsed["investigation"]["total_events"].is_number());
    assert_eq!(parsed["investigation"]["total_events"], 2);
    assert!(parsed["investigation"]["unique_frequencies"].is_number());
    assert!(parsed["investigation"]["conclusion"].is_string());

    // Check events array
    assert!(parsed["events"].is_array());
    let events_arr = parsed["events"].as_array().unwrap();
    assert_eq!(events_arr.len(), 2);

    // Check first event structure
    let first = &events_arr[0];
    assert!(first["event_id"].is_string());
    assert_eq!(first["event_id"], "twister_001_4521");
    assert!(first["timestamp_utc"].is_string());
    assert!(first["rf_frequency_hz"].is_number());
    let freq_val = first["rf_frequency_hz"].as_f64().unwrap();
    assert!(
        (freq_val - 750.0).abs() < 0.1,
        "Frequency should be ~750 Hz"
    );
    assert!(first["rf_confidence"].is_number());
    let conf_val = first["rf_confidence"].as_f64().unwrap();
    assert!((conf_val - 0.92).abs() < 0.01, "Confidence should be ~0.92");
    assert!(first["audio_dc_bias_v"].is_number());
    assert!(first["sdr_dc_bias_v"].is_number());
    assert!(first["mamba_anomaly_db"].is_number());
    assert!(first["forensic_classification"].is_string());
    assert!(first["attack_vector_proof"].is_string());
}

#[test]
fn test_json_export_event_classification() {
    // Test that events are classified correctly based on thresholds
    let events = vec![
        // Simultaneous RF + DC attack
        DetectionWithContext {
            event_id: "sim_001".to_string(),
            timestamp_utc: parse_time("2025-03-06T14:23:45.123Z"),
            rf_freq_hz: 750.0,
            rf_confidence: 0.92,
            bispectrum_method: "test".to_string(),
            sdr_dc_bias_v: 2.679,
            audio_dc_bias_v: Some(0.121),
            mamba_anomaly_db: 22.66,
        },
        // RF only (no audio DC bias)
        DetectionWithContext {
            event_id: "rf_001".to_string(),
            timestamp_utc: parse_time("2025-03-06T14:24:12.456Z"),
            rf_freq_hz: 1500.0,
            rf_confidence: 0.95,
            bispectrum_method: "test".to_string(),
            sdr_dc_bias_v: 1.5,
            audio_dc_bias_v: None,
            mamba_anomaly_db: 19.42,
        },
    ];

    let json = export_json_evidence(&events, "Test", "2025-03-06");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("JSON should parse");

    let events_arr = parsed["events"].as_array().unwrap();

    // First event should be SIMULTANEOUS_RF_DC_ATTACK
    assert_eq!(
        events_arr[0]["forensic_classification"],
        "SIMULTANEOUS_RF_DC_ATTACK"
    );
    assert!(
        events_arr[0]["attack_vector_proof"]
            .as_str()
            .unwrap()
            .contains("simultaneously")
    );

    // Second event should be RF_ONLY_ATTACK (high confidence, no DC)
    assert_eq!(events_arr[1]["forensic_classification"], "RF_ONLY_ATTACK");
}

#[test]
fn test_markdown_summary_readable() {
    // Given: Attack pattern report
    let report = AttackPatternReport {
        period_hours: 24,
        total_attacks: 47,
        unique_frequencies: 3,
        avg_duration_seconds: 2.1,
        attack_time_window: "08:00-22:00".to_string(),
        dc_bias_correlation_percent: 93.6,
        mamba_anomaly_correlation_percent: 100.0,
        attacks_throughout_day: false,
        conclusion: "Coordinated, targeted attacks. Not environmental.".to_string(),
    };

    // When: Export to markdown
    let md = export_markdown_summary(&report);

    // Then: Should be human-readable with investigation conclusion
    assert!(md.contains("47"), "Should show attack count (47)");
    assert!(md.contains("93.6"), "Should show DC correlation percentage");
    assert!(md.contains("100"), "Should show Mamba correlation");
    assert!(md.contains("Coordinated"), "Should include conclusion");
    assert!(md.contains("24"), "Should show period (24 hours)");
    assert!(md.contains("08:00-22:00"), "Should show time window");
    assert!(md.contains("Investigation Report"), "Should be a report");

    // No JSON artifacts (but allow {} in narrative text like "DELIBERATE TARGETING")
    // Just ensure the structure isn't JSON (no [ or ] brackets for JSON arrays)
    assert!(!md.contains("["), "Should not contain JSON array brackets");
    assert!(!md.contains("]"), "Should not contain JSON array brackets");
}

#[test]
fn test_markdown_includes_forensic_section() {
    let report = AttackPatternReport {
        period_hours: 48,
        total_attacks: 85,
        unique_frequencies: 5,
        avg_duration_seconds: 3.5,
        attack_time_window: "06:00-23:00".to_string(),
        dc_bias_correlation_percent: 88.5,
        mamba_anomaly_correlation_percent: 100.0,
        attacks_throughout_day: false,
        conclusion: "Clear evidence of deliberate targeting.".to_string(),
    };

    let md = export_markdown_summary(&report);

    // Should have forensic sections
    assert!(
        md.contains("DC Bias Correlation"),
        "Should have DC Bias section"
    );
    assert!(
        md.contains("Mamba Anomaly Detection"),
        "Should have Mamba section"
    );
    assert!(md.contains("Conclusion"), "Should have Conclusion section");

    // Should explain what these mean forensically
    assert!(
        md.contains("proof of coordination") || md.contains("DELIBERATE"),
        "Should explain forensic significance"
    );
}

#[test]
fn test_csv_timeline() {
    // Given: Sample events
    let events = vec![
        DetectionWithContext {
            event_id: "csv_001".to_string(),
            timestamp_utc: parse_time("2025-03-06T14:23:45.123Z"),
            rf_freq_hz: 750.0,
            rf_confidence: 0.92,
            bispectrum_method: "test".to_string(),
            sdr_dc_bias_v: 2.679,
            audio_dc_bias_v: Some(0.121),
            mamba_anomaly_db: 22.66,
        },
        DetectionWithContext {
            event_id: "csv_002".to_string(),
            timestamp_utc: parse_time("2025-03-06T14:24:12.456Z"),
            rf_freq_hz: 1500.0,
            rf_confidence: 0.88,
            bispectrum_method: "test".to_string(),
            sdr_dc_bias_v: 2.543,
            audio_dc_bias_v: None,
            mamba_anomaly_db: 19.42,
        },
    ];

    // When: Export to CSV
    let csv = export_csv_timeline(&events);

    // Then: Should have proper CSV structure
    let lines: Vec<&str> = csv.lines().collect();

    // Header line
    assert!(!lines.is_empty(), "Should have at least header line");
    assert!(
        lines[0].contains("timestamp"),
        "Header should have timestamp"
    );
    assert!(
        lines[0].contains("frequency"),
        "Header should have frequency"
    );
    assert!(
        lines[0].contains("rf_confidence"),
        "Header should have rf_confidence"
    );
    assert!(lines[0].contains("dc_v"), "Header should have dc_v");
    assert!(lines[0].contains("anomaly"), "Header should have anomaly");

    // Data rows
    assert_eq!(lines.len(), 3, "Should have header + 2 data rows");
    assert!(lines[1].contains("750"), "First event should have 750 Hz");
    assert!(
        lines[2].contains("1500"),
        "Second event should have 1500 Hz"
    );

    // Should be valid CSV (can split on commas)
    let header_fields: Vec<&str> = lines[0].split(',').collect();
    let first_data_fields: Vec<&str> = lines[1].split(',').collect();
    assert_eq!(
        header_fields.len(),
        first_data_fields.len(),
        "All rows should have same field count"
    );
}

#[test]
fn test_csv_timeline_with_missing_audio_bias() {
    // Test CSV handles missing audio DC bias gracefully
    let events = vec![DetectionWithContext {
        event_id: "missing_001".to_string(),
        timestamp_utc: parse_time("2025-03-06T14:23:45.123Z"),
        rf_freq_hz: 750.0,
        rf_confidence: 0.92,
        bispectrum_method: "test".to_string(),
        sdr_dc_bias_v: 2.679,
        audio_dc_bias_v: None, // Missing audio DC bias
        mamba_anomaly_db: 22.66,
    }];

    let csv = export_csv_timeline(&events);
    let lines: Vec<&str> = csv.lines().collect();

    assert_eq!(lines.len(), 2, "Should have header + 1 data row");
    // Should not crash when audio_dc_bias_v is None (should output 0.0)
    assert!(
        lines[1].contains("0.0") || lines[1].contains("0.000"),
        "Missing audio DC should output 0.0"
    );
}

#[test]
fn test_json_export_multiple_events_unique_freqs() {
    // Test that unique frequency count works correctly
    let events = vec![
        DetectionWithContext {
            event_id: "freq_001".to_string(),
            timestamp_utc: parse_time("2025-03-06T14:23:00Z"),
            rf_freq_hz: 750.0,
            rf_confidence: 0.9,
            bispectrum_method: "test".to_string(),
            sdr_dc_bias_v: 2.5,
            audio_dc_bias_v: Some(0.1),
            mamba_anomaly_db: 20.0,
        },
        DetectionWithContext {
            event_id: "freq_002".to_string(),
            timestamp_utc: parse_time("2025-03-06T14:23:05Z"),
            rf_freq_hz: 750.0, // Same frequency
            rf_confidence: 0.91,
            bispectrum_method: "test".to_string(),
            sdr_dc_bias_v: 2.6,
            audio_dc_bias_v: Some(0.12),
            mamba_anomaly_db: 21.0,
        },
        DetectionWithContext {
            event_id: "freq_003".to_string(),
            timestamp_utc: parse_time("2025-03-06T14:23:10Z"),
            rf_freq_hz: 1500.0, // Different frequency
            rf_confidence: 0.88,
            bispectrum_method: "test".to_string(),
            sdr_dc_bias_v: 2.4,
            audio_dc_bias_v: Some(0.09),
            mamba_anomaly_db: 19.0,
        },
    ];

    let json = export_json_evidence(&events, "Test", "2025-03-06");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("JSON should parse");

    // Should count 2 unique frequencies (750 and 1500)
    assert_eq!(parsed["investigation"]["unique_frequencies"], 2);
    assert_eq!(parsed["investigation"]["total_events"], 3);
}

// ── Helper Functions ──────────────────────────────────────────────────────

fn parse_time(time_str: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(time_str)
        .expect("Failed to parse time")
        .with_timezone(&Utc)
}
