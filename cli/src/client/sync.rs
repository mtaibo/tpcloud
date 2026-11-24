//! Folders on this computer kept the same as folders in the cloud, both ways, like Dropbox:
//! `tpcloud sync add ~/TPCloud` and from then on a user service (systemd on Linux, launchd on a
//! Mac) runs `tpcloud sync run`, which looks every few seconds for changes here and every half
//! minute for changes there.
//!
//! Each folder remembers, per file, its size and time here and there after the last time both
//! matched. Comparing now with that tells who changed what: a change on one side is copied to
//! the other, a deletion is too, and a file changed on both sides keeps both (the copy from here
//! renamed «name (conflict date).ext»), so nothing is ever lost. Copies keep the file's time on
//! both sides (the server takes it with the upload), which is what makes the next comparison
//! work. Hidden files are left alone (the server doesn't show them either).

use super::net::{query_value, Remote};
use crate::ui;
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const API: &str = "/store/api/files";
/// How often the service looks at the server when nothing changed here.
const REMOTE_EVERY: Duration = Duration::from_secs(30);
/// How often it looks here.
const LOCAL_EVERY: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, PartialEq, Debug)]
struct Meta {
    size: u64,
    mtime: i64,
}

// ── Settings: which folders, and what each one looked like last time ──

#[derive(Clone, Debug)]
pub struct Pair {
    pub id: String,
    pub local: PathBuf,
    pub remote: String,
    pub network: String,
    pub network_name: String,
    pub paused: bool,
}

fn sync_dir() -> PathBuf {
    super::config_dir().join("sync")
}

fn settings_file() -> PathBuf {
    super::config_dir().join("sync.json")
}

fn state_file(id: &str) -> PathBuf {
    sync_dir().join(format!("{id}.json"))
}

fn status_file() -> PathBuf {
    sync_dir().join("status.json")
}

fn write_private(path: &Path, text: &str) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(path.parent().unwrap())?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text)?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

pub fn load_pairs() -> Vec<Pair> {
    let v: Value = std::fs::read_to_string(settings_file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null);
    v["folders"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| {
            Some(Pair {
                id: f["id"].as_str()?.to_string(),
                local: PathBuf::from(f["local"].as_str()?),
                remote: f["remote"].as_str()?.to_string(),
                network: f["network"].as_str()?.to_string(),
                network_name: f["network_name"].as_str().unwrap_or("").to_string(),
                paused: f["paused"] == true,
            })
        })
        .collect()
}

fn save_pairs(pairs: &[Pair]) -> Result<()> {
    let folders: Vec<Value> = pairs
        .iter()
        .map(|p| json!({"id": p.id, "local": p.local, "remote": p.remote, "network": p.network, "network_name": p.network_name, "paused": p.paused}))
        .collect();
    write_private(&settings_file(), &serde_json::to_string_pretty(&json!({ "folders": folders }))?)
}

#[derive(Default)]
struct State {
    /// path → (here, there) when both last matched.
    files: BTreeMap<String, (Meta, Meta)>,
    dirs: BTreeSet<String>,
}

fn load_state(id: &str) -> State {
    let v: Value = std::fs::read_to_string(state_file(id)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null);
    let mut state = State::default();
    for (path, m) in v["files"].as_object().into_iter().flatten() {
        let n = |i: usize| m[i].as_i64().unwrap_or(-1);
        state.files.insert(path.clone(), (Meta { size: n(0) as u64, mtime: n(1) }, Meta { size: n(2) as u64, mtime: n(3) }));
    }
    state.dirs = v["dirs"].as_array().into_iter().flatten().filter_map(|d| d.as_str().map(str::to_string)).collect();
    state
}

fn save_state(id: &str, state: &State) -> Result<()> {
    let files: serde_json::Map<String, Value> = state.files.iter().map(|(p, (l, r))| (p.clone(), json!([l.size, l.mtime, r.size, r.mtime]))).collect();
    write_private(&state_file(id), &serde_json::to_string(&json!({ "files": files, "dirs": state.dirs }))?)
}

// ── What each side has now ──

fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn mtime_of(meta: &std::fs::Metadata) -> i64 {
    meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn hidden(name: &str) -> bool {
    name.starts_with('.')
}

fn scan_local(root: &Path) -> Result<(BTreeMap<String, Meta>, BTreeSet<String>)> {
    let mut files = BTreeMap::new();
    let mut dirs = BTreeSet::new();
    let mut stack = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, rel)) = stack.pop() {
        let entries = std::fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))?;
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if hidden(&name) {
                continue;
            }
            let Ok(meta) = entry.path().symlink_metadata() else { continue };
            let path = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            if meta.is_dir() {
                dirs.insert(path.clone());
                stack.push((entry.path(), path));
            } else if meta.is_file() {
                files.insert(path, Meta { size: meta.len(), mtime: mtime_of(&meta) });
            }
        }
    }
    Ok((files, dirs))
}

