//! `tpcloud`: one file, nothing else to install, that sets up a TPCloud server, runs it (as a
//! systemd service supervising its own bundled processes) and configures it. On its own it opens a
//! terminal app; called as `tphome` (a link made while Home is on), it controls the house.

mod backup;
mod cloudflare;
mod discovery;
mod engine;
mod envfile;
mod harden;
mod home;
mod install;
mod logs;
mod network;
mod paths;
mod platform;
mod runtime;
mod services;
mod setup;
mod status;
mod storage;
mod supervisor;
mod tui;
mod ui;
mod update;
mod users;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::io::IsTerminal;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "tpcloud", version, about = "Set up, run and configure a TPCloud server. On its own, opens a terminal app.")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// What's running, where it's reached and which modules are on.
    Status,
    /// Install (or update) this server from this file: needs sudo.
    Install {
        /// Take over a server that ran from tpcloud and tphome clones in this folder, with Docker
        /// (copies their settings and data, removes the old containers, leaves the clones).
        #[arg(long, value_name = "FOLDER")]
        adopt: Option<PathBuf>,
        /// Only install; don't start anything.
        #[arg(long)]
        no_start: bool,
        /// Install this file's version, without looking for a newer one on GitHub.
        #[arg(long)]
        this_file: bool,
    },
    /// Get the latest release from GitHub and install it: needs sudo.
    Update {
        /// Install the latest release even if it's the version already here.
        #[arg(long)]
        force: bool,
    },
    /// Back TPCloud's own data up now (settings and databases, not the files): needs sudo.
    Backup {
        /// Where backups go from now on, every night (best on another disk).
        #[arg(long, value_name = "FOLDER")]
        to: Option<PathBuf>,
        /// Stop the nightly backups (the copies made are kept).
        #[arg(long, conflicts_with = "to")]
        off: bool,
        /// List the backups instead of making one.
        #[arg(long, conflicts_with_all = ["to", "off"])]
        list: bool,
    },
    /// Put a backup's settings and databases back (what's there now is backed up first): needs sudo.
    Restore {
        /// The backup's folder (`tpcloud backup --list` shows them).
        backup: PathBuf,
    },
    /// The guided configuration: files, modules, how it's reached, the first account. Needs sudo.
    Setup,
    /// Start everything (systemd).
    Start,
    /// Stop everything; data is kept.
    Stop,
    /// Stop and start again (also picks up changed settings).
    Restart,
    /// A process's log: gateway, auth, store-api, home-api, mqtt-broker, cloudflared.
    Logs {
        /// The logs to show; every one when left out.
        services: Vec<String>,
        /// Keep following the (last) one.
        #[arg(short, long)]
        follow: bool,
        /// How many lines from the end.
        #[arg(short = 'n', long, default_value_t = 50)]
        lines: usize,
    },
    /// Turn modules on and off.
    Module {
        #[command(subcommand)]
        action: ModuleAction,
    },
    /// Accounts.
    Users {
        #[command(subcommand)]
        action: Option<UsersAction>,
    },
    /// The disks here and the storage pool's state.
    Storage {
        #[command(subcommand)]
        action: Option<StorageAction>,
    },
    /// Remove TPCloud from this machine, keeping every account, file and setting: needs sudo.
    Uninstall,
    /// Control the house (the same as the `tphome` command).
    #[command(disable_help_flag = true)]
    Home {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run everything in the foreground (what the systemd service does).
    #[command(hide = true)]
    Serve,
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

#[derive(Subcommand)]
enum StorageAction {
    /// The disks and the pool (the same as `tpcloud storage`).
    List,
    /// Add an empty disk to the pool (erases it): needs sudo.
    Add { disk: String },
    /// Check every block of the pool now and repair what a mirror can: needs sudo.
    Scrub,
    /// Set the pool's error counts back to zero, once its disks have been checked: needs sudo.
    Clear,
    /// Open the encrypted pool's disks (what the boot does).
    #[command(hide = true)]
    Unlock,
}

#[derive(Subcommand)]
enum UsersAction {
    /// Every account.
    List,
    /// A new account (asks for its password): needs sudo.
    Add {
        email: String,
        /// Make it an admin.
        #[arg(long)]
        admin: bool,
    },
}

fn main() {
    // A closed pipe (`tpcloud status | head`) ends the program quietly, like any other command.
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL) };
    if let Err(err) = run() {
        ui::warn(&format!("{err:#}"));
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let called_as = args
        .first()
        .and_then(|a| std::path::Path::new(a).file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_default();
    if called_as == "tphome" {
        return home::run(args);
    }
    let cli = Cli::parse_from(args);
    match cli.command {
        None if std::io::stdout().is_terminal() && std::io::stdin().is_terminal() => tui::run(),
        None | Some(Command::Status) => status::print(),
        Some(Command::Install { adopt, no_start, this_file }) => install::install(adopt, !no_start, this_file),
        Some(Command::Update { force }) => update::command(force),
        Some(Command::Backup { list: true, .. }) => backup::print_list(),
        Some(Command::Backup { to, off, .. }) => backup::command(to, off),
        Some(Command::Restore { backup: copy }) => backup::restore(&copy),
        Some(Command::Setup) => {
            setup::run()?;
            if platform::service::running() {
                platform::service::restart()?;
            } else {
                platform::service::start()?;
            }
            Ok(())
        }
        Some(Command::Start) => {
            install::require_root("Starting")?;
            platform::service::start()?;
            ui::done("Started.");
            Ok(())
        }
        Some(Command::Stop) => {
            install::require_root("Stopping")?;
            platform::service::stop()?;
            ui::done("Stopped. Data is kept.");
            Ok(())
        }
        Some(Command::Restart) => {
            install::require_root("Restarting")?;
            platform::service::restart()?;
            ui::done("Restarted.");
            Ok(())
        }
        Some(Command::Logs { services, follow, lines }) => logs::show(&services, lines, follow),
        Some(Command::Module { action }) => match action {
            ModuleAction::List => module_list(),
            ModuleAction::Enable { module } => install::set_module(&module, true),
            ModuleAction::Disable { module } => install::set_module(&module, false),
        },
        Some(Command::Users { action }) => match action {
            None | Some(UsersAction::List) => users::print_list(),
            Some(UsersAction::Add { email, admin }) => {
                install::require_root("Adding accounts")?;
                let password = rpassword_prompt("Password for the new account (12 characters or more)")?;
                users::add(&email, &password, admin)
            }
        },
        Some(Command::Storage { action }) => match action {
            None | Some(StorageAction::List) => storage::print(),
            Some(StorageAction::Add { disk }) => storage::add(&disk),
            Some(StorageAction::Scrub) => storage::scrub_now(),
            Some(StorageAction::Clear) => storage::clear(),
            Some(StorageAction::Unlock) => storage::unlock(),
        },
        Some(Command::Uninstall) => install::uninstall(),
        Some(Command::Home { args: rest }) => home::run(std::iter::once("tphome".to_string()).chain(rest)),
        Some(Command::Serve) => supervisor::serve(),
    }
}

fn rpassword_prompt(question: &str) -> Result<String> {
    use std::io::{BufRead, Write};
    print!("{question}: ");
    std::io::stdout().flush()?;
    let mut term: libc::termios = unsafe { std::mem::zeroed() };
    let tty = unsafe { libc::tcgetattr(0, &mut term) } == 0;
    if tty {
        let mut silent = term;
        silent.c_lflag &= !libc::ECHO;
        unsafe { libc::tcsetattr(0, libc::TCSANOW, &silent) };
    }
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    if tty {
        unsafe { libc::tcsetattr(0, libc::TCSANOW, &term) };
    }
    println!();
    Ok(line.trim().to_string())
}

fn module_list() -> Result<()> {
    let on = install::modules();
    for (name, what) in [("store", "files, photo backup and share links"), ("home", "blinds and lights, with tphome's switches")] {
        let enabled = on.iter().any(|m| m == name);
        println!("  {} {:<6} {}", if enabled { ui::green("●") } else { ui::dim("○") }, name, ui::dim(what));
    }
    Ok(())
}
