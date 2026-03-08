import re

with open("src/async_event_handler.rs", "r") as f:
    content = f.read()

bad_match = """                        match kernel.read_results() {
                            Ok(results) => {"""

good_match = """                        let results = kernel.read_results();
                        if true {"""

content = content.replace(bad_match, good_match)

bad_match2 = """                                }
                            }
                            Err(e) => {
                                eprintln!("[CPU-EventHandler] Read results failed: {}", e);
                                error_count += 1;
                            }
                        }"""
good_match2 = """                                }
                            }"""
content = content.replace(bad_match2, good_match2)

with open("src/async_event_handler.rs", "w") as f:
    f.write(content)
