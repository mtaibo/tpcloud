//! `tpcloud` as a client of a TPCloud network, from any computer: sign in, see who you are, and
//! (admins) run the network: accounts, sessions, updating the host. The networks and their
//! sessions live in the same file as the Linux app's (~/.config/tpcloud/networks.json; on a Mac
//! ~/Library/Application Support/tpcloud/), so signing in once is signing in for both.

pub mod net;
mod tui;

use crate::ui;
use anyhow::{bail, Context, Result};
use net::Remote;
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;

// ── The saved networks ──

fn config_file() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let dir = if cfg!(target_os = "macos") {
        home.join("Library/Application Support")
    } else {
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".config"))
    };
    dir.join("tpcloud/networks.json")
}

struct Store(Value);

impl Store {
    fn load() -> Store {
        let v = std::fs::read_to_string(config_file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_else(|| json!({}));
        Store(v)
    }

    fn save(&self) -> Result<()> {
        let path = config_file();
        std::fs::create_dir_all(path.parent().unwrap())?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(&self.0)?)?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }

    fn networks(&self) -> Vec<Value> {
        self.0["networks"].as_array().cloned().unwrap_or_default()
    }

    fn current_id(&self) -> Option<String> {
        self.0["current"].as_str().map(str::to_string)
    }

    fn current(&self) -> Option<Value> {
        let id = self.current_id()?;
        self.networks().into_iter().find(|n| n["id"].as_str() == Some(&id))
    }

    fn put(&mut self, network: Value) {
        let id = network["id"].as_str().unwrap_or("").to_string();
        let mut list = self.networks();
        match list.iter_mut().find(|n| n["id"].as_str() == Some(&id)) {
            Some(n) => {
                for (k, v) in network.as_object().unwrap() {
                    n[k] = v.clone();
                }
            }
            None => list.push(network),
        }
        self.0["networks"] = Value::Array(list);
        self.0["current"] = json!(id);
    }

    /// A network by id, or by (the start of) its name.
    fn find(&self, which: &str) -> Option<Value> {
        let w = which.to_lowercase();
        self.networks()
            .into_iter()
            .find(|n| n["id"].as_str() == Some(which) || n["name"].as_str().is_some_and(|name| name.to_lowercase().starts_with(&w)))
    }
}

fn anchor_of(n: &Value) -> Option<Vec<u8>> {
    n["anchor"].as_str().and_then(net::base64_decode)
}

fn local_hosts(n: &Value) -> Vec<String> {
    n["addresses"].as_array().into_iter().flatten().filter(|a| a["kind"] == "local").filter_map(|a| a["url"].as_str()).map(net::host_of).collect()
}

/// The network's addresses, home's first, then Tailscale's, then public ones (and the last used).
fn candidates(n: &Value) -> Vec<String> {
    let rank = |k: &str| match k {
        "local" => 0,
        "tailscale" => 1,
        _ => 2,
    };
    let mut list: Vec<(i32, String)> = n["addresses"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| Some((rank(a["kind"].as_str().unwrap_or("")), a["url"].as_str()?.to_string())))
        .collect();
    list.sort();
    let mut urls: Vec<String> = list.into_iter().map(|(_, u)| u).collect();
    if let Some(last) = n["last_address"].as_str() {
        if !urls.contains(&last.to_string()) {
            urls.push(last.to_string());
        }
    }
    urls
}

/// Signed in to the network in use, at the fastest address that answers.
pub fn connect() -> Result<(Remote, Value)> {
    let store = Store::load();
    let Some(n) = store.current() else { bail!("not signed in to any network: `tpcloud login`") };
    if n["token"].as_str().is_none() {
        bail!("signed out of {}: `tpcloud login`", n["name"].as_str().unwrap_or("it"));
    }
    let token = n["token"].as_str().map(str::to_string);
    let anchor = anchor_of(&n);
    let hosts = local_hosts(&n);
    let id = n["id"].as_str().unwrap_or("").to_string();
    for url in candidates(&n) {
        let remote = Remote::new(&url, anchor.as_deref(), &hosts, token.clone());
        if remote.answers_as(&id) {
            return Ok((remote, n));
        }
    }
    bail!("none of {}'s addresses answer right now", n["name"].as_str().unwrap_or("the network's"))
}

