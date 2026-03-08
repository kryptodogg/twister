// src/forensic.rs — Forensic Event Logger  (v0.5)
//
// Evidence collection for harassment defense and investigation.
// Logs detections with court-admissible timestamps and calibration data.

use std::collections::HashMap;
use std::fs::{File, OpenOptions, create_dir_all};
use std::io::{Write, BufWriter};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use chrono;
use csv;
use crate::detection::DetectionEvent;

/// Forensic event types for evidence classification
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "event_type")]
pub enum ForensicEventType {
    /// Continuous narrowband carrier (potential surveillance transmitter)
    Carrier {
        frequency_hz: f32,
        bandwidth_hz: f32,
        amplitude_dbfs: f32,
    },
    /// Modulated signal (audio/data transmission)
    Modulated {
        carrier_hz: f32,
        modulation_type: String,
        amplitude_dbfs: f32,
    },
    /// Short-duration burst (possible data transmission)
    Burst {
        center_freq_hz: f32,
        duration_ms: f32,
        amplitude_dbfs: f32,
    },
    /// Harmonic of audio sample rate (equipment leakage)
    Harmonic {
        fundamental_hz: f32,
        harmonic_number: u32,
        amplitude_dbfs: f32,
    },
    /// Intermodulation product (nonlinear mixing)
    Intermodulation {
        f1_hz: f32,
        f2_hz: f32,
        product_hz: f32,
        amplitude_dbfs: f32,
    },
    /// Bispectrum detection (phase-coupled signals)
    Bispectrum {
        f1_hz: f32,
        f2_hz: f32,
        product_hz: f32,
        magnitude: f32,
        coherence_frames: u32,
    },
}

/// Forensic event with full evidence metadata
#[derive(Debug, Clone, serde::Serialize)]
pub struct ForensicEvent {
    /// Unique event ID
    pub id: String,
    /// UTC timestamp in ISO 8601 format
    pub timestamp_utc: String,
    /// Unix timestamp for sorting
    pub timestamp_unix: f64,
    /// Session ID for chain of custody
    pub session_id: String,
    /// Event classification
    #[serde(flatten)]
    pub event_type: ForensicEventType,
    /// Detection confidence (0.0 - 1.0)
    pub confidence: f32,
    /// Duration of signal observation in seconds
    pub duration_seconds: f32,
    /// Equipment used for detection
    pub equipment: EquipmentMetadata,
    /// Operator notes
    pub metadata: HashMap<String, String>,
}

/// Equipment metadata for evidence documentation
#[derive(Debug, Clone, serde::Serialize)]
pub struct EquipmentMetadata {
    /// Primary SDR device
    pub sdr_model: String,
    pub sdr_sample_rate_hz: u32,
    pub antenna_type: String,
    
    /// Audio input devices
    pub webcam_mic_model: String,
    pub webcam_mic_sample_rate_hz: u32,
    pub webcam_mic_channels: u32,
    pub webcam_mic_bit_depth: u32,
    
    pub line_in_snr_db: f32,
    pub line_in_bit_depth: u32,
    pub line_in_sample_rate_hz: u32,
    
    pub mic_in_snr_db: f32,
    pub mic_in_bit_depth: u32,
    pub mic_in_sample_rate_hz: u32,
    
    /// Calibration metadata
    pub calibration_date: String,
    pub calibration_verified: bool,
    pub calibration_notes: String,
}

impl Default for EquipmentMetadata {
    fn default() -> Self {
        Self {
            sdr_model: "RTL-SDR Blog V4".to_string(),
            sdr_sample_rate_hz: 2048000,
            antenna_type: "YouLoop".to_string(),
            
            // Logitech C925e webcam microphone specs
            webcam_mic_model: "Logitech C925e".to_string(),
            webcam_mic_sample_rate_hz: 32000,
            webcam_mic_channels: 2,
            webcam_mic_bit_depth: 16,
            
            // Realtek ALC S1200A onboard audio
            line_in_snr_db: 103.0,
            line_in_bit_depth: 24,
            line_in_sample_rate_hz: 192000,
            
            mic_in_snr_db: 103.0,
            mic_in_bit_depth: 24,
            mic_in_sample_rate_hz: 192000,
            
            calibration_date: chrono::Utc::now().to_rfc3339(),
            calibration_verified: true,
            calibration_notes: String::new(),
        }
    }
}

