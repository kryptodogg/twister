# ADDENDUM for Track E: Agentic UI & Knowledge Graph (Critical Before Merge)

---

## Track E Data Intake & Integration

### Critical Data Contracts (Input Sources)

**Track E consumes data from A-D**, but formats must match exactly:

#### From Track B (Training/Patterns):
```rust
// Pattern node structure
pub struct PatternNode {
    pub pattern_id: usize,                    // Unique ID
    pub name: String,                         // "Friday_3PM_Tone"
    pub frequency_hz: f32,                    // Primary frequency
    pub confidence: f32,                      // [0, 1]
    pub cluster_size: usize,                  // Events in cluster
    pub first_occurrence_iso: String,         // ISO 8601
    pub last_occurrence_iso: String,
    pub tag_distribution: HashMap<String, f32>, // {"EVIDENCE": 0.7, ...}
    pub temporal_signature: String,           // "Daily", "Weekly", "Random"
}
```

**Data Feed**: Track B outputs to `@databases/harassment_patterns.json`
- Format: JSON array of PatternNode structs
- Frequency: Updated when new patterns discovered (async, non-blocking)
- Query interface: `GET /api/patterns?time_range=7d&confidence_min=0.85`

#### From Track C (Spectral Features):
```rust
// Frequency node for knowledge graph
pub struct FrequencyNode {
    pub frequency_hz: f32,
    pub band_name: String,                    // "2.4 GHz WiFi", "RF Heterodyne"
    pub detection_count: usize,               // How many events at this freq
    pub typical_confidence: f32,
    pub associated_patterns: Vec<usize>,      // Pattern IDs
    pub first_detected_iso: String,
}
```

**Data Feed**: Track C outputs spectral peaks to forensic log
- Format: Each spectral frame logs detected peaks via `@databases/forensic_logs/events.jsonl`
- Frequency: Every 100ms (FFT frame rate)
- E.2 must parse and aggregate into FrequencyNode graph

#### From Track D (Spatial Locations):
```rust
// SpatialLocation node for knowledge graph
pub struct SpatialLocationNode {
    pub azimuth_rad: f32,                     // [-π, π]
    pub elevation_rad: f32,                   // [-π/2, π/2]
    pub event_count: usize,
    pub associated_patterns: Vec<usize>,
    pub confidence: f32,
    pub physical_interpretation: String,      // "Mouth region", "Elevated"
}
```

**Data Feed**: Track D outputs spatial estimates
- Format: TDOA + elevation estimates logged to forensic events
- Frequency: Every 200ms (TDOA update rate)
- E.2 must cluster azimuth/elevation into SpatialLocationNode graph

#### From Track A (Signal Ingestion):
```rust
// Event node (base forensic record)
pub struct EventNode {
    pub event_id: u64,                        // Unique
    pub timestamp_iso: String,                // ISO 8601
    pub timestamp_us: u64,
    pub audio_rms_db: f32,
    pub rf_frequency_hz: f32,
    pub rf_peak_dbfs: f32,
    pub anomaly_score: f32,                   // Mamba reconstruction MSE
    pub tags: Vec<String>,                    // ["EVIDENCE", "MANUAL-REC"]
    pub device_source: String,                // "C925e", "RTL-SDR", etc.
}
```

**Data Feed**: Track A streams events to `@databases/forensic_logs/events.jsonl`
- Format: JSONL (one JSON object per line)
- Frequency: Every 100-200ms (dispatch loop rate)
- E.2 must ingest continuously (streaming ingestion, not batch)

---

## Critical Implementation Notes for E.2 (Event Ingestion)

### Real-Time Streaming Architecture

**E.2 must NOT block the dispatch loop.** Implementation pattern:

