//! Provides a ConfigManager to read and refresh config from files.
//!

use color_eyre::Result;
use log::*;
use notify::Watcher;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use toml;

pub const DEFAULT_FILE: &str = "procli.toml";

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub struct RestartPolicy {
    pub enabled: bool,
    pub cooloff: u64,
    pub max_restarts: u32,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Service {
    pub name: String,
    pub display: Option<String>,
    pub image: Option<String>,
    pub command: Option<String>,
    pub directory: Option<String>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub restart: Option<RestartPolicy>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Stub {
    pub name: String,
    pub display: Option<String>,
    pub image: Option<String>,
    pub command: Option<String>,
    pub directory: Option<String>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    pub restart: Option<RestartPolicy>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub name: String,
    pub display: Option<String>,
    pub scenario: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Logging {
    #[serde(default = "default_log_buffer_size")]
    pub buffer_size: usize,
    pub file: Option<String>,
    #[serde(default = "default_log_level")]
    pub level: log::LevelFilter,
}

impl Default for Logging {
    fn default() -> Self {
        Self {
            buffer_size: default_log_buffer_size(),
            file: Default::default(),
            level: default_log_level(),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProcliConfig {
    #[serde(default)]
    pub services: Vec<Service>,
    #[serde(default)]
    pub stubs: Vec<Stub>,
    #[serde(default)]
    pub agents: Vec<Agent>,
    #[serde(default)]
    pub logging: Logging,
}

impl ProcliConfig {
    pub fn get_service(&self, name: &str) -> Option<&Service> {
        self.services.iter().find(|s| s.name == name)
    }
    pub fn get_stub(&self, name: &str) -> Option<&Stub> {
        self.stubs.iter().find(|s| s.name == name)
    }
    pub fn get_agent(&self, name: &str) -> Option<&Agent> {
        self.agents.iter().find(|a| a.name == name)
    }
    pub fn contains(&self, name: &str) -> bool {
        self.get_service(name).is_some()
            || self.get_stub(name).is_some()
            || self.get_agent(name).is_some()
    }
}

fn default_log_buffer_size() -> usize {
    10_000
}

fn default_log_level() -> LevelFilter {
    LevelFilter::Info
}

#[derive(Debug)]
pub struct ConfigManager {
    pub file_path: PathBuf,
    config: ProcliConfig,
}

impl ConfigManager {
    pub fn new(file_path: PathBuf) -> Result<ConfigManager> {
        info!(target: "Config", "Watching file {:?}", file_path);
        Ok(ConfigManager {
            file_path: file_path.clone(),
            config: Self::load_from_file(file_path.clone())?,
        })
    }
    pub fn watch(&self) -> color_eyre::Result<()> {
        let mut watcher = notify::recommended_watcher(move |_| {
            // TODO:
            // let _ = sender.send(Event::App(AppEvent::Reload));
        })?;
        watcher.watch(&self.file_path, notify::RecursiveMode::NonRecursive)?;
        Ok(())
    }

    pub fn current(&self) -> ProcliConfig {
        self.config.clone()
    }

    pub fn reload(&mut self) -> Result<ProcliConfig> {
        self.config = Self::load_from_file(self.file_path.clone())?;
        Ok(self.current())
    }

    fn load_from_file(file_path: PathBuf) -> Result<ProcliConfig> {
        let raw = std::fs::read_to_string(file_path)?;
        let t = toml::from_str(&raw)?;
        Ok(t)
    }
}