fn scan_remote(remote: &Remote, root: &str) -> Result<(BTreeMap<String, Meta>, BTreeSet<String>)> {
    let tree = match remote.get(&format!("{API}/tree?path={}", query_value(root))) {
        Ok(t) => t,
        Err(err) if err.to_string() == "Not Found" => bail!("the server needs TPCloud 0.4.8 or newer for synced folders: update it (Server → update)"),
        Err(err) => return Err(err),
    };
    let mut files = BTreeMap::new();
    for f in tree["files"].as_array().into_iter().flatten() {
        let (Some(path), Some(size)) = (f["path"].as_str(), f["size"].as_u64()) else { continue };
        files.insert(path.to_string(), Meta { size, mtime: f["modified"].as_f64().unwrap_or(0.0).floor() as i64 });
    }
    let dirs = tree["dirs"].as_array().into_iter().flatten().filter_map(|d| d.as_str().map(str::to_string)).collect();
    Ok((files, dirs))
}

// ── One pass: compare, then copy what changed ──

#[derive(Default, Clone, Copy)]
pub struct Report {
    pub up: usize,
    pub down: usize,
    pub deleted_here: usize,
    pub deleted_there: usize,
    pub conflicts: usize,
}

impl Report {
    fn anything(&self) -> bool {
        self.up + self.down + self.deleted_here + self.deleted_there + self.conflicts > 0
    }

    pub fn describe(&self) -> String {
        let mut parts = Vec::new();
        let mut add = |n: usize, what: &str| {
            if n > 0 {
                parts.push(format!("{n} {what}"));
            }
        };
        add(self.up, "sent");
        add(self.down, "received");
        add(self.deleted_there, "deleted there");
        add(self.deleted_here, "deleted here");
        add(self.conflicts, "kept twice (changed on both sides)");
        if parts.is_empty() {
            "up to date".into()
        } else {
            parts.join(", ")
        }
    }
}

enum Op {
    Up(String),
    Down(String),
    DeleteHere(String),
    DeleteThere(String),
    Conflict(String),
    Same(String, Meta, Meta),
    Forget(String),
}

/// What changed, decided from now (here, there) against the last match.
fn plan(local: &BTreeMap<String, Meta>, remote: &BTreeMap<String, Meta>, state: &State) -> Vec<Op> {
    let recent = now_secs() - 2; // still being written: next time
    let mut paths: BTreeSet<&String> = local.keys().collect();
    paths.extend(remote.keys());
    paths.extend(state.files.keys());
    let close = |a: &Meta, b: &Meta| a.size == b.size && (a.mtime - b.mtime).abs() <= 1;
    let mut ops = Vec::new();
    for path in paths {
        let (l, r, s) = (local.get(path), remote.get(path), state.files.get(path));
        if l.is_some_and(|l| l.mtime > recent) {
            continue;
        }
        let p = path.clone();
        ops.push(match (l, r, s) {
            (Some(l), Some(r), Some((sl, sr))) => match (l != sl, r != sr) {
                (false, false) => continue,
                (true, false) => Op::Up(p),
                (false, true) => Op::Down(p),
                (true, true) if close(l, r) => Op::Same(p, *l, *r),
                (true, true) => Op::Conflict(p),
            },
            (Some(l), Some(r), None) if close(l, r) => Op::Same(p, *l, *r),
            (Some(_), Some(_), None) => Op::Conflict(p),
            (Some(l), None, Some((sl, _))) if l != sl => Op::Up(p),
            (Some(_), None, Some(_)) => Op::DeleteHere(p),
            (Some(_), None, None) => Op::Up(p),
            (None, Some(r), Some((_, sr))) if r != sr => Op::Down(p),
            (None, Some(_), Some(_)) => Op::DeleteThere(p),
            (None, Some(_), None) => Op::Down(p),
            (None, None, _) => Op::Forget(p),
        });
    }
    ops
}

fn join_remote(root: &str, rel: &str) -> String {
    if rel.is_empty() {
        root.to_string()
    } else {
        format!("{}/{rel}", root.trim_end_matches('/'))
    }
}

fn parent_of(rel: &str) -> &str {
    rel.rsplit_once('/').map_or("", |(p, _)| p)
}

fn set_mtime(path: &Path, mtime: i64) -> Result<()> {
    let when = UNIX_EPOCH + Duration::from_secs(mtime.max(0) as u64);
    std::fs::File::options().write(true).open(path)?.set_modified(when)?;
    Ok(())
}

/// «photo (conflict 2026-11-24 18.30 desktop).jpg», next to the original.
fn conflict_name(rel: &str) -> String {
    let (dir, name) = rel.rsplit_once('/').map_or(("", rel), |(d, n)| (d, n));
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s, format!(".{e}")),
        _ => (name, String::new()),
    };
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let t = now_secs() as libc::time_t;
    unsafe { libc::localtime_r(&t, &mut tm) };
    let host = std::fs::read_to_string("/etc/hostname").ok().map(|h| h.trim().to_string()).filter(|h| !h.is_empty()).unwrap_or_else(|| "this computer".into());
    let new = format!("{stem} (conflict {:04}-{:02}-{:02} {:02}.{:02} {host}){ext}", tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday, tm.tm_hour, tm.tm_min);
    if dir.is_empty() {
        new
    } else {
        format!("{dir}/{new}")
    }
}

/// Holds the folder for one pass: the service and `tpcloud sync now` never copy at once.
struct Lock(std::fs::File);

