use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_id: i32,
    pub api_hash: String,
    pub interval_secs: u64,
    pub bio_template: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_id: 123456789,
            api_hash: "".to_string(),
            interval_secs: 60,
            bio_template: "▶️ {artist} - {title} ({progress} / {duration})".to_string(),
        }
    }
}

impl Config {
    pub fn load_or_create(path: PathBuf) -> anyhow::Result<Self> {
        if !path.exists() {
            let cfg = Self::default();
            let serialized = serde_json::to_string_pretty(&cfg)?;
            std::fs::write(&path, serialized)?;
        }

        let serialized = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&serialized)?)
    }
}
