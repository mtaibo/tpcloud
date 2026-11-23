//! `tpcloud setup`: the guided configuration, as questions in the terminal. Run again, it offers the
//! current values as answers, so changing one thing is just pressing enter through the rest.

use crate::{engine, envfile, install, network, paths, storage, ui};
use anyhow::{bail, Result};
use std::io::{BufRead, Write};

const TPCLOUD_ENV: &str = "\
# TPCloud's settings. `sudo tpcloud setup` asks for them again; after editing by hand,
# `sudo tpcloud restart`.

# -------  Address  -------

TPCLOUD_URL=https://cloud.example.com
TPCLOUD_SITE=:80
TPCLOUD_TLS=none
TPCLOUD_MODULES=store

# -------  What runs  -------

COMPOSE_PROFILES=store
#TUNNEL_TOKEN=

# -------  Auth  -------

RP_NAME=TPCloud
#TPCLOUD_GEOIP=1

# -------  Store  -------

EXTERNAL_DISK_PATH=/var/lib/tpcloud/files

# -------  Backups  -------

# A nightly copy of this server's own data (not the files), best on another disk; empty = none.
TPCLOUD_BACKUP_DIR=
#TPCLOUD_BACKUP_KEEP=14
";

const HOME_ENV: &str = "\
# Home's settings: shared by its API and the switches' firmware.
WIFI_SSID=
WIFI_PASS=
MQTT_IP=
MQTT_PORT=1883
MQTT_USER=
MQTT_PASS=
";

pub fn default_env() -> &'static str {
    TPCLOUD_ENV
}

pub fn default_home_env() -> String {
    HOME_ENV.replace("MQTT_IP=", &format!("MQTT_IP={}", lan_ip().unwrap_or_default()))
}

/// This machine's address on the local network (the one a route to the internet would use).
pub fn lan_ip() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("1.1.1.1:80").ok()?;
    Some(socket.local_addr().ok()?.ip().to_string())
}

fn read_line() -> Result<String> {
    let mut line = String::new();
    if std::io::stdin().lock().read_line(&mut line)? == 0 {
        bail!("no answer (the terminal closed)");
    }
    Ok(line.trim().to_string())
}

fn ask(question: &str, default: &str) -> Result<String> {
    if default.is_empty() {
        print!("  {question}: ");
    } else {
        print!("  {question} {}: ", ui::dim(&format!("[{default}]")));
    }
    std::io::stdout().flush()?;
    let answer = read_line()?;
    Ok(if answer.is_empty() { default.to_string() } else { answer })
}

fn ask_yes(question: &str, default: bool) -> Result<bool> {
    let answer = ask(question, if default { "Y/n" } else { "y/N" })?;
    Ok(match answer.to_ascii_lowercase().as_str() {
        "y" | "yes" | "s" | "si" | "sí" => true,
        "n" | "no" => false,
        _ => default,
    })
}

fn ask_secret(question: &str) -> Result<String> {
    print!("  {question}: ");
    std::io::stdout().flush()?;
    // Echo off while it's typed.
    let mut term: libc::termios = unsafe { std::mem::zeroed() };
    let tty = unsafe { libc::tcgetattr(0, &mut term) } == 0;
    if tty {
        let mut silent = term;
        silent.c_lflag &= !libc::ECHO;
        unsafe { libc::tcsetattr(0, libc::TCSANOW, &silent) };
    }
    let answer = read_line();
    if tty {
        unsafe { libc::tcsetattr(0, libc::TCSANOW, &term) };
    }
    println!();
    answer
}

pub fn choose(question: &str, options: &[(&str, &str)], default: usize) -> Result<usize> {
    println!("  {question}");
    for (i, (name, what)) in options.iter().enumerate() {
        println!("    {} {}  {}", ui::accent(&format!("{}.", i + 1)), ui::bold(name), ui::dim(what));
    }
    loop {
        let answer = ask("Choice", &(default + 1).to_string())?;
        if let Ok(n) = answer.parse::<usize>() {
            if (1..=options.len()).contains(&n) {
                return Ok(n - 1);
            }
        }
    }
}

