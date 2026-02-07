use color_eyre::eyre::eyre;
use tokio::process::Command;

use crate::proc::process::ProcessConfig;

pub fn build_command<T>(from: &T) -> color_eyre::Result<Command>
where
    T: ProcessConfig,
{
    let cmd = match from.image() {
        Some(image) => {
            // Docker based:
            //  `docker run --rm -e K=V -w /opt/mounted -v <dir>:/opt/mounted <image> <command>`
            let mut c = Command::new("docker");
            c.args(["run", "--rm"]);
            // env vars
            for (k, v) in from.environment() {
                c.arg("-e").arg(format!("{}={}", k, v));
            }
            // optional directory mount
            if let Some(d) = from.directory()? {
                let mut mount = d;
                mount.push("");
                c.args(["-w", "/opt/mounted", "-v"]).arg(mount);
            }
            c.arg(image);
            // optional command
            if let Some(c2) = from.command() {
                let strings = shlex::split(&c2).ok_or(eyre!("Bad command string"))?;
                c.args(strings);
            }
            c
        }
        None => {
            // Local command:
            let command = from
                .command()
                .ok_or(eyre!("Must specify command if no image"))?;
            let strings = shlex::split(&command).ok_or(eyre!("Bad command string"))?;
            let program = strings
                .first()
                .ok_or(eyre!("Must specify command if no image"))?;
            let mut c = Command::new(program);
            c.args(strings.iter().skip(1));
            // Env vars
            for (k, v) in &from.environment() {
                c.env(k, v);
            }
            // Optional dir
            if let Some(d) = from.directory()? {
                c.current_dir(d);
            }
            c
        }
    };
    Ok(cmd)
}
