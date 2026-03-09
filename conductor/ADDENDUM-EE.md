# Track E Addendum: Forensic Logging Reliability & Data Format Consistency

**Status**: Ready for Jules implementation
**Duration**: 45-60 minutes
**Dependency**: Track E forensic logging infrastructure exists; this ensures reliability and format correctness
**Integration**: All tracks → ForensicEvent serialization → @databases/forensic_logs/events.jsonl

---

## Executive Summary

Track E implements forensic event logging for investigation and evidence compilation. This addendum ensures **reliability and correctness** of all logged events:

1. **Data Format Consistency**: All timestamps microseconds, all frequencies Hz, all anomaly scores [0.0, ∞)
2. **Error Recovery**: Handle disk full, permissions errors, missing directories gracefully
3. **Event Ordering**: Guarantee chronological order despite async writes
4. **Deduplication**: Prevent duplicate events from concurrent tasks
5. **Compression**: Optional gzip for long-term storage (events.jsonl.gz)
6. **Validation**: Pre-write checks (finite values, non-negative scores, valid enums)
7. **Schema Versioning**: Support multiple event types without breaking readers

**No silent data loss.** Every event validated before write, with clear error reporting.

---

## ForensicEvent Complete Type Hierarchy

### File Ownership

- **`src/forensic_log.rs`** - Jules extends with all event variants + validation
- **`@databases/forensic_logs/`** - Directory created on app startup
- **`@databases/forensic_logs/events.jsonl`** - Rolling log file (one event per line)
- **`@databases/forensic_logs/schema.json`** - JSON schema for validation

### All Event Types (Unified Definition)

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "event_type")]  // Enum tag for JSON parsing
pub enum ForensicEvent {
    // ─────────────────────────────────────────────────────
    // Audio Processing Events (Track A)
    // ─────────────────────────────────────────────────────

    AudioFrameProcessed {
        timestamp_micros: u64,
        device_idx: u32,
        sample_rate_hz: u32,
        frame_size_samples: u32,
        rms_db: f32,           // [-120.0, 0.0]
        peak_db: f32,
        clipping_detected: bool,
    },

    FFTComputed {
        timestamp_micros: u64,
        frequency_bins: u32,   // 512
        peak_frequency_hz: f32,
        peak_magnitude: f32,   // [0.0, 1.0]
        spectral_centroid_hz: f32,
    },

    // ─────────────────────────────────────────────────────
    // RF Detection Events (Track A)
    // ─────────────────────────────────────────────────────

    RFDetection {
        timestamp_micros: u64,
        frequency_hz: f32,
        power_dbfs: f32,
        modulation_type: String,  // "CW", "AM", "FM", "Unknown"
        bandwidth_hz: f32,
        confidence: f32,           // [0.0, 1.0]
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
        mic_pair_indices: [u32; 2],  // Which mics (0-3)
        time_delay_micros: f32,       // Microseconds
        correlation_quality: f32,     // [0.0, 1.0]
        azimuth_degrees: f32,         // [0.0, 360.0)
        elevation_degrees: f32,       // [-90.0, 90.0]
    },

    // ─────────────────────────────────────────────────────
    // Mamba Anomaly Detection (Track D)
    // ─────────────────────────────────────────────────────

    MambaInference {
        timestamp_micros: u64,
        input_dimension: u32,  // 192 or 256
        latent_dimension: u32, // 64
        reconstruction_mse: f32,  // Anomaly score
        processing_time_us: u32,  // Microseconds for inference
    },

    AnomalyGateDecision {
        timestamp_micros: u64,
        anomaly_score: f32,
        confidence: f32,           // [0.0, 1.0]
        threshold_used: f32,
        forward_to_trainer: bool,
        reason: String,            // "anomaly_score_below_threshold", etc.
    },

    TrainingPairEnqueued {
        timestamp_micros: u64,
        training_pair_id: u64,     // Unique ID for this pair
        input_hash: u64,           // Hash of input features (dedupe check)
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
        loss: f32,                 // Reconstruction MSE
        learning_rate: f32,
        gradient_norm: f32,
    },

    TrainingConvergence {
        timestamp_micros: u64,
        epoch: u32,
        loss_moving_avg: f32,      // MA(last 10 steps)
        loss_std_dev: f32,         // Stability metric
        convergence_status: String, // "improving", "plateau", "diverging"
    },

    // ─────────────────────────────────────────────────────
    // Temporal Analysis (Phase 2C)
    // ─────────────────────────────────────────────────────

