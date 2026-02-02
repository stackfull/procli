use std::{
    path::absolute,
    process::{ExitStatus, Stdio},
    time::{self, Duration, Instant},
};

use color_eyre::eyre::eyre;
use log::*;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{ChildStderr, ChildStdout, Command},
    select,
    sync::{mpsc::UnboundedSender, oneshot},
    time::sleep,
};
use uuid::Uuid;

use crate::{
    config::{RestartPolicy, Service},
    event::{AppEvent, Event},
};

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
    cmd: Command,
    closer: Option<oneshot::Receiver<()>>,
    pub state: ProcessState,
    pub restarts: u32,
    pub restart_policy: RestartPolicy,
    pub pid: Option<Pid>,
    pub stats: Vec<ProcessStats>,
    pub stats_max: ProcessStats,
}

impl Process {
    pub fn kill(&mut self) {
        drop(self.closer.take());
    }

    fn push_stats(&mut self, stats: ProcessStats) {
        self.stats.push(stats);
        self.stats_max.cpu_percent = self.stats_max.cpu_percent.max(stats.cpu_percent);
        self.stats_max.memory_mb = self.stats_max.memory_mb.max(stats.memory_mb);
        self.stats_max.uptime = self.stats_max.uptime.max(stats.uptime);
        self.stats_max.timestamp = stats.timestamp;
    }
}
#[derive(Debug)]
pub struct ProcessManager {
    pub processes: Vec<Process>,
    sender: UnboundedSender<Event>,
    sys: sysinfo::System,
}

#[derive(Debug, Clone, Copy)]
pub struct ProcessStats {
    pub timestamp: Instant,
    pub cpu_percent: f32,
    pub memory_mb: f32,
    pub uptime: Duration,
}

impl Default for ProcessStats {
    fn default() -> Self {
        Self {
            timestamp: Instant::now(),
            cpu_percent: 0.0,
            memory_mb: 0.0,
            uptime: Duration::ZERO,
        }
    }
}

