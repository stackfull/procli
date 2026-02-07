use std::{
    collections::HashMap,
    ffi::OsString,
    path::absolute,
    process::{ExitStatus, Stdio},
    time::{self, Instant},
};

use color_eyre::eyre::Result;
use log::*;
use sysinfo::Pid;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{ChildStderr, ChildStdout, Command},
    select,
    sync::{mpsc::UnboundedSender, oneshot},
};
use uuid::Uuid;

use crate::{
    config::{RestartPolicy, Service, Stub},
    event::{AppEvent, Event},
    proc::{command::build_command, stats::ProcessStats},
};

pub trait Named {
    fn name(&self) -> String;
    fn display(&self) -> String;
}

pub trait ProcessConfig {
    fn image(&self) -> Option<String>;
    fn command(&self) -> Option<String>;
    fn directory(&self) -> Result<Option<OsString>>;
    fn environment(&self) -> HashMap<String, String>;
    fn restart_policy(&self) -> RestartPolicy;
}

impl Named for Service {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn display(&self) -> String {
        self.display.clone().unwrap_or(self.name.clone())
    }
}

impl Named for Stub {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn display(&self) -> String {
        self.display.clone().unwrap_or(self.name.clone())
    }
}

impl ProcessConfig for Service {
    fn image(&self) -> Option<String> {
        self.image.clone()
    }

    fn command(&self) -> Option<String> {
        self.command.clone()
    }

    fn directory(&self) -> Result<Option<OsString>> {
        let dir = match self.directory.as_ref() {
            Some(d) => Some(absolute(d)?.into_os_string()),
            None => None,
        };
        Ok(dir)
    }

    fn environment(&self) -> HashMap<String, String> {
        self.environment.clone()
    }
    fn restart_policy(&self) -> RestartPolicy {
        self.restart.unwrap_or_default()
    }
}

impl ProcessConfig for Stub {
    fn image(&self) -> Option<String> {
        self.image.clone()
    }

    fn command(&self) -> Option<String> {
        self.command.clone()
    }

    fn directory(&self) -> Result<Option<OsString>> {
        let dir = match self.directory.as_ref() {
            Some(d) => Some(absolute(d)?.into_os_string()),
            None => None,
        };
        Ok(dir)
    }

    fn environment(&self) -> HashMap<String, String> {
        self.environment.clone()
    }
    fn restart_policy(&self) -> RestartPolicy {
        self.restart.unwrap_or_default()
    }
}

#[derive(Debug)]
pub enum ProcessRestart {
    NoRestart,
    RestartAt(time::Instant),
}

#[derive(Debug)]
pub enum ProcessState {
    Starting,
    Running,
    Killing(ProcessRestart),
    Stopped(ProcessRestart, ExitStatus),
}

#[derive(Debug)]
pub struct Process {
    pub name: String,
    pub display: String,
    pub uuid: Uuid,
    pub cmd: Command,
    closer: Option<oneshot::Receiver<()>>,
    pub state: ProcessState,
    pub restarts: u32,
    pub restart_policy: RestartPolicy,
    pub pid: Option<Pid>,
    pub last_start: Option<Instant>,
    pub last_stop: Option<Instant>,
    pub stats: Vec<ProcessStats>,
    pub stats_max: ProcessStats,
}

impl Process {
    pub fn new<T>(svc: &T) -> color_eyre::Result<Process>
    where
        T: Named + ProcessConfig,
    {
        let mut cmd: Command = build_command(svc)?;
        cmd.stderr(Stdio::piped());
        cmd.stdout(Stdio::piped());
        Ok(Self {
            name: svc.name(),
            display: svc.display(),
            cmd,
            uuid: Uuid::nil(),
            state: ProcessState::Starting,
            restarts: 0,
            restart_policy: svc.restart_policy(),
            pid: None,
            last_start: None,
            last_stop: None,
            stats: Vec::default(),
            stats_max: ProcessStats::default(),
            closer: None,
        })
    }

    pub fn spawn(&mut self, sender: UnboundedSender<Event>) -> color_eyre::Result<Uuid> {
        let now = Instant::now();
        self.last_start = Some(now);
        let uuid = Uuid::new_v4();
        self.uuid = uuid;
        info!(target: &self.name, "Spawning process {} for {}", uuid, &self.name);

        let mut child = self.cmd.spawn()?;
        self.pid = child.id().map(Pid::from_u32);

        let stdout = child.stdout.take().unwrap();
        tokio::spawn(stdout_log_pump(self.name.to_string(), stdout));
        let stderr = child.stderr.take().unwrap();
        tokio::spawn(stderr_log_pump(self.name.to_string(), stderr));

        let (closed, closer) = oneshot::channel();
        self.closer = Some(closer);
        tokio::spawn(death_handler(
            self.name.to_string(),
            uuid,
            closed,
            sender,
            child,
        ));
        Ok(uuid)
    }

    pub fn kill(&mut self) {
        drop(self.closer.take());
    }

    pub fn push_stats(&mut self, stats: ProcessStats) {
        self.stats.push(stats);
        self.stats_max.cpu_percent = self.stats_max.cpu_percent.max(stats.cpu_percent);
        self.stats_max.memory_mb = self.stats_max.memory_mb.max(stats.memory_mb);
        self.stats_max.uptime = self.stats_max.uptime.max(stats.uptime);
        self.stats_max.timestamp = stats.timestamp;
        self.state = ProcessState::Running;
    }
}

async fn stdout_log_pump(name: String, stdout: ChildStdout) {
    let mut reader = BufReader::new(stdout).lines();
    while let Some(line) = reader.next_line().await.unwrap() {
        info!(target: &name, "{}", line);
    }
    debug!(target: &name, "Stdout reader exiting");
}

async fn stderr_log_pump(name: String, stderr: ChildStderr) {
    let mut reader = BufReader::new(stderr).lines();
    while let Some(line) = reader.next_line().await.unwrap() {
        info!(target: &name, "{}", line);
    }
    debug!(target: &name, "Stderr reader exiting");
}

async fn death_handler(
    name: String,
    uuid: Uuid,
    mut closed: oneshot::Sender<()>,
    sender: UnboundedSender<Event>,
    mut child: tokio::process::Child,
) {
    loop {
        select! {
            status = child.wait() => {
                info!(target: &name, "Process exit {:?}", status);
                sender.send(Event::App(AppEvent::ProcessDied(uuid, status.unwrap()))).expect("sending process died message");
                return;
            }
            _ = closed.closed() => {
                info!(target: &name, "Process kill...");
                if let Err(err) = child.start_kill() {
                    error!("Can't kill process {}: {}", name, err);
                }
            }
        }
    }
}
