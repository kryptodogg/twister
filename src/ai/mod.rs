//! AI Reasoning — V3 Track D Dorothy Cognitive Loop
//!
//! # V3 Architecture Notes
//! - copilot_interface, evidence_chain, reasoning_engine deleted
//! - All being rewritten for V3 architecture
//! - query_tools remains as MCP tool handler stub

// pub mod copilot_interface; — deleted, V3 rewrite
// pub mod evidence_chain; — deleted, V3 rewrite
pub mod query_tools;
// pub mod reasoning_engine; — deleted, V3 rewrite

// pub use copilot_interface::*; — deleted
// pub use evidence_chain::*; — deleted
pub use query_tools::*;
// pub use reasoning_engine::*; — deleted
