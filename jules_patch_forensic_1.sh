cat << 'INNER_EOF' > src/forensic.rs.new
// src/forensic.rs — Forensic Event Logger  (v0.5)
//
// Evidence collection for harassment defense and investigation.
// Logs detections with court-admissible timestamps and calibration data.

use crate::detection::DetectionEvent;
use chrono;
use csv;
use std::collections::HashMap;
use std::fs::{File, OpenOptions, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::io::AsyncWriteExt;

pub fn get_current_micros() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "event_type")]
pub enum ForensicEvent {
    // ─────────────────────────────────────────────────────
    // Audio Processing Events (Track A)
    // ─────────────────────────────────────────────────────

    AudioFrameProcessed {
        timestamp_micros: u64,
        device_idx: u32,
        sample_rate_hz: u32,
        frame_size_samples: u32,
        rms_db: f32,
        peak_db: f32,
        clipping_detected: bool,
    },

    FFTComputed {
        timestamp_micros: u64,
        frequency_bins: u32,
        peak_frequency_hz: f32,
        peak_magnitude: f32,
        spectral_centroid_hz: f32,
    },

    // ─────────────────────────────────────────────────────
    // RF Detection Events (Track A)
    // ─────────────────────────────────────────────────────

    RFDetection {
        timestamp_micros: u64,
        frequency_hz: f32,
        power_dbfs: f32,
        modulation_type: String,
        bandwidth_hz: f32,
        confidence: f32,
    },

    HeterodyneMixing {
        timestamp_micros: u64,
        carrier_hz: f32,
        audio_hz: f32,
        lower_sideband_hz: f32,
        upper_sideband_hz: f32,
        mixing_power_db: f32,
    },

    // ─────────────────────────────────────────────────────
    // TDOA Events (Spatial Analysis)
    // ─────────────────────────────────────────────────────

    TDOAEstimation {
        timestamp_micros: u64,
        mic_pair_indices: [u32; 2],
        time_delay_micros: f32,
        correlation_quality: f32,
        azimuth_degrees: f32,
        elevation_degrees: f32,
    },

    // ─────────────────────────────────────────────────────
    // Mamba Anomaly Detection (Track D)
    // ─────────────────────────────────────────────────────

    MambaInference {
        timestamp_micros: u64,
        input_dimension: u32,
        latent_dimension: u32,
        reconstruction_mse: f32,
        processing_time_us: u32,
    },

    AnomalyGateDecision {
        timestamp_micros: u64,
        anomaly_score: f32,
        confidence: f32,
        threshold_used: f32,
        forward_to_trainer: bool,
        reason: String,
    },

    TrainingPairEnqueued {
        timestamp_micros: u64,
        training_pair_id: u64,
        input_hash: u64,
        anomaly_score: f32,
        confidence: f32,
    },

    // ─────────────────────────────────────────────────────
    // Training & Convergence (Track D)
    // ─────────────────────────────────────────────────────

    TrainingStepCompleted {
        timestamp_micros: u64,
        epoch: u32,
        batch_number: u32,
        loss: f32,
        learning_rate: f32,
        gradient_norm: f32,
    },

    TrainingConvergence {
        timestamp_micros: u64,
        epoch: u32,
        loss_moving_avg: f32,
        loss_std_dev: f32,
        convergence_status: String,
    },

    // ─────────────────────────────────────────────────────
    // Temporal Analysis (Phase 2C)
    // ─────────────────────────────────────────────────────

    MotifDiscovered {
        timestamp_micros: u64,
        motif_id: u32,
        motif_name: String,
        cluster_size: u32,
        silhouette_score: f32,
        temporal_periodicity_hours: f32,
        confidence: f32,
    },

    PatternRecurrence {
        timestamp_micros: u64,
        motif_id: u32,
        previous_occurrence_timestamp_micros: u64,
        time_since_last_micros: u64,
        interval_stability: f32,
    },

    // ─────────────────────────────────────────────────────
    // GUI & Control Events (Track B)
    // ─────────────────────────────────────────────────────

    ControlAction {
        timestamp_micros: u64,
        action: String,
        parameter_name: String,
        old_value: String,
        new_value: String,
    },

    ParametersLoaded {
        timestamp_micros: u64,
        source: String,
        device_idx: u32,
        gain_multiplier: f32,
        frequency_hz: f32,
        camera_resolution: String,
    },

