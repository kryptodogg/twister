with open('src/forensic.rs', 'r') as f:
    content = f.read()

# Make the absolute minimal stub replacements safely using Python
start_gate = content.find("pub fn log_gate_decision(&mut self, score: f32, confidence: f32, threshold: f32, forward: bool, reason: &str) -> anyhow::Result<()> {")
end_gate = content.find("Ok(())\n    }", start_gate) + len("Ok(())\n    }")

if start_gate != -1:
    stub_gate = """pub fn log_gate_decision(&self, _score: f32, _confidence: f32, _threshold: f32, _forward: bool, _reason: &str) -> anyhow::Result<()> {
        // STUB: V3 Node.js WebSocket migration.
        // Rust no longer logs cognitive decisions locally.
        Ok(())
    }"""
    content = content[:start_gate] + stub_gate + content[end_gate:]

# Note: this finds the FIRST instance of pub fn log_detection, which is the broken one that swallows the second one!
# Wait, let's just replace all `pub fn log_detection` up to their `Ok(()) \n    }` with stubs
start_det1 = content.find("pub fn log_detection(&mut self, event: &DetectionEvent) -> anyhow::Result<()> {")
if start_det1 != -1:
    end_det1 = content.find("Ok(())\n    }", start_det1) + len("Ok(())\n    }")
    stub_det1 = """pub fn log_detection(&self, _event: &DetectionEvent) -> anyhow::Result<()> {
        // STUB: V3 Node.js WebSocket migration.
        Ok(())
    }"""
    content = content[:start_det1] + stub_det1 + content[end_det1:]

# There is ANOTHER log_detection
start_det2 = content.find("pub fn log_detection(&self, event: &DetectionEvent) -> Result<(), LogError> {")
if start_det2 != -1:
    end_det2 = content.find("self.log(fe)\n    }", start_det2) + len("self.log(fe)\n    }")
    stub_det2 = """pub fn log_detection_old(&self, _event: &DetectionEvent) -> Result<(), LogError> {
        Ok(())
    }"""
    content = content[:start_det2] + stub_det2 + content[end_det2:]


with open('src/forensic.rs', 'w') as f:
    f.write(content)
