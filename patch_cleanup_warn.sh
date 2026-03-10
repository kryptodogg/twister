sed -i '/use std::fs;/d' src/ml/event_corpus.rs
sed -i '/use std::io::{BufRead, BufReader};/d' src/ml/event_corpus.rs
sed -i '/use burn::tensor::Device;/d' src/ml/event_corpus.rs
sed -i '/use burn::tensor::{Bool, Int, Float};/d' src/ml/pattern_discovery.rs
sed -i '/use burn::backend::Wgpu;/d' src/ml/wav2vec2_loader.rs
sed -i 's/use chrono::{DateTime, Utc};/use chrono::Utc;/g' src/knowledge_graph/cognee_schema.rs
