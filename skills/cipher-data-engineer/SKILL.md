# Cipher Data Engineer Skill

Data encoding, OFDM framing, serialization, PLAS grid sorting, SOGS attributes,
efficient storage and retrieval, embedding management, Neo4j/Qdrant integration.

## Domain
- Neo4j graph database (Cypher queries, schema design)
- Qdrant vector store (32-dim embeddings, cosine similarity)
- Serialization (serde, JSONL, safetensors)
- Write-back queues (async, non-blocking)
- Latent caches (in-memory similarity search)
- Embedding storage and retrieval
- Error recovery (retry logic, exponential backoff)

## Trigger Patterns
"Neo4j", "Qdrant", "embedding", "graph", "serialization", "database",
"write-back", "cache", "Cypher", "graph.rs", "embeddings.rs"

## Available Functions
- `store_carrier()` — Neo4j carrier node with latent
- `query_neighbors()` — Graph neighborhood lookup
- `find_similar_carriers()` — Latent cosine similarity
- `store_detection()` — Qdrant embedding upsert
- `create_writeback_queue()` — Async persistence
- `serialize_event()` — DetectionEvent → JSONL

## Neo4j Schema

### Node Labels
```cypher
(:Carrier {id, frequency_hz, magnitude, phase_rad, stability, latent_embedding, first_seen, last_seen, source})
(:Product {id, product_type, frequency_hz, magnitude, parent_f1, parent_f2, coherence_frames})
(:Session {id, start_time, end_time, detection_count, input_source})
```

### Relationships
```cypher
(:Carrier)-[:GENERATES {type}]->(:Product)
(:Carrier)-[:INTERMODULATES_WITH {order}]->(:Carrier)
(:Session)-[:CONTAINS]->(:Carrier)
(:Carrier)-[:SIMILAR_TO {cosine}]->(:Carrier)
```

## Code Patterns

### Write-Back Queue
```rust
let (tx, rx) = crossbeam_channel::bounded(1024);
thread::spawn(|| while let Ok(op) = rx.recv() { /* persist */ });
```

### Latent Similarity Cache
```rust
fn find_similar(&self, query: &[f32], threshold: f32) -> Vec<String> {
    self.latents.iter()
        .filter(|(_, l)| cosine_similarity(query, l) > threshold)
        .map(|(id, _, _)| id.clone())
        .collect()
}
```

### Retry with Exponential Backoff
```rust
let mut retries = 0;
loop {
    match connect().await {
        Ok(c) => return Ok(c),
        Err(e) if retries < 5 => sleep(2^retries).await,
        Err(e) => return Err(e),
    }
}
```
