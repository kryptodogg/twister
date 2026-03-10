with open('src/ml/mod.rs', 'r') as f:
    ml_content = f.read()

# Replace the incorrect import line
ml_content = ml_content.replace(
    "pub use anomaly_gate::{AnomalyGateConfig, AnomalyGateDecision};",
    "" # Let's just remove it, the structures might not exist there, or aren't used publicly
)

with open('src/ml/mod.rs', 'w') as f:
    f.write(ml_content)