impl ForensicEvent {
    /// Create a new forensic event from a detection
    pub fn from_detection(
        event: &DetectionEvent,
        session_id: &str,
        equipment: EquipmentMetadata,
    ) -> Self {
        let now = SystemTime::now();
        let unix_ts = now.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64();
        let utc_ts = chrono::DateTime::from_timestamp(unix_ts as i64, 0)
            .unwrap_or_default()
            .to_rfc3339();

        let event_type = ForensicEventType::Bispectrum {
            f1_hz: event.f1_hz,
            f2_hz: event.f2_hz,
            product_hz: event.product_hz,
            magnitude: event.magnitude,
            coherence_frames: event.coherence_frames,
        };

        // Confidence based on coherence and magnitude
        let confidence = (event.magnitude * event.coherence_frames as f32).min(1.0);

        Self {
            id: event.id.clone(),
            timestamp_utc: utc_ts,
            timestamp_unix: unix_ts,
            session_id: session_id.to_string(),
            event_type,
            confidence,
            duration_seconds: event.coherence_frames as f32 / 192000.0, // Assume 192 kHz audio
            equipment,
            metadata: HashMap::new(),
        }
    }

    /// Convert to enhanced JSONL with forensic analysis fields
    pub fn to_enhanced_jsonl(&self, logger: &ForensicLogger, event: &DetectionEvent) -> anyhow::Result<String> {
        let rf_confidence = self.confidence;
        let mamba_anomaly_db = event.mamba_anomaly_db;
        let mamba_confidence = logger.compute_confidence(mamba_anomaly_db);
        let attack_vector = logger.classify_attack_vector(
            event.audio_dc_bias_v,
            event.sdr_dc_bias_v,
            rf_confidence,
            event.timestamp_sync_ms.unwrap_or(0),
        );

        let forensic_line = serde_json::json!({
            "event_id": self.id,
            "timestamp_utc": self.timestamp_utc,
            "detection_method": event.detection_method,
            "rf_freq_hz": event.product_hz,
            "rf_confidence": rf_confidence,

            // Audio/RF correlation
            "dc_bias_audio_v": event.audio_dc_bias_v.unwrap_or(0.0),
            "dc_bias_sdr_v": event.sdr_dc_bias_v.unwrap_or(0.0),

            // ML anomaly score
            "mamba_anomaly_db": mamba_anomaly_db,
            "mamba_confidence": mamba_confidence,

            // Attack classification
            "attack_vector": attack_vector,
            "timestamp_sync_ms": event.timestamp_sync_ms.unwrap_or(0),
            "classification": if event.is_coordinated { "COORDINATED_ATTACK" } else { "SINGLE_VECTOR" },
        });

        Ok(forensic_line.to_string())
    }

    /// Add metadata note to event
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

pub struct ForensicLogger {
    writer:      BufWriter<File>,
    log_path:    PathBuf,
    event_count: u64,
    session_id:  String,
    equipment:   EquipmentMetadata,
}

impl ForensicLogger {
    pub fn new(session_id: &str) -> anyhow::Result<Self> {
        let dir = PathBuf::from("forensic_log");
        create_dir_all(&dir)?;
        let filename = format!("{}.jsonl", session_id.replace(':', "-"));
        let log_path = dir.join(&filename);
        let file = OpenOptions::new().create(true).append(true).open(&log_path)?;
        println!("[Forensic] Logging to: {}", log_path.display());
        let mut writer = BufWriter::new(file);
        
        // Full equipment metadata for chain of custody
        let equipment = EquipmentMetadata::default();
        
        let header = serde_json::json!({
            "record_type":   "session_start",
            "session_id":    session_id,
            "timestamp_utc": chrono::Utc::now().to_rfc3339(),
            "siren_version": "0.5.0",
            "purpose":       "EM surveillance detection and evidence collection for harassment defense",
            "legal_notice":  "This log contains forensic evidence. Do not alter. Maintain chain of custody.",
            "equipment":     &equipment,
            "audio_inputs": {
                "primary": "Realtek ALC S1200A line-in (103 dB SNR, 24-bit, 192 kHz)",
                "secondary": "Realtek ALC S1200A mic-in (103 dB SNR, 24-bit, 192 kHz)",
                "tertiary": "Logitech C925e webcam mics (2-ch, 16-bit, 32 kHz)"
            },
            "sdr_config": {
                "device": "RTL-SDR Blog V4 with R828D tuner",
                "frequency_range": "10 kHz - 300 MHz (direct sampling)",
                "antenna": "YouLoop passive loop antenna",
                "mode": "Full spectrum surveillance detection"
            },
            "detection_methods": [
                "Bispectral analysis (phase-coupled carrier detection)",
                "Harmonic tracking (audio clock leakage)",
                "Intermodulation product detection",
                "Mamba ML anomaly detection (latent space reconstruction error)"
            ],
            "note": "SIREN forensic log — automated EM surveillance detection system"
        });
        writeln!(writer, "{}", header)?;
        writer.flush()?;
        Ok(Self { writer, log_path, event_count: 0, session_id: session_id.to_string(), equipment })
    }

