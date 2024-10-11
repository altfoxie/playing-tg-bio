use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub service: Service,
    pub interval: u64,
    pub template: String,
    pub default: String,
    pub telegram: TelegramConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Service {
    #[serde(rename = "spotify")]
    Spotify,
    #[cfg(target_os = "macos")]
    #[serde(rename = "apple_music")]
    AppleMusic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TelegramConfig {
    #[serde(rename = "bio")]
    Bio { api_id: i32, api_hash: String },

    #[serde(rename = "channel")]
    Channel {
        token: String,
        channel_id: i64,
        message_id: i64,
    },
}

impl Default for Config {
    fn default() -> Self {
        Self {
            service: Service::Spotify,
            interval: 60,
            template: "{artist} — {title} [{progress} / {duration}]".to_string(),
            default: "nothing playing".to_string(),
            telegram: TelegramConfig::Bio {
                api_id: 123456789,
                api_hash: "".to_string(),
            },
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