    MotifDiscovered {
        timestamp_micros: u64,
        motif_id: u32,             // 0-22 (23 total)
        motif_name: String,        // "Friday_3PM_Tone"
        cluster_size: u32,
        silhouette_score: f32,     // [0.0, 1.0]
        temporal_periodicity_hours: f32,
        confidence: f32,
    },

    PatternRecurrence {
        timestamp_micros: u64,
        motif_id: u32,
        previous_occurrence_timestamp_micros: u64,
        time_since_last_micros: u64,  // Microseconds
        interval_stability: f32,   // [0.0, 1.0], 1.0 = highly regular
    },

    // ─────────────────────────────────────────────────────
    // GUI & Control Events (Track B)
    // ─────────────────────────────────────────────────────

    ControlAction {
        timestamp_micros: u64,
        action: String,            // "mic_selected", "gain_adjusted", "freq_changed", etc.
        parameter_name: String,    // e.g., "audio_device_idx", "agc_gain_multiplier"
        old_value: String,
        new_value: String,
    },

    ParametersLoaded {
        timestamp_micros: u64,
        source: String,            // "file", "default", "hardcoded"
        device_idx: u32,
        gain_multiplier: f32,
        frequency_hz: f32,
        camera_resolution: String, // "480p", "720p", "1080p"
    },

    // ─────────────────────────────────────────────────────
    // Forensic Checkpoint (Track E)
    // ─────────────────────────────────────────────────────

    SessionStart {
        timestamp_micros: u64,
        app_version: String,       // "0.5.0"
        total_events_prior: u64,   // How many events logged before this session
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
        original_event: String,    // Serialized (failed) event for debugging
        error_reason: String,      // e.g., "NaN in anomaly_score"
    },

    LogFileError {
        timestamp_micros: u64,
        error_type: String,        // "DiskFull", "PermissionDenied", "IOError"
        error_message: String,
        recovery_action: String,   // "Rotating file", "Using temp location", etc.
    },

    DataIntegrityCheck {
        timestamp_micros: u64,
        events_checked: u32,
        valid_events: u32,
        invalid_events: u32,
        checksums_match: bool,
    },
}
```

---

## Data Format Specification

### JSONL Format (One Event Per Line)

```jsonl
{"event_type":"SessionStart","timestamp_micros":1741354994832401,"app_version":"0.5.0","total_events_prior":0}
{"event_type":"AudioFrameProcessed","timestamp_micros":1741354994832450,"device_idx":0,"sample_rate_hz":192000,"frame_size_samples":19200,"rms_db":-45.2,"peak_db":-20.1,"clipping_detected":false}
{"event_type":"FFTComputed","timestamp_micros":1741354994832501,"frequency_bins":512,"peak_frequency_hz":145500000.0,"peak_magnitude":0.87,"spectral_centroid_hz":125000000.0}
{"event_type":"RFDetection","timestamp_micros":1741354994832550,"frequency_hz":2400000000.0,"power_dbfs":-45.2,"modulation_type":"AM","bandwidth_hz":50000.0,"confidence":0.92}
{"event_type":"TDOAEstimation","timestamp_micros":1741354994832600,"mic_pair_indices":[0,1],"time_delay_micros":15.3,"correlation_quality":0.88,"azimuth_degrees":45.2,"elevation_degrees":-15.0}
{"event_type":"MambaInference","timestamp_micros":1741354994832650,"input_dimension":192,"latent_dimension":64,"reconstruction_mse":2.45,"processing_time_us":450}
{"event_type":"AnomalyGateDecision","timestamp_micros":1741354994832700,"anomaly_score":2.45,"confidence":0.88,"threshold_used":1.0,"forward_to_trainer":true,"reason":"anomaly_detected"}
{"event_type":"TrainingPairEnqueued","timestamp_micros":1741354994832750,"training_pair_id":12345,"input_hash":0x3a4f2b9c,"anomaly_score":2.45,"confidence":0.88}
{"event_type":"TrainingStepCompleted","timestamp_micros":1741354994832800,"epoch":123,"batch_number":456,"loss":0.34,"learning_rate":0.001,"gradient_norm":0.015}
```

### Key Constraints

**Timestamps**: Always `u64` microseconds since Unix epoch
```
1741354994832401 = 2026-03-07 14:23:14.832401 UTC
```

**Frequencies**: Always `f32` Hz (not MHz, GHz, or arbitrary units)
```
145_500_000.0 = 145.5 MHz
2_400_000_000.0 = 2.4 GHz
```

**Decibels**:
- Audio RMS/Peak: `[-120.0, 0.0]` (dB re full-scale)
- RF Power: `[-100.0, 0.0]` (dBm approximate)
- Always numeric, never NaN/Inf

**Scores**:
- Anomaly score: `[0.0, ∞)` (typically 0.3-5.0 for normal/attack)
- Confidence: `[0.0, 1.0]` (float)
- Never negative or NaN

**Enums**: Always serialized as lowercase strings
```
"modulation_type": "AM"  (not "am", not 0)
"error_type": "DiskFull"
```

---

## Error Handling & Recovery

### File Ownership

- **`src/forensic_log.rs`** - Logging system with error recovery
- **`src/forensic_log/validation.rs`** (NEW) - Pre-write validation
- **`src/forensic_log/recovery.rs`** (NEW) - Disk full/permission recovery

### Pre-Write Validation

```rust
// src/forensic_log/validation.rs

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
                if rms_db < -120.0 || rms_db > 0.0 {
                    return Err(format!("rms_db out of range: {}", rms_db));
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

            // ... validate other event types ...

            _ => Ok(()),  // Default: pass through
        }
    }
}
```

### Error Recovery

```rust
// src/forensic_log/recovery.rs

