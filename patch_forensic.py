import re
with open("src/forensic.rs", "r") as f:
    text = f.read()

# Fix the incomplete log_detection method
text = re.sub(
    r'    pub fn log_detection\(&mut self, event: &DetectionEvent\) -> anyhow::Result<\(\)> \{\n        self\.event_count \+= 1;\n\n        // Create forensic event with full metadata\n        let forensic_event =\n            ForensicEvent::from_detection\(event, &self\.session_id, self\.equipment\.clone\(\)\);\n\n        // Log as forensic event\n        let record = serde_json::to_string\(&forensic_event\)\?;\n        writeln!\(self\.writer, "\{\}", record\)\?;\n\n    pub fn log_detection\(&self, event: &DetectionEvent\) -> Result<\(\), LogError> \{',
    r'''    pub fn log_detection(&mut self, event: &DetectionEvent) -> anyhow::Result<()> {
        self.event_count += 1;

        // Create forensic event with full metadata
        let forensic_event =
            ForensicEvent::from_detection(event, &self.session_id, self.equipment.clone());

        // Log as forensic event
        let record = serde_json::to_string(&forensic_event)?;
        writeln!(self.writer, "{}", record)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn log_detection_v2(&self, event: &DetectionEvent) -> Result<(), LogError> {''',
    text
)

with open("src/forensic.rs", "w") as f:
    f.write(text)
