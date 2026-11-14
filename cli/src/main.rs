//! `tpcloud`: one file that installs a TPCloud server, starts and stops it, shows how it's doing
//! and turns modules on and off. Called as `tphome` (a link made while Home is on), it controls
//! the house.

mod docker;
mod envfile;
mod home;
mod install;
mod paths;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use docker::Project;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "tpcloud", version, about = "Install, run and configure a TPCloud server.")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// What's running, where it's reached and which modules are on (the default).
    Status,
    /// Install (or update) this server from this file: needs sudo.
    Install {
        /// Take over a server that ran from tpcloud and tphome clones in this folder (copies their
        /// settings and data, leaves the clones as they were).
        #[arg(long, value_name = "FOLDER")]
        adopt: Option<PathBuf>,
        /// Only install; don't start anything.
        #[arg(long)]
        no_start: bool,
    },
    /// Build what changed and start everything.
    Start,
    /// Stop everything (data is kept).
    Stop,
    /// Stop and start again.
    Restart,
    /// Show a service's log (gateway, auth, store-api, home-api, mqtt-broker, cloudflared).
    Logs {
        /// The containers to show; every one when left out.
        services: Vec<String>,
        /// Keep following it.
        #[arg(short, long)]
        follow: bool,
        /// How many lines from the end.
        #[arg(short = 'n', long, default_value = "50")]
        lines: String,
    },
    /// Turn modules on and off.
    Module {
        #[command(subcommand)]
        action: ModuleAction,
    },
    /// Control the house (the same as the `tphome` command).
    #[command(disable_help_flag = true)]
    Home {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand)]
enum ModuleAction {
    /// Which modules there are and which are on.
    List,
    /// Turn a module on (store, home): needs sudo.
    Enable { module: String },
    /// Turn a module off, keeping its data: needs sudo.
    Disable { module: String },
}

fn main() {
    if let Err(err) = run() {
        ui::warn(&format!("{err:#}"));
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args: Vec<String> = std::env::args().collect();
    let called_as = args
        .first()
        .and_then(|a| std::path::Path::new(a).file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_default();
    if called_as == "tphome" {
        return home::run(args);
    }
    let cli = Cli::parse_from(args.clone());
    match cli.command.unwrap_or(Command::Status) {
        Command::Status => status(),
        Command::Install { adopt, no_start } => install::install(adopt, !no_start),
        Command::Start => start(),
        Command::Stop => stop(),
        Command::Restart => {
            stop()?;
            start()
        }
        Command::Logs { services, follow, lines } => {
            let names = if services.is_empty() { all_container_names()? } else { services };
            docker::logs(&names, follow, &lines)
        }
        Command::Module { action } => match action {
            ModuleAction::List => module_list(),
            ModuleAction::Enable { module } => install::set_module(&module, true),
            ModuleAction::Disable { module } => install::set_module(&module, false),
        },
        Command::Home { args: rest } => {
            args = std::iter::once("tphome".to_string()).chain(rest).collect();
            home::run(args)
        }
    }
}

pub fn start() -> Result<()> {
    ui::step("Starting TPCloud (images are built here; the first time takes a while)");
    docker::compose(Project::Cloud, &["up", "-d", "--build", "--remove-orphans"])?;
    if install::home_enabled() {
        ui::step("Starting Home");
        docker::compose(Project::Home, &["up", "-d", "--build", "--remove-orphans"])?;
    }
    ui::done("Running.");
    Ok(())
}

fn stop() -> Result<()> {
    // Home first: its API sits on tpcloud's network, which goes away with tpcloud.
    if install::home_enabled() {
        ui::step("Stopping Home");
        docker::compose(Project::Home, &["down"])?;
    }
    ui::step("Stopping TPCloud");
    docker::compose(Project::Cloud, &["down"])?;
    ui::done("Stopped. Data is kept.");
    Ok(())
}

fn all_container_names() -> Result<Vec<String>> {
    let mut names: Vec<String> = docker::containers(Project::Cloud)?.into_iter().map(|c| c.name).collect();
    names.extend(docker::containers(Project::Home)?.into_iter().map(|c| c.name));
    Ok(names)
}

fn status() -> Result<()> {
    let env = paths::env_file();
    if !env.exists() {
        println!("TPCloud isn't installed here yet: `sudo tpcloud install`.");
        return Ok(());
    }
    let url = envfile::get(&env, "TPCLOUD_URL").unwrap_or_else(|| "(no address set)".into());
    println!("{} {}  {}", ui::bold("TPCloud"), env!("CARGO_PKG_VERSION"), ui::accent(&url));
    let modules = install::modules();
    println!("{}  {}", ui::dim("Modules"), if modules.is_empty() { "none".into() } else { modules.join(", ") });
    if let Some(path) = envfile::get(&env, "EXTERNAL_DISK_PATH") {
        println!("{}  {path}", ui::dim("Files  "));
    }
    println!("{}  {}", ui::dim("Data   "), paths::state_dir().display());
    println!();
    let mut any = false;
    for project in [Project::Cloud, Project::Home] {
        for c in docker::containers(project)? {
            any = true;
            let dot = if c.state == "running" { ui::green("●") } else { ui::dim("○") };
            println!("  {dot} {:<12} {}", c.name, ui::dim(&c.status));
        }
    }
    if !any {
        println!("  Nothing is running: `sudo tpcloud start`.");
    }
    Ok(())
}

fn module_list() -> Result<()> {
    let on = install::modules();
    for (name, what) in [("store", "files, photo backup and share links"), ("home", "blinds and lights, with tphome's switches")] {
        let enabled = on.iter().any(|m| m == name);
        println!(
            "  {} {:<6} {}",
            if enabled { ui::green("●") } else { ui::dim("○") },
            name,
            ui::dim(what)
        );
    }
    Ok(())
}
