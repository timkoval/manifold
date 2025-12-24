//! Configuration management for manifold
//!
//! Handles the ~/.manifold/ directory structure and config.toml

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Default boundary for new specs
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DefaultBoundary {
    #[default]
    Personal,
    Work,
    Company,
}

impl std::fmt::Display for DefaultBoundary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DefaultBoundary::Personal => write!(f, "personal"),
            DefaultBoundary::Work => write!(f, "work"),
            DefaultBoundary::Company => write!(f, "company"),
        }
    }
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub default_boundary: DefaultBoundary,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub mcp: McpConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_boundary: DefaultBoundary::default(),
            llm: LlmConfig::default(),
            mcp: McpConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmConfig {
    pub endpoint: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub port: u16,
    pub host: String,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            host: "127.0.0.1".to_string(),
        }
    }
}

/// Returns the path to the manifold home directory (~/.manifold)
pub fn manifold_home() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".manifold"))
}

/// Returns paths to all manifold directories
pub struct ManifoldPaths {
    pub root: PathBuf,
    pub config: PathBuf,
    pub db: PathBuf,
    pub db_file: PathBuf,
    pub schemas: PathBuf,
    pub exports: PathBuf,
    pub cache: PathBuf,
}

impl ManifoldPaths {
    pub fn new() -> Result<Self> {
        let root = manifold_home()?;
        Ok(Self {
            config: root.join("config.toml"),
            db: root.join("db"),
            db_file: root.join("db/manifold.db"),
            schemas: root.join("schemas"),
            exports: root.join("exports"),
            cache: root.join("cache"),
            root,
        })
    }

    /// Create all directories if they don't exist
    pub fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.root).context("Failed to create manifold root")?;
        fs::create_dir_all(&self.db).context("Failed to create db directory")?;
        fs::create_dir_all(&self.schemas).context("Failed to create schemas directory")?;
        fs::create_dir_all(&self.schemas.join("plugins"))
            .context("Failed to create plugins directory")?;
        fs::create_dir_all(&self.exports).context("Failed to create exports directory")?;
        fs::create_dir_all(&self.cache).context("Failed to create cache directory")?;
        Ok(())
    }

    /// Check if manifold has been initialized
    pub fn is_initialized(&self) -> bool {
        self.config.exists() && self.db_file.exists()
    }
}

/// Load configuration from disk
/// Used for default boundary, LLM settings, and MCP server config
pub fn load_config() -> Result<Config> {
    let paths = ManifoldPaths::new()?;
    if !paths.config.exists() {
        return Ok(Config::default());
    }
    let content = fs::read_to_string(&paths.config).context("Failed to read config.toml")?;
    toml::from_str(&content).context("Failed to parse config.toml")
}

/// Save configuration to disk
pub fn save_config(config: &Config) -> Result<()> {
    let paths = ManifoldPaths::new()?;
    let content = toml::to_string_pretty(config).context("Failed to serialize config")?;
    fs::write(&paths.config, content).context("Failed to write config.toml")?;
    Ok(())
}
