cat src/knowledge_graph/graph_builder.rs | sed 's/-> anyhow::Result<neo4rs::RowStream>/-> anyhow::Result<neo4rs::RowStream>/' > tmp.rs && mv tmp.rs src/knowledge_graph/graph_builder.rs