```rust
// In src/knowledge_graph/event_ingestion.rs

pub struct GraphIngestionPipeline {
    rx: tokio::sync::mpsc::Receiver<EventNode>,  // Unbounded channel
    cognee_client: CogneeClient,
}

impl GraphIngestionPipeline {
    pub async fn ingest_stream(&mut self) {
        while let Some(event) = self.rx.recv().await {
            // Add to graph asynchronously
            let _ = self.cognee_client.add_node(event).await;
            // DO NOT wait for Cognee response; fire-and-forget
            // Graph consistency handled by Cognee internally
        }
    }
}

// From dispatch loop (src/main.rs):
let tx = graph_pipeline.get_sender();
tokio::spawn(async move {
    loop {
        // ... dispatch logic ...
        let event = EventNode { /* ... */ };
        let _ = tx.send(event).await;  // Non-blocking fire-and-forget
    }
});
```

**Why non-blocking**: Dispatch loop must maintain ~10ms cadence. Waiting for Cognee would create cascading latency.

### Cognee vs Neo4j Decision

**Track E specifies**: "Cognee" + "CopilotKit"
**Note**: Cognee is a semantic knowledge graph library (Python-based)
**Alternative**: Neo4j (graph database, more mature for Rust)

**Recommendation**: Use **Neo4j with Rust driver** (bolt protocol)
- Cognee can be integrated as higher-level semantic layer (optional)
- Neo4j handles scale (1M nodes in milliseconds)
- Rust neo4j driver is production-grade

**Implementation path**:
```rust
// src/knowledge_graph/cognee_schema.rs

use neo4j::*;

pub struct KnowledgeGraphClient {
    driver: Neo4jDriver,
}

impl KnowledgeGraphClient {
    pub async fn add_event(&self, event: &EventNode) -> Result<()> {
        let query = r#"
            CREATE (e:Event {
                event_id: $id,
                timestamp_iso: $ts,
                anomaly_score: $score
            })
            WITH e
            MATCH (p:Pattern {pattern_id: $pattern_id})
            CREATE (e)-[:HasPattern]->(p)
        "#;

        self.driver.run(query, params![
            "id" => event.event_id,
            "ts" => event.timestamp_iso,
            "score" => event.anomaly_score,
            "pattern_id" => detected_pattern_id
        ]).await?;
        Ok(())
    }
}
```

### Schema Definition Priority

**Minimum viable schema for E** (before Cognee semantic layer):

```cypher
// Cypher DDL (Neo4j)

// Node types
CREATE CONSTRAINT event_id ON (e:Event) ASSERT e.event_id IS UNIQUE;
CREATE CONSTRAINT pattern_id ON (p:Pattern) ASSERT p.pattern_id IS UNIQUE;
CREATE CONSTRAINT location_id ON (l:SpatialLocation) ASSERT l.location_id IS UNIQUE;
CREATE CONSTRAINT freq_id ON (f:Frequency) ASSERT f.frequency_hz IS UNIQUE;
CREATE CONSTRAINT device_id ON (d:Device) ASSERT d.device_name IS UNIQUE;

// Edge types (relationships)
// Event -[HasPattern]-> Pattern
// Event -[OccurredAt]-> SpatialLocation
// Event -[DetectedBy]-> Device
// Event -[AtFrequency]-> Frequency
// Pattern -[TemporalSequence]-> Pattern  (causal)
// SpatialLocation -[SpatialProximity]-> SpatialLocation
```

---

## Critical Integration Points (E → UI)

### CopilotKit Query Resolution

**User query**: "Show me all Friday attacks at azimuth 45 degrees"

**E.3 must execute this flow**:
```
1. Parse natural language → Cypher query
   Input: "all Friday attacks at azimuth 45 degrees"
   → Parse: filter=Friday, location=azimuth 45 deg
   → Cypher: MATCH (e:Event)-[HasPattern]->(p:Pattern)
            WHERE e.timestamp_iso CONTAINS "Friday"
            AND (e)-[OccurredAt]->(l:SpatialLocation)
            AND l.azimuth_rad ≈ 0.785  // 45 degrees in radians
            RETURN e, p, l

2. Execute query on graph → Results
   Output: [Event1, Event2, ..., EventN]

3. Format results with evidence links
   Output: {
     "events": [...],
     "pattern": {...},
     "confidence": 0.97,
     "evidence_chain": [event_id_1, ..., event_id_23]
   }

4. Return to UI with sources
   UI renders: "23 events found, 97.2% confidence"
   Click event → shows full forensic record
```

