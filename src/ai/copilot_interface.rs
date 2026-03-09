use crate::knowledge_graph::KnowledgeGraphClient;
use std::sync::Arc;

/// Represents a message in the multi-turn CopilotKit conversation
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String, // "user", "assistant", "system"
    pub content: String,
    pub citations: Vec<String>, // Evidence IDs
}

/// The Copilot Interface mimics CopilotKit's conversational UI backend.
/// It uses local inference (e.g., Lm Studio / Phi4) via OpenAI-compatible endpoints.
pub struct CopilotInterface {
    pub graph: Arc<KnowledgeGraphClient>,
    pub endpoint_url: String, // E.g., "http://localhost:1234/v1/chat/completions"
    pub conversation_history: Vec<ChatMessage>,
}

impl CopilotInterface {
    pub fn new(graph: Arc<KnowledgeGraphClient>, endpoint_url: String) -> Self {
        Self {
            graph,
            endpoint_url,
            conversation_history: Vec::new(),
        }
    }

    /// Add a system prompt specifying Copilot's role as a Forensic Analyst AI.
    pub fn initialize(&mut self) {
        self.conversation_history.push(ChatMessage {
            role: "system".to_string(),
            content: "You are a forensic analyst AI assisting with tracking harassment motifs. \
                      You have access to a knowledge graph of events, patterns, spatial locations, \
                      and timestamps. Use the tools provided to query the graph and provide structured \
                      answers with evidence links.".to_string(),
            citations: Vec::new(),
        });
    }

    /// Processes a natural language query and interacts with the ReasoningEngine
    /// to retrieve data from the graph, build an evidence chain, and generate a response.
    pub async fn send_query(&mut self, query: &str) -> String {
        // Record user message
        self.conversation_history.push(ChatMessage {
            role: "user".to_string(),
            content: query.to_string(),
            citations: Vec::new(),
        });

        // The ReasoningEngine handles the multi-step retrieval (RAG + Tools)
        let engine = crate::ai::reasoning_engine::ReasoningEngine::new(self.graph.clone());
        let (response, chain) = engine.process(query).await;

        // Map the reasoning chain's citations into the message
        let citations: Vec<String> = chain
            .source_event_ids
            .iter()
            .map(|id| format!("Event {}", id))
            .collect();

        // Record the assistant's reply
        self.conversation_history.push(ChatMessage {
            role: "assistant".to_string(),
            content: response.clone(),
            citations,
        });

        response
    }

    /// Returns the active conversation history
    pub fn get_history(&self) -> Vec<ChatMessage> {
        self.conversation_history.clone()
    }
}
