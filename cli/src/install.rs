//! Installing from the embedded bundle, adopting a server set up from git clones, and turning
//! modules on and off.

use crate::docker::{self, Project};
use crate::{envfile, paths, ui};
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// tpcloud's compose, services and web apps plus tphome's API, packed at build time
/// (cli/build-bundle.sh).
static BUNDLE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/bundle.tar.gz"));

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

/// The full install (or update, run again): binary, code, state folder, then start.
pub fn install(adopt: Option<PathBuf>, start: bool) -> Result<()> {
    require_root("Installing")?;
    let state = paths::state_dir();

    ui::step("Installing the tpcloud command");
    install_binary()?;

    ui::step(&format!("Unpacking TPCloud {} into {}", env!("CARGO_PKG_VERSION"), paths::app_dir().display()));
    unpack_bundle()?;

    ui::step(&format!("Preparing {}", state.display()));
    for dir in ["data", "gateway-extra", "home/db", "home/mosquitto"] {
        std::fs::create_dir_all(state.join(dir))?;
    }

    if let Some(from) = adopt {
        if paths::env_file().exists() {
            bail!("{} already has settings; adopting would overwrite them", state.display());
        }
        prebuild(&from)?;
        adopt_clones(&from)?;
    }

    if !paths::env_file().exists() {
        std::fs::copy(paths::app_dir().join("tpcloud/.env.example"), paths::env_file())?;
        restrict(&paths::env_file())?;
        ui::warn(&format!(
            "No settings yet: edit {} (address, storage folder, modules) and run `sudo tpcloud start`.",
            paths::env_file().display()
        ));
        sync_links()?;
        return Ok(());
    }
    restrict(&paths::env_file())?;
    if home_enabled() && !paths::home_env_file().exists() {
        std::fs::copy(paths::app_dir().join("tphome/.env.example"), paths::home_env_file())?;
        restrict(&paths::home_env_file())?;
    }
    sync_links()?;

    if start {
        crate::start()?;
    }
    ui::done("TPCloud is installed.");
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

fn unpack_bundle() -> Result<()> {
    let app = paths::app_dir();
    std::fs::create_dir_all(&app)?;
    for project in ["tpcloud", "tphome"] {
        let dir = app.join(project);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).with_context(|| format!("removing the old {}", dir.display()))?;
        }
    }
    let decoder = flate2::read::GzDecoder::new(BUNDLE);
    tar::Archive::new(decoder).unpack(&app).context("unpacking the bundle")?;
    std::fs::write(app.join("VERSION"), format!("{}\n", env!("CARGO_PKG_VERSION")))?;
    Ok(())
}

/// Builds every image before anything is stopped, from the clones' own settings: on a Raspberry
/// Pi that takes minutes, which would otherwise all be downtime.
fn prebuild(from: &Path) -> Result<()> {
    let cloud_env = from.join("tpcloud/.env");
    let home_env = from.join("tphome/.env");
    if !cloud_env.exists() {
        bail!("{} has no tpcloud/.env to adopt", from.display());
    }
    ui::step("Building the images while the old containers keep running");
    docker::compose_with_env(Project::Cloud, &cloud_env, &home_env, &["build"])?;
    let home_on = envfile::get_list(&cloud_env, "TPCLOUD_MODULES").iter().any(|m| m == "home");
    if home_on && home_env.exists() {
        docker::compose_with_env(Project::Home, &cloud_env, &home_env, &["build"])?;
    }
    Ok(())
}

/// Takes over a server that ran from clones of tpcloud and tphome side by side in `from` (the
/// layout before this command existed). Its containers are stopped first so the databases are
/// copied whole; the clones are left as they were, as a way back.
fn adopt_clones(from: &Path) -> Result<()> {
    let cloud = from.join("tpcloud");
    let home = from.join("tphome");
    if !cloud.join(".env").exists() {
        bail!("{} has no tpcloud/.env to adopt", from.display());
    }
    let state = paths::state_dir();

    ui::step("Stopping the running containers so their data is copied whole");
    for project in [Project::Home, Project::Cloud] {
        let ids: Vec<String> = docker::containers(project)?.into_iter().map(|c| c.name).collect();
        if !ids.is_empty() {
            run("docker", &[&["stop"][..], &ids.iter().map(String::as_str).collect::<Vec<_>>()].concat())?;
        }
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
    Ok(())
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

/// Settings readable by root and the docker group (who run tpcloud's commands), nobody else.
fn restrict(path: &Path) -> Result<()> {
    run("chmod", &["640", &path.to_string_lossy()])?;
    let _ = Command::new("chgrp").args(["docker", &path.to_string_lossy()]).status();
    Ok(())
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
    let has = list.iter().any(|m| m == module);
    if on == has {
        ui::done(&format!("{module} is already {}.", if on { "on" } else { "off" }));
        return Ok(());
    }
    if on {
        list.push(module.to_string());
    } else {
        list.retain(|m| m != module);
    }
    envfile::set_list(&env, "TPCLOUD_MODULES", &list)?;
    // Store's API runs behind a compose profile; Home is its own compose project.
    if module == "store" {
        let mut profiles = envfile::get_list(&env, "COMPOSE_PROFILES");
        profiles.retain(|p| p != "store");
        if on {
            profiles.push("store".into());
        }
        envfile::set_list(&env, "COMPOSE_PROFILES", &profiles)?;
    }
    if module == "home" && on && !paths::home_env_file().exists() {
        std::fs::copy(paths::app_dir().join("tphome/.env.example"), paths::home_env_file())?;
        restrict(&paths::home_env_file())?;
        ui::warn(&format!("Home's settings are new: check MQTT_IP in {}.", paths::home_env_file().display()));
    }
    sync_links()?;

    match (module, on) {
        ("home", true) => {
            docker::compose(Project::Home, &["up", "-d", "--build", "--remove-orphans"])?;
        }
        ("home", false) => {
            docker::compose(Project::Home, &["down"])?;
        }
        ("store", false) => {
            docker::compose(Project::Cloud, &["rm", "--stop", "--force", "store-api"])?;
        }
        _ => {}
    }
    // auth reads TPCLOUD_MODULES (the dashboard and the app show only those); recreating picks it up.
    docker::compose(Project::Cloud, &["up", "-d", "--remove-orphans"])?;
    ui::done(&format!(
        "{module} is {}.{}",
        if on { "on" } else { "off" },
        if module == "home" && on { " The `tphome` command is there now." } else { "" }
    ));
    Ok(())
}
