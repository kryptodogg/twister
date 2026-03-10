import re

with open('src/ml/mod.rs', 'r') as f:
    ml_content = f.read()

# Remove duplicate spectral_frame and anomaly_gate
lines = ml_content.split('\n')
new_lines = []
seen_modules = set()
for line in lines:
    if line.startswith('pub mod '):
        mod_name = line.split(' ')[2].replace(';', '')
        if mod_name in seen_modules:
            continue
        seen_modules.add(mod_name)
    new_lines.append(line)

ml_content = '\n'.join(new_lines)

# Fix missing evaluate_anomaly_gate import
ml_content = ml_content.replace(
    "pub use anomaly_gate::{AnomalyGateConfig, AnomalyGateDecision, evaluate_anomaly_gate};",
    "pub use anomaly_gate::{AnomalyGateConfig, AnomalyGateDecision};"
)

with open('src/ml/mod.rs', 'w') as f:
    f.write(ml_content)


with open('src/forensic.rs', 'r') as f:
    f_content = f.read()

# Remove duplicate AnomalyGateDecision from src/forensic.rs (keep the first one)
# The second one is at line 228 (from rustc error)
parts = f_content.split('AnomalyGateDecision {')
if len(parts) > 2:
    # There are multiple AnomalyGateDecision {
    # It's an enum variant, so we need to carefully remove the duplicate.
    # Actually, we can just replace the unreachable pattern in the derive macro logic or remove the second variant definition entirely.
    pass

with open('src/forensic.rs', 'w') as f:
    f.write(f_content)