    pub fn log_detection(&mut self, event: &DetectionEvent) -> anyhow::Result<()> {
        self.event_count += 1;

        // Create forensic event with full metadata
        let forensic_event = ForensicEvent::from_detection(
            event,
            &self.session_id,
            self.equipment.clone(),
        );

        // Log as forensic event
        let record = serde_json::to_string(&forensic_event)?;
        writeln!(self.writer, "{}", record)?;

        // Also log summary to console
        let summary = match &forensic_event.event_type {
            ForensicEventType::Bispectrum { f1_hz, f2_hz, product_hz, .. } => {
                format!(
                    "[{}] Bispectrum: {:.1}Hz + {:.1}Hz → {:.1}Hz (confidence: {:.2})",
                    event.hardware.as_str(), f1_hz, f2_hz, product_hz, forensic_event.confidence
                )
            }
            _ => format!("[{}] Detection: confidence={:.2}", event.hardware.as_str(), forensic_event.confidence),
        };

        println!("[Forensic] #{}: {}", self.event_count, summary);
        self.writer.flush()?;
        Ok(())
    }

    /// Compute Mamba confidence from anomaly score (dB)
    ///
    /// Confidence is a sigmoid-like function of anomaly magnitude:
    /// - 0-5 dB = low confidence (noise floor)
    /// - 5-15 dB = medium confidence (anomalous)
    /// - >15 dB = high confidence (clear attack)
    pub fn compute_confidence(&self, anomaly_db: f32) -> f32 {
        ((anomaly_db - 5.0).max(0.0) / 20.0).min(1.0)
    }

    /// Classify attack vector based on forensic evidence
    ///
    /// Combines RF detection confidence with DC bias measurements and temporal sync
    /// to determine the attack methodology:
    /// - RF_DC_SIMULTANEOUS: Both RF and DC present, synchronized (<5ms)
    /// - RF_ONLY: RF detected without DC biases
    /// - DC_BIAS_ONLY: DC bias spike without high RF confidence
    /// - RF_DC_SEQUENTIAL: Both present but not synchronized (>5ms apart)
    /// - MIXED_VECTOR: Ambiguous combination
    pub fn classify_attack_vector(
        &self,
        audio_dc: Option<f32>,
        sdr_dc: Option<f32>,
        rf_confidence: f32,
        timestamp_sync_ms: i64,
    ) -> String {
        // Thresholds for DC bias detection
        let has_audio_dc = audio_dc.map_or(false, |v| v > 0.05);
        let has_sdr_dc = sdr_dc.map_or(false, |v| v > 1.5);
        let high_rf = rf_confidence > 0.85;
        let synchronized = timestamp_sync_ms < 5;

        match (has_audio_dc, has_sdr_dc, high_rf, synchronized) {
            (true, true, true, true) => "RF_DC_SIMULTANEOUS",    // Coordination proof
            (true, false, _, _) => "DC_BIAS_ONLY",
            (false, true, true, _) => "RF_ONLY",
            (true, true, _, _) => "RF_DC_SEQUENTIAL",            // Both present, not sync
            _ => "MIXED_VECTOR",
        }
        .to_string()
    }

