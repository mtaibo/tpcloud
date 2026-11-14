//! What's running: the supervisor's own report (run/status.json) and systemd's view of it.

use crate::{backup, envfile, install, paths, runtime, ui, update};
use anyhow::Result;
use std::process::Command;

#[derive(Clone)]
pub struct ProcessState {
    pub name: String,
    pub running: bool,
    pub since: u64,
    pub restarts: u64,
    pub last_exit: Option<String>,
}

#[derive(Clone)]
pub struct Status {
    pub installed: bool,
    pub active: bool,
    pub version: Option<String>,
    pub url: String,
    pub modules: Vec<String>,
    pub files: String,
    pub processes: Vec<ProcessState>,
    /// Where backups go and how the last one went, as one line.
    pub backup: String,
    pub backup_ok: bool,
    /// A newer release the service found on GitHub.
    pub update: Option<String>,
}

pub fn read() -> Status {
    let env = paths::env_file();
    let active = Command::new("systemctl")
        .args(["is-active", "--quiet", "tpcloud"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let report: serde_json::Value = std::fs::read_to_string(paths::status_file())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    let processes = report["services"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|p| ProcessState {
                    name: p["name"].as_str().unwrap_or("?").to_string(),
                    running: active && p["running"].as_bool().unwrap_or(false),
                    since: p["since"].as_u64().unwrap_or(0),
                    restarts: p["restarts"].as_u64().unwrap_or(0),
                    last_exit: p["last_exit"].as_str().map(str::to_string),
                })
                .collect()
        })
        .unwrap_or_default();
    let (backup, backup_ok) = backup_line();
    Status {
        installed: env.exists() || paths::runtime_dir().exists(),
        active,
        version: runtime::installed_version(),
        url: envfile::get(&env, "TPCLOUD_URL").unwrap_or_default(),
        modules: install::modules(),
        files: envfile::get(&env, "EXTERNAL_DISK_PATH").unwrap_or_default(),
        processes,
        backup,
        backup_ok,
        update: update::available(),
    }
}

fn backup_line() -> (String, bool) {
    let Some(dir) = backup::dir() else { return ("off (`sudo tpcloud backup --to <folder>`)".into(), false) };
    match backup::last() {
        Some(l) if l.ok => (format!("{} ago → {}", ago(l.at), dir.display()), true),
        Some(l) => (format!("failed {} ago: {}", ago(l.at), l.detail), false),
        None => (format!("none yet → {}", dir.display()), true),
    }
}

pub fn ago(since: u64) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let s = now.saturating_sub(since);
    match s {
        0..=59 => format!("{s} s"),
        60..=3599 => format!("{} min", s / 60),
        3600..=86399 => format!("{} h {} min", s / 3600, s % 3600 / 60),
        _ => format!("{} d {} h", s / 86400, s % 86400 / 3600),
    }
}

pub fn print() -> Result<()> {
    let st = read();
    if !st.installed {
        println!("TPCloud isn't installed here yet: `sudo ./tpcloud install`.");
        return Ok(());
    }
    println!(
        "{} {}  {}  {}",
        ui::bold("TPCloud"),
        st.version.as_deref().unwrap_or("?"),
        ui::accent(if st.url.is_empty() { "(no address set)" } else { &st.url }),
        if st.active { ui::green("running") } else { ui::dim("stopped") }
    );
    println!("{}  {}", ui::dim("Modules"), if st.modules.is_empty() { "none".into() } else { st.modules.join(", ") });
    if !st.files.is_empty() {
        println!("{}  {}", ui::dim("Files  "), st.files);
    }
    println!("{}  {}", ui::dim("Data   "), paths::state_dir().display());
    println!("{}  {}", ui::dim("Backup "), if st.backup_ok { st.backup.clone() } else { ui::warn_text(&st.backup) });
    if let Some(v) = &st.update {
        println!("{}  {}", ui::dim("Update "), ui::accent(&format!("TPCloud {v} is out: `sudo tpcloud update`")));
    }
    println!();
    if st.processes.is_empty() || !st.active {
        println!("  Nothing is running: `sudo tpcloud start`.");
        return Ok(());
    }
    for p in &st.processes {
        let dot = if p.running { ui::green("●") } else { ui::warn_dot() };
        let detail = if p.running {
            format!("up {}{}", ago(p.since), if p.restarts > 0 { format!(", restarted {}×", p.restarts) } else { String::new() })
        } else {
            format!("down{}", p.last_exit.as_deref().map(|e| format!(" ({e})")).unwrap_or_default())
        };
        println!("  {dot} {:<12} {}", p.name, ui::dim(&detail));
    }
    Ok(())
}
