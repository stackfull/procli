//! Provides a ConfigManager to read and refresh config from files.
//!

use color_eyre::Result;
use config;
use log::*;
use notify::{RecommendedWatcher, Watcher};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, time::Duration};
use tokio::sync::mpsc::UnboundedSender;

use crate::event::{AppEvent, Event};

pub const DEFAULT_FILE: &str = "prat.toml";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RestartPolicy {
    pub enabled: bool,
    pub backoff_min: Duration,
    pub backoff_max: Duration,
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
pub struct PratConfig {
    #[serde(default)]
    pub services: Vec<Service>,
    #[serde(default)]
    pub stubs: Vec<Stub>,
    #[serde(default)]
    pub agents: Vec<Agent>,
    #[serde(default = "default_log_buffer_size")]
    pub log_buffer_size: usize,
}

fn default_log_buffer_size() -> usize {
    10_000
}

#[derive(Debug)]
pub struct ConfigManager {
    pub file_path: PathBuf,
    config: PratConfig,
    _watcher: RecommendedWatcher,
}

impl ConfigManager {
    pub fn new(file_path: PathBuf, sender: UnboundedSender<Event>) -> Result<ConfigManager> {
        let captured = sender.clone();
        let mut watcher = notify::recommended_watcher(move |_| {
            let _ = captured.send(Event::App(AppEvent::Reload));
        })?;
        info!(target: "Config", "Watching file {:?}", file_path);
        watcher.watch(&file_path, notify::RecursiveMode::NonRecursive)?;
        Ok(ConfigManager {
            file_path: file_path.clone(),
            config: Self::load_from_file(file_path.clone())?,
            _watcher: watcher,
        })
    }

    pub fn current(&self) -> PratConfig {
        self.config.clone()
    }

    pub fn reload(&mut self) -> Result<PratConfig> {
        self.config = Self::load_from_file(self.file_path.clone())?;
        Ok(self.current())
    }

    fn load_from_file(file_path: PathBuf) -> Result<PratConfig> {
        let raw = config::Config::builder()
            .add_source(config::File::from(file_path))
            .add_source(config::Environment::with_prefix("PRAT_"))
            .build()?;
        Ok(raw.try_deserialize()?)
    }
}
