//! `tpcloud`: one file, nothing else to install. On a server it sets TPCloud up, runs it (as a
//! systemd service supervising its own bundled processes) and configures it; on any computer it's a
//! client of a network: sign in, and (admins) run accounts, sessions and the host's updates. Called
//! as `tphome` (a link made while Home is on), it controls the house, here or over the network.

mod client;
#[cfg(feature = "host")]
mod backup;
#[cfg(feature = "host")]
mod cloudflare;
#[cfg(feature = "host")]
mod discovery;
#[cfg(feature = "host")]
mod engine;
#[cfg(feature = "host")]
mod envfile;
#[cfg(feature = "host")]
mod harden;
mod help;
mod home;
#[cfg(feature = "host")]
mod install;
#[cfg(feature = "host")]
mod logs;
#[cfg(feature = "host")]
mod network;
#[cfg(feature = "host")]
mod paths;
#[cfg(feature = "host")]
mod platform;
#[cfg(feature = "host")]
mod runtime;
#[cfg(feature = "host")]
mod services;
#[cfg(feature = "host")]
mod setup;
#[cfg(feature = "host")]
mod setup_server;
#[cfg(feature = "host")]
mod status;
#[cfg(feature = "host")]
mod storage;
#[cfg(feature = "host")]
mod supervisor;
#[cfg(feature = "host")]
mod tui;
mod ui;
#[cfg(feature = "host")]
mod uninstall;
#[cfg(feature = "host")]
mod update;
#[cfg(feature = "host")]
mod users;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::io::IsTerminal;
#[cfg(feature = "host")]
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "tpcloud", version, about = "TPCloud from the terminal: sign in to your network and run it, or set up and run a server.", styles = help::styles())]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// On a server: what's running, where it's reached, which modules are on. Elsewhere: the network
    /// you're signed in to.
    Status,
    /// Sign in to a TPCloud network (found on this Wi-Fi, or at an address).
    Login {
        /// tpcloud.local, cloud.example.com…; left out: the network in use, or the one found here.
        address: Option<String>,
        #[arg(long)]
        email: Option<String>,
    },
    /// Sign out of the network in use.
    Logout,
    /// The networks this computer knows; `use <name>` or `remove <name>`.
    Networks {
        /// use or remove
        verb: Option<String>,
        /// The network's name (or its start) or id.
        name: Option<String>,
    },
    /// Who you're signed in as, and where.
    Whoami,
    /// Your sessions (`--all`: everyone's, admins); `revoke <id>` closes one.
    Sessions {
        #[arg(long)]
        all: bool,
        /// revoke
        verb: Option<String>,
        id: Option<i64>,
    },
    /// The host's TPCloud version (admins); `check` looks for a newer one, `update` installs it.
    Host {
        /// check or update
        action: Option<String>,
    },
    #[cfg(feature = "host")]
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
    /// On a server: get the latest release and install it (sudo). Elsewhere: update this program.
    Update {
        /// Install the latest release even if it's the version already here.
        #[arg(long)]
        force: bool,
    },
    #[cfg(feature = "host")]
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
    #[cfg(feature = "host")]
    /// Put a backup's settings and databases back (what's there now is backed up first): needs sudo.
    Restore {
        /// The backup's folder (`tpcloud backup --list` shows them).
        backup: PathBuf,
    },
    #[cfg(feature = "host")]
    /// The guided configuration: files, modules, how it's reached, the first account. Needs sudo.
    Setup,
    #[cfg(feature = "host")]
    /// Start everything (systemd).
    Start,
    #[cfg(feature = "host")]
    /// Stop everything; data is kept.
    Stop,
    #[cfg(feature = "host")]
    /// Stop and start again (also picks up changed settings).
    Restart,
    #[cfg(feature = "host")]
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
    #[cfg(feature = "host")]
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
    #[cfg(feature = "host")]
    /// The disks here and the storage pool's state.
    Storage {
        #[command(subcommand)]
        action: Option<StorageAction>,
    },
    /// Delete TPCloud from this computer.
    ///
    /// On a server (sudo) it asks what goes: only the program, or everything (with a backup first,
    /// or nothing kept). Anywhere else, every trace at once. `uninstall` still works too.
    #[command(alias = "uninstall")]
    Delete,
    /// Control the house (the same as the `tphome` command).
    #[command(disable_help_flag = true)]
    Home {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[cfg(feature = "host")]
    /// Run everything in the foreground (what the systemd service does).
    #[command(hide = true)]
    Serve,
}