fn heading(text: &str) {
    println!("\n{}", ui::bold(text));
}

pub fn run() -> Result<()> {
    install::require_root("Setting TPCloud up")?;
    std::fs::create_dir_all(paths::state_dir())?;
    let env = paths::env_file();
    if !env.exists() {
        std::fs::write(&env, TPCLOUD_ENV)?;
    }
    install::restrict(&env)?;
    let f = engine::facts();
    let mut a = engine::current(&f);
    let current = |key: &str| envfile::get(&env, key).unwrap_or_default();

    println!("\n{}", ui::accent(&ui::bold("TPCloud setup")));
    println!("{}", ui::dim("Enter keeps the value in brackets. Nothing changes until the end."));

    heading("Network");
    println!("  {}", ui::dim("What this TPCloud is called in the apps, e.g. Home or the family's name."));
    a.name = loop {
        let answer = ask("Name", &a.name)?;
        match engine::check_name(&answer) {
            Some(m) => ui::warn(&m),
            None => break answer,
        }
    };

    heading("Files");
    println!("  {}", ui::dim("Where Store keeps everyone's files and photos: a big disk is best."));
    a.files = ask_storage(&env, &f, &a.files)?;

    heading("Backups");
    println!("  {}", ui::dim("Every night, a copy of TPCloud's own data (accounts, the photo index, albums, share"));
    println!("  {}", ui::dim("links, the house, settings; a few MB) to a folder, best on another disk. Not your files:"));
    println!("  {}", ui::dim("back that disk up on its own. Type none for no backups."));
    a.backup_dir = loop {
        let answer = ask("Folder", if a.backup_dir.is_empty() { "none" } else { &a.backup_dir })?;
        let answer = if answer == "none" || answer == "-" { String::new() } else { answer };
        match engine::check_backup_dir(&answer) {
            Some(m) => ui::warn(&m),
            None => break answer,
        }
    };

    heading("Modules");
    a.store = ask_yes("Store (files, photo backup from the app, share links)?", a.store)?;
    a.home = ask_yes("Home (blinds and lights with tphome's switches)?", a.home)?;

    let host = f.host.clone();
    loop {
        heading("At home");
        println!("  {}", ui::dim(&format!("On this network the apps find it by themselves, at https://{host}, with this machine's")));
        println!("  {}", ui::dim("own certificate (the apps trust it when they add the network)."));
        a.local = ask_yes("Answer on this network?", a.local)?;
        if a.local {
            println!("  {}", ui::dim("If your router's DNS gives this machine a name of its own (e.g. cloud.casa), type it; - for none."));
            let was = a.local_names.join(",");
            let answer = ask("Other name", if was.is_empty() { "-" } else { &was })?;
            a.local_names = if answer == "-" {
                vec![]
            } else {
                answer.split(',').map(|n| n.trim().trim_start_matches("https://").trim_end_matches('/').to_lowercase()).filter(|n| !n.is_empty()).collect()
            };
            if let Some(m) = a.local_names.iter().find_map(|n| engine::check_hostname(n)) {
                ui::warn(&m);
                continue;
            }
        } else {
            a.local_names.clear();
        }

        heading("From outside");
        let ts = ask_yes("Through Tailscale (private: only your tailnet's devices)?", a.tailscale.is_some())?;
        a.tailscale = if ts { Some(engine::tailscale_name()?) } else { None };
        let default_domain = match a.domain {
            engine::Domain::None => 0,
            engine::Domain::Cloudflare { .. } => 1,
            engine::Domain::OpenPort { .. } => 2,
        };
        let mode = choose(
            "At your own domain?",
            &[
                ("No", ""),
                ("Through Cloudflare", "a tunnel, no ports opened; you sign in to Cloudflare once"),
                ("With an open port", "port 443 forwarded to this machine"),
            ],
            default_domain,
        )?;
        let was_host = match &a.domain {
            engine::Domain::Cloudflare { hostname, .. } | engine::Domain::OpenPort { hostname } => hostname.clone(),
            engine::Domain::None => String::new(),
        };
        a.domain = if mode == 0 {
            engine::Domain::None
        } else {
            let answer = ask("Address (e.g. cloud.example.com)", &was_host)?;
            let hostname = answer.trim_start_matches("https://").trim_end_matches('/').to_lowercase();
            if let Some(m) = engine::check_hostname(&hostname) {
                ui::warn(&m);
                continue;
            }
            if mode == 2 {
                engine::Domain::OpenPort { hostname }
            } else {
                let managed = !current(network::TUNNEL_ID).is_empty() && crate::cloudflare::credentials().exists();
                let new = managed
                    || !f.has_token
                    || choose(
                        "Which tunnel?",
                        &[
                            ("The one you have", "made in Cloudflare's dashboard, its token already here"),
                            ("A new one", "made from here, after signing in to Cloudflare"),
                        ],
                        0,
                    )? == 1;
                engine::Domain::Cloudflare { hostname, new }
            }
        };
        if !a.local && a.tailscale.is_none() && a.domain == engine::Domain::None {
            ui::warn("Then nothing could reach it: pick at least one.");
            continue;
        }
        break;
    }

    if f.needs_owner {
        heading("Your account");
        println!("  {}", ui::dim("The first account, and the server's admin."));
        let email = loop {
            let email = ask("Email", "")?.to_lowercase();
            if email.contains('@') && email.contains('.') {
                break email;
            }
            ui::warn("That doesn't look like an email.");
        };
        let password = loop {
            let password = ask_secret("Password (12 characters or more)")?;
            if password.chars().count() < 12 {
                ui::warn("Too short.");
                continue;
            }
            if ask_secret("Same password again")? == password {
                break password;
            }
            ui::warn("They don't match.");
        };
        a.owner = Some((email, password));
    }

    println!();
    let outcome = engine::apply(&a, &mut Terminal)?;
    ui::done(&format!("Saved. TPCloud will answer at {}.", ui::accent(&outcome.url)));
    if !outcome.others.is_empty() {
        println!("  {}", ui::dim(&format!("And at {}.", outcome.others.join(", "))));
    }
    if !a.backup_dir.is_empty() {
        println!("  {}", ui::dim(&format!("Backups: every night to {} (`sudo tpcloud backup` makes one now).", a.backup_dir)));
    }
    if a.local {
        println!("  {}", ui::dim("The apps trust this machine's certificate by themselves. A browser at home has to trust it"));
        println!("  {}", ui::dim(&format!("once: download it from https://{host}/auth/network/ca.crt and add it to the system's certificates.")));
    }
    if a.tailscale.is_some() {
        println!("  {}", ui::dim("Tailscale gives the certificate: MagicDNS and HTTPS have to be on for your tailnet."));
    }
    if a.home {
        println!("  {}", ui::dim(&format!("Home's switches will find this machine at {} (MQTT on port 1883).", lan_ip().unwrap_or_default())));
    }
    Ok(())
}

