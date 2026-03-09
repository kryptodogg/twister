sed -i 's/let _ = self.driver.run(query(query)).await;/let q = neo4rs::query(query); let _ = self.driver.run(q).await;/g' src/knowledge_graph/graph_builder.rs