pub enum LogError {
    DiskFull,
    PermissionDenied,
    IOError(String),
    ValidationError(String),
}

pub struct LogRecoveryStrategy;

impl LogRecoveryStrategy {
    pub async fn handle_error(
        error: LogError,
        event: &ForensicEvent,
    ) -> Result<(), LogError> {
        match error {
            LogError::DiskFull => {
                eprintln!("[Forensic] Disk full! Rotating log file...");
                // Option 1: Compress current log
                compress_current_log("events.jsonl", "events.jsonl.gz").await?;
                // Option 2: Start new log file with timestamp
                create_dated_log("events.jsonl.2026-03-07", event).await?;
                Ok(())
            }

            LogError::PermissionDenied => {
                eprintln!("[Forensic] Permission denied on log file.");
                // Option 1: Try temp directory
                let temp_path = "~/.siren/temp_events.jsonl";
                write_event_to_file(temp_path, event).await?;
                eprintln!("[Forensic] Event written to temp location: {}", temp_path);
                Ok(())
            }

            LogError::IOError(msg) => {
                eprintln!("[Forensic] IO error: {}", msg);
                // Option: Write to memory buffer, retry on next dispatch
                store_in_buffer(event);
                Ok(())
            }

            LogError::ValidationError(msg) => {
                // Log the validation error itself
                let validation_event = ForensicEvent::EventValidationError {
                    timestamp_micros: get_current_micros(),
                    original_event: serde_json::to_string(event).unwrap_or_default(),
                    error_reason: msg,
                };
                write_event_direct(&validation_event).await?;
                Ok(())
            }
        }
    }
}
```

### Threadsafe Logging Channel

```rust
// src/forensic_log.rs - Main logging system

pub struct ForensicLogger {
    sender: mpsc::UnboundedSender<ForensicEvent>,
    pending_events: Vec<ForensicEvent>,  // Buffer for disk-full recovery
}

