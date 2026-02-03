//! Configuration management for Phago CLI.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Phago project configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub colony: ColonyConfig,
    #[serde(default)]
    pub digester: DigesterConfig,
    #[serde(default)]
    pub wiring: WiringConfig,
    #[serde(default)]
    pub query: QueryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColonyConfig {
    #[serde(default = "default_tick_rate")]
    pub tick_rate: u64,
    #[serde(default = "default_max_agents")]
    pub max_agents: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigesterConfig {
    #[serde(default = "default_max_idle")]
    pub max_idle: u64,
    #[serde(default = "default_sense_radius")]
    pub sense_radius: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WiringConfig {
    #[serde(default = "default_edge_decay")]
    pub edge_decay_rate: f64,
    #[serde(default = "default_prune_threshold")]
    pub prune_threshold: f64,
    #[serde(default = "default_tentative_weight")]
    pub tentative_weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryConfig {
    #[serde(default = "default_alpha")]
    pub default_alpha: f64,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
}

// Default value functions
fn default_tick_rate() -> u64 { 100 }
fn default_max_agents() -> usize { 50 }
fn default_max_idle() -> u64 { 50 }
fn default_sense_radius() -> f64 { 5.0 }
fn default_edge_decay() -> f64 { 0.01 }
fn default_prune_threshold() -> f64 { 0.05 }
fn default_tentative_weight() -> f64 { 0.1 }
fn default_alpha() -> f64 { 0.5 }
fn default_max_results() -> usize { 10 }

impl Default for Config {
    fn default() -> Self {
        Self {
            colony: ColonyConfig::default(),
            digester: DigesterConfig::default(),
            wiring: WiringConfig::default(),
            query: QueryConfig::default(),
        }
    }
}

impl Default for ColonyConfig {
    fn default() -> Self {
        Self {
            tick_rate: default_tick_rate(),
            max_agents: default_max_agents(),
        }
    }
}

impl Default for DigesterConfig {
    fn default() -> Self {
        Self {
            max_idle: default_max_idle(),
            sense_radius: default_sense_radius(),
        }
    }
}

impl Default for WiringConfig {
    fn default() -> Self {
        Self {
            edge_decay_rate: default_edge_decay(),
            prune_threshold: default_prune_threshold(),
            tentative_weight: default_tentative_weight(),
        }
    }
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self {
            default_alpha: default_alpha(),
            max_results: default_max_results(),
        }
    }
}

impl Config {
    /// Load config from phago.toml in the current or parent directories.
    pub fn load() -> Result<Self> {
        if let Some(path) = find_config_file() {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config: {}", path.display()))?;
            toml::from_str(&content)
                .with_context(|| format!("Failed to parse config: {}", path.display()))
        } else {
            Ok(Config::default())
        }
    }

    /// Save config to the specified path.
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config: {}", path.display()))?;
        Ok(())
    }

    /// Generate default config as TOML string.
    pub fn default_toml() -> String {
        let config = Config::default();
        toml::to_string_pretty(&config).unwrap()
    }
}

/// Find phago.toml in current or parent directories.
fn find_config_file() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let config_path = dir.join("phago.toml");
        if config_path.exists() {
            return Some(config_path);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

/// Get the Phago data directory (.phago/).
pub fn data_dir() -> Result<PathBuf> {
    let dir = std::env::current_dir()?.join(".phago");
    Ok(dir)
}

/// Get the sessions directory.
pub fn sessions_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("sessions"))
}

/// Get the current session file path.
pub fn current_session_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("current.json"))
}
