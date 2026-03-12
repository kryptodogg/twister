use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use crate::ml::field_particle::FieldParticle;

pub struct ForensicSnapshotManager;

impl ForensicSnapshotManager {
    /// Capture current hologram state to a forensic spreadsheet.
    /// Strictly blocks test artifact directories (examples, tests).
    pub fn save_snapshot(path_str: &str, particles: &[FieldParticle]) -> Result<(), String> {
        let path = Path::new(path_str);
        if path.components().any(|c| {
            let s = c.as_os_str().to_string_lossy();
            s == "tests" || s == "examples"
        }) {
            return Err("Test files must not be used in production".to_string());
        }

        let file = File::create(path).map_err(|e| e.to_string())?;
        let mut writer = BufWriter::new(file);

        // Forensic Header
        writeln!(writer, "timestamp_us,source_id,intensity,pos_x,pos_y,pos_z,conf_0,conf_1,conf_2,conf_3")
            .map_err(|e| e.to_string())?;

        for p in particles {
            writeln!(
                writer,
                "{},{},{:.6},{:.4},{:.4},{:.4},{:.2},{:.2},{:.2},{:.2}",
                p.timestamp_us,
                p.source_id,
                p.intensity,
                p.position[0],
                p.position[1],
                p.position[2],
                p.confidence[0],
                p.confidence[1],
                p.confidence[2],
                p.confidence[3]
            ).map_err(|e| e.to_string())?;
        }

        Ok(())
    }
}
