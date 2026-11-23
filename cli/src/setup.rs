//! `tpcloud setup`: the guided configuration, as questions in the terminal. Run again, it offers the
//! current values as answers, so changing one thing is just pressing enter through the rest.

use crate::{envfile, install, network, paths, ui, users};
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

fn choose(question: &str, options: &[(&str, &str)], default: usize) -> Result<usize> {
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
    let first = !env.exists();
    if first {
        std::fs::write(&env, TPCLOUD_ENV)?;
    }
    install::restrict(&env)?;
    let current = |key: &str| envfile::get(&env, key).unwrap_or_default();

    let default_name = network::default_name();
    println!("\n{}", ui::accent(&ui::bold("TPCloud setup")));
    println!("{}", ui::dim("Enter keeps the value in brackets. Nothing changes until the end."));

    heading("Network");
    println!("  {}", ui::dim("What this TPCloud is called in the apps, e.g. Home or the family's name."));
    let network_name = loop {
        let was = current(network::NAME);
        let answer = ask("Name", if was.is_empty() { &default_name } else { &was })?;
        if answer.chars().count() > 64 {
            ui::warn("64 characters at most.");
            continue;
        }
        // The settings file would read the rest as a comment.
        if answer.contains(" #") {
            ui::warn("No # after a space.");
            continue;
        }
        break answer;
    };

    heading("Files");
    println!("  {}", ui::dim("Where Store keeps everyone's files and photos: a big disk is best."));
    let storage = ask("Folder", &if first { "/var/lib/tpcloud/files".into() } else { current("EXTERNAL_DISK_PATH") })?;

    heading("Backups");
    println!("  {}", ui::dim("Every night, a copy of TPCloud's own data (accounts, the photo index, albums, share"));
    println!("  {}", ui::dim("links, the house, settings; a few MB) to a folder, best on another disk. Not your files:"));
    println!("  {}", ui::dim("back that disk up on its own. Type none for no backups."));
    let backup_dir = loop {
        let answer = ask("Folder", &if current("TPCLOUD_BACKUP_DIR").is_empty() { "none".into() } else { current("TPCLOUD_BACKUP_DIR") })?;
        if answer == "none" || answer == "-" {
            break String::new();
        }
        if !answer.starts_with('/') {
            ui::warn("Give the full path, starting with /.");
            continue;
        }
        if std::path::Path::new(&answer).starts_with(paths::state_dir()) {
            ui::warn(&format!("Not inside {}: it would copy itself.", paths::state_dir().display()));
            continue;
        }
        break answer;
    };

    heading("Modules");
    let modules = envfile::get_list(&env, "TPCLOUD_MODULES");
    let store = ask_yes("Store (files, photo backup from the app, share links)?", first || modules.iter().any(|m| m == "store"))?;
    let home = ask_yes("Home (blinds and lights with tphome's switches)?", modules.iter().any(|m| m == "home"))?;

    let host = format!("{}.local", network::host_label());
    let was = current("TPCLOUD_URL");
    let (url, site, tls, https_publish, reach) = loop {
        heading("At home");
        println!("  {}", ui::dim(&format!("On this network the apps find it by themselves, at https://{host}, with this machine's")));
        println!("  {}", ui::dim("own certificate (the apps trust it when they add the network)."));
        let local = ask_yes("Answer on this network?", first || current(network::LOCAL) == "on")?;
        let mut local_names = String::new();
        if local {
            println!("  {}", ui::dim("If your router's DNS gives this machine a name of its own (e.g. cloud.casa), type it; - for none."));
            let was_names = current(network::LOCAL_NAMES);
            let answer = ask("Other name", if was_names.is_empty() { "-" } else { &was_names })?;
            if answer != "-" {
                local_names = answer.split(',').map(|n| n.trim().trim_start_matches("https://").trim_end_matches('/').to_lowercase()).filter(|n| !n.is_empty()).collect::<Vec<_>>().join(",");
            }
        }

        heading("From outside");
        let ts = ask_yes("Through Tailscale (private: only your tailnet's devices)?", !current(network::TAILSCALE).is_empty() || was.contains(".ts.net"))?;
        let ts_name = if ts { Some(tailscale_name()?) } else { None };
        let had_domain = !current(network::TUNNEL_HOSTNAME).is_empty() || !current("TUNNEL_TOKEN").is_empty() || (!first && !was.contains(".ts.net") && !was.ends_with(".local") && !was.is_empty());
        let default_domain = if !current(network::TUNNEL_HOSTNAME).is_empty() || !current("TUNNEL_TOKEN").is_empty() { 1 } else if had_domain { 2 } else { 0 };
        let domain_mode = choose(
            "At your own domain?",
            &[
                ("No", ""),
                ("Through Cloudflare", "a tunnel, no ports opened; you sign in to Cloudflare once"),
                ("With an open port", "port 443 forwarded to this machine"),
            ],
            default_domain,
        )?;
        let mut domain = String::new();
        let mut tunnel = TunnelChoice::None;
        if domain_mode > 0 {
            let was_domain = if was.contains(".ts.net") || was.ends_with(".local") { String::new() } else { was.trim_start_matches("https://").to_string() };
            let answer = ask("Address (e.g. cloud.example.com)", &was_domain)?;
            domain = answer.trim_start_matches("https://").trim_end_matches('/').to_lowercase();
            if !domain.contains('.') {
                ui::warn("That needs to be a full address, like cloud.example.com.");
                continue;
            }
        }
        if domain_mode == 1 {
            let managed = !current(network::TUNNEL_ID).is_empty() && crate::cloudflare::credentials().exists();
            tunnel = if !managed && !current("TUNNEL_TOKEN").is_empty() {
                match choose(
                    "Which tunnel?",
                    &[
                        ("The one you have", "made in Cloudflare's dashboard, its token already here"),
                        ("A new one", "made from here, after signing in to Cloudflare"),
                    ],
                    0,
                )? {
                    0 => TunnelChoice::Token,
                    _ => TunnelChoice::Managed,
                }
            } else {
                TunnelChoice::Managed
            };
        }
        if !local && ts_name.is_none() && domain.is_empty() {
            ui::warn("Then nothing could reach it: pick at least one.");
            continue;
        }

        // The main address (links, sign-in, passkeys): the domain, else Tailscale's, else home's.
        let choice = if domain_mode == 1 {
            (format!("https://{domain}"), format!("http://{domain}"), "none", "")
        } else if domain_mode == 2 {
            (format!("https://{domain}"), domain.clone(), "none", "443")
        } else if let Some(name) = &ts_name {
            (format!("https://{name}"), format!("https://{name}"), "none", "443")
        } else {
            (format!("https://{host}"), format!("https://{host}"), "internal", "443")
        };
        break (choice.0, choice.1, choice.2, choice.3, Reach { local, local_names, ts_name, domain, tunnel });
    };

    let need_owner = first || users::count().unwrap_or(0) == 0;
    let mut owner = None;
    if need_owner {
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
        owner = Some((email, password));
    }

    // Everything asked: write it.
    std::fs::create_dir_all(&storage)?;
    let mut module_list = Vec::new();
    let mut profiles: Vec<String> = Vec::new();
    if store {
        module_list.push("store".to_string());
        profiles.push("store".into());
    }
    if home {
        module_list.push("home".to_string());
    }
    if !matches!(reach.tunnel, TunnelChoice::None) {
        profiles.push("tunnel".into());
    }
    let set = |k: &str, v: &str| envfile::set(&env, k, v);
    set(network::NAME, &network_name)?;
    network::ensure(&env)?;
    // The tunnel first: it's the step that can fail (a sign-in left halfway, a taken name), and
    // then nothing else has changed.
    match reach.tunnel {
        TunnelChoice::Managed => {
            heading("Cloudflare");
            let id = current(network::ID);
            crate::cloudflare::login()?;
            let tunnel_id = crate::cloudflare::create(&format!("tpcloud-{}", &id[..id.len().min(8)]), Some(&current(network::TUNNEL_ID)))?;
            // A domain the sign-in didn't pick fails here: forget that sign-in, so the next setup asks
            // for the right one.
            let routed = crate::cloudflare::route(&tunnel_id, &reach.domain, false).map_err(|err| {
                crate::cloudflare::logout();
                err.context("sign in again picking that address's domain (run setup again)")
            })?;
            match routed {
                crate::cloudflare::Route::Done => {}
                crate::cloudflare::Route::Taken => {
                    if !ask_yes(&format!("{} already points somewhere else. Point it at this machine instead?", reach.domain), false)? {
                        bail!("{} was left as it was: run setup again with another address", reach.domain);
                    }
                    crate::cloudflare::route(&tunnel_id, &reach.domain, true)?;
                }
            }
            set(network::TUNNEL_ID, &tunnel_id)?;
            set(network::TUNNEL_HOSTNAME, &reach.domain)?;
            ui::done(&format!("Tunnel ready: {} → this machine.", reach.domain));
        }
        TunnelChoice::Token | TunnelChoice::None => {
            // A token tunnel keeps its ingress in Cloudflare's dashboard; nothing of ours to run.
            set(network::TUNNEL_HOSTNAME, "")?;
        }
    }
    set(network::LOCAL, if reach.local { "on" } else { "off" })?;
    set(network::LOCAL_NAMES, &reach.local_names)?;
    set(network::TAILSCALE, reach.ts_name.as_deref().unwrap_or(""))?;
    set("TPCLOUD_URL", &url)?;
    set("TPCLOUD_SITE", &site)?;
    set("TPCLOUD_TLS", tls)?;
    set("TPCLOUD_HTTPS_PUBLISH", https_publish)?;
    set("TPCLOUD_OTA_PUBLISH", if home { "80" } else { "" })?;
    envfile::set_list(&env, "TPCLOUD_MODULES", &module_list)?;
    envfile::set_list(&env, "COMPOSE_PROFILES", &profiles)?;
    set("EXTERNAL_DISK_PATH", &storage)?;
    set("TPCLOUD_BACKUP_DIR", &backup_dir)?;
    if home {
        let home_env = paths::home_env_file();
        if !home_env.exists() {
            std::fs::create_dir_all(home_env.parent().unwrap())?;
            std::fs::write(&home_env, default_home_env())?;
        }
        install::restrict(&home_env)?;
    }
    install::sync_links()?;
    if reach.ts_name.is_some() {
        // The gateway answers on Tailscale itself now; a `tailscale serve` on 443 from an older
        // setup would take the port from it.
        let _ = std::process::Command::new("tailscale")
            .args(["serve", "--https=443", "off"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    if let Some((email, password)) = owner {
        users::create_owner(&email, &password)?;
    }
    ui::done(&format!("Saved. TPCloud will answer at {}.", ui::accent(&url)));
    let others: Vec<String> = network::addresses(&crate::services::Settings::load()).into_iter().filter(|a| *a != url).collect();
    if !others.is_empty() {
        println!("  {}", ui::dim(&format!("And at {}.", others.join(", "))));
    }
    if !backup_dir.is_empty() {
        println!("  {}", ui::dim(&format!("Backups: every night to {} (`sudo tpcloud backup` makes one now).", backup_dir)));
    }
    if reach.local {
        println!("  {}", ui::dim("The apps trust this machine's certificate by themselves. A browser at home has to trust it"));
        println!("  {}", ui::dim(&format!("once: download it from https://{host}/auth/network/ca.crt and add it to the system's certificates.")));
    }
    if reach.ts_name.is_some() {
        println!("  {}", ui::dim("Tailscale gives the certificate: MagicDNS and HTTPS have to be on for your tailnet."));
    }
    if home {
        println!("  {}", ui::dim(&format!("Home's switches will find this machine at {} (MQTT on port 1883).", lan_ip().unwrap_or_default())));
    }
    Ok(())
}

/// Where the setup's answers say the network should answer.
struct Reach {
    local: bool,
    local_names: String,
    ts_name: Option<String>,
    domain: String,
    tunnel: TunnelChoice,
}

enum TunnelChoice {
    None,
    /// A tunnel made by hand in Cloudflare's dashboard, run from the token already in the settings.
    Token,
    /// One made from the setup in the owner's account.
    Managed,
}

fn tailscale_name() -> Result<String> {
    let out = std::process::Command::new("tailscale").args(["status", "--json"]).output();
    let Ok(out) = out else { bail!("Tailscale isn't installed here: install it, sign in, and run setup again") };
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
    match json["Self"]["DNSName"].as_str() {
        Some(name) if !name.is_empty() => Ok(name.trim_end_matches('.').to_string()),
        _ => bail!("Tailscale isn't signed in here: run `sudo tailscale up` and setup again"),
    }
}

