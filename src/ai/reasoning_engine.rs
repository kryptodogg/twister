use crate::ai::evidence_chain::{EvidenceChain, ReasoningStep};
use crate::ai::query_tools::{FindEventsByLocationTool, FindPatternForFrequencyTool, Tool};
use crate::knowledge_graph::KnowledgeGraphClient;
use std::sync::Arc;
use std::time::Instant;

/// The ReasoningEngine acts as the AI Agent orchestration layer.
/// It receives a natural language query, decides which tools to call,
/// constructs a proof chain, and returns a structured response with citations.
pub struct ReasoningEngine {
    graph: Arc<KnowledgeGraphClient>,
    tools: Vec<Box<dyn Tool>>,
}

impl ReasoningEngine {
    pub fn new(graph: Arc<KnowledgeGraphClient>) -> Self {
        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(FindEventsByLocationTool),
            Box::new(FindPatternForFrequencyTool),
        ];

        Self { graph, tools }
    }

    /// Processes a query using the ReAct (Reasoning and Acting) pattern.
    /// This is where the local LLM integration would happen.
    /// For the MVP, we simulate the LLM's thought process based on keyword matching.
    pub async fn process(&self, query: &str) -> (String, EvidenceChain) {
        let start = Instant::now();
        let mut chain = EvidenceChain::new(query.to_string());

        let q_lower = query.to_lowercase();
        let mut response = String::new();
        let mut final_confidence = 0.0;
        let mut source_ids = Vec::new();

        // LLM Think Step 1: Decompose query
        let step1 = ReasoningStep {
            description: "Decomposing query to identify key entities".to_string(),
            tool_called: None,
            result: format!("Extracted intent from: {}", query),
            confidence: 0.95,
            evidence: vec![],
        };
        chain.add_step(step1);

        // LLM Think Step 2: Select and Execute Tool
        if q_lower.contains("azimuth 45") {
            let tool = &self.tools[0];
            let step2 = ReasoningStep {
                description: "Selecting tool to find spatial locations".to_string(),
                tool_called: Some(tool.name().to_string()),
                result: "Executing tool...".to_string(),
                confidence: 0.90,
                evidence: vec!["tool_call_req".to_string()],
            };
            chain.add_step(step2);

            let res = tool
                .execute("{\"azimuth\": 45.0}", &self.graph)
                .await
                .unwrap_or_else(|e| (format!("Error: {}", e), vec![]));
            let result_str = res.0;
            source_ids = res.1;
            final_confidence = 0.97;

            let step3 = ReasoningStep {
                description: "Interpreting tool results".to_string(),
                tool_called: None,
                result: result_str.clone(),
                confidence: 0.97,
                evidence: source_ids.iter().map(|s| s.to_string()).collect(),
            };
            chain.add_step(step3);
            response = format!(
                "Based on the forensic knowledge graph, here are the findings for azimuth 45 degrees: {}",
                result_str
            );
        } else if q_lower.contains("heterodyning") {
            let tool = &self.tools[1];
            let res = tool
                .execute("{\"frequency\": 2400000000}", &self.graph)
                .await
                .unwrap();
            let result_str = res.0;
            source_ids = res.1;
            final_confidence = 0.85;

            let step2 = ReasoningStep {
                description: "Analyzing pattern links for frequency".to_string(),
                tool_called: Some(tool.name().to_string()),
                result: result_str.clone(),
                confidence: 0.85,
                evidence: vec!["pattern-1", "freq-2400000000"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            };
            chain.add_step(step2);
            response = format!("The pattern analysis indicates: {}", result_str);
        } else {
            response = "I'm sorry, I couldn't find any relevant data for that query in the knowledge graph.".to_string();
        }

        // Finalize
        let elapsed = start.elapsed();
        chain.finalize(elapsed, final_confidence, source_ids);

        // Track E.3 latency check (<2 seconds response time)
        if elapsed.as_secs_f32() > 2.0 {
            eprintln!(
                "WARNING: Copilot response time exceeded 2 seconds: {:?}",
                elapsed
            );
        }

        (response, chain)
    }
}
