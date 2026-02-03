//! Provides a ConfigManager to read and refresh config from files.
//!

use color_eyre::Result;
use config;
use log::*;
use notify::{RecommendedWatcher, Watcher};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use tokio::sync::mpsc::UnboundedSender;

use crate::event::{AppEvent, Event};

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
    pub restart: RestartPolicy,
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
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub name: String,
    pub display: Option<String>,
    pub scenario: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProcliConfig {
    #[serde(default)]
    pub services: Vec<Service>,
    #[serde(default)]
    pub stubs: Vec<Stub>,
    #[serde(default)]
    pub agents: Vec<Agent>,
    #[serde(default = "default_log_buffer_size")]
    pub log_buffer_size: usize,
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

#[derive(Debug)]
pub struct ConfigManager {
    pub file_path: PathBuf,
    config: ProcliConfig,
    _watcher: RecommendedWatcher,
}

impl ConfigManager {
    pub fn new(file_path: PathBuf, sender: UnboundedSender<Event>) -> Result<ConfigManager> {
        let mut watcher = notify::recommended_watcher(move |_| {
            let _ = sender.send(Event::App(AppEvent::Reload));
        })?;
        info!(target: "Config", "Watching file {:?}", file_path);
        watcher.watch(&file_path, notify::RecursiveMode::NonRecursive)?;
        Ok(ConfigManager {
            file_path: file_path.clone(),
            config: Self::load_from_file(file_path.clone())?,
            _watcher: watcher,
        })
    }

    pub fn current(&self) -> ProcliConfig {
        self.config.clone()
    }

    pub fn reload(&mut self) -> Result<ProcliConfig> {
        self.config = Self::load_from_file(self.file_path.clone())?;
        Ok(self.current())
    }

    fn load_from_file(file_path: PathBuf) -> Result<ProcliConfig> {
        let raw = config::Config::builder()
            .add_source(config::File::from(file_path))
            .add_source(config::Environment::with_prefix("PROCLI_"))
            .build()?;
        Ok(raw.try_deserialize()?)
    }
}