impl Lock {
    fn take(id: &str) -> Option<Lock> {
        use std::os::unix::io::AsRawFd;
        std::fs::create_dir_all(sync_dir()).ok()?;
        let file = std::fs::File::create(sync_dir().join(format!("{id}.lock"))).ok()?;
        (unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0).then_some(Lock(file))
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        use std::os::unix::io::AsRawFd;
        unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}

pub fn pass(pair: &Pair, remote: &Remote, progress: &mut dyn FnMut(usize, usize, &str)) -> Result<Report> {
    let Some(_lock) = Lock::take(&pair.id) else { bail!("it's being synced already") };
    if !pair.local.is_dir() {
        bail!("{} isn't there any more (moved, or a disk that isn't connected)", pair.local.display());
    }
    let mut state = load_state(&pair.id);
    let (local, local_dirs) = scan_local(&pair.local)?;
    let (mut there, there_dirs) = scan_remote(remote, &pair.remote)?;
    let ops = plan(&local, &there, &state);

    // A sudden mass deletion is more likely a mistake (an emptied disk, a wrong folder) than
    // wanted: stop and say so instead of copying it to the other side.
    let deletions = ops.iter().filter(|o| matches!(o, Op::DeleteHere(_) | Op::DeleteThere(_))).count();
    if deletions > 20 && deletions * 2 > state.files.len() {
        bail!("{deletions} of {} files would be deleted at once; nothing was touched. If that's what you want: `tpcloud sync remove` the folder and add it again", state.files.len());
    }

    let mut report = Report::default();
    let total = ops.iter().filter(|o| !matches!(o, Op::Same(..) | Op::Forget(_))).count();
    let mut done = 0;
    let mut since_save = 0;
    let here = |rel: &str| pair.local.join(rel);
    let upload = |rel: &str, file: &Path, mtime: i64| -> Result<()> {
        let dir = join_remote(&pair.remote, parent_of(rel));
        remote.upload_file(&format!("{API}/upload?path={}&mtime={mtime}", query_value(&dir)), "files", file)?;
        Ok(())
    };
    let download = |rel: &str, dest: &Path, mtime: i64| -> Result<Meta> {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        remote.download_to(&format!("{API}/download?path={}", query_value(&join_remote(&pair.remote, rel))), dest)?;
        set_mtime(dest, mtime)?;
        let meta = std::fs::metadata(dest)?;
        Ok(Meta { size: meta.len(), mtime: mtime_of(&meta) })
    };
    let result = (|| -> Result<()> {
        for op in &ops {
            if !matches!(op, Op::Same(..) | Op::Forget(_)) {
                done += 1;
            }
            match op {
                Op::Same(p, l, r) => {
                    state.files.insert(p.clone(), (*l, *r));
                }
                Op::Forget(p) => {
                    state.files.remove(p);
                }
                Op::Up(p) => {
                    progress(done, total, &format!("↑ {p}"));
                    let l = local[p];
                    upload(p, &here(p), l.mtime)?;
                    there.insert(p.clone(), l);
                    state.files.insert(p.clone(), (l, l));
                    report.up += 1;
                }
                Op::Down(p) => {
                    progress(done, total, &format!("↓ {p}"));
                    let r = there[p];
                    let l = download(p, &here(p), r.mtime)?;
                    state.files.insert(p.clone(), (l, r));
                    report.down += 1;
                }
                Op::DeleteThere(p) => {
                    progress(done, total, &format!("× {p}"));
                    remote.send("DELETE", &format!("{API}?path={}", query_value(&join_remote(&pair.remote, p))), None).or_else(|e| if e.to_string() == "Not found" { Ok(Value::Null) } else { Err(e) })?;
                    there.remove(p);
                    state.files.remove(p);
                    report.deleted_there += 1;
                }
                Op::DeleteHere(p) => {
                    progress(done, total, &format!("× {p}"));
                    match std::fs::remove_file(here(p)) {
                        Ok(()) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                        Err(e) => return Err(e.into()),
                    }
                    state.files.remove(p);
                    report.deleted_here += 1;
                }
                Op::Conflict(p) => {
                    progress(done, total, &format!("⇅ {p}"));
                    // Ours under a new name (sent too), theirs under the real one.
                    let copy = conflict_name(p);
                    std::fs::rename(here(p), here(&copy))?;
                    let meta = std::fs::metadata(here(&copy))?;
                    let ours = Meta { size: meta.len(), mtime: mtime_of(&meta) };
                    upload(&copy, &here(&copy), ours.mtime)?;
                    state.files.insert(copy.clone(), (ours, ours));
                    there.insert(copy, ours);
                    let r = there[p];
                    let l = download(p, &here(p), r.mtime)?;
                    state.files.insert(p.clone(), (l, r));
                    report.conflicts += 1;
                }
            }
            since_save += 1;
            if since_save >= 25 {
                save_state(&pair.id, &state)?;
                since_save = 0;
            }
        }

        // Folders: new ones made on the other side, deleted ones deleted there once empty.
        let (_, now_local_dirs) = scan_local(&pair.local)?;
        let mut there_dirs = there_dirs.clone();
        for d in now_local_dirs.difference(&there_dirs.clone()) {
            if !state.dirs.contains(d) {
                remote.send("POST", &format!("{API}/mkdir"), Some(json!({"path": join_remote(&pair.remote, d)}))).ok();
                there_dirs.insert(d.clone());
            }
        }
        for d in there_dirs.difference(&now_local_dirs) {
            if !state.dirs.contains(d) && !local_dirs.contains(d) {
                std::fs::create_dir_all(here(d))?;
            }
        }
        let has_files = |files: &BTreeMap<String, Meta>, d: &str| files.keys().any(|f| f.starts_with(&format!("{d}/")));
        let (now_local, mut now_local_dirs) = scan_local(&pair.local)?;
        // Deepest first, so a folder's empty subfolders go before it.
        let mut gone_here: Vec<&String> = state.dirs.iter().filter(|d| !now_local_dirs.contains(*d) && there_dirs.contains(*d)).collect();
        gone_here.sort_by_key(|d| std::cmp::Reverse(d.len()));
        for d in gone_here {
            if !has_files(&there, d) {
                remote.send("DELETE", &format!("{API}?path={}", query_value(&join_remote(&pair.remote, d))), None).ok();
                there_dirs.remove(d);
            }
        }
        let mut gone_there: Vec<&String> = state.dirs.iter().filter(|d| !there_dirs.contains(*d) && now_local_dirs.contains(*d)).collect();
        gone_there.sort_by_key(|d| std::cmp::Reverse(d.len()));
        for d in gone_there {
            if !has_files(&now_local, d) && std::fs::remove_dir(here(d)).is_ok() {
                now_local_dirs.remove(d);
            }
        }
        state.dirs = now_local_dirs.intersection(&there_dirs).cloned().collect();
        Ok(())
    })();
    save_state(&pair.id, &state)?;
    result.map(|_| report)
}

// ── Status, for `tpcloud sync` and the terminal app ──

fn record_status(id: &str, outcome: &Result<Report>) {
    let path = status_file();
    let mut all: Value = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_else(|| json!({}));
    let previous = all[id].clone();
    all[id] = match outcome {
        Ok(r) => json!({
            "at": now_secs(),
            "ok": true,
            "last": if r.anything() { json!(r.describe()) } else { previous["last"].clone() },
            "last_at": if r.anything() { json!(now_secs()) } else { previous["last_at"].clone() },
        }),
        Err(e) => json!({"at": now_secs(), "ok": false, "error": format!("{e:#}"), "last": previous["last"], "last_at": previous["last_at"]}),
    };
    let _ = write_private(&path, &all.to_string());
}

pub fn status_of(id: &str) -> Value {
    let all: Value = std::fs::read_to_string(status_file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null);
    all[id].clone()
}

pub fn ago(secs: i64) -> String {
    let d = (now_secs() - secs).max(0);
    match d {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", d / 60),
        3600..=86399 => format!("{} h ago", d / 3600),
        _ => format!("{} days ago", d / 86400),
    }
}

/// Where the terminal app puts downloads: ~/Downloads (or the XDG one, or home).
pub fn downloads_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_DOWNLOAD_DIR") {
        return PathBuf::from(dir);
    }
    let user_dirs = std::fs::read_to_string(home().join(".config/user-dirs.dirs")).unwrap_or_default();
    for line in user_dirs.lines() {
        if let Some(v) = line.strip_prefix("XDG_DOWNLOAD_DIR=") {
            return PathBuf::from(v.trim_matches('"').replace("$HOME", &home().to_string_lossy()));
        }
    }
    let dl = home().join("Downloads");
    if dl.is_dir() || !home().join("Descargas").is_dir() {
        dl
    } else {
        home().join("Descargas")
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

/// ~/TPCloud rather than /home/someone/TPCloud.
pub fn pretty(path: &Path) -> String {
    match path.strip_prefix(home()) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".into(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

/// What a folder in the cloud is called for people: «your files», «shared», or its path.
pub fn remote_label(remote: &str) -> String {
    let parts: Vec<&str> = remote.split('/').collect();
    match parts.as_slice() {
        ["users", _] => "your files".into(),
        ["users", _, rest @ ..] => format!("your files/{}", rest.join("/")),
        ["shared"] => "shared".into(),
        _ => remote.to_string(),
    }
}

// ── The service that keeps them synced ──

const UNIT: &str = "tpcloud-sync.service";
const AGENT: &str = "com.migueltaibo.tpcloud.sync";

fn unit_file() -> PathBuf {
    let config = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".config"));
    config.join("systemd/user").join(UNIT)
}

