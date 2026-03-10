//! Forensics pipeline for RF-Audio event logging
//!
//! Provides:
//! - ForensicEvent struct with all features
//! - Qdrant vector storage (latent as primary vector)
//! - Neo4j graph relationships (Event, RFSource, Location, MusicProgram, NoiseProfile)

pub mod event;
pub mod qdrant_store;
pub mod neo4j_graph;

pub use event::{ForensicEvent, EventMetadata, RFContext, AudioContext};
pub use qdrant_store::{QdrantForensics, QdrantConfig, VectorSearch};
pub use neo4j_graph::{Neo4jForensics, Neo4jConfig, GraphRelationship};