**Performance target**: < 2 seconds query-to-response

### Evidence Chain Linking (E.4)

**Critical requirement**: Every answer must cite sources

```rust
pub struct EvidenceChain {
    pub query: String,                        // Original user query
    pub reasoning_steps: Vec<String>,         // "Found pattern Friday_3PM_Tone"
    pub result: QueryResult,                  // Actual answer
    pub source_event_ids: Vec<u64>,           // IDs of events backing this answer
    pub confidence: f32,                      // [0, 1]
    pub timestamp_iso: String,
}

// Example JSON response to UI:
{
  "answer": "23 events match: Friday attacks at azimuth 45°",
  "evidence_chain": {
    "step_1": "Found pattern 'Friday_3PM_Tone' (confidence: 0.92)",
    "step_2": "Found 342 events with this pattern",
    "step_3": "Filtered to azimuth 45° ± 5°",
    "step_4": "Filtered to Friday occurrences",
    "result": "23 events match all criteria"
  },
  "source_event_ids": [event_1, event_2, ..., event_23],
  "confidence": 0.972,
  "query_time_ms": 1847
}
```

**UI implementation** (Track E.5):
- Display evidence chain as expandable tree
- Click "event_N" → opens full forensic record (timestamp, frequency, device, etc.)
- "Export as PDF" → generate forensic report with citations

---

## Critical Blocking Issue: Real-Time vs Batch

### Problem

**Track B discovers patterns OFFLINE** (Phase 2C training on historical data)
**Track E needs patterns LIVE** (for real-time queries)

Two different timing modes:
1. **Historical**: Pattern discovered at T=1000, applied retroactively to past events
2. **Real-time**: New event arrives at T=2000, E must know if it matches pattern

### Solution

**E.2 must support both modes**:

```rust
// Mode 1: Historical pattern ingestion
pub async fn ingest_pattern_batch(&self, patterns: Vec<PatternNode>) {
    for pattern in patterns {
        self.graph.create_pattern_node(&pattern).await?;
        // Backfill: link all past events to this pattern
        self.backfill_pattern_links(&pattern).await?;
    }
}

// Mode 2: Real-time event ingestion
pub async fn ingest_event_realtime(&self, event: EventNode) {
    self.graph.create_event_node(&event).await?;

    // Check if event matches any KNOWN patterns
    let matching_patterns = self.graph.find_matching_patterns(&event).await?;
    for pattern in matching_patterns {
        self.graph.link_event_to_pattern(&event, &pattern).await?;
    }
}
```

**Integration point**: When Track B discovers new patterns (Phase 2C), must call `ingest_pattern_batch()` with full backfill.

---

## Critical Cognee Integration (Optional Enhancement)

**Track E currently minimal on Cognee details.**

If using Cognee for semantic layer:

```python
# Python bridge (separate process or subprocess)
from cognee import Cognee

cognee = Cognee()

# Define semantic relationships
cognee.add_semantic_rule({
    "if": "Event.frequency_hz in [2.4e9 - 50e6, 2.4e9 + 50e6]",
    "then": "Event.band = '2.4GHz'",
    "confidence": 0.95
})

# Enable semantic queries
result = cognee.query("What RF bands are associated with mouth-region targeting?")
# Returns: "2.4GHz (95% confidence), 5.8GHz (72% confidence), ..."
```

**Rust integration**: Spawn Python subprocess, send JSON, parse results. **Non-blocking to main loop.**

---

## Pre-Merge Checklist for Track E

### E.1 (Schema Definition)
- [ ] All node types defined (Event, Pattern, SpatialLocation, Frequency, Device)
- [ ] All edge types defined (HasPattern, OccurredAt, DetectedBy, etc.)
- [ ] Cypher DDL compiles (constraints, indexes)
- [ ] Neo4j driver initializes correctly