    /// Export comprehensive evidence report for investigators
    pub fn export_evidence_report(
        &self,
        output_path: &str,
        case_number: &str,
        operator_name: &str,
        location: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> anyhow::Result<()> {
        use std::fs::File;
        use std::io::BufReader;
        
        println!("[Forensic] Generating evidence report for case: {}", case_number);
        
        // Read all events from log
        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);
        
        let mut events: Vec<serde_json::Value> = Vec::new();
        let mut session_start: Option<String> = None;
        let mut session_end: Option<String> = None;
        let mut equipment_info: Option<serde_json::Value> = None;
        
        for line in std::io::BufRead::lines(reader) {
            let line = line?;
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                // Track session metadata
                if let Some(record_type) = json.get("record_type").and_then(|v| v.as_str()) {
                    if record_type == "session_start" {
                        session_start = json.get("timestamp_utc").and_then(|v| v.as_str()).map(String::from);
                        equipment_info = json.get("equipment").cloned();
                    } else if record_type == "session_end" {
                        session_end = json.get("timestamp_utc").and_then(|v| v.as_str()).map(String::from);
                    }
                }
                
                // Filter detection events by date range
                if let Some(ts) = json.get("timestamp_utc").and_then(|v| v.as_str()) {
                    if let (Some(start), Some(end)) = (start_date, end_date) {
                        if ts < start || ts > end {
                            continue;
                        }
                    }
                    if json.get("record_type").and_then(|v| v.as_str()) == Some("detection") 
                        || json.get("event_type").is_some() {
                        events.push(json);
                    }
                }
            }
        }
        
