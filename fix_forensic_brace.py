import re

with open('src/forensic.rs', 'r') as f:
    content = f.read()

start = content.find('pub fn log_detection')
end = content.find('pub fn close')
if start != -1 and end != -1:
    content = content[:start] + 'pub fn log_detection(&mut self, _event: &DetectionEvent) -> anyhow::Result<()> {\n        Ok(())\n    }\n\n    ' + content[end:]

with open('src/forensic.rs', 'w') as f:
    f.write(content)