fn agent_file() -> PathBuf {
    home().join("Library/LaunchAgents").join(format!("{AGENT}.plist"))
}

fn quiet(cmd: &str, args: &[&str]) -> bool {
    std::process::Command::new(cmd).args(args).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().is_ok_and(|s| s.success())
}

fn uid() -> u32 {
    unsafe { libc::getuid() }
}

/// Starts (or restarts, to pick up the folders) the service, made to start on its own at login.
fn start_service() -> Result<()> {
    let exe = std::env::current_exe()?.canonicalize()?;
    if cfg!(target_os = "macos") {
        let plist = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
             <plist version=\"1.0\"><dict>\n<key>Label</key><string>{AGENT}</string>\n\
             <key>ProgramArguments</key><array><string>{}</string><string>sync</string><string>run</string></array>\n\
             <key>RunAtLoad</key><true/>\n<key>KeepAlive</key><dict><key>SuccessfulExit</key><false/></dict>\n\
             <key>ProcessType</key><string>Background</string>\n</dict></plist>\n",
            exe.display()
        );
        let file = agent_file();
        std::fs::create_dir_all(file.parent().unwrap())?;
        std::fs::write(&file, plist)?;
        let target = format!("gui/{}", uid());
        quiet("launchctl", &["bootout", &format!("{target}/{AGENT}")]);
        if !quiet("launchctl", &["bootstrap", &target, &file.to_string_lossy()]) {
            bail!("launchd didn't take the sync agent ({})", file.display());
        }
    } else {
        let unit = format!(
            "[Unit]\nDescription=TPCloud: synced folders\nAfter=network-online.target\n\n\
             [Service]\nExecStart={} sync run\nRestart=on-failure\nRestartSec=15\n\n\
             [Install]\nWantedBy=default.target\n",
            exe.display()
        );
        let file = unit_file();
        std::fs::create_dir_all(file.parent().unwrap())?;
        std::fs::write(&file, unit)?;
        quiet("systemctl", &["--user", "daemon-reload"]);
        if !quiet("systemctl", &["--user", "enable", UNIT]) || !quiet("systemctl", &["--user", "restart", UNIT]) {
            bail!("systemd didn't start the sync service (`systemctl --user status {UNIT}` says why)");
        }
    }
    Ok(())
}