#[cfg(feature = "host")]
#[derive(Subcommand)]
enum ModuleAction {
    /// Which modules there are and which are on.
    List,
    /// Turn a module on (store, home): needs sudo.
    Enable { module: String },
    /// Turn a module off, keeping its data: needs sudo.
    Disable { module: String },
}

#[cfg(feature = "host")]
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
    /// A new account (asks for its password).
    Add {
        email: String,
        /// Make it an admin.
        #[arg(long)]
        admin: bool,
    },
    /// Delete an account (asks to confirm).
    Remove { email: String },
    /// Make an account an admin, or stop it being one.
    Admin { email: String },
}

fn main() {
    // A closed pipe (`tpcloud status | head`) ends the program quietly, like any other command.
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL) };
    if let Err(err) = run() {
        ui::warn(&format!("{err:#}"));
        std::process::exit(1);
    }
}

/// This computer is a TPCloud server (its settings are here).
fn host_here() -> bool {
    #[cfg(feature = "host")]
    {
        paths::env_file().exists()
    }
    #[cfg(not(feature = "host"))]
    {
        false
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
    if args.len() == 2 && matches!(args[1].as_str(), "-h" | "--help" | "help") {
        help::print(host_here());
        return Ok(());
    }
    let cli = Cli::parse_from(args);
    match cli.command {
        #[cfg(feature = "host")]
        None if host_here() && std::io::stdout().is_terminal() && std::io::stdin().is_terminal() => tui::run(),
        #[cfg(feature = "host")]
        None | Some(Command::Status) if host_here() => status::print(),
        None if client::signed_in() && std::io::stdout().is_terminal() && std::io::stdin().is_terminal() => client::open(),
        None | Some(Command::Status) | Some(Command::Whoami) => {
            if client::signed_in() {
                client::whoami()
            } else {
                println!("Not signed in to any network: `tpcloud login`. `tpcloud --help` lists everything.");
                Ok(())
            }
        }
        Some(Command::Login { address, email }) => client::login(address, email),
        Some(Command::Logout) => client::logout(),
        Some(Command::Networks { verb, name }) => match (verb.as_deref(), name) {
            (None, _) => client::networks(None),
            (Some(v @ ("use" | "remove")), Some(n)) => client::networks(Some((v.to_string(), n))),
            _ => anyhow::bail!("`tpcloud networks`, `tpcloud networks use <name>` or `tpcloud networks remove <name>`"),
        },
        Some(Command::Sessions { all, verb, id }) => match (verb.as_deref(), id) {
            (None, _) => client::sessions(all, None),
            (Some("revoke"), Some(id)) => client::sessions(all, Some(id)),
            _ => anyhow::bail!("`tpcloud sessions [--all]` or `tpcloud sessions [--all] revoke <id>`"),
        },
        Some(Command::Host { action }) => match action.as_deref() {
            None | Some("check") | Some("update") => client::host(action.as_deref()),
            Some(other) => anyhow::bail!("{other}? `tpcloud host`, `tpcloud host check` or `tpcloud host update`"),
        },
        #[cfg(feature = "host")]
        Some(Command::Update { force }) if host_here() => update::command(force),
        Some(Command::Update { .. }) => client::self_update(),
        // Accounts: over the network when signed in to one; on a server, also right here.
        Some(Command::Users { action }) if client::signed_in() || !host_here() => client::users(match action {
            None | Some(UsersAction::List) => client::UsersRemote::List,
            Some(UsersAction::Add { email, admin }) => client::UsersRemote::Add { email, admin },
            Some(UsersAction::Remove { email }) => client::UsersRemote::Remove { email },
            Some(UsersAction::Admin { email }) => client::UsersRemote::Admin { email },
        }),
        #[cfg(feature = "host")]
        Some(Command::Users { action }) => match action {
            None | Some(UsersAction::List) => users::print_list(),
            Some(UsersAction::Add { email, admin }) => {
                install::require_root("Adding accounts")?;
                let password = ui::secret("Password for the new account (12 characters or more)")?;
                users::add(&email, &password, admin)
            }
            Some(_) => anyhow::bail!("removing accounts and changing roles is done signed in: `tpcloud login`, then again"),
        },
        #[cfg(not(feature = "host"))]
        Some(Command::Users { .. }) => unreachable!(),
        Some(Command::Home { args: rest }) => home::run(std::iter::once("tphome".to_string()).chain(rest)),
        #[cfg(feature = "host")]
        Some(Command::Install { adopt, no_start, this_file }) => install::install(adopt, !no_start, this_file),
        #[cfg(feature = "host")]
        Some(Command::Backup { list: true, .. }) => backup::print_list(),
        #[cfg(feature = "host")]
        Some(Command::Backup { to, off, .. }) => backup::command(to, off),
        #[cfg(feature = "host")]
        Some(Command::Restore { backup: copy }) => backup::restore(&copy),
        #[cfg(feature = "host")]
        Some(Command::Setup) => {
            setup::run()?;
            if platform::service::running() {
                platform::service::restart()?;
            } else {
                platform::service::start()?;
            }
            Ok(())
        }
        #[cfg(feature = "host")]
        Some(Command::Start) => {
            install::require_root("Starting")?;
            platform::service::start()?;
            ui::done("Started.");
            Ok(())
        }
        #[cfg(feature = "host")]
        Some(Command::Stop) => {
            install::require_root("Stopping")?;
            platform::service::stop()?;
            ui::done("Stopped. Data is kept.");
            Ok(())
        }
        #[cfg(feature = "host")]
        Some(Command::Restart) => {
            install::require_root("Restarting")?;
            platform::service::restart()?;
            ui::done("Restarted.");
            Ok(())
        }
        #[cfg(feature = "host")]
        Some(Command::Logs { services, follow, lines }) => logs::show(&services, lines, follow),
        #[cfg(feature = "host")]
        Some(Command::Module { action }) => match action {
            ModuleAction::List => module_list(),
            ModuleAction::Enable { module } => install::set_module(&module, true),
            ModuleAction::Disable { module } => install::set_module(&module, false),
        },
        #[cfg(feature = "host")]
        Some(Command::Storage { action }) => match action {
            None | Some(StorageAction::List) => storage::print(),
            Some(StorageAction::Add { disk }) => storage::add(&disk),
            Some(StorageAction::Scrub) => storage::scrub_now(),
            Some(StorageAction::Clear) => storage::clear(),
            Some(StorageAction::Unlock) => storage::unlock(),
        },
        #[cfg(feature = "host")]
        Some(Command::Delete) if host_here() || std::path::Path::new(platform::APP_DIR).exists() => uninstall::run(),
        Some(Command::Delete) => client::uninstall(),
        #[cfg(feature = "host")]
        Some(Command::Serve) => supervisor::serve(),
    }
}

#[cfg(feature = "host")]
fn module_list() -> Result<()> {
    let on = install::modules();
    for (name, what) in [("store", "files, photo backup and share links"), ("home", "blinds and lights, with tphome's switches")] {
        let enabled = on.iter().any(|m| m == name);
        println!("  {} {:<6} {}", if enabled { ui::green("●") } else { ui::dim("○") }, name, ui::dim(what));
    }
    Ok(())
}
