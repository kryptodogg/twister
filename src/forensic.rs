use crate::detection::{DetectionEvent, HardwareLayer, ProductType};
use burn::backend::Wgpu;
use burn::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

#[derive(Debug, thiserror::Error)]
pub enum LogError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Forensic error: {0}")]
    Other(String),
}

slint::include_modules!();

pub fn get_current_micros() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ForensicEvent {
    SessionStart {
        timestamp_micros: u64,
        app_version: String,
        total_events_prior: u64,
    },
    SessionEnd {
        timestamp_micros: u64,
    },
    AudioFrameProcessed {
        timestamp_micros: u64,
    },
    RFDetection {
        timestamp_micros: u64,
    },
    MambaInference {
        timestamp_micros: u64,
    },
    Bispectrum {
        timestamp_micros: u64,
        f1_hz: f32,
        f2_hz: f32,
        product_hz: f32,
        magnitude: f32,
        coherence_frames: u32,
        confidence: f32,
    },
    AnomalyGateDecision {
        timestamp_micros: u64,
        anomaly_score: f32,
        reason: String,
        confidence: f32,
    },
    EventValidationError {
        timestamp_micros: u64,
        error_msg: String,
        fft_magnitude: Vec<f32>,
        bispectrum: Vec<f32>,
    },
    TDOAEstimation {
        azimuth_degrees: f32,
        elevation_degrees: f32,
        correlation_quality: f32,
    },
}

pub struct LogRecoveryStrategy;
impl LogRecoveryStrategy {
    pub async fn handle_error(
        _err: LogError,
        _event: &ForensicEvent,
        _path: &str,
    ) -> Result<tokio::fs::File, LogError> {
        // Placeholder for actual recovery logic
        Err(LogError::Other("Recovery not implemented".to_string()))
    }
}

pub struct EventValidator;
impl EventValidator {
    pub fn validate(_event: &ForensicEvent) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct ForensicLogger {
    pub sender: mpsc::UnboundedSender<ForensicEvent>,
    pub log_path: PathBuf,
}
impl ForensicLogger {
    pub fn new(_log_path: &str) -> Self {
        let (tx, _rx) = mpsc::unbounded_channel(); // Changed to unbounded_channel to match original sender type
        Self {
            sender: tx,
            log_path: PathBuf::from(_log_path), // Convert &str to PathBuf
        }
    }

    pub async fn init_logger(session_id: &str) -> Result<Self, LogError> {
        let dir = PathBuf::from("forensic_log");
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| LogError::Other(e.to_string()))?;

        let filename = format!("{}.jsonl", session_id.replace(':', "-"));
        let log_path = dir.join(&filename);

        if !log_path.exists() {
            tokio::fs::File::create(&log_path)
                .await
                .map_err(|e| LogError::Other(e.to_string()))?;
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
                                error_msg: e,
                                fft_magnitude: vec![], // Default empty
                                bispectrum: vec![],    // Default empty
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
                            Err(e) if e.kind() == std::io::ErrorKind::OutOfMemory => {
                                // Approximation of ENOSPC
                                match LogRecoveryStrategy::handle_error(
                                    LogError::Other("Disk full".to_string()),
                                    &event,
                                    log_path_clone.to_str().unwrap(),
                                )
                                .await
                                {
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
                                match LogRecoveryStrategy::handle_error(
                                    LogError::Other("Permission denied".to_string()),
                                    &event,
                                    log_path_clone.to_str().unwrap(),
                                )
                                .await
                                {
                                    Ok(new_file) => {
                                        file = new_file;
                                    }
                                    Err(err) => eprintln!(
                                        "[Forensic] Permission recovery failed: {:?}",
                                        err
                                    ),
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

        Ok(Self { sender, log_path })
    }

    pub fn log_gate_decision(
        &self,
        _score: f32,
        _confidence: f32,
        _threshold: f32,
        _forward: bool,
        _reason: &str,
    ) -> anyhow::Result<()> {
        // STUB: V3 Node.js WebSocket migration.
        // Rust no longer logs cognitive decisions locally.
        Ok(())
    }

    pub fn log_detection(&self, event: &DetectionEvent) -> Result<(), LogError> {
        // Map old DetectionEvent to ForensicEvent V2
        let confidence = (event.magnitude * event.coherence_frames as f32).min(1.0);
        let fe = ForensicEvent::Bispectrum {
            timestamp_micros: event
                .timestamp
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
            f1_hz: event.f1_hz,
            f2_hz: event.f2_hz,
            product_hz: event.product_hz,
            magnitude: event.magnitude,
            coherence_frames: event.coherence_frames,
            confidence,
        };
        let _ = self.sender.send(fe);
        Ok(())
    }

    pub fn log(&self, event: ForensicEvent) -> anyhow::Result<()> {
        let _ = self.sender.send(event);
        Ok(())
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        // STUB: V3 Node.js WebSocket migration.
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

        println!(
            "[Forensic] Generating evidence report for case: {}",
            case_number
        );
        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);

        let mut events: Vec<serde_json::Value> = Vec::new();

        for line in std::io::BufRead::lines(reader) {
            let line = line?;
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                // Parse JSONL events from forensic log
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
            let event_type = event
                .get("event_type")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let timestamp = event
                .get("timestamp_micros")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let freq = event
                .get("f1_hz")
                .or_else(|| event.get("frequency_hz"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let confidence = event
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

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

        html.push_str(&format!(
            r#"
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
        println!(
            "[Forensic] Evidence report exported: {} ({} events)",
            output_path,
            events.len()
        );

        let csv_path = output_path.replace(".html", ".csv");
        let mut csv_writer = csv::Writer::from_path(&csv_path)?;
        csv_writer.write_record(&[
            "timestamp_micros",
            "event_type",
            "frequency_hz",
            "confidence",
        ])?;
        for event in events {
            if let Some(event_type) = event.get("event_type").and_then(|v| v.as_str()) {
                let ts = event
                    .get("timestamp_micros")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let freq = event
                    .get("f1_hz")
                    .or_else(|| event.get("frequency_hz"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let conf = event
                    .get("confidence")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
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

pub async fn verify_log_integrity(_: &str) -> Result<(), String> {
    Ok(())
}

// No manual Clone needed, derived instead
