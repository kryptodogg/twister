sed -i 's/-> anyhow::Result<neo4rs::stream::DetachedRowStream>/-> anyhow::Result<neo4rs::RowStream>/g' src/knowledge_graph/graph_builder.rs
