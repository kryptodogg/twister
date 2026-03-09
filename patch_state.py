import re

with open("src/state.rs", "r") as f:
    content = f.read()

if "pub gate_status: Mutex<String>," not in content:
    replacement = """    // ── Twister: musical auto-tuner ────────────────────────────────────────────
    pub gate_status: Mutex<String>,
    pub last_gate_reason: Mutex<String>,
    pub training_pairs_dropped: AtomicU32,
    pub gate_rejections_low_anomaly: AtomicU32,
    pub gate_rejections_low_confidence: AtomicU32,
    pub gate_rejections_other: AtomicU32,

    /// Most recent snapped note name, e.g. "A4", "C#5". "---" when silent."""

    content = content.replace("    // ── Twister: musical auto-tuner ────────────────────────────────────────────\n    /// Most recent snapped note name, e.g. \"A4\", \"C#5\". \"---\" when silent.", replacement)

    replacement2 = """            reconstructed_peak: AtomicF32::new(0.0),

            gate_status: Mutex::new("IDLE".to_string()),
            last_gate_reason: Mutex::new("".to_string()),
            training_pairs_dropped: AtomicU32::new(0),
            gate_rejections_low_anomaly: AtomicU32::new(0),
            gate_rejections_low_confidence: AtomicU32::new(0),
            gate_rejections_other: AtomicU32::new(0),

            note_name: Mutex::new("---".to_string()),"""

    content = content.replace("            reconstructed_peak: AtomicF32::new(0.0),\n\n            note_name: Mutex::new(\"---\".to_string()),", replacement2)

with open("src/state.rs", "w") as f:
    f.write(content)