/// `tpcloud` alone, signed in: the network in a terminal app.
pub fn open() -> Result<()> {
    let (remote, n) = connect()?;
    tui::run(remote, n)
}

/// Whether this computer is signed in to a network (for choosing local or remote commands).
pub fn signed_in() -> bool {
    Store::load().current().is_some_and(|n| n["token"].is_string())
}

// ── Signing in ──

fn read_line(question: &str, default: &str) -> Result<String> {
    if default.is_empty() {
        print!("{question}: ");
    } else {
        print!("{question} {}: ", ui::dim(&format!("[{default}]")));
    }
    std::io::stdout().flush()?;
    let mut line = String::new();
    if std::io::stdin().read_line(&mut line)? == 0 {
        bail!("no answer");
    }
    let line = line.trim().to_string();
    Ok(if line.is_empty() { default.to_string() } else { line })
}

/// TPCloud networks announcing themselves here (Bonjour): (name, address).
fn discover() -> Vec<(String, String)> {
    let Ok(daemon) = mdns_sd::ServiceDaemon::new() else { return vec![] };
    let Ok(rx) = daemon.browse("_tpcloud._tcp.local.") else { return vec![] };
    let mut found: Vec<(String, String)> = Vec::new();
    let until = std::time::Instant::now() + std::time::Duration::from_millis(2500);
    while let Some(left) = until.checked_duration_since(std::time::Instant::now()) {
        let Ok(event) = rx.recv_timeout(left) else { break };
        if let mdns_sd::ServiceEvent::ServiceResolved(info) = event {
            let host = info.get_properties().get_property_val_str("host").map(str::to_string).unwrap_or_else(|| info.get_hostname().trim_end_matches('.').to_string());
            let name = info.get_fullname().split("._tpcloud").next().unwrap_or("").replace("\\032", " ");
            let item = (name, format!("https://{host}"));
            if !found.contains(&item) {
                found.push(item);
            }
        }
    }
    let _ = daemon.shutdown();
    found
}

/// A QR code drawn with half blocks, two rows of modules per line.
fn print_qr(text: &str) {
    let Ok(code) = qrcode::QrCode::new(text.as_bytes()) else { return };
    let width = code.width();
    let colors = code.to_colors();
    let dark = |x: isize, y: isize| x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < width && colors[y as usize * width + x as usize] == qrcode::Color::Dark;
    let quiet = 2isize;
    let mut y = -quiet;
    while y < width as isize + quiet {
        let mut line = String::from("  ");
        for x in -quiet..width as isize + quiet {
            // Light modules drawn, dark ones left as the terminal's background: reads on dark themes.
            line.push(match (dark(x, y), dark(x, y + 1)) {
                (false, false) => '█',
                (false, true) => '▀',
                (true, false) => '▄',
                (true, true) => ' ',
            });
        }
        println!("{line}");
        y += 2;
    }
}

fn open_in_browser(url: &str) {
    if std::env::var_os("TPCLOUD_NO_BROWSER").is_some() {
        return;
    }
    let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let _ = std::process::Command::new(opener).arg(url).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
}

/// Signs in by approving from a browser (where passkeys work) or a phone already signed in.
fn approve_in_browser(remote: &mut Remote) -> Result<()> {
    let (url, cookie) = remote.start_approval()?;
    println!();
    println!("  Approve this sign-in in the browser that just opened, or scan this with your phone:");
    println!();
    print_qr(&url);
    println!();
    println!("  {}", ui::dim(&url));
    open_in_browser(&url);
    let mut said_scanned = false;
    for _ in 0..90 {
        std::thread::sleep(std::time::Duration::from_millis(1500));
        match remote.poll_approval(&cookie)?.as_str() {
            "approved" => return Ok(()),
            "scanned" if !said_scanned => {
                ui::step("Opened: approve it there");
                said_scanned = true;
            }
            "denied" => bail!("the sign-in was declined"),
            "expired" => bail!("the code expired: `tpcloud login` again"),
            _ => {}
        }
    }
    bail!("nobody approved it in time: `tpcloud login` again")
}