impl ProcessManager {
    pub fn new(sender: UnboundedSender<Event>) -> Self {
        let ticker = sender.clone();
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(2)).await;
                ticker
                    .send(Event::App(AppEvent::StatsRefresh))
                    .expect("sending process died message");
            }
        });
        Self {
            processes: vec![],
            sender,
            sys: System::new(),
        }
    }

    /// Refresh the sysinfo stats.
    fn refresh_stats(&mut self) {
        let pids: Vec<Pid> = self.processes.iter().filter_map(|p| p.pid).collect();
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&pids),
            true,
            ProcessRefreshKind::everything(),
        );
    }

    /// Distribute the most recent stats to the `Process` objects
    fn assign_stats(&mut self) {
        let proc_infos = self.sys.processes();
        for proc in self.processes.iter_mut().filter(|p| p.pid.is_some()) {
            if let Some(info) = proc_infos.get(&proc.pid.unwrap()) {
                proc.push_stats(ProcessStats {
                    timestamp: Instant::now(),
                    cpu_percent: info.cpu_usage(),
                    memory_mb: info.memory() as f32 / 1_000_000.0,
                    uptime: Duration::from_secs(info.run_time()),
                });
                proc.state = ProcessState::Running;
            }
        }
    }

    fn check_restarts(&mut self) {
        let now = Instant::now();
        let mut names: Vec<String> = Vec::new();
        for proc in self.processes.iter_mut() {
            if let ProcessState::Stopped(ProcessRestart::RestartAt(t), _) = &proc.state {
                if *t > now {
                    continue;
                }
                names.push(proc.name.clone());
                proc.restarts += 1;
            }
        }
        for name in names {
            info!(target: &name, "Restarting process");
            if let Err(err) = self.spawn(&name) {
                error!("Failed to restart process {}: {}", name, err);
            }
        }
    }

    /// Spawn an actual process for the given state.
    ///
    /// Each process gets a new UUID (PID is less reliable) and output pumping
    /// tasks as well as death handler etc.
    ///
    fn spawn(&mut self, name: &str) -> color_eyre::Result<Uuid> {
        let proc = self
            .processes
            .iter_mut()
            .find(|p| p.name == name)
            .ok_or(eyre!("No such process"))?;

        let uuid = Uuid::new_v4();
        proc.uuid = uuid;
        info!(target: name, "Spawning process {} for {}", uuid, name);

        let mut child = proc.cmd.spawn()?;
        proc.pid = child.id().map(Pid::from_u32);

        let stdout = child.stdout.take().unwrap();
        tokio::spawn(stdout_log_pump(name.to_string(), stdout));
        let stderr = child.stderr.take().unwrap();
        tokio::spawn(stderr_log_pump(name.to_string(), stderr));

        let (closed, closer) = oneshot::channel();
        proc.closer = Some(closer);
        tokio::spawn(death_handler(
            name.to_string(),
            uuid,
            closed,
            self.sender.clone(),
            child,
        ));

        self.refresh_stats();
        Ok(uuid)
    }

    /// Try to call this less frequently than once a second.
    pub fn tick(&mut self) {
        debug!("ProcessManager tick");
        self.refresh_stats();
        self.assign_stats();
        self.check_restarts();
    }

    /// Define a new process for the given service.
    ///
    /// If a process with the same name is already running, it is only restarted
    /// if the config has changed. If it is in a restart cooloff period, it is
    /// started immediately.
    ///
    pub fn upsert(&mut self, svc: &Service) -> color_eyre::Result<Uuid> {
        // TODO: check if already running

        let mut cmd = svc.command_args()?;
        cmd.stderr(Stdio::piped());
        cmd.stdout(Stdio::piped());

        self.processes.push(Process {
            name: svc.name.clone(),
            display: svc.display.clone().unwrap_or(svc.name.clone()),
            cmd,
            uuid: Uuid::nil(),
            state: ProcessState::Starting,
            restarts: 0,
            restart_policy: svc.restart,
            pid: None,
            stats: Vec::default(),
            stats_max: ProcessStats::default(),
            closer: None,
        });

        self.spawn(&svc.name)
    }

    pub fn process_died(&mut self, id: Uuid, status: ExitStatus) {
        if let Some(proc) = self.processes.iter_mut().find(|p| p.uuid == id) {
            if proc.restart_policy.enabled && proc.restarts < proc.restart_policy.max_restarts {
                let restart_at = Instant::now() + Duration::from_secs(proc.restart_policy.cooloff); //TODO: add jitter
                proc.state = ProcessState::Stopped(ProcessRestart::RestartAt(restart_at), status);
            } else {
                proc.state = ProcessState::Stopped(ProcessRestart::NoRestart, status);
            }
        } else {
            error!("Received process died for unknown process {}", id);
        }
    }

    pub fn remove(&mut self, name: &str) -> color_eyre::Result<()> {
        if let Some(proc) = self.processes.iter_mut().find(|p| p.name == name) {
            info!(target: name, "Killing process");
            proc.kill();
            Ok(())
        } else {
            Err(eyre!("No such process"))
        }
    }
}

impl Service {
    fn command_args(&self) -> color_eyre::Result<Command> {
        let dir = match self.directory.as_ref() {
            Some(d) => Some(absolute(d)?.into_os_string()),
            None => None,
        };

        let cmd = match self.image.as_ref() {
            Some(image) => {
                // Docker based:
                //  `docker run --rm -e K=V -w /opt/mounted -v <dir>:/opt/mounted <image> <command>`
                let mut c = Command::new("docker");
                c.args(["run", "--rm"]);
                // env vars
                for (k, v) in &self.environment {
                    c.arg("-e").arg(format!("{}={}", k, v));
                }
                // optional directory mount
                if let Some(d) = dir {
                    let mut mount = d;
                    mount.push("");
                    c.args(["-w", "/opt/mounted", "-v"]).arg(mount);
                }
                c.arg(image);
                // optional command
                if let Some(c2) = self.command.as_ref() {
                    let strings = shlex::split(c2).ok_or(eyre!("Bad command string"))?;
                    c.args(strings);
                }
                c
            }
            None => {
                // Local command:
                let command = self
                    .command
                    .as_ref()
                    .ok_or(eyre!("Must specify command if no image"))?;
                let strings = shlex::split(&command).ok_or(eyre!("Bad command string"))?;
                let program = strings
                    .first()
                    .ok_or(eyre!("Must specify command if no image"))?;
                let mut c = Command::new(program);
                c.args(strings.iter().skip(1));
                // Env vars
                for (k, v) in &self.environment {
                    c.env(k, v);
                }
                // Optional dir
                if let Some(d) = self.directory.as_ref() {
                    c.current_dir(d);
                }
                c
            }
        };

        Ok(cmd)
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
