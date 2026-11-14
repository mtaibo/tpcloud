//! `tpcloud setup`: the guided configuration, as questions in the terminal. Run again, it offers the
//! current values as answers, so changing one thing is just pressing enter through the rest.

use crate::{envfile, install, paths, ui, users};
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

fn hostname() -> String {
    std::fs::read_to_string("/etc/hostname").map(|h| h.trim().to_string()).ok().filter(|h| !h.is_empty()).unwrap_or_else(|| "tpcloud".into())
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

    println!("\n{}", ui::accent(&ui::bold("TPCloud setup")));
    println!("{}", ui::dim("Enter keeps the value in brackets. Nothing changes until the end."));

    heading("Files");
    println!("  {}", ui::dim("Where Store keeps everyone's files and photos: a big disk is best."));
    let storage = ask("Folder", &if first { "/var/lib/tpcloud/files".into() } else { current("EXTERNAL_DISK_PATH") })?;

    heading("Modules");
    let modules = envfile::get_list(&env, "TPCLOUD_MODULES");
    let store = ask_yes("Store (files, photo backup from the app, share links)?", first || modules.iter().any(|m| m == "store"))?;
    let home = ask_yes("Home (blinds and lights with tphome's switches)?", modules.iter().any(|m| m == "home"))?;

    heading("How you'll reach it");
    let host = hostname();
    let was = current("TPCLOUD_URL");
    let default_access = if !current("TUNNEL_TOKEN").is_empty() { 2 } else if was.ends_with(".local") { 0 } else if was.contains(".ts.net") { 1 } else if first { 0 } else { 3 };
    let access = choose(
        "From where?",
        &[
            ("Only at home", &format!("https://{host}.local, on this network")),
            ("Anywhere, privately", "with Tailscale (installed and signed in on this machine)"),
            ("Your domain, through Cloudflare", "a Cloudflare Tunnel, no ports opened"),
            ("Your domain, with an open port", "port 443 forwarded to this machine"),
        ],
        default_access,
    )?;
    let (url, site, tls, https_publish, mut tunnel_token) = match access {
        0 => (format!("https://{host}.local"), format!("https://{host}.local"), "internal", "443", String::new()),
        1 => {
            let name = tailscale_name()?;
            (format!("https://{name}"), ":80".into(), "none", "", String::new())
        }
        _ => {
            let domain = ask("Address (e.g. cloud.example.com)", was.trim_start_matches("https://"))?;
            let domain = domain.trim_start_matches("https://").trim_end_matches('/').to_string();
            if access == 2 {
                (format!("https://{domain}"), format!("http://{domain}"), "none", "", current("TUNNEL_TOKEN"))
            } else {
                (format!("https://{domain}"), domain.clone(), "none", "443", String::new())
            }
        }
    };
    if access == 2 {
        println!("  {}", ui::dim("Cloudflare → Zero Trust → Networks → Tunnels → create one, copy its token,"));
        println!("  {}", ui::dim("and point its public hostname at HTTP localhost:8080."));
        let token = ask_secret(if tunnel_token.is_empty() { "Tunnel token" } else { "Tunnel token (enter keeps the current one)" })?;
        if !token.is_empty() {
            tunnel_token = token;
        }
        if tunnel_token.is_empty() {
            bail!("a tunnel needs its token");
        }
    }

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
    if access == 2 {
        profiles.push("tunnel".into());
    }
    let set = |k: &str, v: &str| envfile::set(&env, k, v);
    set("TPCLOUD_URL", &url)?;
    set("TPCLOUD_SITE", &site)?;
    set("TPCLOUD_TLS", tls)?;
    set("TPCLOUD_HTTPS_PUBLISH", https_publish)?;
    set("TPCLOUD_OTA_PUBLISH", if home { "80" } else { "" })?;
    envfile::set_list(&env, "TPCLOUD_MODULES", &module_list)?;
    envfile::set_list(&env, "COMPOSE_PROFILES", &profiles)?;
    set("EXTERNAL_DISK_PATH", &storage)?;
    if !tunnel_token.is_empty() {
        set("TUNNEL_TOKEN", &tunnel_token)?;
    }
    if home {
        let home_env = paths::home_env_file();
        if !home_env.exists() {
            std::fs::create_dir_all(home_env.parent().unwrap())?;
            std::fs::write(&home_env, default_home_env())?;
        }
        install::restrict(&home_env)?;
    }
    install::sync_links()?;
    if access == 1 {
        let status = std::process::Command::new("tailscale").args(["serve", "--bg", "--https=443", "http://127.0.0.1:8080"]).status();
        if !status.map(|s| s.success()).unwrap_or(false) {
            ui::warn("Couldn't run `tailscale serve`: turn HTTPS on for your tailnet and run `tailscale serve --bg --https=443 http://127.0.0.1:8080`.");
        }
    }
    if let Some((email, password)) = owner {
        users::create_owner(&email, &password)?;
    }
    ui::done(&format!("Saved. TPCloud will answer at {}.", ui::accent(&url)));
    if access == 0 {
        println!("  {}", ui::dim("Each device has to trust this machine's certificate once: Caddy's local authority, in"));
        println!("  {}", ui::dim(&format!("{}", paths::state_dir().join("caddy/data/caddy/pki/authorities/local/root.crt").display())));
    }
    if home {
        println!("  {}", ui::dim(&format!("Home's switches will find this machine at {} (MQTT on port 1883).", lan_ip().unwrap_or_default())));
    }
    Ok(())
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