/// `tpcloud login [address]`.
pub fn login(address: Option<String>, email: Option<String>) -> Result<()> {
    let mut store = Store::load();
    let address = match address {
        Some(a) => a,
        None => match store.current() {
            Some(n) => candidates(&n).last().cloned().unwrap_or_default(),
            None => {
                ui::step("Looking for TPCloud on this network");
                let found = discover();
                match found.len() {
                    0 => read_line("Its address (e.g. tpcloud.local or cloud.example.com)", "")?,
                    1 => {
                        println!("  Found {} at {}", ui::bold(&found[0].0), found[0].1);
                        found[0].1.clone()
                    }
                    _ => {
                        for (i, (name, url)) in found.iter().enumerate() {
                            println!("  {} {}  {}", ui::accent(&format!("{}.", i + 1)), ui::bold(name), ui::dim(url));
                        }
                        let pick: usize = read_line("Which one", "1")?.parse().unwrap_or(1);
                        found.get(pick.saturating_sub(1)).context("not on the list")?.1.clone()
                    }
                }
            }
        },
    };
    let (card, anchor) = net::load_card(&address)?;
    let base = net::normalize(&address).unwrap();
    let previous = store.find(&card.id);
    let hosts: Vec<String> = card.addresses.iter().filter(|(_, k)| k == "local").map(|(u, _)| net::host_of(u)).collect();
    let anchor = anchor.or_else(|| previous.as_ref().and_then(anchor_of));
    let mut remote = Remote::new(&base, anchor.as_deref(), &hosts, None);
    println!("Signing in to {} {}", ui::bold(&card.name), ui::dim(&format!("(TPCloud {} at {base})", card.version)));
    let default_email = email.clone().or_else(|| previous.as_ref().and_then(|p| p["email"].as_str().map(str::to_string))).unwrap_or_default();
    let in_browser = email.is_none() && {
        println!("  {} In the browser {}", ui::accent("1."), ui::dim("(a passkey, or approve from a device already signed in)"));
        println!("  {} Email and password here", ui::accent("2."));
        read_line("How", "1")? != "2"
    };
    if in_browser {
        approve_in_browser(&mut remote)?;
    } else {
        let email = read_line("Email", &default_email)?.to_lowercase();
        let password = ui::secret("Password")?;
        if remote.login(&email, &password)? {
            let code = read_line("Code from your authenticator app", "")?;
            remote.send("POST", "/auth/password/totp/verify", Some(json!({ "code": code })))?;
        }
    }
    let me = remote.get("/auth/passkey/me")?;
    let email = me["email"].as_str().unwrap_or(&default_email).to_string();
    store.put(json!({
        "id": card.id,
        "name": card.name,
        "addresses": card.addresses.iter().map(|(u, k)| json!({"url": u, "kind": k})).collect::<Vec<_>>(),
        "anchor": anchor.map(|d| net::base64_encode(&d)),
        "last_address": base,
        "token": remote.token,
        "email": email,
    }));
    store.save()?;
    ui::done(&format!("Signed in to {} as {}{}.", card.name, me["email"].as_str().unwrap_or(&email), if me["is_admin"] == true { " (admin)" } else { "" }));
    sync_tphome(me["modules"].as_array().is_some_and(|m| m.iter().any(|x| x == "home")));
    Ok(())
}

/// `tphome` beside `tpcloud` while the network has Home (where this user may put it).
fn sync_tphome(home: bool) {
    let Ok(me) = std::env::current_exe() else { return };
    let Some(dir) = me.parent() else { return };
    let link = dir.join("tphome");
    let present = link.symlink_metadata().is_ok();
    // The installer usually made it already (it had the password); otherwise, where we may.
    if home && !present && std::os::unix::fs::symlink(&me, &link).is_err() {
        println!("  {}", ui::dim(&format!("Home is on in this network. For `tphome`: sudo ln -s {} {}", me.display(), link.display())));
    }
    if home {
        println!("  {}", ui::dim("`tphome` controls the house; `tpcloud` alone opens everything."));
    }
}

