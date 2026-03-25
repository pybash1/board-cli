use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    pub data_dir: PathBuf,
    pub theme: String,
    pub auto_save: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            data_dir: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".board-cli"),
            theme: "default".to_string(),
            auto_save: true,
        }
    }
}

impl AppConfig {
    #[allow(dead_code)]
    pub fn load() -> anyhow::Result<Self> {
        // In a real app, you'd load from a config file
        // For now, return default
        Ok(Self::default())
    }
}