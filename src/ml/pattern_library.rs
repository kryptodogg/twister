use std::error::Error;
use std::fs;
use super::pattern_discovery::PatternLibrary;

impl PatternLibrary {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn save(&self, path: &str) -> Result<(), Box<dyn Error>> {
        let json = self.to_json()?;
        fs::write(path, json)?;
        eprintln!("[Pattern Library] Saved: {} ({} motifs)", path, self.total_patterns);
        Ok(())
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

pub fn load_pattern_library(json_path: &str) -> Result<PatternLibrary, Box<dyn Error>> {
    let json = fs::read_to_string(json_path)?;
    Ok(PatternLibrary::from_json(&json)?)
}