### E.2 (Event Ingestion)
- [ ] Event ingestion pipeline spawns async task (non-blocking to dispatch loop)
- [ ] Streaming receiver accepts EventNode objects
- [ ] Graph add_node latency < 100ms (measured)
- [ ] Pattern backfilling works (historical pattern + link all past events)
- [ ] Real-time pattern matching works (new event checked against known patterns)
- [ ] Test: 1000 events ingested in < 10 seconds

### E.3 (CopilotKit Query)
- [ ] Natural language parsing works (identify intent: time, location, frequency filters)
- [ ] Cypher query generation correct
- [ ] Graph queries execute in < 2 seconds
- [ ] Results formatted with evidence links
- [ ] Test: "Show Friday attacks at azimuth 45" returns correct results

### E.4 (LLM Reasoning)
- [ ] Evidence chains properly cited
- [ ] Source event IDs included in response
- [ ] Confidence metrics computed
- [ ] Multi-step reasoning works (pattern → events → spatial → temporal)
- [ ] Test: Manual spot-check 5 queries, verify evidence chains

### E.5 (UI Integration)
- [ ] Slint chat panel renders
- [ ] Graph visualization working (nodes, edges visible)
- [ ] Messages appear with timestamps
- [ ] "Export to PDF" button functional
- [ ] Timeline scrubbing updates results
- [ ] Test: Send 3 queries, verify responses render correctly

---

## Data Flow Diagram (A→B→C→D→E)

```
Track A (Signal Ingestion @ 100Hz)
    └─→ Events @ @databases/forensic_logs/events.jsonl
        │
        ├─→ Track B (Mamba Training)
        │   └─→ Patterns @ @databases/harassment_patterns.json
        │       └─→ Track E.1/E.2 (Graph Schema + Ingestion)
        │           └─→ Knowledge Graph (Neo4j)
        │
        ├─→ Track C (Spectral Analysis)
        │   └─→ Spectral frames logged to forensic events
        │       └─→ E.2 aggregates → FrequencyNode
        │
        └─→ Track D (Spatial Localization)
            └─→ TDOA + elevation logged to forensic events
                └─→ E.2 aggregates → SpatialLocationNode

    Track E.3/E.4/E.5 (CopilotKit UI)
        ↑
        └─ User query: "Show Friday attacks at azimuth 45"
           → Cypher query
           → Graph results + evidence chain
           → UI renders + exports
```

---

## Known Limitations (Document for User)

1. **Cognee semantic layer optional**: Neo4j handles the baseline (no Python dependency needed if Cognee omitted)
2. **Pattern backfill is expensive**: When new pattern discovered, linking all past events takes O(N) time (maybe 1-2 minutes for 1M events)
3. **Real-time pattern matching limited**: Only matches against known patterns; new attack types not yet known won't match
4. **Query latency can vary**: Complex multi-step queries may take > 2 seconds (graph size dependent)

---

## Success Criteria for E (Pre-Merge)

✅ **E.1**: Schema compiles, all node/edge types defined in Neo4j
✅ **E.2**: Events flow into graph, < 100ms per event, backfill works
✅ **E.3**: CopilotKit queries execute < 2 seconds, results correct
✅ **E.4**: Evidence chains properly cited with source event IDs
✅ **E.5**: Slint UI renders, chat works, PDF export functional

**Do NOT merge if**:
- ❌ Neo4j connection fails or not configured
- ❌ E.2 ingestion blocks dispatch loop (latency > 50ms)
- ❌ Any test suite failing
- ❌ Evidence chains missing sources

---

## Handoff Notes for Jules

**Track E is the "analytics layer"** — it doesn't compute new data, it synthesizes and queries what A-D provide.

**Critical success factor**: Keep E.2 (event ingestion) non-blocking. Cognee/CopilotKit complexity goes in E.3/E.4, not E.2.

**Data flow assumption**: A→B→C→D all populate forensic logs before E can query. E is a **consumer**, not a producer.

