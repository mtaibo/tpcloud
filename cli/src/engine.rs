//! The setup itself, whoever asks the questions: the terminal (`setup.rs`), the setup page in a
//! browser or the apps (`setup_server.rs`). They gather `Answers`; `check` says what's wrong with
//! them and `apply` makes them true. Anything that needs the person halfway through (opening the
//! Cloudflare sign-in link, writing down the recovery key, replacing a DNS record) goes through
//! `Progress`, which each front end shows its own way.

use crate::{envfile, install, network, paths, storage, users};
use anyhow::{bail, Result};
use std::path::Path;

#[derive(Clone, Debug)]
pub enum Files {
    /// A folder on a disk that's already set up: never erased.
    Folder(String),
    /// A new pool on these empty disks (erased).
    Pool { disks: Vec<String>, level: storage::Level, encrypt: bool },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Domain {
    None,
    /// Through a Cloudflare Tunnel: one made from here (`new`), or the token already set.
    Cloudflare { hostname: String, new: bool },
    /// Port 443 forwarded to this machine; Caddy gets the certificate.
    OpenPort { hostname: String },
}

#[derive(Clone, Debug)]
pub struct Answers {
    pub name: String,
    pub files: Files,
    /// Empty: no nightly backups.
    pub backup_dir: String,
    pub store: bool,
    pub home: bool,
    pub local: bool,
    pub local_names: Vec<String>,
    /// This machine's Tailscale name, when it should answer there.
    pub tailscale: Option<String>,
    pub domain: Domain,
    /// The first account, when there's none yet: email and password.
    pub owner: Option<(String, String)>,
    /// Replace the hostname's DNS record if it points somewhere else (asked first).
    pub overwrite_dns: bool,
}

/// What the machine is like, for the questions' defaults and choices.
pub struct Facts {
    pub first: bool,
    pub host: String,
    pub disks: Vec<storage::Disk>,
    pub pool: Option<String>,
    pub tailscale: Option<String>,
    pub has_token: bool,
    pub needs_owner: bool,
    pub accounts: usize,
}

pub fn facts() -> Facts {
    let env = paths::env_file();
    let accounts = users::count().unwrap_or(0);
    Facts {
        first: !env.exists() || envfile::get(&env, "TPCLOUD_SETUP").as_deref() == Some("pending"),
        host: format!("{}.local", network::host_label()),
        disks: crate::platform::storage::disks().unwrap_or_default(),
        pool: storage::status_line(&env).map(|(l, _)| l),
        tailscale: tailscale_name().ok(),
        has_token: envfile::get(&env, "TUNNEL_TOKEN").is_some_and(|t| !t.is_empty()),
        needs_owner: accounts == 0,
        accounts,
    }
}

/// The answers the settings give now (a first setup's defaults otherwise).
pub fn current(f: &Facts) -> Answers {
    let env = paths::env_file();
    let get = |k: &str| envfile::get(&env, k).unwrap_or_default();
    let url = get("TPCLOUD_URL");
    let domain_host = url.trim_start_matches("https://").trim_end_matches('/').to_string();
    let domain = if !get(network::TUNNEL_HOSTNAME).is_empty() {
        Domain::Cloudflare { hostname: get(network::TUNNEL_HOSTNAME), new: true }
    } else if f.has_token && !f.first {
        Domain::Cloudflare { hostname: domain_host.clone(), new: false }
    } else if !f.first && !url.is_empty() && !url.contains(".ts.net") && !url.ends_with(".local") && !url.contains("example.com") {
        Domain::OpenPort { hostname: domain_host }
    } else {
        Domain::None
    };
    let modules = envfile::get_list(&env, "TPCLOUD_MODULES");
    Answers {
        name: Some(get(network::NAME)).filter(|n| !n.is_empty()).unwrap_or_else(network::default_name),
        files: Files::Folder(Some(get("EXTERNAL_DISK_PATH")).filter(|p| !p.is_empty() && !f.first).unwrap_or_else(|| "/var/lib/tpcloud/files".into())),
        backup_dir: get("TPCLOUD_BACKUP_DIR"),
        store: f.first || modules.iter().any(|m| m == "store"),
        home: modules.iter().any(|m| m == "home"),
        local: f.first || get(network::LOCAL) == "on",
        local_names: envfile::get_list(&env, network::LOCAL_NAMES),
        tailscale: Some(get(network::TAILSCALE)).filter(|t| !t.is_empty()),
        domain,
        owner: None,
        overwrite_dns: false,
    }
}

/// A problem with one answer: which (for the form to point at it) and what to say.
#[derive(Debug)]
pub struct Problem {
    pub field: &'static str,
    pub message: String,
}

fn problem(field: &'static str, message: impl Into<String>) -> Problem {
    Problem { field, message: message.into() }
}

pub fn check_name(name: &str) -> Option<String> {
    if name.trim().is_empty() {
        Some("Give it a name.".into())
    } else if name.chars().count() > 64 {
        Some("64 characters at most.".into())
    } else if name.contains(" #") || name.contains('\n') {
        Some("No # after a space.".into())
    } else {
        None
    }
}

pub fn check_backup_dir(dir: &str) -> Option<String> {
    if dir.is_empty() {
        None
    } else if !dir.starts_with('/') {
        Some("Give the full path, starting with /.".into())
    } else if Path::new(dir).starts_with(paths::state_dir()) {
        Some(format!("Not inside {}: it would copy itself.", paths::state_dir().display()))
    } else {
        None
    }
}

pub fn check_hostname(host: &str) -> Option<String> {
    let ok = host.contains('.') && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') && !host.starts_with('.') && !host.ends_with('.');
    (!ok).then(|| "That needs to be a full address, like cloud.example.com.".into())
}

pub fn check_owner(email: &str, password: &str) -> Option<(&'static str, String)> {
    if !(email.contains('@') && email.contains('.')) {
        Some(("owner_email", "That doesn't look like an email.".into()))
    } else if password.chars().count() < 12 {
        Some(("owner_password", "The password needs 12 characters or more.".into()))
    } else {
        None
    }
}

/// Everything wrong with these answers (empty: good to apply).
pub fn check(a: &Answers, f: &Facts) -> Vec<Problem> {
    let mut out = Vec::new();
    if let Some(m) = check_name(&a.name) {
        out.push(problem("name", m));
    }
    match &a.files {
        Files::Folder(p) if !p.starts_with('/') => out.push(problem("files", "Give the full path, starting with /.")),
        Files::Folder(_) => {}
        Files::Pool { disks, level, .. } => {
            if disks.is_empty() {
                out.push(problem("files", "Pick at least one disk."));
            }
            for d in disks {
                match f.disks.iter().find(|x| &x.path == d) {
                    None => out.push(problem("files", format!("{d} isn't a disk here."))),
                    Some(x) if x.busy.is_some() => out.push(problem("files", format!("{d} can't be used: {}.", x.busy.as_deref().unwrap_or("")))),
                    _ => {}
                }
            }
            if !storage::levels_for(disks.len()).contains(level) {
                out.push(problem("files", format!("{} needs more disks.", level.name())));
            }
        }
    }
    if let Some(m) = check_backup_dir(&a.backup_dir) {
        out.push(problem("backup_dir", m));
    }
    for n in &a.local_names {
        if let Some(m) = check_hostname(n) {
            out.push(problem("local_names", m));
        }
    }
    match &a.domain {
        Domain::Cloudflare { hostname, .. } | Domain::OpenPort { hostname } => {
            if let Some(m) = check_hostname(hostname) {
                out.push(problem("domain", m));
            }
        }
        Domain::None => {}
    }
    if let Domain::Cloudflare { new: false, .. } = a.domain {
        if !f.has_token {
            out.push(problem("domain", "There's no tunnel token here: make a new tunnel."));
        }
    }
    if !a.local && a.tailscale.is_none() && a.domain == Domain::None {
        out.push(problem("reach", "Then nothing could reach it: pick at least one way."));
    }
    match &a.owner {
        Some((email, password)) => {
            if let Some((field, m)) = check_owner(email, password) {
                out.push(problem(field, m));
            }
        }
        None if f.needs_owner => out.push(problem("owner_email", "The first account is needed.")),
        None => {}
    }
    out
}

/// How a front end shows what happens while the answers are applied.
pub trait Progress {
    fn step(&mut self, text: &str);
    fn done(&mut self, text: &str);
    /// The Cloudflare sign-in link: the person opens it on any device.
    fn sign_in(&mut self, url: &str);
    /// The encrypted pool's recovery key, shown once.
    fn recovery_key(&mut self, key: &str);
    /// The hostname has a DNS record already: replace it? (Front ends that can't ask return
    /// `Answers::overwrite_dns`.)
    fn replace_dns(&mut self, hostname: &str) -> bool;
}

pub struct Outcome {
    pub url: String,
    pub others: Vec<String>,
}

/// Makes the answers true. The steps most likely to stop (erasing disks, the Cloudflare sign-in)
/// come first, so nothing else has changed when they do.
pub fn apply(a: &Answers, p: &mut dyn Progress) -> Result<Outcome> {
    install::require_root("Setting TPCloud up")?;
    std::fs::create_dir_all(paths::state_dir())?;
    let env = paths::env_file();
    if !env.exists() {
        std::fs::write(&env, crate::setup::default_env())?;
    }
    install::restrict(&env)?;
    let f = facts();
    let problems = check(a, &f);
    if let Some(first) = problems.first() {
        bail!("{}", first.message);
    }
    let set = |k: &str, v: &str| envfile::set(&env, k, v);
    set(network::NAME, &a.name)?;
    network::ensure(&env)?;

    let storage_dir = match &a.files {
        Files::Folder(path) => path.clone(),
        Files::Pool { disks, level, encrypt } => {
            p.step("Making the storage pool");
            let made = crate::platform::storage::create_pool(disks, *level, *encrypt, Path::new(storage::DEFAULT_MOUNT))?;
            set(storage::POOL, storage::DEFAULT_MOUNT)?;
            set(storage::POOL_UUID, &made.uuid)?;
            envfile::set_list(&env, storage::POOL_LUKS, &made.luks)?;
            p.done(&format!("Storage ready at {} ({}).", storage::DEFAULT_MOUNT, level.name()));
            if let Some(key) = made.recovery {
                p.recovery_key(&key);
            }
            format!("{}/files", storage::DEFAULT_MOUNT)
        }
    };
    std::fs::create_dir_all(&storage_dir)?;

    match &a.domain {
        Domain::Cloudflare { hostname, new: true } => {
            p.step("Signing in to Cloudflare");
            let id = envfile::get(&env, network::ID).unwrap_or_default();
            crate::cloudflare::login(&mut |url| p.sign_in(url))?;
            let tunnel_id = crate::cloudflare::create(&format!("tpcloud-{}", &id[..id.len().min(8)]), envfile::get(&env, network::TUNNEL_ID).as_deref())?;
            // A domain the sign-in didn't pick fails here: forget that sign-in, so the next try
            // asks for the right one.
            let routed = crate::cloudflare::route(&tunnel_id, hostname, a.overwrite_dns).map_err(|err| {
                crate::cloudflare::logout();
                err.context("sign in to Cloudflare again, picking that address's domain")
            })?;
            if let crate::cloudflare::Route::Taken = routed {
                if !p.replace_dns(hostname) {
                    bail!("{hostname} already points somewhere else and was left as it was: pick another address");
                }
                crate::cloudflare::route(&tunnel_id, hostname, true)?;
            }
            set(network::TUNNEL_ID, &tunnel_id)?;
            set(network::TUNNEL_HOSTNAME, hostname)?;
            p.done(&format!("Tunnel ready: {hostname} → this machine."));
        }
        _ => set(network::TUNNEL_HOSTNAME, "")?,
    }

    // The main address (links, sign-in, passkeys): the domain, else Tailscale's, else home's.
    let host = &f.host;
    let (url, site, tls, https_publish) = match &a.domain {
        Domain::Cloudflare { hostname, .. } => (format!("https://{hostname}"), format!("http://{hostname}"), "none", ""),
        Domain::OpenPort { hostname } => (format!("https://{hostname}"), hostname.clone(), "none", "443"),
        Domain::None => match &a.tailscale {
            Some(name) => (format!("https://{name}"), format!("https://{name}"), "none", "443"),
            None => (format!("https://{host}"), format!("https://{host}"), "internal", "443"),
        },
    };
    let mut modules = Vec::new();
    let mut profiles = Vec::new();
    if a.store {
        modules.push("store".to_string());
        profiles.push("store".to_string());
    }
    if a.home {
        modules.push("home".to_string());
    }
    if matches!(a.domain, Domain::Cloudflare { .. }) {
        profiles.push("tunnel".to_string());
    }
    set(network::LOCAL, if a.local { "on" } else { "off" })?;
    envfile::set_list(&env, network::LOCAL_NAMES, &a.local_names)?;
    set(network::TAILSCALE, a.tailscale.as_deref().unwrap_or(""))?;
    set("TPCLOUD_URL", &url)?;
    set("TPCLOUD_SITE", &site)?;
    set("TPCLOUD_TLS", tls)?;
    set("TPCLOUD_HTTPS_PUBLISH", https_publish)?;
    set("TPCLOUD_OTA_PUBLISH", if a.home { "80" } else { "" })?;
    envfile::set_list(&env, "TPCLOUD_MODULES", &modules)?;
    envfile::set_list(&env, "COMPOSE_PROFILES", &profiles)?;
    set("EXTERNAL_DISK_PATH", &storage_dir)?;
    set("TPCLOUD_BACKUP_DIR", &a.backup_dir)?;
    if a.home {
        let home_env = paths::home_env_file();
        if !home_env.exists() {
            std::fs::create_dir_all(home_env.parent().unwrap())?;
            std::fs::write(&home_env, crate::setup::default_home_env())?;
        }
        install::restrict(&home_env)?;
    }
    install::sync_links()?;
    install::write_unit()?;
    if a.tailscale.is_some() {
        // The gateway answers on Tailscale itself; a `tailscale serve` on 443 from an older setup
        // would take the port from it.
        let _ = std::process::Command::new("tailscale")
            .args(["serve", "--https=443", "off"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    if let Some((email, password)) = &a.owner {
        if users::count().unwrap_or(0) == 0 {
            p.step("Making your account");
            users::create_owner(email, password)?;
        }
    }
    set("TPCLOUD_SETUP", "done")?;
    let others = network::addresses(&crate::services::Settings::load()).into_iter().filter(|x| *x != url).collect();
    Ok(Outcome { url, others })
}

pub fn tailscale_name() -> Result<String> {
    let out = std::process::Command::new("tailscale").args(["status", "--json"]).output();
    let Ok(out) = out else { bail!("Tailscale isn't installed here: install it and sign in first") };
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
    match json["Self"]["DNSName"].as_str() {
        Some(name) if !name.is_empty() => Ok(name.trim_end_matches('.').to_string()),
        _ => bail!("Tailscale isn't signed in here: run `sudo tailscale up` first"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> Facts {
        Facts { first: true, host: "box.local".into(), disks: vec![], pool: None, tailscale: None, has_token: false, needs_owner: true, accounts: 0 }
    }

    fn answers() -> Answers {
        Answers {
            name: "Casa".into(),
            files: Files::Folder("/srv/files".into()),
            backup_dir: String::new(),
            store: true,
            home: false,
            local: true,
            local_names: vec![],
            tailscale: None,
            domain: Domain::None,
            owner: Some(("me@example.com".into(), "correct-horse-battery".into())),
            overwrite_dns: false,
        }
    }

    #[test]
    fn good_answers_pass() {
        assert!(check(&answers(), &facts()).is_empty());
    }

    #[test]
    fn each_problem_names_its_field() {
        let mut a = answers();
        a.name = "a #b".into();
        a.local = false;
        a.owner = Some(("nope".into(), "short".into()));
        a.files = Files::Pool { disks: vec!["/dev/sdz".into()], level: storage::Level::Raid1, encrypt: true };
        let fields: Vec<&str> = check(&a, &facts()).iter().map(|p| p.field).collect();
        assert_eq!(fields, vec!["name", "files", "files", "reach", "owner_email"]);
    }

    #[test]
    fn a_token_tunnel_needs_a_token() {
        let mut a = answers();
        a.domain = Domain::Cloudflare { hostname: "cloud.example.com".into(), new: false };
        assert_eq!(check(&a, &facts())[0].field, "domain");
    }
}
