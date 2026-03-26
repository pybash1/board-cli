use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use anyhow::{Context, Result};
use crate::api::DeviceCode;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfig {
    pub data_dir: PathBuf,
    pub theme: String,
    pub auto_save: bool,
    pub device_code: Option<String>,
    pub app_password: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            data_dir: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".board-cli"),
            theme: "default".to_string(),
            auto_save: true,
            device_code: None,
            app_password: None,
        }
    }
}

impl AppConfig {
    /// Get the config file path: ~/.config/board/config.toml
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
            .join("board");

        Ok(config_dir.join("config.toml"))
    }

    /// Load configuration from file or create default if it doesn't exist
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let config_content = fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

            let config: AppConfig = toml::from_str(&config_content)
                .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;

            Ok(config)
        } else {
            // Create default config and save it
            let default_config = Self::default();
            default_config.save()?;
            Ok(default_config)
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }

        let config_content = toml::to_string_pretty(self)
            .context("Failed to serialize config to TOML")?;

        fs::write(&config_path, config_content)
            .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

        Ok(())
    }

    /// Get the device code as DeviceCode type if present
    pub fn get_device_code(&self) -> Option<DeviceCode> {
        self.device_code.as_ref().map(|code| DeviceCode::from(code.clone()))
    }

    /// Set the device code and auto-save the configuration
    pub fn set_device_code(&mut self, device_code: DeviceCode) -> Result<()> {
        self.device_code = Some(device_code.as_str().to_string());
        self.save()
    }

    /// Clear the device code and auto-save the configuration
    pub fn clear_device_code(&mut self) -> Result<()> {
        self.device_code = None;
        self.save()
    }

    /// Check if a device code is configured
    pub fn has_device_code(&self) -> bool {
        self.device_code.is_some()
    }

    /// Get the app password if present
    pub fn get_app_password(&self) -> Option<&str> {
        self.app_password.as_deref()
    }

    /// Check if an app password is configured
    pub fn has_app_password(&self) -> bool {
        self.app_password.is_some()
    }
}