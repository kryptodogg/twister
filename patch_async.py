import re

with open("src/async_event_handler.rs", "r") as f:
    content = f.read()

# Fix 1: dispatch_autonomous_batch returns ()
content = content.replace("if let Err(e) = kernel.dispatch_autonomous_batch() {\n                        eprintln!(\"[GPU-Dispatch] Dispatch failed: {}\", e);\n                    }", "kernel.dispatch_autonomous_batch();")

# Fix 2: read_results returns &[DispatchResultVBuffer] instead of Result
content = content.replace("""match kernel.read_results() {
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
                        }""", """let results = kernel.read_results();
                        for (i, res) in results.iter().enumerate() {
                            if i < latest.len() {
                                latest[i] = res.clone();
                            }
                        }""")

with open("src/async_event_handler.rs", "w") as f:
    f.write(content)
