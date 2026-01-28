use std::{
    path::{Path, absolute},
    process::Stdio,
};

use color_eyre::eyre::eyre;
use log::*;
use tokio::process::Command;

use tokio::io::{AsyncBufReadExt, BufReader};

use crate::config::Service;

#[derive(Debug)]
pub struct Process {
    name: String,
    display: String,
    cmd: Command,
    child: Option<tokio::process::Child>,
}

#[derive(Debug, Default)]
pub struct ProcessManager {
    processes: Vec<Process>,
}

impl ProcessManager {
    pub fn start(&mut self, svc: &Service) -> color_eyre::Result<&Process> {
        let mut cmd = svc.command_args()?;
        info!(target: &svc.name, "Spawning {:?}", cmd);
        cmd.stderr(Stdio::piped());
        cmd.stdout(Stdio::piped());
        let proc = self.processes.push_mut(Process {
            name: svc.name.clone(),
            display: svc.display.clone().unwrap_or(svc.name.clone()),
            cmd,
            child: None,
        });
        let mut child = proc.cmd.spawn()?;
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let name = svc.name.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Some(line) = reader.next_line().await.unwrap() {
                info!(target: &name, "{}", line);
            }
        });
        let name = svc.name.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Some(line) = reader.next_line().await.unwrap() {
                error!(target: &name, "{}", line);
            }
        });
        proc.child.replace(child);
        Ok(proc)
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