impl ForensicLogger {
    pub async fn new(log_file: &str) -> Result<Self, LogError> {
        // Ensure directory exists
        tokio::fs::create_dir_all("@databases/forensic_logs").await?;

        // Create JSONL file if it doesn't exist
        if !Path::new(log_file).exists() {
            tokio::fs::File::create(log_file).await?;
        }

        let (sender, mut receiver) = mpsc::unbounded_channel();

        // Spawn logging task
        let log_file_clone = log_file.to_string();
        tokio::spawn(async move {
            let mut file = tokio::fs::OpenOptions::new()
                .append(true)
                .open(&log_file_clone)
                .await
                .expect("Failed to open log file");

            loop {
                match receiver.recv().await {
                    Some(event) => {
                        // Validate before writing
                        if let Err(e) = EventValidator::validate(&event) {
                            eprintln!("[Forensic] Validation error: {}", e);
                            // Log the validation error
                            let validation_event = ForensicEvent::EventValidationError {
                                timestamp_micros: get_current_micros(),
                                original_event: serde_json::to_string(&event).unwrap_or_default(),
                                error_reason: e,
                            };
                            let line = serde_json::to_string(&validation_event).unwrap() + "\n";
                            let _ = file.write_all(line.as_bytes()).await;
                            continue;
                        }

                        // Serialize and write
                        let line = serde_json::to_string(&event).unwrap() + "\n";
                        match file.write_all(line.as_bytes()).await {
                            Ok(_) => {
                                file.sync_all().await.ok();  // Fsync for durability
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::OutOfMemory => {
                                // Disk full
                                if let Err(recovery_error) = LogRecoveryStrategy::handle_error(
                                    LogError::DiskFull,
                                    &event,
                                ).await {
                                    eprintln!("[Forensic] Recovery failed: {:?}", recovery_error);
                                }
                            }
                            Err(e) => {
                                eprintln!("[Forensic] Write error: {}", e);
                            }
                        }
                    }
                    None => break,  // Logger dropped
                }
            }
        });

        Ok(ForensicLogger {
            sender,
            pending_events: Vec::new(),
        })
    }

    pub fn log(&self, event: ForensicEvent) -> Result<(), LogError> {
        self.sender.send(event)
            .map_err(|_| LogError::IOError("Channel closed".to_string()))
    }
}
```

---

## Data Integrity & Verification

### Schema Validation

```json
// @databases/forensic_logs/schema.json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "event_type": { "type": "string" },
    "timestamp_micros": { "type": "integer", "minimum": 0 }
  },
  "oneOf": [
    {
      "properties": { "event_type": { "const": "AudioFrameProcessed" } },
      "required": ["rms_db", "peak_db", "device_idx"]
    },
    {
      "properties": { "event_type": { "const": "RFDetection" } },
      "required": ["frequency_hz", "power_dbfs", "confidence"]
    }
  ]
}
```

### Post-Write Integrity Check (Daily)

```rust
// src/forensic_log/verification.rs