    // ─────────────────────────────────────────────────────
    // Forensic Checkpoint (Track E)
    // ─────────────────────────────────────────────────────

    SessionStart {
        timestamp_micros: u64,
        app_version: String,
        total_events_prior: u64,
    },

    SessionEnd {
        timestamp_micros: u64,
        events_logged_this_session: u64,
        total_events: u64,
    },

    // ─────────────────────────────────────────────────────
    // Error & Recovery (Track E)
    // ─────────────────────────────────────────────────────

    EventValidationError {
        timestamp_micros: u64,
        original_event: String,
        error_reason: String,
    },

    LogFileError {
        timestamp_micros: u64,
        error_type: String,
        error_message: String,
        recovery_action: String,
    },

    DataIntegrityCheck {
        timestamp_micros: u64,
        events_checked: u32,
        valid_events: u32,
        invalid_events: u32,
        checksums_match: bool,
    },

    // Legacy mapping (to be removed in Phase 2)
    LegacyBispectrum {
        timestamp_micros: u64,
        f1_hz: f32,
        f2_hz: f32,
        product_hz: f32,
        magnitude: f32,
        coherence_frames: u32,
        confidence: f32,
    }
}

pub struct EventValidator;

impl EventValidator {
    pub fn validate(event: &ForensicEvent) -> Result<(), String> {
        match event {
            ForensicEvent::AudioFrameProcessed {
                timestamp_micros,
                rms_db,
                peak_db,
                ..
            } => {
                if !rms_db.is_finite() {
                    return Err("rms_db NaN or Inf".to_string());
                }
                if *rms_db < -120.0 || *rms_db > 0.0 {
                    return Err(format!("rms_db out of range: {}", rms_db));
                }
                if !peak_db.is_finite() {
                    return Err("peak_db NaN or Inf".to_string());
                }
                if *timestamp_micros == 0 {
                    return Err("timestamp_micros is zero".to_string());
                }
                Ok(())
            }

            ForensicEvent::RFDetection {
                frequency_hz,
                power_dbfs,
                confidence,
                ..
            } => {
                if !frequency_hz.is_finite() || *frequency_hz <= 0.0 {
                    return Err(format!("Invalid frequency: {}", frequency_hz));
                }
                if !power_dbfs.is_finite() {
                    return Err("power_dbfs NaN or Inf".to_string());
                }
                if *confidence < 0.0 || *confidence > 1.0 {
                    return Err(format!("confidence out of [0,1]: {}", confidence));
                }
                Ok(())
            }

            ForensicEvent::MambaInference {
                reconstruction_mse,
                ..
            } => {
                if !reconstruction_mse.is_finite() || *reconstruction_mse < 0.0 {
                    return Err(format!("Invalid reconstruction_mse: {}", reconstruction_mse));
                }
                Ok(())
            }

            ForensicEvent::AnomalyGateDecision {
                anomaly_score,
                confidence,
                ..
            } => {
                if !anomaly_score.is_finite() || *anomaly_score < 0.0 {
                    return Err(format!("Invalid anomaly_score: {}", anomaly_score));
                }
                if *confidence < 0.0 || *confidence > 1.0 {
                    return Err(format!("confidence out of [0,1]: {}", confidence));
                }
                Ok(())
            }

            ForensicEvent::LegacyBispectrum {
                f1_hz,
                f2_hz,
                product_hz,
                magnitude,
                confidence,
                ..
            } => {
                if !f1_hz.is_finite() || !f2_hz.is_finite() || !product_hz.is_finite() {
                    return Err("frequency NaN or Inf".to_string());
                }
                if !magnitude.is_finite() {
                    return Err("magnitude NaN or Inf".to_string());
                }
                if *confidence < 0.0 || *confidence > 1.0 {
                    return Err(format!("confidence out of [0,1]: {}", confidence));
                }
                Ok(())
            }

            _ => Ok(()),
        }
    }
}

pub enum LogError {
    DiskFull,
    PermissionDenied,
    IOError(String),
    ValidationError(String),
}

impl std::fmt::Debug for LogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DiskFull => write!(f, "DiskFull"),
            Self::PermissionDenied => write!(f, "PermissionDenied"),
            Self::IOError(s) => write!(f, "IOError({})", s),
            Self::ValidationError(s) => write!(f, "ValidationError({})", s),
        }
    }
}

pub struct LogRecoveryStrategy;