/// The terminal's way of showing the setup as it's applied.
struct Terminal;

impl engine::Progress for Terminal {
    fn step(&mut self, text: &str) {
        ui::step(text);
    }
    fn done(&mut self, text: &str) {
        ui::done(text);
    }
    fn sign_in(&mut self, url: &str) {
        println!("  {}", ui::dim("Open this link on any device, sign in to Cloudflare and pick your domain:"));
        println!("\n      {}\n", ui::accent(url));
    }
    fn recovery_key(&mut self, key: &str) {
        println!();
        println!("  {}", ui::bold("Recovery key"));
        println!("  {}", ui::dim("This machine opens the disks by itself. If it breaks, this key opens them anywhere"));
        println!("  {}", ui::dim("(`cryptsetup open` on any Linux). It's shown only now: write it down and keep it safe."));
        println!("\n      {}\n", ui::accent(key));
        let _ = ask("Press enter once it's written down", "");
    }
    fn replace_dns(&mut self, hostname: &str) -> bool {
        ask_yes(&format!("{hostname} already points somewhere else. Point it at this machine instead?"), false).unwrap_or(false)
    }
}

/// Where Store's files go: a folder (nothing erased), or a new pool on empty disks.
fn ask_storage(env: &std::path::Path, f: &engine::Facts, was: &engine::Files) -> Result<engine::Files> {
    let default_folder = match was {
        engine::Files::Folder(p) => p.clone(),
        engine::Files::Pool { .. } => format!("{}/files", storage::DEFAULT_MOUNT),
    };
    let free: Vec<&storage::Disk> = f.disks.iter().filter(|d| d.busy.is_none()).collect();
    if let Some(line) = &f.pool {
        println!("  {}", ui::dim(&format!("Storage pool: {line}.")));
    }
    let new_pool = storage::configured(env).is_none()
        && !free.is_empty()
        && choose(
            "Where?",
            &[
                ("A folder", "on a disk that's already set up; nothing is erased"),
                ("New storage on empty disks", &format!("{} disk(s) here could be used: TPCloud formats them", free.len())),
            ],
            0,
        )? == 1;
    if !new_pool {
        loop {
            let folder = ask("Folder", &default_folder)?;
            if !folder.starts_with('/') {
                ui::warn("Give the full path, starting with /.");
                continue;
            }
            if let Some((free_bytes, total)) = storage::space(std::path::Path::new(&folder)) {
                println!("  {}", ui::dim(&format!("{} free of {} there.", storage::human(free_bytes), storage::human(total))));
            }
            return Ok(engine::Files::Folder(folder));
        }
    }

    for (i, d) in free.iter().enumerate() {
        let on_it = if d.contents.is_empty() { ui::dim("empty") } else { ui::warn_text(&format!("has {}", d.contents.join(", "))) };
        println!("    {} {}  {}", ui::accent(&format!("{}.", i + 1)), d.describe(), on_it);
    }
    let chosen: Vec<&storage::Disk> = loop {
        let answer = ask("Disks (e.g. 1, or 1,2 for two)", if free.len() == 1 { "1" } else { "" })?;
        let picks: Option<Vec<usize>> = answer.split(',').map(|s| s.trim().parse::<usize>().ok().filter(|n| (1..=free.len()).contains(n))).collect();
        match picks {
            Some(mut p) if !p.is_empty() => {
                p.sort();
                p.dedup();
                break p.iter().map(|n| free[n - 1]).collect();
            }
            _ => ui::warn("Numbers from the list, separated by commas."),
        }
    };
    let sizes: Vec<u64> = chosen.iter().map(|d| d.size).collect();
    let levels = storage::levels_for(chosen.len());
    let level = if levels.len() == 1 {
        levels[0]
    } else {
        let texts: Vec<(String, String)> = levels
            .iter()
            .map(|l| {
                let what = match l {
                    storage::Level::Raid1 => "every file on two disks: one can fail",
                    storage::Level::Raid10 => "mirrored pairs, faster: one per pair can fail",
                    storage::Level::Single => "all the space, no copy: a failing disk loses its files",
                };
                (format!("{} ({} usable)", l.name(), storage::human(storage::usable(&sizes, *l))), what.to_string())
            })
            .collect();
        let options: Vec<(&str, &str)> = texts.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        levels[choose("How?", &options, 0)?]
    };
    println!("  {}", ui::dim("Encrypted, a disk taken away alone can't be read; this machine opens it by itself."));
    let encrypt = ask_yes("Encrypt them?", true)?;
    println!();
    for d in &chosen {
        println!("  {} {}{}", ui::warn_dot(), d.describe(), if d.contents.is_empty() { String::new() } else { format!(" ({})", d.contents.join(", ")) });
    }
    let answer = ask("Everything on these disks will be erased at the end of the setup. Type ERASE to go on", "")?;
    if answer != "ERASE" {
        bail!("nothing was changed");
    }
    Ok(engine::Files::Pool { disks: chosen.iter().map(|d| d.path.clone()).collect(), level, encrypt })
}

