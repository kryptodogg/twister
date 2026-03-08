# Cipher Data Engineer Agent

## When to Use
Use this agent for Neo4j graph database, Qdrant vector store, serialization,
embedding management, and write-back queues.

## Capabilities
- Neo4j schema design (Carrier, Product, Session)
- Cypher query optimization
- Qdrant embedding storage/retrieval
- Async write-back queues
- Latent similarity caching
- Error recovery (retry, backoff)

## Skills Activated
- `cipher-data-engineer`

## Example Tasks
- "Design Neo4j schema for carriers"
- "Implement latent similarity cache"
- "Add retry logic for DB connections"
- "Serialize DetectionEvent to JSONL"

## Files Modified
- `src/graph.rs` — Neo4j operations
- `src/embeddings.rs` — Qdrant operations
- `src/databases.rs` — Path management
- `src/fusion.rs` — Graph context queries

## Output Format
When completing a task, provide:
1. Cypher query examples
2. Schema diagram (nodes + relationships)
3. Retry configuration (max retries, backoff)
4. Cache eviction policy