pub fn service_running() -> bool {
    if cfg!(target_os = "macos") {
        quiet("launchctl", &["print", &format!("gui/{}/{AGENT}", uid())])
    } else {
        quiet("systemctl", &["--user", "is-active", "--quiet", UNIT])
    }
}

/// Stops the service and takes it off the login (`tpcloud delete`, or no folders left).
pub fn remove_service() {
    if cfg!(target_os = "macos") {
        quiet("launchctl", &["bootout", &format!("gui/{}/{AGENT}", uid())]);
        let _ = std::fs::remove_file(agent_file());
    } else if unit_file().exists() {
        quiet("systemctl", &["--user", "disable", "--now", UNIT]);
        let _ = std::fs::remove_file(unit_file());
        quiet("systemctl", &["--user", "daemon-reload"]);
    }
}

// ── Commands ──

fn expand(path: &Path) -> PathBuf {
    match path.strip_prefix("~") {
        Ok(rest) => home().join(rest),
        Err(_) if path.is_relative() => std::env::current_dir().unwrap_or_default().join(path),
        Err(_) => path.to_path_buf(),
    }
}

fn short_id(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(path.to_string_lossy().as_bytes());
    digest.iter().take(6).map(|b| format!("{b:02x}")).collect()
}

/// A line that rewrites itself while a pass runs (only in a terminal).
fn progress_line() -> impl FnMut(usize, usize, &str) {
    let tty = std::io::stdout().is_terminal();
    move |done: usize, total: usize, what: &str| {
        if tty {
            let width = 60;
            let what: String = if what.chars().count() > width { format!("…{}", what.chars().rev().take(width - 1).collect::<Vec<_>>().into_iter().rev().collect::<String>()) } else { what.to_string() };
            print!("\r\x1b[2K  {} {}", ui::accent(&format!("{done}/{total}")), ui::dim(&what));
            let _ = std::io::stdout().flush();
        }
    }
}

fn clear_line() {
    if std::io::stdout().is_terminal() {
        print!("\r\x1b[2K");
        let _ = std::io::stdout().flush();
    }
}

/// `tpcloud sync`: the folders and how each one is doing.
pub fn status() -> Result<()> {
    let pairs = load_pairs();
    println!();
    if pairs.is_empty() {
        println!("  {}", ui::bold("No synced folders yet."));
        println!();
        println!("  {} {}", ui::accent("tpcloud sync add ~/TPCloud"), ui::dim("keeps that folder the same as your files in the cloud"));
        println!();
        return Ok(());
    }
    println!("  {}", ui::bold("Synced folders"));
    println!();
    let running = service_running();
    for p in &pairs {
        let s = status_of(&p.id);
        let (dot, state) = if p.paused {
            (ui::dim("○"), ui::dim("paused"))
        } else if s["ok"] == false {
            (ui::warn_text("●"), ui::warn_text(s["error"].as_str().unwrap_or("failed")))
        } else if s["at"].is_i64() {
            (ui::green("●"), ui::dim(&format!("up to date · checked {}", ago(s["at"].as_i64().unwrap_or(0)))))
        } else {
            (ui::accent("●"), ui::dim("waiting for its first sync"))
        };
        println!("  {dot} {}  {}  {}", ui::bold(&pretty(&p.local)), ui::dim("⇄"), format!("{} {}", p.network_name, ui::dim(&format!("· {}", remote_label(&p.remote)))));
        println!("    {state}");
        if let (Some(last), Some(at)) = (s["last"].as_str(), s["last_at"].as_i64()) {
            println!("    {}", ui::dim(&format!("last change: {last}, {}", ago(at))));
        }
        println!();
    }
    if !running && pairs.iter().any(|p| !p.paused) {
        ui::warn("The sync service isn't running: `tpcloud sync resume` starts it again.");
        println!();
    }
    println!("  {}", ui::dim("tpcloud sync add · remove <folder> · pause · resume · now"));
    println!();
    Ok(())
}