/// `tpcloud logout`.
pub fn logout() -> Result<()> {
    let mut store = Store::load();
    let Some(mut n) = store.current() else { bail!("not signed in to any network") };
    if let Ok((remote, _)) = connect() {
        let _ = remote.send("POST", "/auth/passkey/logout", None);
    }
    n["token"] = Value::Null;
    let name = n["name"].as_str().unwrap_or("").to_string();
    store.put(n);
    store.save()?;
    ui::done(&format!("Signed out of {name}."));
    Ok(())
}

/// `tpcloud networks`.
pub fn networks(action: Option<(String, String)>) -> Result<()> {
    let mut store = Store::load();
    match action {
        None => {
            let current = store.current_id();
            if store.networks().is_empty() {
                println!("No networks yet: `tpcloud login`.");
            }
            for n in store.networks() {
                let here = n["id"].as_str() == current.as_deref();
                println!(
                    "  {} {}  {}{}",
                    if here { ui::accent("●") } else { ui::dim("○") },
                    ui::bold(n["name"].as_str().unwrap_or("")),
                    ui::dim(n["last_address"].as_str().unwrap_or("")),
                    ui::dim(&if n["token"].is_string() { format!("  {}", n["email"].as_str().unwrap_or("")) } else { "  signed out".into() })
                );
            }
        }
        Some((verb, which)) => {
            let n = store.find(&which).with_context(|| format!("no network called {which} (`tpcloud networks` lists them)"))?;
            let id = n["id"].as_str().unwrap_or("").to_string();
            let name = n["name"].as_str().unwrap_or("").to_string();
            if verb == "use" {
                store.0["current"] = json!(id);
                ui::done(&format!("Using {name}."));
            } else {
                let rest: Vec<Value> = store.networks().into_iter().filter(|x| x["id"].as_str() != Some(&id)).collect();
                if store.current_id().as_deref() == Some(&id) {
                    store.0["current"] = rest.first().map(|x| x["id"].clone()).unwrap_or(Value::Null);
                }
                store.0["networks"] = Value::Array(rest);
                ui::done(&format!("Forgot {name}."));
            }
            store.save()?;
        }
    }
    Ok(())
}

/// `tpcloud whoami` (and `tpcloud status` on a computer that isn't a host).
pub fn whoami() -> Result<()> {
    let (remote, n) = connect()?;
    let me = remote.get("/auth/passkey/me")?;
    let card = remote.get("/.well-known/tpcloud").unwrap_or(Value::Null);
    println!("{} {}  {}", ui::bold(n["name"].as_str().unwrap_or("")), ui::dim(card["version"].as_str().unwrap_or("")), ui::accent(&remote.base));
    println!("{}  {}{}", ui::dim("Signed in as"), me["email"].as_str().unwrap_or(""), if me["is_admin"] == true { ui::dim("  admin") } else { String::new() });
    let modules: Vec<&str> = me["modules"].as_array().into_iter().flatten().filter_map(|m| m.as_str()).collect();
    println!("{}  {}", ui::dim("Modules     "), if modules.is_empty() { "none".into() } else { modules.join(", ") });
    Ok(())
}

// ── Admin ──

fn user_id(remote: &Remote, email: &str) -> Result<(i64, Value)> {
    let users = remote.get("/auth/admin/users")?;
    let u = users.as_array().into_iter().flatten().find(|u| u["email"].as_str() == Some(&email.to_lowercase())).cloned().with_context(|| format!("there's no account {email}"))?;
    Ok((u["id"].as_i64().unwrap_or(0), u))
}

pub enum UsersRemote {
    List,
    Add { email: String, admin: bool },
    Remove { email: String },
    Admin { email: String },
}

