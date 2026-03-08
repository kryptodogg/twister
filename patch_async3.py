import re

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

# Actually let's just replace all instances of match kernel.read_results()
# that match the error lines 196 and 242.
# Wait, my previous python script replaced `bad_match` which might have had a different string. Let's just do it directly.

# Line 196: `match kernel.read_results()`
# Let's see the context
