use std::{
    process::ExitStatus,
    time::{Duration, Instant},
};

use color_eyre::eyre::{OptionExt, eyre};
use log::*;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::{sync::mpsc::UnboundedSender, time::sleep};
use uuid::Uuid;

use crate::{
    event::{AppEvent, Event},
    proc::{
        process::{Named, Process, ProcessConfig, ProcessRestart, ProcessState},
        stats::ProcessStats,
    },
};

#[derive(Debug)]
pub struct ProcessManager {
    pub processes: Vec<Process>,
    sender: UnboundedSender<Event>,
    sys: sysinfo::System,
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
        let timestamp = Instant::now();
        for proc in self.processes.iter_mut().filter(|p| p.pid.is_some()) {
            if let Some(info) = proc_infos.get(&proc.pid.unwrap()) {
                proc.push_stats(ProcessStats::new(timestamp, info));
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

    fn find(&mut self, name: &str) -> Option<&mut Process> {
        self.processes.iter_mut().find(|p| p.name == name)
    }

    /// Spawn an actual process for the given state.
    ///
    /// Each process gets a new UUID (PID is less reliable) and output pumping
    /// tasks as well as death handler etc.
    ///
    fn spawn(&mut self, name: &str) -> color_eyre::Result<Uuid> {
        let sender = self.sender.clone();
        let proc = self.find(name).ok_or(eyre!("No such process"))?;
        let uuid = proc.spawn(sender)?;
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
    pub fn upsert<T>(&mut self, svc: &T) -> color_eyre::Result<Uuid>
    where
        T: Named + ProcessConfig,
    {
        let name = svc.name();
        // TODO: check if already running
        // if let Some(existing) = self.find(&name) {
        //     match &existing.state {
        //         ProcessState::Starting => todo!(),
        //         ProcessState::Running => todo!(),
        //         ProcessState::Killing(process_restart) => todo!(),
        //         ProcessState::Stopped(process_restart, exit_status) => todo!(),
        //     }
        // }
        self.processes.push(Process::new(svc)?);
        self.spawn(&name)
    }

    pub fn process_died(&mut self, id: Uuid, status: ExitStatus) {
        if let Some(proc) = self.processes.iter_mut().find(|p| p.uuid == id) {
            let time_of_death = Instant::now();
            if proc.restart_policy.enabled && proc.restarts < proc.restart_policy.max_restarts {
                let restart_at = time_of_death + Duration::from_secs(proc.restart_policy.cooloff); //TODO: add jitter
                proc.state = ProcessState::Stopped(ProcessRestart::RestartAt(restart_at), status);
            } else {
                proc.state = ProcessState::Stopped(ProcessRestart::NoRestart, status);
            }
            proc.last_stop = Some(time_of_death);
        } else {
            error!("Received process died for unknown process {}", id);
        }
    }

    pub fn remove(&mut self, name: &str) -> color_eyre::Result<()> {
        let proc = self.find(name).ok_or_eyre("No such process")?;
        info!(target: name, "Killing process");
        proc.kill();
        Ok(())
    }
}