/// `tpcloud users …` against the network (admins).
pub fn users(action: UsersRemote) -> Result<()> {
    let (remote, _) = connect()?;
    match action {
        UsersRemote::List => {
            for u in remote.get("/auth/admin/users")?.as_array().into_iter().flatten() {
                let mut tags = Vec::new();
                if u["is_admin"] == true {
                    tags.push("admin".to_string());
                }
                if u["totp_enabled"] == true {
                    tags.push("2FA".into());
                }
                if let Some(n) = u["active_session_count"].as_i64().filter(|n| *n > 0) {
                    tags.push(format!("{n} session(s)"));
                }
                println!("  {} {}  {}", if u["is_admin"] == true { ui::accent("●") } else { ui::dim("●") }, u["email"].as_str().unwrap_or(""), ui::dim(&tags.join("  ")));
            }
        }
        UsersRemote::Add { email, admin } => {
            let password = ui::secret("Password for the new account (12 characters or more)")?;
            let made = remote.send("POST", "/auth/admin/users", Some(json!({ "email": email, "password": password })))?;
            if admin {
                remote.send("PATCH", &format!("/auth/admin/users/{}", made["id"].as_i64().unwrap_or(0)), None)?;
            }
            ui::done(&format!("Account {email} made{}.", if admin { ", admin" } else { "" }));
        }
        UsersRemote::Remove { email } => {
            let (id, _) = user_id(&remote, &email)?;
            let answer = read_line(&format!("Delete {email} and everything of theirs? Type the email to confirm"), "")?;
            if answer.to_lowercase() != email.to_lowercase() {
                bail!("nothing was deleted");
            }
            remote.send("DELETE", &format!("/auth/admin/users/{id}"), None)?;
            ui::done(&format!("{email} deleted."));
        }
        UsersRemote::Admin { email } => {
            let (id, _) = user_id(&remote, &email)?;
            let v = remote.send("PATCH", &format!("/auth/admin/users/{id}"), None)?;
            ui::done(&format!("{email} is {}.", if v["is_admin"] == true { "an admin now" } else { "no longer an admin" }));
        }
    }
    Ok(())
}

/// `tpcloud sessions [--all] [revoke ID]`.
pub fn sessions(all: bool, revoke: Option<i64>) -> Result<()> {
    let (remote, _) = connect()?;
    if let Some(id) = revoke {
        if all {
            remote.send("DELETE", &format!("/auth/admin/sessions/{id}"), None)?;
        } else {
            remote.send("DELETE", &format!("/auth/account/session/{id}"), None)?;
        }
        ui::done(&format!("Session {id} closed."));
        return Ok(());
    }
    let list = if all { remote.get("/auth/admin/sessions")? } else { remote.get("/auth/account/profile")?["sessions"].clone() };
    for s in list.as_array().into_iter().flatten() {
        let who = s["user_email"].as_str().map(|e| format!("{e}  ")).unwrap_or_default();
        let device = s["device"]["label"].as_str().or(s["device_info"].as_str()).unwrap_or("?");
        println!(
            "  {:>5}  {}{}  {}{}",
            s["id"].as_i64().unwrap_or(0),
            who,
            ui::bold(device),
            ui::dim(&format!("{} · last seen {}", s["last_ip"].as_str().unwrap_or(""), s["last_seen_at"].as_str().unwrap_or("").get(..16).unwrap_or(""))),
            if s["current"] == true { ui::accent("  this one") } else { String::new() }
        );
    }
    Ok(())
}