/// A new synced folder, checked and made, not saved yet: what `add` and the terminal app share.
pub fn new_pair(remote: &Remote, network: &Value, folder: Option<PathBuf>, remote_path: Option<String>) -> Result<Pair> {
    let me = remote.get("/auth/passkey/me")?;
    let email = me["email"].as_str().context("couldn't tell who you're signed in as")?.to_string();
    if !me["modules"].as_array().is_some_and(|m| m.iter().any(|x| x == "store")) {
        bail!("{} doesn't have Store (files) turned on", network["name"].as_str().unwrap_or("this network"));
    }
    let mine = format!("users/{email}");
    let remote_path = match remote_path.as_deref().map(|r| r.trim_matches('/')) {
        None | Some("") => mine.clone(),
        Some("shared") => "shared".into(),
        Some(r) if r.starts_with("shared/") || r.starts_with(&format!("{mine}/")) || r == mine => r.to_string(),
        Some(r) => format!("{mine}/{r}"),
    };
    let local = expand(&folder.unwrap_or_else(|| PathBuf::from("~/TPCloud")));
    std::fs::create_dir_all(&local).with_context(|| format!("making {}", local.display()))?;
    let local = local.canonicalize()?;
    if let Some(other) = load_pairs().iter().find(|p| local.starts_with(&p.local) || p.local.starts_with(&local)) {
        bail!("{} is synced already, and folders can't be inside each other", pretty(&other.local));
    }
    Ok(Pair {
        id: short_id(&local),
        local,
        remote: remote_path,
        network: network["id"].as_str().unwrap_or("").to_string(),
        network_name: network["name"].as_str().unwrap_or("").to_string(),
        paused: false,
    })
}

/// Its first sync, then saved and handed to the service.
pub fn start_pair(pair: Pair, remote: &Remote, progress: &mut dyn FnMut(usize, usize, &str)) -> Result<Report> {
    let _ = std::fs::remove_file(state_file(&pair.id)); // a fresh start for this folder
    let outcome = pass(&pair, remote, progress);
    record_status(&pair.id, &outcome);
    let report = outcome?;
    let mut pairs = load_pairs();
    pairs.push(pair);
    save_pairs(&pairs)?;
    start_service()?;
    Ok(report)
}

/// `tpcloud sync add [folder] [--remote path]`.
pub fn add(folder: Option<PathBuf>, remote_path: Option<String>) -> Result<()> {
    let (remote, network) = super::connect()?;
    let pair = new_pair(&remote, &network, folder, remote_path)?;
    println!();
    println!("  {} {}  {}  {} {}", ui::bold("Syncing"), ui::bold(&ui::accent(&pretty(&pair.local))), ui::dim("⇄"), pair.network_name, ui::dim(&format!("· {}", remote_label(&pair.remote))));
    println!();
    let outcome = start_pair(pair, &remote, &mut progress_line());
    clear_line();
    let report = outcome?;
    println!("  {} {}", ui::green("✓"), if report.anything() { report.describe() } else { "Both sides were the same already".into() });
    println!("  {} {}", ui::green("✓"), ui::dim("Kept in sync from now on, also after restarting"));
    println!();
    println!("  {} {}", ui::accent("tpcloud sync"), ui::dim("shows how it's doing"));
    println!();
    Ok(())
}

fn find_pair(pairs: &[Pair], folder: &Path) -> Option<usize> {
    let want = expand(folder);
    let want = want.canonicalize().unwrap_or(want);
    pairs.iter().position(|p| p.local == want || p.id == folder.to_string_lossy())
}

/// A folder stops being synced (by id); its files stay on both sides.
pub fn drop_pair(id: &str) -> Result<Pair> {
    let mut pairs = load_pairs();
    let i = pairs.iter().position(|p| p.id == id).context("that folder isn't synced any more")?;
    let pair = pairs.remove(i);
    save_pairs(&pairs)?;
    let _ = std::fs::remove_file(state_file(&pair.id));
    let _ = std::fs::remove_file(sync_dir().join(format!("{}.lock", pair.id)));
    if pairs.is_empty() {
        remove_service();
    } else {
        start_service()?;
    }
    Ok(pair)
}

/// Every folder paused, or syncing again.
pub fn pause_all(paused: bool) -> Result<()> {
    let mut pairs = load_pairs();
    if pairs.is_empty() {
        bail!("there are no synced folders: `tpcloud sync add`");
    }
    for p in &mut pairs {
        p.paused = paused;
    }
    save_pairs(&pairs)?;
    if paused {
        remove_service();
    } else {
        start_service()?;
    }
    Ok(())
}

/// Every folder once, quietly: the terminal app's «sync now».
pub fn sync_all_now() -> Vec<(Pair, Result<Report>)> {
    load_pairs()
        .into_iter()
        .filter(|p| !p.paused)
        .map(|p| {
            let outcome = super::connect_network(&p.network).and_then(|(remote, _)| pass(&p, &remote, &mut |_, _, _| {}));
            record_status(&p.id, &outcome);
            (p, outcome)
        })
        .collect()
}

