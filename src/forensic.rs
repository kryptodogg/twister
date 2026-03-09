
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ForensicEvent {
    SessionStart { timestamp_micros: u64 },
    SessionEnd { timestamp_micros: u64 },
    AudioFrameProcessed { timestamp_micros: u64 },
    RFDetection { timestamp_micros: u64 },
    MambaInference { timestamp_micros: u64 },
    Bispectrum { timestamp_micros: u64 },
    AnomalyGateDecision { timestamp_micros: u64 },
}

pub struct ForensicLogger {}
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

    pub fn log_gate_decision(&mut self, score: f32, confidence: f32, threshold: f32, forward: bool, reason: &str) -> anyhow::Result<()> {
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

    pub fn log_detection(&self, event: &DetectionEvent) -> Result<(), LogError> {
        // Map old DetectionEvent to ForensicEvent V2
        let confidence = (event.magnitude * event.coherence_frames as f32).min(1.0);
        let fe = ForensicEvent::Bispectrum {
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

    pub async fn shutdown(&self) -> Result<(), LogError> {
        let session_end = ForensicEvent::SessionEnd {
            timestamp_micros: get_current_micros(),
            events_logged_this_session: 0, // Simplified for now since counting requires shared state
            total_events: 0, // Placeholder
        };

        let _ = self.sender.send(session_end);
        // Note: we can't drop self.sender here because we only have a reference.
        // The channel will close when all clones of ForensicLogger are dropped.

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

pub async fn verify_log_integrity(_: &str) -> Result<(), String> { Ok(()) }

impl Clone for ForensicLogger {
    fn clone(&self) -> Self { Self {} }
}
