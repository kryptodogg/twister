with open("src/async_event_handler.rs", "r") as f:
    content = f.read()

bad_match = """                        match kernel.read_results() {
                            Ok(results) => {
                                // Accumulate
                                for (i, res) in results.iter().enumerate() {
                                    if i < latest.len() {
                                        latest[i] = res.clone();
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("[GPU-Dispatch] Read results failed: {}", e);
                            }
                        }"""

good_match = """                        let results = kernel.read_results();
                        for (i, res) in results.iter().enumerate() {
                            if i < latest.len() {
                                latest[i] = res.clone();
                            }
                        }"""

content = content.replace(bad_match, good_match)

with open("src/async_event_handler.rs", "w") as f:
    f.write(content)
