# Track E: Agentic UI & Knowledge Graph (Cognee + CopilotKit)

**For**: Assigned developer(s)
**Goal**: Build intelligent assistant with knowledge graph; natural language queries over forensic events, patterns, spatial data

---

## Overview

Track E synthesizes all data from Tracks A-D into a coherent knowledge base and conversational interface. Cognee builds a knowledge graph of harassment events, patterns, motifs, spatial locations, and temporal relationships. CopilotKit enables users to ask questions like 'Show me all Friday attacks at azimuth 45 degrees' and get structured answers with evidence links.

**Why this matters**:
- Knowledge synthesis: 97-day data becomes queryable intelligence
- Evidence chains: Events -> Patterns -> Spatial locations -> Timestamps
- Natural language access: Non-technical users query forensic data
- AI-assisted investigation: LLM finds subtle correlations
- Audit trail: Every query logged with sources
- Real-time + historical: Live updates + playback

---

## Track E.1: Cognee Knowledge Graph Schema (1.5 days)

Define node/edge types for forensic knowledge graph

- src/knowledge_graph/cognee_schema.rs (300 lines)
- src/knowledge_graph/graph_builder.rs (200 lines)

Node types: Event, Pattern, SpatialLocation, Frequency, Timestamp, Device
Edge types: HasPattern, OccurredAt, DetectedBy, TemporalSequence, SpatialProximity

---

## Track E.2: Event Ingestion (1.5 days)

Stream forensic events into knowledge graph in real-time

- src/knowledge_graph/event_ingestion.rs (400 lines)
- src/knowledge_graph/graph_sync.rs (200 lines)

100+ events/second ingestion rate, <100ms latency, 1M node capacity

---

## Track E.3: CopilotKit Query Interface (2 days)

Conversational interface to knowledge graph

- src/ai/copilot_interface.rs (350 lines)
- src/ai/query_tools.rs (300 lines)

Multi-turn conversation, graph queries, structured responses, evidence links

---

## Track E.4: LLM Reasoning & Evidence Chains (1.5 days)

Multi-step reasoning with full proof chains

- src/ai/reasoning_engine.rs (250 lines)
- src/ai/evidence_chain.rs (200 lines)

Decompose queries, build proof chains, cite sources, confidence metrics

---

## Track E.5: UI Integration (1.5 days)

Slint chat interface with graph visualization

- ui/copilot_panel.slint (300 lines)
- src/ui/copilot_handler.rs (200 lines)

Chat messages, graph rendering, timeline scrubbing, PDF export

---

## Example Queries

'Show me all Friday attacks at azimuth 45 degrees'
-> Graph query: Event nodes with SpatialLocation (45 deg), temporal filter (Friday)
-> Results: 23 events, 97.2% confidence

'What is the pattern for 2.4 GHz heterodyning?'
-> Find Frequency nodes, Pattern links, temporal analysis
-> Results: Daily pattern, 85% confidence, 1200+ events

---

## Knowledge Graph Structure

Nodes: Event, Pattern, SpatialLocation, Frequency, Timestamp
Edges: HasPattern, OccurredAt, DetectedBy, TargetsFrequency, TemporalSequence

Graph size: 1M nodes (97 days)
Query latency: <2 seconds

---

## Integration

Input: Forensic events (B.1), patterns (C.2), spatial locations (D.1)
Output: Chat UI, graph visualization, evidence reports

---

## Success Criteria

E.1: Schema compiles, all node/edge types defined
E.2: Event ingestion <100ms latency
E.3: CopilotKit conversation works, <2s response time
E.4: Evidence chains properly cited
E.5: Slint UI renders graph + messages

All tests passing, 0 warnings

---

**Last Updated**: 2026-03-08
**Review**: Ready for assignment
