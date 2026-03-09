use std::time::Duration;

/// Represents a single reasoning step in the AI Agent's evidence chain.
#[derive(Debug, Clone)]
pub struct ReasoningStep {
    pub description: String,
    pub tool_called: Option<String>,
    pub result: String,
    pub confidence: f32,
    pub evidence: Vec<String>, // List of string descriptors or query segments
}

/// The final response format for a CopilotKit query, containing the actual answer
/// alongside the verifiable audit trail (evidence chain).
#[derive(Debug, Clone)]
pub struct EvidenceChain {
    pub query: String,
    pub steps: Vec<ReasoningStep>,
    pub source_event_ids: Vec<u64>, // IDs of events backing this answer
    pub confidence: f32,
    pub query_time_ms: u64,
    pub timestamp_iso: String,
}

impl EvidenceChain {
    pub fn new(query: String) -> Self {
        Self {
            query,
            steps: Vec::new(),
            source_event_ids: Vec::new(),
            confidence: 0.0,
            query_time_ms: 0,
            timestamp_iso: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn add_step(&mut self, step: ReasoningStep) {
        self.steps.push(step);
    }

    pub fn finalize(&mut self, latency: Duration, final_confidence: f32, source_ids: Vec<u64>) {
        self.query_time_ms = latency.as_millis() as u64;
        self.confidence = final_confidence;
        self.source_event_ids = source_ids;
    }
}
