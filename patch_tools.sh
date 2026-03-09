sed -i 's/let mut stream = graph.execute_query/let (mut txn, mut stream) = graph.execute_query/g' src/ai/query_tools.rs
sed -i 's/stream.next().await/stream.next(\&mut txn).await/g' src/ai/query_tools.rs
