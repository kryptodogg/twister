with open('src/forensic.rs', 'r') as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if 'pub fn log_detection(&mut self, event: &DetectionEvent) -> anyhow::Result<()> {' in line:
        start_idx = i
        break

for i in range(start_idx, len(lines)):
    if 'pub fn log_detection(&self, event: &DetectionEvent) -> Result<(), LogError> {' in lines[i]:
        end_idx = i
        break
else:
    end_idx = len(lines) - 50

new_lines = lines[:start_idx] + ['    pub fn log_detection(&mut self, _event: &DetectionEvent) -> anyhow::Result<()> {\n', '        Ok(())\n', '    }\n\n'] + lines[end_idx:]

with open('src/forensic.rs', 'w') as f:
    f.writelines(new_lines)