/// `tpcloud sync remove <folder>`: it stops being synced; the files stay on both sides.
pub fn remove(folder: &Path) -> Result<()> {
    let pairs = load_pairs();
    let i = find_pair(&pairs, folder).with_context(|| format!("{} isn't a synced folder (`tpcloud sync` lists them)", folder.display()))?;
    let pair = drop_pair(&pairs[i].id)?;
    println!();
    println!("  {} {} {}", ui::green("✓"), ui::bold(&pretty(&pair.local)), ui::dim("isn't synced any more; its files stay here and in the cloud"));
    println!();
    Ok(())
}

/// `tpcloud sync pause` / `resume`.
pub fn set_paused(paused: bool) -> Result<()> {
    pause_all(paused)?;
    println!();
    println!("  {} {}", ui::green("✓"), if paused { "Paused: nothing is copied until `tpcloud sync resume`" } else { "Syncing again" });
    println!();
    Ok(())
}

/// `tpcloud sync now`: every folder once, here, with its progress.
pub fn now() -> Result<()> {
    let pairs = load_pairs();
    if pairs.is_empty() {
        bail!("there are no synced folders: `tpcloud sync add`");
    }
    println!();
    for p in pairs.iter().filter(|p| !p.paused) {
        let outcome = super::connect_network(&p.network).and_then(|(remote, _)| pass(p, &remote, &mut progress_line()));
        clear_line();
        record_status(&p.id, &outcome);
        match outcome {
            Ok(r) => println!("  {} {}  {}", ui::green("✓"), ui::bold(&pretty(&p.local)), ui::dim(&r.describe())),
            Err(e) => println!("  {} {}  {}", ui::warn_text("!"), ui::bold(&pretty(&p.local)), ui::warn_text(&format!("{e:#}"))),
        }
    }
    println!();
    Ok(())
}

/// How a folder looks right now, cheaply: changes when anything in it does.
fn fingerprint(root: &Path) -> Option<(usize, u64, i64)> {
    let (files, dirs) = scan_local(root).ok()?;
    Some((files.len() + dirs.len(), files.values().map(|m| m.size).sum(), files.values().map(|m| m.mtime).max().unwrap_or(0)))
}

