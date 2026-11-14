//! Installing from this file (runtime, state folder, systemd service), adopting a server that ran
//! from git clones with Docker, and turning modules on and off.

use crate::{envfile, paths, runtime, setup, ui};
use anyhow::{bail, Context, Result};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn require_root(what: &str) -> Result<()> {
    if unsafe { libc::geteuid() } != 0 {
        bail!("{what} needs root: run it with sudo");
    }
    Ok(())
}

pub fn modules() -> Vec<String> {
    envfile::get_list(&paths::env_file(), "TPCLOUD_MODULES")
}

pub fn home_enabled() -> bool {
    modules().iter().any(|m| m == "home")
}

pub fn install(adopt: Option<PathBuf>, start: bool) -> Result<()> {
    require_root("Installing")?;
    let state = paths::state_dir();

    ui::step("Installing the tpcloud command");
    install_binary()?;
    ui::step(&format!("Unpacking TPCloud {} into {}", env!("CARGO_PKG_VERSION"), paths::app_dir().display()));
    runtime::unpack()?;
    ui::step(&format!("Preparing {}", state.display()));
    ensure_group();
    for dir in ["data/auth", "data/store", "gateway-extra", "home/db", "home/mosquitto", "caddy", "logs", "run"] {
        std::fs::create_dir_all(state.join(dir))?;
    }

    if let Some(from) = adopt {
        if paths::env_file().exists() {
            bail!("{} already has settings; adopting would overwrite them", state.display());
        }
        adopt_clones(&from)?;
    }

    if !paths::env_file().exists() {
        if std::io::stdin().is_terminal() {
            setup::run()?;
        } else {
            std::fs::write(paths::env_file(), setup::default_env())?;
            restrict(&paths::env_file())?;
            ui::warn(&format!("No settings yet: run `sudo tpcloud setup`, or edit {}.", paths::env_file().display()));
            sync_links()?;
            write_unit()?;
            return Ok(());
        }
    }
    restrict(&paths::env_file())?;
    if home_enabled() && !paths::home_env_file().exists() {
        std::fs::write(paths::home_env_file(), setup::default_home_env())?;
        restrict(&paths::home_env_file())?;
    }
    sync_links()?;
    write_unit()?;
    if start {
        ui::step("Starting TPCloud");
        systemctl(&["enable", "tpcloud"])?;
        systemctl(&["restart", "tpcloud"])?;
    }
    ui::done("TPCloud is installed. `tpcloud` opens it.");
    Ok(())
}

fn install_binary() -> Result<()> {
    let me = std::env::current_exe().context("can't find this program's own path")?;
    let target = paths::bin_dir().join("tpcloud");
    std::fs::create_dir_all(paths::bin_dir())?;
    if me.canonicalize().ok() == target.canonicalize().ok() {
        return Ok(());
    }
    // Copied next to the target and renamed over it, so a running `tpcloud` is never half written.
    let tmp = target.with_extension("new");
    std::fs::copy(&me, &tmp).with_context(|| format!("copying to {}", tmp.display()))?;
    std::fs::rename(&tmp, &target)?;
    Ok(())
}

/// systemd runs `tpcloud serve`, which runs everything else.
fn write_unit() -> Result<()> {
    let unit = format!(
        "[Unit]\nDescription=TPCloud\nAfter=network-online.target\nWants=network-online.target\n\n\
         [Service]\nExecStart={} serve\nRestart=always\nRestartSec=3\n\
         # SIGTERM to tpcloud, which stops its processes in order; anything left is killed after the timeout.\n\
         KillMode=mixed\nTimeoutStopSec=30\nLimitNOFILE=65536\n\n\
         [Install]\nWantedBy=multi-user.target\n",
        paths::bin_dir().join("tpcloud").display()
    );
    std::fs::write(paths::systemd_unit(), unit)?;
    systemctl(&["daemon-reload"])
}

pub fn systemctl(args: &[&str]) -> Result<()> {
    let status = Command::new("systemctl").args(args).status().context("couldn't run systemctl")?;
    if !status.success() {
        bail!("systemctl {} failed", args.join(" "));
    }
    Ok(())
}