impl LogRecoveryStrategy {
    pub async fn handle_error(
        error: LogError,
        event: &ForensicEvent,
        original_path: &str,
    ) -> Result<tokio::fs::File, LogError> {
        match error {
            LogError::DiskFull => {
                eprintln!("[Forensic] Disk full! Rotating log file...");
                let rotated_path = format!("{}.{}.backup", original_path, chrono::Utc::now().format("%Y-%m-%d-%H%M%S"));
                // Try renaming
                let _ = tokio::fs::rename(original_path, &rotated_path).await;
                // Create fresh file
                let file = tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(original_path)
                    .await
                    .map_err(|e| LogError::IOError(e.to_string()))?;
                Ok(file)
            }
            LogError::PermissionDenied => {
                eprintln!("[Forensic] Permission denied on log file.");
                let temp_path = "temp_events.jsonl"; // Simplified fallback for MVP
                let mut file = tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(temp_path)
                    .await
                    .map_err(|e| LogError::IOError(e.to_string()))?;
                let line = serde_json::to_string(event).unwrap() + "\n";
                file.write_all(line.as_bytes()).await.map_err(|e| LogError::IOError(e.to_string()))?;
                Ok(file)
            }
            LogError::IOError(msg) => {
                eprintln!("[Forensic] IO error: {}", msg);
                Err(LogError::IOError(msg))
            }
            LogError::ValidationError(_) => {
                Err(error)
            }
        }
    }
}

pub struct ForensicLogger {
    sender: mpsc::UnboundedSender<ForensicEvent>,
    log_path: PathBuf,
}