/// `tpcloud host [check|update]` (admins): the host's version, and updating it from here.
pub fn host(action: Option<&str>) -> Result<()> {
    let (remote, n) = connect()?;
    match action {
        Some("check") => {
            remote.send("POST", "/auth/admin/host/check", None)?;
            ui::step("Looking for a newer TPCloud");
            std::thread::sleep(std::time::Duration::from_millis(3000));
        }
        Some("update") => {
            let h = remote.get("/auth/admin/host")?;
            if h["available"] != true {
                ui::done(&format!("{} has the latest TPCloud ({}).", n["name"].as_str().unwrap_or("The host"), h["version"].as_str().unwrap_or("")));
                return Ok(());
            }
            remote.send("POST", "/auth/admin/host/update", None)?;
            ui::step(&format!("Updating {} to TPCloud {}", n["name"].as_str().unwrap_or("the host"), h["latest"].as_str().unwrap_or("")));
            let mut last = String::new();
            for _ in 0..240 {
                std::thread::sleep(std::time::Duration::from_millis(1500));
                // While it restarts it doesn't answer: keep asking.
                let Ok(h) = remote.get("/auth/admin/host") else { continue };
                let state = h["update"]["state"].as_str().unwrap_or("").to_string();
                if state != last {
                    match state.as_str() {
                        "downloading" => ui::step("Downloading"),
                        "installing" => ui::step("Installing and restarting"),
                        "done" | "latest" => {
                            ui::done(&format!("Updated: TPCloud {}.", h["version"].as_str().unwrap_or("")));
                            return Ok(());
                        }
                        "failed" => bail!("the update failed: {}", h["update"]["detail"].as_str().unwrap_or("")),
                        _ => {}
                    }
                    last = state;
                }
            }
            bail!("it's taking long: `tpcloud host` shows how it's going");
        }
        _ => {}
    }
    let h = remote.get("/auth/admin/host")?;
    println!("{}  TPCloud {}", ui::bold(n["name"].as_str().unwrap_or("Host")), h["version"].as_str().unwrap_or(""));
    match h["update"]["state"].as_str() {
        Some(s @ ("checking" | "downloading" | "installing")) => println!("  {}", ui::accent(&format!("updating ({s})…"))),
        Some("failed") => println!("  {}", ui::warn_text(&format!("the last update failed: {}", h["update"]["detail"].as_str().unwrap_or("")))),
        _ if h["available"] == true => println!("  {}", ui::accent(&format!("TPCloud {} is out: `tpcloud host update`", h["latest"].as_str().unwrap_or("")))),
        _ => println!("  {}", ui::dim("the latest version")),
    }
    Ok(())
}

// ── Keeping this program up to date where it isn't a host ──

/// `tpcloud update` on a computer that isn't a host: the newest `tpcloud` for this system,
/// checked against GitHub's checksum, in place of this file.
pub fn self_update() -> Result<()> {
    let asset = if cfg!(target_os = "macos") {
        "tpcloud-cli-macos".to_string()
    } else {
        format!("tpcloud-cli-linux-{}", match std::env::consts::ARCH { "aarch64" => "arm64", "x86_64" => "amd64", other => other })
    };
    let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(300)).user_agent(&format!("tpcloud/{}", env!("CARGO_PKG_VERSION"))).build();
    let release = agent.get("https://api.github.com/repos/mtaibo/tpcloud/releases/latest").call()?;
    let release = net::json_of(release)?;
    let version = release["tag_name"].as_str().unwrap_or("").trim_start_matches('v').to_string();
    let newer = |a: &str, b: &str| {
        let p = |v: &str| v.split('.').map(|x| x.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
        p(a) > p(b)
    };
    if !newer(&version, env!("CARGO_PKG_VERSION")) {
        ui::done(&format!("tpcloud {} is the latest.", env!("CARGO_PKG_VERSION")));
        return Ok(());
    }
    let a = release["assets"].as_array().into_iter().flatten().find(|a| a["name"] == asset.as_str()).cloned().with_context(|| format!("the release has no {asset}"))?;
    ui::step(&format!("Downloading tpcloud {version}"));
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut agent.get(a["browser_download_url"].as_str().unwrap_or("")).call()?.into_reader(), &mut bytes)?;
    if let Some(expected) = a["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")) {
        if net::fingerprint(&bytes) != expected {
            bail!("the download doesn't match GitHub's checksum; nothing was changed");
        }
    }
    let me = std::env::current_exe()?.canonicalize()?;
    let tmp = me.with_extension("new");
    std::fs::write(&tmp, &bytes).with_context(|| format!("can't write next to {} (try with sudo)", me.display()))?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
    std::fs::rename(&tmp, &me)?;
    ui::done(&format!("tpcloud {version} installed."));
    Ok(())
}
