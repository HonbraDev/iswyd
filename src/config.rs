use serde::{Deserialize, Serialize};
use std::{io, path::PathBuf};
use thiserror::Error;

/// A filesystem-based configuration store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub discor_token: String,
    pub mong_connstring: String,
    pub guild_whitelist: Vec<u64>,
}

#[derive(Debug, Error)]
pub enum ConfigLoadSaveError {
    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),

    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),

    #[error(transparent)]
    Io(#[from] io::Error),
}

impl Config {
    /// Load a configuration file from the filesystem
    pub async fn load(path: &PathBuf) -> Result<Self, ConfigLoadSaveError> {
        let file = tokio::fs::read_to_string(path).await?;
        let config = toml::from_str(&file)?;
        Ok(config)
    }

    #[allow(dead_code)]
    /// Save the current configuration as a file to the filesystem
    pub async fn save(&self, path: &PathBuf) -> Result<(), ConfigLoadSaveError> {
        let file = toml::to_string(&self)?;
        tokio::fs::write(path, file).await?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            discor_token: "💀".to_string(),
            mong_connstring: "skull emoji".to_string(),
            guild_whitelist: vec![],
        }
    }
}
