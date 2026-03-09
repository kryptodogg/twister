import re

with open('src/forensic.rs', 'r') as f:
    content = f.read()

content = content.replace("ForensicEventType::AnomalyGateDecision", "ForensicEvent::AnomalyGateDecision")

content = content.replace(
"""            event_type: ForensicEvent::AnomalyGateDecision {
                confidence: event.confidence,
                is_anomaly: event.is_anomaly,
            },""",
"""            event_type: ForensicEvent::AnomalyGateDecision {
                timestamp_micros: 0,
                confidence: event.confidence,
                is_anomaly: event.is_anomaly,
            },""")

content = content.replace(
"""    AnomalyGateDecision {
        confidence: f32,
        is_anomaly: bool,
    },""",
"""    AnomalyGateDecision {
        timestamp_micros: u64,
        confidence: f32,
        is_anomaly: bool,
    },""")

content = content.replace("            session_id: self.session_id.clone(),\n", "")
content = content.replace("            equipment: self.equipment.clone(),\n", "")

content = content.replace(
"""pub struct ForensicLogger {
    sender: Sender<String>,
    pub log_path: PathBuf,
}""",
"""pub struct ForensicLogger {
    sender: Sender<String>,
    pub log_path: PathBuf,
    writer: std::io::BufWriter<std::fs::File>,
}""")

content = content.replace(
"""    pub fn new(log_dir: &Path) -> anyhow::Result<Self> {
        let (sender, receiver) = bounded(1000);
        let log_path = log_dir.join("forensic_log.jsonl");

        let mut logger = Self {
            sender,
            log_path,
        };""",
"""    pub fn new(log_dir: &Path) -> anyhow::Result<Self> {
        let (sender, receiver) = bounded(1000);
        let log_path = log_dir.join("forensic_log.jsonl");
        let file = std::fs::OpenOptions::new().create(true).append(true).open(&log_path)?;

        let mut logger = Self {
            sender,
            log_path,
            writer: std::io::BufWriter::new(file),
        };""")

content = content.replace(
"""                frequency_hz,
                confidence,
            } => {""",
"""                frequency_hz,
            } => {""")

with open('src/forensic.rs', 'w') as f:
    f.write(content)
