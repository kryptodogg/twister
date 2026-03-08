import re

with open('src/async_event_handler.rs', 'r') as f:
    text = f.read()

text = re.sub(r'eprintln!\("Dispatch failed: \{\}", e\);', 'eprintln!("Dispatch failed.");', text)
text = re.sub(r'eprintln!\("Read results failed: \{\}", e\);', 'eprintln!("Read results failed.");', text)

with open('src/async_event_handler.rs', 'w') as f:
    f.write(text)
