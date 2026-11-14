//! Running Docker Compose on the installed projects, with their data and settings pointed at the
//! state folder, and reading what's running.

use crate::paths;
use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Project {
    /// gateway (with every web app), auth, store-api, cloudflared.
    Cloud,
    /// tphome's home-api and mqtt-broker.
    Home,
}

impl Project {
    pub fn name(self) -> &'static str {
        match self {
            Project::Cloud => "tpcloud",
            Project::Home => "tphome",
        }
    }
}

/// `docker compose <args>` for one project, as installed under the app and state folders.
pub fn compose(project: Project, args: &[&str]) -> Result<()> {
    compose_with_env(project, &paths::env_file(), &paths::home_env_file(), args)
}

/// The same with other settings files: building an adopted server's images from its clones'
/// settings while its old containers keep running.
pub fn compose_with_env(project: Project, cloud_env: &Path, home_env: &Path, args: &[&str]) -> Result<()> {
    let app = paths::app_dir().join(project.name());
    let state = paths::state_dir();
    let status = Command::new("docker")
        .arg("compose")
        .arg("--project-directory")
        .arg(&app)
        .arg("-f")
        .arg(app.join("docker-compose.yml"))
        .arg("--env-file")
        .arg(if project == Project::Cloud { cloud_env } else { home_env })
        .args(args)
        // Read by the compose files instead of their ./data, .env and extra/ defaults.
        .env("TPCLOUD_DATA", state.join("data"))
        .env("TPCLOUD_ENV_FILE", cloud_env)
        .env("TPCLOUD_GATEWAY_EXTRA", state.join("gateway-extra"))
        .env("TPHOME_DB", state.join("home").join("db"))
        .env("TPHOME_MQTT_DATA", state.join("home").join("mosquitto"))
        .env("TPHOME_ENV_FILE", home_env)
        .status()
        .context("couldn't run docker (is it installed, and can this user use it?)")?;
    if !status.success() {
        bail!("docker compose {} failed for {}", args.join(" "), project.name());
    }
    Ok(())
}

pub struct Container {
    pub name: String,
    pub state: String,
    pub status: String,
}

/// Every container of a project, running or not.
pub fn containers(project: Project) -> Result<Vec<Container>> {
    let out = Command::new("docker")
        .args(["ps", "-a", "--filter"])
        .arg(format!("label=com.docker.compose.project={}", project.name()))
        .args(["--format", "{{.Names}}\t{{.State}}\t{{.Status}}"])
        .stderr(Stdio::inherit())
        .output()
        .context("couldn't run docker")?;
    if !out.status.success() {
        bail!("docker ps failed (can this user use docker?)");
    }
    let mut list: Vec<Container> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '\t');
            Some(Container {
                name: parts.next()?.to_string(),
                state: parts.next()?.to_string(),
                status: parts.next()?.to_string(),
            })
        })
        .collect();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(list)
}

/// The address a container answers on, on its Docker network (reachable from this machine).
pub fn container_ip(name: &str) -> Result<String> {
    let out = Command::new("docker")
        .args(["inspect", "-f", "{{range .NetworkSettings.Networks}}{{.IPAddress}} {{end}}", name])
        .output()
        .context("couldn't run docker")?;
    if !out.status.success() {
        bail!("{name} isn't running");
    }
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .map(str::to_string)
        .with_context(|| format!("{name} has no address"))
}

/// `docker logs` for one container (or several), passed straight through.
pub fn logs(names: &[String], follow: bool, tail: &str) -> Result<()> {
    if names.len() == 1 || follow {
        // `docker logs` takes one container; following several would need a multiplexer.
        let name = names.first().context("no container")?;
        let mut cmd = Command::new("docker");
        cmd.args(["logs", "--tail", tail]);
        if follow {
            cmd.arg("-f");
        }
        cmd.arg(name).status().context("couldn't run docker")?;
        return Ok(());
    }
    for name in names {
        use std::io::Write;
        println!("── {name}");
        std::io::stdout().flush()?;
        // A container's log comes out on both stdout and stderr; one stream keeps it under its header.
        Command::new("docker")
            .args(["logs", "--tail", tail, name])
            .stderr(std::io::stdout())
            .status()
            .context("couldn't run docker")?;
    }
    Ok(())
}
