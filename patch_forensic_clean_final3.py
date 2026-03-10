with open('src/forensic.rs', 'r') as f:
    content = f.read()

# Make the absolute minimal stub replacements safely using Python
# Only replace the method bodies without touching structs or other things.

start_gate = content.find("pub fn log_gate_decision(&mut self, score: f32, confidence: f32, threshold: f32, forward: bool, reason: &str) -> anyhow::Result<()> {")
end_gate = content.find("Ok(())\n    }", start_gate) + len("Ok(())\n    }")

if start_gate != -1:
    stub_gate = """pub fn log_gate_decision(&mut self, _score: f32, _confidence: f32, _threshold: f32, _forward: bool, _reason: &str) -> anyhow::Result<()> {
        // STUB: V3 Node.js WebSocket migration.
        // Rust no longer logs cognitive decisions locally.
        Ok(())
    }"""
    content = content[:start_gate] + stub_gate + content[end_gate:]

start_det1 = content.find("pub fn log_detection(&mut self, event: &DetectionEvent) -> anyhow::Result<()> {")
if start_det1 != -1:
    end_det1 = content.find("Ok(())\n    }", start_det1) + len("Ok(())\n    }")
    stub_det1 = """pub fn log_detection_old(&mut self, _event: &DetectionEvent) -> anyhow::Result<()> {
        // STUB: V3 Node.js WebSocket migration.
        Ok(())
    }"""
    content = content[:start_det1] + stub_det1 + content[end_det1:]

with open('src/forensic.rs', 'w') as f:
    f.write(content)

with open('src/main.rs', 'r') as f:
    content = f.read()

content = content.replace("                            if let Ok(mut f) = fdc2.lock() {", "                            if true { // fdc2.lock() removed since it's not a Mutex \n                            let mut f = fdc2.clone();")

with open('src/main.rs', 'w') as f:
    f.write(content)