impl ForensicLogger {
    pub async fn new(session_id: &str) -> Result<Self, LogError> {
        let dir = PathBuf::from("forensic_log");
        tokio::fs::create_dir_all(&dir).await.map_err(|e| LogError::IOError(e.to_string()))?;

        let filename = format!("{}.jsonl", session_id.replace(':', "-"));
        let log_path = dir.join(&filename);

        if !log_path.exists() {
            tokio::fs::File::create(&log_path).await.map_err(|e| LogError::IOError(e.to_string()))?;
        }

        let (sender, mut receiver) = mpsc::unbounded_channel();
        let log_path_clone = log_path.clone();

        tokio::spawn(async move {
            let mut file = tokio::fs::OpenOptions::new()
                .append(true)
                .open(&log_path_clone)
                .await
                .expect("Failed to open log file");

            let mut event_count: u64 = 0;

            loop {
                match receiver.recv().await {
                    Some(event) => {
                        event_count += 1;
                        // Validate
                        if let Err(e) = EventValidator::validate(&event) {
                            eprintln!("[Forensic] Validation error: {}", e);
                            let validation_event = ForensicEvent::EventValidationError {
                                timestamp_micros: get_current_micros(),
                                original_event: serde_json::to_string(&event).unwrap_or_default(),
                                error_reason: e,
                            };
                            let line = serde_json::to_string(&validation_event).unwrap() + "\n";
                            let _ = file.write_all(line.as_bytes()).await;
                            continue;
                        }

                        let line = serde_json::to_string(&event).unwrap() + "\n";
                        match file.write_all(line.as_bytes()).await {
                            Ok(_) => {
                                let _ = file.sync_all().await;
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::OutOfMemory => { // Approximation of ENOSPC
                                match LogRecoveryStrategy::handle_error(LogError::DiskFull, &event, log_path_clone.to_str().unwrap()).await {
                                    Ok(new_file) => {
                                        file = new_file;
                                        let _ = file.write_all(line.as_bytes()).await;
                                    }
                                    Err(recovery_err) => {
                                        eprintln!("[Forensic] Recovery failed: {:?}", recovery_err);
                                    }
                                }
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                                match LogRecoveryStrategy::handle_error(LogError::PermissionDenied, &event, log_path_clone.to_str().unwrap()).await {
                                    Ok(new_file) => { file = new_file; }
                                    Err(err) => eprintln!("[Forensic] Permission recovery failed: {:?}", err),
                                }
                            }
                            Err(e) => {
                                eprintln!("[Forensic] Write error: {}", e);
                            }
                        }
                    }
                    None => break,
                }
            }
        });

        Ok(Self {
            sender,
            log_path,
        })
    }

    pub fn log(&self, event: ForensicEvent) -> Result<(), LogError> {
        self.sender.send(event).map_err(|_| LogError::IOError("Channel closed".to_string()))
    }

    pub fn log_detection(&self, event: &DetectionEvent) -> Result<(), LogError> {
        // Map old DetectionEvent to ForensicEvent V2
        let confidence = (event.magnitude * event.coherence_frames as f32).min(1.0);
        let fe = ForensicEvent::LegacyBispectrum {
            timestamp_micros: get_current_micros(),
            f1_hz: event.f1_hz,
            f2_hz: event.f2_hz,
            product_hz: event.product_hz,
            magnitude: event.magnitude,
            coherence_frames: event.coherence_frames,
            confidence,
        };
        self.log(fe)
    }

    pub fn compute_confidence(&self, anomaly_db: f32) -> f32 {
        ((anomaly_db - 5.0).max(0.0) / 20.0).min(1.0)
    }

    pub fn classify_attack_vector(
        &self,
        audio_dc: Option<f32>,
        sdr_dc: Option<f32>,
        rf_confidence: f32,
        timestamp_sync_ms: i64,
    ) -> String {
        let has_audio_dc = audio_dc.map_or(false, |v| v > 0.05);
        let has_sdr_dc = sdr_dc.map_or(false, |v| v > 1.5);
        let high_rf = rf_confidence > 0.85;
        let synchronized = timestamp_sync_ms < 5;

        match (has_audio_dc, has_sdr_dc, high_rf, synchronized) {
            (true, true, true, true) => "RF_DC_SIMULTANEOUS",
            (true, false, _, _) => "DC_BIAS_ONLY",
            (false, true, true, _) => "RF_ONLY",
            (true, true, _, _) => "RF_DC_SEQUENTIAL",
            _ => "MIXED_VECTOR",
        }
        .to_string()
    }

    pub async fn shutdown(self) -> Result<(), LogError> {
        let session_end = ForensicEvent::SessionEnd {
            timestamp_micros: get_current_micros(),
            events_logged_this_session: 0, // Simplified for now since counting requires shared state
            total_events: 0, // Placeholder
        };

        let _ = self.sender.send(session_end);
        drop(self.sender); // Drop sender to close channel and stop task

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        Ok(())
    }

    pub fn log_path(&self) -> &PathBuf {
        &self.log_path
    }

    pub fn export_evidence_report(
        &self,
        output_path: &str,
        case_number: &str,
        operator_name: &str,
        location: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> anyhow::Result<()> {
        use std::io::BufReader;

        println!("[Forensic] Generating evidence report for case: {}", case_number);
        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);

        let mut events: Vec<serde_json::Value> = Vec::new();

        for line in std::io::BufRead::lines(reader) {
            let line = line?;
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                // In new format, we check timestamp_micros roughly to mimic date ranges if needed
                events.push(json);
            }
        }

        let mut html = String::new();
        html.push_str(&format!(r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Forensic Evidence Report - {}</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; }}
        h1 {{ color: #333; border-bottom: 2px solid #333; padding-bottom: 10px; }}
        h2 {{ color: #555; margin-top: 30px; }}
        table {{ border-collapse: collapse; width: 100%; margin: 20px 0; }}
        th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
        th {{ background-color: #4CAF50; color: white; }}
        tr:nth-child(even) {{ background-color: #f2f2f2; }}
        .warning {{ background-color: #ffeb3b; padding: 10px; border-left: 4px solid #f44336; }}
        .evidence {{ background-color: #e3f2fd; padding: 15px; margin: 10px 0; }}
        .timestamp {{ font-family: monospace; color: #666; }}
        .footer {{ margin-top: 50px; border-top: 1px solid #ccc; padding-top: 20px; font-size: 0.9em; color: #666; }}
    </style>
</head>
<body>
    <h1>Forensic Evidence Report</h1>
    <div class="evidence">
        <h2>Case Information</h2>
        <table>
            <tr><th>Case Number</th><td>{}</td></tr>
            <tr><th>Report Generated</th><td class="timestamp">{}</td></tr>
            <tr><th>Operator</th><td>{}</td></tr>
            <tr><th>Location</th><td>{}</td></tr>
            <tr><th>Total Events</th><td><strong>{}</strong></td></tr>
        </table>
    </div>

    <div class="warning">
        <strong>⚠️ Chain of Custody Notice:</strong> This report contains forensic evidence.
        Do not alter, modify, or delete. Maintain proper chain of custody documentation.
        Original log file: <code>{}</code>
    </div>
"#,
            case_number,
            case_number,
            chrono::Utc::now().to_rfc3339(),
            operator_name,
            location,
            events.len(),
            self.log_path.display()
        ));

        html.push_str(
            r#"
    <h2>Detection Events Timeline</h2>
    <table>
        <tr>
            <th>Timestamp (Micros)</th>
            <th>Event Type</th>
            <th>Frequency (Hz)</th>
            <th>Confidence</th>
        </tr>
"#,
        );

        for event in &events {
            let event_type = event.get("event_type").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let timestamp = event.get("timestamp_micros").and_then(|v| v.as_u64()).unwrap_or(0);
            let freq = event.get("f1_hz").or_else(|| event.get("frequency_hz")).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let confidence = event.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);

            html.push_str(&format!(
                r#"
        <tr>
            <td class="timestamp">{}</td>
            <td>{}</td>
            <td>{:.2}</td>
            <td>{:.3}</td>
        </tr>
"#,
                timestamp, event_type, freq, confidence
            ));
        }

        html.push_str(&format!(r#"
    </table>
    <div class="footer">
        <p>Generated: {} | Case: {} | Events: {}</p>
    </div>
</body>
</html>"#,
            chrono::Utc::now().to_rfc3339(),
            case_number,
            events.len()
        ));

        std::fs::write(output_path, html)?;
        println!("[Forensic] Evidence report exported: {} ({} events)", output_path, events.len());

        let csv_path = output_path.replace(".html", ".csv");
        let mut csv_writer = csv::Writer::from_path(&csv_path)?;
        csv_writer.write_record(&["timestamp_micros", "event_type", "frequency_hz", "confidence"])?;
        for event in events {
            if let Some(event_type) = event.get("event_type").and_then(|v| v.as_str()) {
                let ts = event.get("timestamp_micros").and_then(|v| v.as_u64()).unwrap_or(0);
                let freq = event.get("f1_hz").or_else(|| event.get("frequency_hz")).and_then(|v| v.as_f64()).unwrap_or(0.0);
                let conf = event.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
                csv_writer.write_record(&[
                    ts.to_string(),
                    event_type.to_string(),
                    format!("{:.2}", freq),
                    format!("{:.3}", conf),
                ])?;
            }
        }
        csv_writer.flush()?;
        Ok(())
    }
}

pub async fn verify_log_integrity(log_file: &str) -> Result<(), String> {
    let file = tokio::fs::read_to_string(log_file).await.map_err(|e| e.to_string())?;

    let mut valid_count = 0;
    let mut invalid_count = 0;
    let mut last_timestamp = 0u64;
    let mut out_of_order_count = 0;

    for (line_num, line) in file.lines().enumerate() {
        if line.is_empty() {
            continue;
        }

        match serde_json::from_str::<ForensicEvent>(line) {
            Ok(event) => {
                let ts = match &event {
                    ForensicEvent::SessionStart { timestamp_micros, .. } => *timestamp_micros,
                    ForensicEvent::AudioFrameProcessed { timestamp_micros, .. } => *timestamp_micros,
                    ForensicEvent::RFDetection { timestamp_micros, .. } => *timestamp_micros,
                    ForensicEvent::MambaInference { timestamp_micros, .. } => *timestamp_micros,
                    ForensicEvent::LegacyBispectrum { timestamp_micros, .. } => *timestamp_micros,
                    ForensicEvent::SessionEnd { timestamp_micros, .. } => *timestamp_micros,
                    _ => 0,
                };

                if ts > 0 && ts < last_timestamp {
                    out_of_order_count += 1;
                    eprintln!("[Forensic] Event {} out of order: {} < {}", line_num, ts, last_timestamp);
                }
                last_timestamp = ts.max(last_timestamp);

                if let Err(e) = EventValidator::validate(&event) {
                    invalid_count += 1;
                    eprintln!("[Forensic] Event {} invalid: {}", line_num, e);
                } else {
                    valid_count += 1;
                }
            }
            Err(e) => {
                invalid_count += 1;
                eprintln!("[Forensic] Event {} parse error: {}", line_num, e);
            }
        }
    }

    eprintln!("[Forensic] Integrity check: {} valid, {} invalid, {} out of order",
              valid_count, invalid_count, out_of_order_count);

    if invalid_count == 0 && out_of_order_count == 0 {
        Ok(())
    } else {
        Err(format!("{} errors found", invalid_count + out_of_order_count))
    }
}
INNER_EOF
mv src/forensic.rs.new src/forensic.rs
cargo check