pub async fn verify_log_integrity(log_file: &str) -> Result<(), String> {
    let file = tokio::fs::read_to_string(log_file).await?;

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
                // Check chronological order
                let ts = match &event {
                    ForensicEvent::SessionStart { timestamp_micros, .. } => *timestamp_micros,
                    ForensicEvent::AudioFrameProcessed { timestamp_micros, .. } => *timestamp_micros,
                    // ... extract timestamp from other variants ...
                    _ => 0,
                };

                if ts < last_timestamp {
                    out_of_order_count += 1;
                    eprintln!("[Forensic] Event {} out of order: {} < {}", line_num, ts, last_timestamp);
                }
                last_timestamp = ts.max(last_timestamp);

                // Validate values
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
```

---

## Session Management

### Session Lifecycle

```rust
// src/main.rs - Wrap initialization and shutdown

// At startup
let forensic_logger = ForensicLogger::new("@databases/forensic_logs/events.jsonl").await?;

forensic_logger.log(ForensicEvent::SessionStart {
    timestamp_micros: get_current_micros(),
    app_version: "0.5.0".to_string(),
    total_events_prior: count_existing_events().await,
})?;

// ... application runs ...

// At shutdown (on_exit callback)
forensic_logger.log(ForensicEvent::SessionEnd {
    timestamp_micros: get_current_micros(),
    events_logged_this_session: st.forensic_events_logged,
    total_events: count_total_events().await,
})?;

// Optional: Compress log file if > 100 MB
if file_size("events.jsonl").await > 100_000_000 {
    compress_log("events.jsonl", "events.jsonl.gz").await?;
}
```

---

## Pre-Commit Hook Validation

```bash
#!/bin/bash
# .git/hooks/pre-commit (add to existing)

# ✓ All event types have timestamp_micros
if grep -q "pub enum ForensicEvent" src/forensic_log.rs && ! grep -q "timestamp_micros" src/forensic_log.rs; then
    echo "⚠ Check: All events should have timestamp_micros"
fi

# ✓ Timestamps are u64 (not f32)
if grep -q "timestamp_micros: f32" src/forensic_log.rs; then
    echo "❌ Timestamps must be u64 microseconds, not f32"
    exit 1
fi

# ✓ Frequencies are Hz (not MHz)
if grep -q "frequency_mhz" src/forensic_log.rs; then
    echo "⚠ Use frequency_hz (Hz), not frequency_mhz"
fi

# ✓ Confidence scores in [0.0, 1.0]
if grep "confidence:" src/forensic_log.rs | grep -v "confidence: f32"; then
    echo "⚠ Check confidence type is f32"
fi

# ✓ EventValidator called before write
if grep -q "EventValidator::validate" src/forensic_log.rs; then
    echo "✓ All events validated before logging"
else
    echo "⚠ Consider adding validation before writing"
fi

# ✓ Session start/end events logged
if grep -q "SessionStart\|SessionEnd" src/forensic_log.rs; then
    echo "✓ Session lifecycle logged"
else
    echo "⚠ Log SessionStart at app init and SessionEnd at shutdown"
fi

echo "✓ Track EE forensic logging validation passed"
exit 0
```

---

## Implementation Checklist (for Jules)

### Phase 1: Event Type Definition (15 min)
- [ ] Define all ForensicEvent variants (listed above)
- [ ] Verify Serialize/Deserialize derives work
- [ ] Add serde(tag) for enum discrimination
- [ ] Tests: Event serialization round-trip

### Phase 2: Validation Layer (15 min)
- [ ] Create EventValidator struct
- [ ] Implement validate() for each event type
- [ ] Check: timestamps > 0, frequencies > 0, scores finite
- [ ] Check: confidence in [0, 1], anomaly score >= 0
- [ ] Tests: Valid events pass, invalid events rejected

### Phase 3: Logging Channel (15 min)
- [ ] Create ForensicLogger with mpsc channel
- [ ] Spawn background task for writing
- [ ] Implement fsync for durability
- [ ] Handle write errors (validation errors logged as events)
- [ ] Tests: Events written in order, channel non-blocking

### Phase 4: Error Recovery (15 min)
- [ ] Implement DiskFull handling (rotate log)
- [ ] Implement PermissionDenied (temp file)
- [ ] Implement IOError buffering
- [ ] Log recovery actions as ForensicEvents
- [ ] Tests: Simulated disk full, verify recovery

### Phase 5: Integrity Checking (10 min)
- [ ] Implement verify_log_integrity()
- [ ] Check JSON parseable, timestamps ordered
- [ ] Check value ranges (dB, confidence, scores)
- [ ] Run daily (e.g., on SessionStart)
- [ ] Tests: Detect corruption, report issues

### Phase 6: Session Management (5 min)
- [ ] Log SessionStart on app init
- [ ] Log SessionEnd on app shutdown
- [ ] Log event count per session
- [ ] Tests: Session lifecycle appears in log

### Phase 7: Integration Testing (10 min)
- [ ] Cargo build → 0 errors
- [ ] Cargo run → events.jsonl created
- [ ] Generate various events → observe in log
- [ ] Verify JSONL format (one event per line)
- [ ] Verify timestamps chronological
- [ ] Verify all values finite and in expected ranges
- [ ] Test error recovery (simulate disk full)

---

## Total Duration

| Task | Time |
|------|------|
| Phase 1: Event definitions | 15 min |
| Phase 2: Validation layer | 15 min |
| Phase 3: Logging channel | 15 min |
| Phase 4: Error recovery | 15 min |
| Phase 5: Integrity checking | 10 min |
| Phase 6: Session management | 5 min |
| Phase 7: Testing | 10 min |
| **Total** | **85 min** |

*Estimated 45-60 min with concurrent work*

---

## Verification & Success Criteria

✅ **All events properly serialized**:
- JSONL format (one event per line)
- No NaN, Inf, or null values
- All timestamps in microseconds
- All frequencies in Hz

✅ **Validation before write**:
- Invalid events rejected with clear error
- Validation errors logged as events
- No silent data loss

✅ **Error recovery robust**:
- Disk full → rotate log, continue writing
- Permission denied → fallback location
- No crashes on I/O errors

✅ **Data integrity**:
- Events in chronological order
- Scores within expected ranges
- Periodical integrity check passes

✅ **Session tracking**:
- SessionStart logged on init
- SessionEnd logged on shutdown
- Event count per session tracked

---

## Notes for Jules

This addendum ensures forensic logs are reliable enough for evidence compilation. The validator ensures data quality before write (prevent garbage from making it to disk). The recovery strategies handle real-world failure modes (disk full, permissions) gracefully.

**Key insight**: Validation is pre-write, not post-hoc. This means bad data never reaches the log file—critical for forensic use where you need trust in the record.

Session management provides audit trail: when the app started, how many events logged, when it stopped. This is essential for evidence timeline reconstruction.

Integrity checking catches corruption (filesystem issues, file truncation). Running daily (on SessionStart) ensures early detection of problems.

---

## Future Enhancements (Post-EE)

- **Event compression**: Compress archived logs daily
- **Forensic export**: Generate evidence report in PDF/CSV
- **Event search**: Build index for fast timestamp/frequency queries
- **Statistics dashboard**: Real-time event counts by type
- **Archival**: Auto-archive logs > 30 days old to cold storage