/// Takes over a server that ran from clones of tpcloud and tphome side by side in `from`, with
/// Docker. Its containers are stopped so the databases are copied whole, then removed so they
/// can't come back on the next boot; the clones are left as they were, as a way back.
fn adopt_clones(from: &Path) -> Result<()> {
    let cloud = from.join("tpcloud");
    let home = from.join("tphome");
    if !cloud.join(".env").exists() {
        bail!("{} has no tpcloud/.env to adopt", from.display());
    }
    let state = paths::state_dir();
    let containers = docker_containers();
    if !containers.is_empty() {
        ui::step(&format!("Stopping the old containers ({})", containers.join(", ")));
        run("docker", &[&["stop"][..], &containers.iter().map(String::as_str).collect::<Vec<_>>()].concat())?;
    }

    ui::step(&format!("Copying settings and data from {}", from.display()));
    copy_file(&cloud.join(".env"), &paths::env_file())?;
    copy_dir_contents(&cloud.join("data"), &state.join("data"))?;
    let extra = cloud.join("services/gateway/extra");
    if extra.is_dir() {
        for entry in std::fs::read_dir(&extra)? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "caddy") {
                copy_file(&path, &state.join("gateway-extra").join(path.file_name().unwrap()))?;
            }
        }
    }
    if home.join(".env").exists() {
        copy_file(&home.join(".env"), &paths::home_env_file())?;
        copy_dir_contents(&home.join("db"), &state.join("home/db"))?;
        copy_dir_contents(&home.join("api/mosquitto/data"), &state.join("home/mosquitto"))?;
    }
    if !containers.is_empty() {
        ui::step("Removing the old containers (their data is in the clones and copied here)");
        run("docker", &[&["rm"][..], &containers.iter().map(String::as_str).collect::<Vec<_>>()].concat())?;
    }
    Ok(())
}

/// The containers of the old compose projects, if Docker is there at all.
fn docker_containers() -> Vec<String> {
    let mut names = Vec::new();
    for project in ["tphome", "tpcloud"] {
        let Ok(out) = Command::new("docker")
            .args(["ps", "-a", "--format", "{{.Names}}", "--filter"])
            .arg(format!("label=com.docker.compose.project={project}"))
            .output()
        else {
            return names;
        };
        names.extend(String::from_utf8_lossy(&out.stdout).lines().map(str::to_string));
    }
    names
}

fn copy_file(from: &Path, to: &Path) -> Result<()> {
    run("cp", &["-a", &from.to_string_lossy(), &to.to_string_lossy()])
}

fn copy_dir_contents(from: &Path, to: &Path) -> Result<()> {
    if !from.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(to)?;
    run("cp", &["-a", &format!("{}/.", from.display()), &to.to_string_lossy()])
}

/// Settings readable by root and the `tpcloud` group (who use `tpcloud status`, `tphome`…), nobody
/// else: they hold the tunnel's token.
pub fn restrict(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o640))?;
    let _ = Command::new("chgrp").args(["tpcloud", &path.to_string_lossy()]).status();
    Ok(())
}

/// The `tpcloud` group, with whoever ran the install through sudo in it (from their next login).
fn ensure_group() {
    let _ = Command::new("groupadd").args(["-f", "tpcloud"]).status();
    if let Ok(user) = std::env::var("SUDO_USER") {
        if !user.is_empty() && user != "root" {
            let _ = Command::new("usermod").args(["-aG", "tpcloud", &user]).status();
        }
    }
}

fn run(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program).args(args).status().with_context(|| format!("couldn't run {program}"))?;
    if !status.success() {
        bail!("{program} {} failed", args.join(" "));
    }
    Ok(())
}

/// `tphome` exists exactly while the home module is on: a link to this same program.
pub fn sync_links() -> Result<()> {
    let link = paths::bin_dir().join("tphome");
    let exists = link.symlink_metadata().is_ok();
    if home_enabled() && !exists {
        std::os::unix::fs::symlink("tpcloud", &link).with_context(|| format!("creating {}", link.display()))?;
    } else if !home_enabled() && exists {
        std::fs::remove_file(&link)?;
    }
    Ok(())
}

pub fn set_module(module: &str, on: bool) -> Result<()> {
    require_root("Changing modules")?;
    if module != "home" && module != "store" {
        bail!("unknown module {module} (there's store and home)");
    }
    let env = paths::env_file();
    let mut list = modules();
    if on == list.iter().any(|m| m == module) {
        ui::done(&format!("{module} is already {}.", if on { "on" } else { "off" }));
        return Ok(());
    }
    if on {
        list.push(module.to_string());
    } else {
        list.retain(|m| m != module);
    }
    envfile::set_list(&env, "TPCLOUD_MODULES", &list)?;
    if module == "store" {
        // The compose files' profile, still read for Store (and the tunnel).
        let mut profiles = envfile::get_list(&env, "COMPOSE_PROFILES");
        profiles.retain(|p| p != "store");
        if on {
            profiles.push("store".into());
        }
        envfile::set_list(&env, "COMPOSE_PROFILES", &profiles)?;
    }
    if module == "home" && on && !paths::home_env_file().exists() {
        std::fs::write(paths::home_env_file(), setup::default_home_env())?;
        restrict(&paths::home_env_file())?;
    }
    sync_links()?;
    systemctl(&["restart", "tpcloud"])?;
    ui::done(&format!(
        "{module} is {}.{}",
        if on { "on" } else { "off" },
        if module == "home" && on { " The `tphome` command is there now." } else { "" }
    ));
    Ok(())
}