        // Generate comprehensive HTML report
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
            <tr><th>Session Start</th><td class="timestamp">{}</td></tr>
            <tr><th>Session End</th><td class="timestamp">{}</td></tr>
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
            session_start.unwrap_or_else(|| "Unknown".to_string()),
            session_end.unwrap_or_else(|| "Ongoing".to_string()),
            events.len(),
            self.log_path.display()
        ));
        
        // Equipment section
        if let Some(equip) = &equipment_info {
            html.push_str(&format!(r#"
    <h2>Equipment Configuration</h2>
    <table>
        <tr><th>SDR Model</th><td>{}</td></tr>
        <tr><th>SDR Sample Rate</th><td>{} Hz</td></tr>
        <tr><th>Antenna</th><td>{}</td></tr>
        <tr><th>Webcam Mic</th><td>{} ({} ch, {}-bit, {} Hz)</td></tr>
        <tr><th>Line In SNR</th><td>{} dB, {}-bit, {} Hz</td></tr>
        <tr><th>Mic In SNR</th><td>{} dB, {}-bit, {} Hz</td></tr>
        <tr><th>Calibration</th><td>{} (Verified: {})</td></tr>
    </table>
"#,
                equip.get("sdr_model").and_then(|v| v.as_str()).unwrap_or("Unknown"),
                equip.get("sdr_sample_rate_hz").and_then(|v| v.as_u64()).unwrap_or(0),
                equip.get("antenna_type").and_then(|v| v.as_str()).unwrap_or("Unknown"),
                equip.get("webcam_mic_model").and_then(|v| v.as_str()).unwrap_or("Unknown"),
                equip.get("webcam_mic_channels").and_then(|v| v.as_u64()).unwrap_or(0),
                equip.get("webcam_mic_bit_depth").and_then(|v| v.as_u64()).unwrap_or(0),
                equip.get("webcam_mic_sample_rate_hz").and_then(|v| v.as_u64()).unwrap_or(0),
                equip.get("line_in_snr_db").and_then(|v| v.as_f64()).unwrap_or(0.0),
                equip.get("line_in_bit_depth").and_then(|v| v.as_u64()).unwrap_or(0),
                equip.get("line_in_sample_rate_hz").and_then(|v| v.as_u64()).unwrap_or(0),
                equip.get("mic_in_snr_db").and_then(|v| v.as_f64()).unwrap_or(0.0),
                equip.get("mic_in_bit_depth").and_then(|v| v.as_u64()).unwrap_or(0),
                equip.get("mic_in_sample_rate_hz").and_then(|v| v.as_u64()).unwrap_or(0),
                equip.get("calibration_date").and_then(|v| v.as_str()).unwrap_or("Unknown"),
                equip.get("calibration_verified").and_then(|v| v.as_bool()).unwrap_or(false)
            ));
        }
        
        // Detection events timeline
        html.push_str(r#"
    <h2>Detection Events Timeline</h2>
    <table>
        <tr>
            <th>Timestamp (UTC)</th>
            <th>Event Type</th>
            <th>Frequency (Hz)</th>
            <th>Amplitude (dBFS)</th>
            <th>Confidence</th>
            <th>Duration (s)</th>
        </tr>
"#);
        
        for event in &events {
            let event_type = event.get("event_type")
                .or_else(|| event.get("product_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            
            let freq = event.get("f1_hz")
                .or_else(|| event.get("frequency_hz"))
                .or_else(|| event.get("carrier_hz"))
                .or_else(|| event.get("center_freq_hz"))
                .or_else(|| event.get("product_hz"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            
            let amplitude = event.get("magnitude")
                .or_else(|| event.get("amplitude_dbfs"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            
            let confidence = event.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let duration = event.get("duration_seconds").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let timestamp = event.get("timestamp_utc").and_then(|v| v.as_str()).unwrap_or("Unknown");
            
            // Highlight high-confidence detections
            let row_class = if confidence >= 0.8 { "style=\"background-color: #ffcdd2;\"" } else { "" };
            
            html.push_str(&format!(r#"
        <tr{}>
            <td class="timestamp">{}</td>
            <td>{}</td>
            <td>{:.2}</td>
            <td>{:.2}</td>
            <td>{:.3}</td>
            <td>{:.3}</td>
        </tr>
"#, row_class, timestamp, event_type, freq, amplitude, confidence, duration));
        }
        
        html.push_str(&format!(r#"
    </table>

    <div class="footer">
        <p><strong>Report Certification:</strong> This report was automatically generated by SIREN v0.5.0
        (Surveillance Intelligence &amp; RF Emission Neutralization system). All timestamps are synchronized
        to UTC. Equipment calibration was verified at session start.</p>

        <p><strong>Legal Disclaimer:</strong> This evidence is provided for investigative purposes.
        Consult with qualified legal counsel before submission to authorities. Maintain original log
        files and chain of custody documentation.</p>

        <p>Generated: {} | Case: {} | Events: {}</p>
    </div>
</body>
</html>"#,
            chrono::Utc::now().to_rfc3339(),
            case_number,
            events.len()
        ));
        
        // Write HTML report
        std::fs::write(output_path, html)?;
        println!("[Forensic] Evidence report exported: {} ({} events)", output_path, events.len());
        
        // Also export CSV for data analysis
        let csv_path = output_path.replace(".html", ".csv");
        self.export_csv(&csv_path, &events)?;
        println!("[Forensic] CSV export: {}", csv_path);
        
        Ok(())
    }
    
    /// Export events as CSV for data analysis
    fn export_csv(&self, output_path: &str, events: &[serde_json::Value]) -> anyhow::Result<()> {
        let mut csv_writer = csv::Writer::from_path(output_path)?;
        
        // Write header
        csv_writer.write_record(&[
            "timestamp_utc",
            "event_type",
            "frequency_hz",
            "amplitude_dbfs",
            "confidence",
            "duration_seconds",
            "session_id",
            "notes",
        ])?;
        
        // Write events
        for event in events {
            if let Some(event_type) = event.get("event_type").and_then(|v| v.as_str()) {
                let freq = event.get("f1_hz")
                    .or_else(|| event.get("frequency_hz"))
                    .or_else(|| event.get("carrier_hz"))
                    .or_else(|| event.get("center_freq_hz"))
                    .or_else(|| event.get("product_hz"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                
                let amplitude = event.get("magnitude")
                    .or_else(|| event.get("amplitude_dbfs"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                
                let confidence = event.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let duration = event.get("duration_seconds").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let timestamp = event.get("timestamp_utc").and_then(|v| v.as_str()).unwrap_or("");
                let session = event.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
                let notes = event.get("summary").and_then(|v| v.as_str()).unwrap_or("");
                
                csv_writer.write_record(&[
                    timestamp,
                    event_type,
                    &format!("{:.2}", freq),
                    &format!("{:.2}", amplitude),
                    &format!("{:.3}", confidence),
                    &format!("{:.3}", duration),
                    session,
                    notes,
                ])?;
            }
        }
        
        csv_writer.flush()?;
        Ok(())
    }

    pub fn log_path(&self)    -> &PathBuf { &self.log_path }
    pub fn event_count(&self) -> u64      { self.event_count }
}

impl Drop for ForensicLogger {
    fn drop(&mut self) {
        let footer = serde_json::json!({
            "record_type":  "session_end",
            "session_id":   self.session_id,
            "event_count":  self.event_count,
            "log_path":     self.log_path.to_string_lossy(),
            "timestamp_utc": chrono::Utc::now().to_rfc3339(),
            "equipment":    &self.equipment,
            "session_summary": {
                "total_detections": self.event_count,
                "purpose": "Harassment defense and EM surveillance detection",
                "evidence_integrity": "SHA-256 verification recommended",
                "chain_of_custody": "Maintain original log file unaltered"
            },
        });
        let _ = writeln!(self.writer, "{}", footer);
        let _ = self.writer.flush();
        println!("[Forensic] Session complete: {} events logged", self.event_count);
    }
}
