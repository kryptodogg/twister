sed -i 's/-> anyhow::Result<neo4rs::RowStream>/-> anyhow::Result<neo4rs::stream::DetachedRowStream>/g' src/knowledge_graph/graph_builder.rs