/// `tpcloud sync run`: what the service runs. Each folder is looked at every few seconds here
/// and every half minute there; a failure (no network, the server updating) is retried later.
pub fn run() -> Result<()> {
    let mut seen: HashMap<String, (Option<(usize, u64, i64)>, Option<Instant>)> = HashMap::new();
    let mut backoff: HashMap<String, Instant> = HashMap::new();
    loop {
        let pairs = load_pairs();
        if pairs.iter().all(|p| p.paused) {
            println!("No folders to sync: stopping.");
            return Ok(());
        }
        for p in pairs.iter().filter(|p| !p.paused) {
            let print = fingerprint(&p.local);
            let entry = seen.entry(p.id.clone()).or_insert((None, None));
            let changed_here = print.is_some() && print != entry.0;
            let due = entry.1.is_none_or(|t| t.elapsed() >= REMOTE_EVERY);
            let waiting = backoff.get(&p.id).is_some_and(|until| Instant::now() < *until);
            if !(changed_here || due) || waiting {
                continue;
            }
            let outcome = super::connect_network(&p.network).and_then(|(remote, _)| pass(p, &remote, &mut |_, _, _| {}));
            match &outcome {
                Ok(r) if r.anything() => println!("{}: {}", pretty(&p.local), r.describe()),
                Ok(_) => {}
                Err(e) => println!("{}: {e:#}", pretty(&p.local)),
            }
            if outcome.is_err() {
                backoff.insert(p.id.clone(), Instant::now() + REMOTE_EVERY);
            } else {
                backoff.remove(&p.id);
            }
            record_status(&p.id, &outcome);
            let entry = seen.get_mut(&p.id).unwrap();
            entry.0 = fingerprint(&p.local);
            entry.1 = Some(Instant::now());
        }
        std::thread::sleep(LOCAL_EVERY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(size: u64, mtime: i64) -> Meta {
        Meta { size, mtime }
    }

    fn kinds(ops: &[Op]) -> Vec<String> {
        ops.iter()
            .map(|o| match o {
                Op::Up(p) => format!("up {p}"),
                Op::Down(p) => format!("down {p}"),
                Op::DeleteHere(p) => format!("rm-here {p}"),
                Op::DeleteThere(p) => format!("rm-there {p}"),
                Op::Conflict(p) => format!("both {p}"),
                Op::Same(p, ..) => format!("same {p}"),
                Op::Forget(p) => format!("forget {p}"),
            })
            .collect()
    }

    #[test]
    fn decides_who_changed_what() {
        let old = 1_000_000;
        let mut state = State::default();
        for p in ["kept", "edited-here", "edited-there", "edited-both", "deleted-here", "deleted-there", "deleted-both"] {
            state.files.insert(p.into(), (m(1, old), m(1, old)));
        }
        let mut local = BTreeMap::new();
        let mut remote = BTreeMap::new();
        for p in ["kept", "edited-there", "deleted-there"] {
            local.insert(p.to_string(), m(1, old));
        }
        for p in ["kept", "edited-here", "deleted-here"] {
            remote.insert(p.to_string(), m(1, old));
        }
        local.insert("edited-here".into(), m(2, old + 10));
        remote.insert("edited-there".into(), m(3, old + 20));
        local.insert("edited-both".into(), m(4, old + 30));
        remote.insert("edited-both".into(), m(5, old + 40));
        local.insert("new-here".into(), m(1, old));
        remote.insert("new-there".into(), m(1, old));
        local.insert("new-both-same".into(), m(7, old));
        remote.insert("new-both-same".into(), m(7, old + 1));
        local.insert("new-both-different".into(), m(7, old));
        remote.insert("new-both-different".into(), m(8, old));
        local.insert("being-written".into(), m(1, now_secs()));
        let got = kinds(&plan(&local, &remote, &state));
        let want = [
            "forget deleted-both",
            "rm-there deleted-here",
            "rm-here deleted-there",
            "both edited-both",
            "up edited-here",
            "down edited-there",
            "both new-both-different",
            "same new-both-same",
            "up new-here",
            "down new-there",
        ];
        assert_eq!(got, want);
    }

    /// Against a real store-api: TPCLOUD_TEST_STORE=http://127.0.0.1:18402 (its file routes under
    /// /store, signed in as me@test) and TPCLOUD_TEST_DISK=<its disk>/users/me@test.
    /// `cargo test --no-default-features -- --ignored synced_folder`
    #[test]
    #[ignore]
    fn synced_folder_end_to_end() {
        let (Ok(url), Ok(disk)) = (std::env::var("TPCLOUD_TEST_STORE"), std::env::var("TPCLOUD_TEST_DISK")) else { return };
        let disk = PathBuf::from(disk);
        let tmp = std::env::temp_dir().join(format!("tpcloud-sync-{}", std::process::id()));
        let local = tmp.join("here");
        std::fs::create_dir_all(&local).unwrap();
        std::fs::create_dir_all(&disk).unwrap();
        std::env::set_var("HOME", &tmp);
        std::env::set_var("XDG_CONFIG_HOME", tmp.join("config"));
        let old = now_secs() - 100;
        let write = |p: &Path, text: &str, t: i64| {
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
            set_mtime(p, t).unwrap();
        };
        let read = |p: &Path| std::fs::read_to_string(p).unwrap_or_default();
        let remote = Remote::new(&url, None, &[], None);
        let pair = Pair { id: "test".into(), local: local.clone(), remote: "users/me@test".into(), network: String::new(), network_name: String::new(), paused: false };
        let run = || pass(&pair, &remote, &mut |_, _, _| {}).unwrap();

        // First time: each side gets what the other has.
        write(&local.join("a.txt"), "a1", old);
        write(&local.join("sub/b.txt"), "b1", old);
        write(&disk.join("r.txt"), "r1", old);
        std::fs::create_dir_all(disk.join("empty-there")).unwrap();
        let r = run();
        assert_eq!((r.up, r.down), (2, 1));
        assert_eq!(read(&disk.join("a.txt")), "a1");
        assert_eq!(read(&disk.join("sub/b.txt")), "b1");
        assert_eq!(read(&local.join("r.txt")), "r1");
        assert!(local.join("empty-there").is_dir());
        assert_eq!(mtime_of(&std::fs::metadata(disk.join("a.txt")).unwrap()), old, "the upload keeps the time");
        assert!(!run().anything(), "nothing changed: nothing to do");

        // A change on each side, and a deletion on each side.
        write(&local.join("a.txt"), "a2 longer", old + 10);
        write(&disk.join("sub/b.txt"), "b2 from there", old + 20);
        std::fs::remove_file(disk.join("r.txt")).unwrap();
        write(&local.join("new.txt"), "n", old);
        let r = run();
        assert_eq!((r.up, r.down, r.deleted_here), (2, 1, 1), "{}", r.describe());
        assert_eq!(read(&disk.join("a.txt")), "a2 longer");
        assert_eq!(read(&local.join("sub/b.txt")), "b2 from there");
        assert!(!local.join("r.txt").exists());
        std::fs::remove_file(local.join("new.txt")).unwrap();
        let r = run();
        assert_eq!(r.deleted_there, 1);
        assert!(!disk.join("new.txt").exists());

        // Changed on both sides: both kept.
        write(&local.join("a.txt"), "a3 here", old + 30);
        write(&disk.join("a.txt"), "a3 there!", old + 40);
        let r = run();
        assert_eq!(r.conflicts, 1);
        assert_eq!(read(&local.join("a.txt")), "a3 there!");
        let copies: Vec<_> = std::fs::read_dir(&local).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.contains("conflict")).collect();
        assert_eq!(copies.len(), 1, "{copies:?}");
        assert_eq!(read(&local.join(&copies[0])), "a3 here");
        assert_eq!(read(&disk.join(&copies[0])), "a3 here");
        assert!(!run().anything());

        // A folder deleted here goes there too, files and all.
        std::fs::remove_dir_all(local.join("sub")).unwrap();
        let r = run();
        assert_eq!(r.deleted_there, 1);
        assert!(!disk.join("sub").exists(), "the emptied folder went too");
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn conflict_copies_keep_folder_and_extension() {
        let name = conflict_name("photos/2026/beach.jpg");
        assert!(name.starts_with("photos/2026/beach (conflict "), "{name}");
        assert!(name.ends_with(").jpg"), "{name}");
        let bare = conflict_name("notes");
        assert!(bare.starts_with("notes (conflict ") && bare.ends_with(')'), "{bare}");
    }
}
