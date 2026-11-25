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
use std::sync::atomic::{AtomicBool, Ordering};
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
    /// On demand (Linux): the files show here but are fetched when opened (see `drive`).
    pub on_demand: bool,
    /// Inside a folder on demand: what's kept here for good (paths inside it, "" = all of it).
    pub pinned: Vec<String>,
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
                on_demand: f["on_demand"] == true,
                pinned: f["pinned"].as_array().into_iter().flatten().filter_map(|p| p.as_str().map(str::to_string)).collect(),
            })
        })
        .collect()
}

fn save_pairs(pairs: &[Pair]) -> Result<()> {
    let folders: Vec<Value> = pairs
        .iter()
        .map(|p| json!({"id": p.id, "local": p.local, "remote": p.remote, "network": p.network, "network_name": p.network_name, "paused": p.paused, "on_demand": p.on_demand, "pinned": p.pinned}))
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
    /// Stopped halfway when asked (what was copied is kept; the next pass carries on).
    pub stopped: bool,
}

/// How far a pass has got, for a bar and a time left.
pub struct Step<'a> {
    pub done: usize,
    pub total: usize,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub started: Instant,
    pub what: &'a str,
}

impl Step<'_> {
    /// Each file costs a round trip besides its bytes: counted as this many bytes, so a folder
    /// of thousands of small photos doesn't promise to be done in seconds.
    const PER_FILE: u64 = 256 * 1024;

    pub fn fraction(&self) -> f64 {
        let work = self.bytes_total + self.total as u64 * Self::PER_FILE;
        if work == 0 {
            return 1.0;
        }
        ((self.bytes_done + self.done as u64 * Self::PER_FILE) as f64 / work as f64).min(1.0)
    }

    /// Once there's enough to go on (a few seconds and a bit of work done).
    pub fn left(&self) -> Option<Duration> {
        let elapsed = self.started.elapsed().as_secs_f64();
        let f = self.fraction();
        (elapsed > 3.0 && f > 0.002).then(|| Duration::from_secs_f64(elapsed * (1.0 - f) / f))
    }
}

/// «about 25 min left», «about 2 h left», «less than a minute left».
pub fn time_left(left: Option<Duration>) -> String {
    match left.map(|d| d.as_secs()) {
        None => "working out the time left".into(),
        Some(0..=59) => "less than a minute left".into(),
        Some(s @ 60..=3599) => format!("about {} min left", (s + 30) / 60),
        Some(s) => format!("about {} h {} min left", s / 3600, (s % 3600) / 60),
    }
}

pub fn human(bytes: u64) -> String {
    match bytes {
        0..=1023 => format!("{bytes} B"),
        1024..=1_048_575 => format!("{:.0} KB", bytes as f64 / 1024.0),
        1_048_576..=1_073_741_823 => format!("{:.1} MB", bytes as f64 / 1_048_576.0),
        _ => format!("{:.1} GB", bytes as f64 / 1_073_741_824.0),
    }
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
fn plan(local: &BTreeMap<String, Meta>, remote: &BTreeMap<String, Meta>, state: &State, skip_recent: bool) -> Vec<Op> {
    // Still being written: next time (not when nothing is writing, before a folder goes on demand).
    let recent = if skip_recent { now_secs() - 2 } else { i64::MAX };
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

/// One pass over a folder. `stop` ends it between two files (what's copied stays copied).
pub fn pass(pair: &Pair, remote: &Remote, progress: &mut dyn FnMut(&Step), stop: &AtomicBool) -> Result<Report> {
    pass_with(pair, remote, progress, stop, false)
}

/// `push_only`: only what's here goes up (new or changed files; one changed on both sides goes
/// up as a conflict copy); nothing comes down and nothing is deleted. What a folder with files
/// does before becoming one on demand.
fn pass_with(pair: &Pair, remote: &Remote, progress: &mut dyn FnMut(&Step), stop: &AtomicBool, push_only: bool) -> Result<Report> {
    let Some(_lock) = Lock::take(&pair.id) else { bail!("it's being synced already") };
    if !pair.local.is_dir() {
        bail!("{} isn't there any more (moved, or a disk that isn't connected)", pair.local.display());
    }
    let mut state = load_state(&pair.id);
    let (local, local_dirs) = scan_local(&pair.local)?;
    let (mut there, there_dirs) = scan_remote(remote, &pair.remote)?;
    let mut ops = plan(&local, &there, &state, !push_only);
    if push_only {
        ops.retain(|o| matches!(o, Op::Up(_) | Op::Conflict(_) | Op::Same(..)));
    }

    // A sudden mass deletion is more likely a mistake (an emptied disk, a wrong folder) than
    // wanted: stop and say so instead of copying it to the other side.
    let deletions = ops.iter().filter(|o| matches!(o, Op::DeleteHere(_) | Op::DeleteThere(_))).count();
    if deletions > 20 && deletions * 2 > state.files.len() {
        bail!("{deletions} of {} files would be deleted at once; nothing was touched. If that's what you want: `tpcloud sync remove` the folder and add it again", state.files.len());
    }

    let mut report = Report::default();
    let total = ops.iter().filter(|o| !matches!(o, Op::Same(..) | Op::Forget(_))).count();
    let bytes_of = |op: &Op| -> u64 {
        match op {
            Op::Up(p) => local.get(p).map_or(0, |m| m.size),
            Op::Down(p) => there.get(p).map_or(0, |m| m.size),
            Op::Conflict(p) => local.get(p).map_or(0, |m| m.size) + there.get(p).map_or(0, |m| m.size),
            _ => 0,
        }
    };
    let bytes_total: u64 = ops.iter().map(bytes_of).sum();
    let sizes: Vec<u64> = ops.iter().map(bytes_of).collect();
    let started = Instant::now();
    let mut bytes_done = 0u64;
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
        for (i, op) in ops.iter().enumerate() {
            if stop.load(Ordering::Relaxed) {
                report.stopped = true;
                return Ok(());
            }
            let mut progress = |what: &str| progress(&Step { done, total, bytes_done, bytes_total, started, what });
            match op {
                Op::Same(p, l, r) => {
                    state.files.insert(p.clone(), (*l, *r));
                }
                Op::Forget(p) => {
                    state.files.remove(p);
                }
                Op::Up(p) => {
                    progress(&format!("↑ {p}"));
                    let l = local[p];
                    upload(p, &here(p), l.mtime)?;
                    there.insert(p.clone(), l);
                    state.files.insert(p.clone(), (l, l));
                    report.up += 1;
                }
                Op::Down(p) => {
                    progress(&format!("↓ {p}"));
                    let r = there[p];
                    let l = download(p, &here(p), r.mtime)?;
                    state.files.insert(p.clone(), (l, r));
                    report.down += 1;
                }
                Op::DeleteThere(p) => {
                    progress(&format!("× {p}"));
                    remote.send("DELETE", &format!("{API}?path={}", query_value(&join_remote(&pair.remote, p))), None).or_else(|e| if e.to_string() == "Not found" { Ok(Value::Null) } else { Err(e) })?;
                    there.remove(p);
                    state.files.remove(p);
                    report.deleted_there += 1;
                }
                Op::DeleteHere(p) => {
                    progress(&format!("× {p}"));
                    match std::fs::remove_file(here(p)) {
                        Ok(()) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                        Err(e) => return Err(e.into()),
                    }
                    state.files.remove(p);
                    report.deleted_here += 1;
                }
                Op::Conflict(p) => {
                    progress(&format!("⇅ {p}"));
                    // Ours under a new name (sent too), theirs under the real one.
                    let copy = conflict_name(p);
                    std::fs::rename(here(p), here(&copy))?;
                    let meta = std::fs::metadata(here(&copy))?;
                    let ours = Meta { size: meta.len(), mtime: mtime_of(&meta) };
                    upload(&copy, &here(&copy), ours.mtime)?;
                    state.files.insert(copy.clone(), (ours, ours));
                    there.insert(copy, ours);
                    if !push_only {
                        let r = there[p];
                        let l = download(p, &here(p), r.mtime)?;
                        state.files.insert(p.clone(), (l, r));
                    }
                    report.conflicts += 1;
                }
            }
            if !matches!(op, Op::Same(..) | Op::Forget(_)) {
                done += 1;
                bytes_done += sizes[i];
            }
            since_save += 1;
            if since_save >= 25 {
                save_state(&pair.id, &state)?;
                since_save = 0;
            }
        }

        if push_only {
            return Ok(());
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

/// A pass under way, for `tpcloud sync` and the terminal app (written every couple of seconds).
fn record_progress(id: &str, step: &Step) {
    let path = status_file();
    let mut all: Value = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_else(|| json!({}));
    if !all[id].is_object() {
        all[id] = json!({});
    }
    all[id]["progress"] = json!({
        "at": now_secs(),
        "done": step.done,
        "total": step.total,
        "bytes_done": step.bytes_done,
        "bytes_total": step.bytes_total,
        "left": step.left().map(|d| d.as_secs()),
        "percent": (step.fraction() * 100.0) as u32,
    });
    let _ = write_private(&path, &all.to_string());
}

/// A pass under way in the service, if there's one (said in the last minute): «42% · 2811 of
/// 33466 files · about 25 min left».
pub fn progress_of(status: &Value) -> Option<String> {
    let p = &status["progress"];
    let at = p["at"].as_i64()?;
    if now_secs() - at > 60 {
        return None;
    }
    let left = p["left"].as_u64().map(Duration::from_secs);
    Some(format!(
        "syncing · {}% · {} of {} files · {}",
        p["percent"].as_u64().unwrap_or(0),
        p["done"].as_u64().unwrap_or(0),
        p["total"].as_u64().unwrap_or(0),
        time_left(left)
    ))
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

/// How a folder on demand is doing, written by the service every half minute.
pub fn record_demand_status(pair: &Pair, online: bool) {
    let path = status_file();
    let mut all: Value = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_else(|| json!({}));
    all[&pair.id] = json!({
        "at": now_secs(),
        "ok": online,
        "error": if online { Value::Null } else { json!("offline: what's here still opens") },
        "cached": cached_here(pair),
    });
    let _ = write_private(&path, &all.to_string());
}

/// Bytes a folder on demand keeps on this computer.
pub fn cached_here(pair: &Pair) -> u64 {
    #[cfg(target_os = "linux")]
    {
        super::drive::cached_bytes(&pair.id)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pair;
        0
    }
}

/// Folders on demand work here (Linux with FUSE).
pub fn demand_possible() -> bool {
    #[cfg(target_os = "linux")]
    {
        super::drive::available()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Moves every file from `from` into `to` under the same paths (renaming when they're on the
/// same disk, copying when not), then removes the folders left empty in `from`.
fn move_tree(from: &Path, to: &Path) -> Result<u64> {
    let mut moved = 0;
    let mut stack = vec![from.to_path_buf()];
    let mut dirs = Vec::new();
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))?.flatten() {
            let path = e.path();
            let meta = path.symlink_metadata()?;
            let rel = path.strip_prefix(from).unwrap().to_path_buf();
            if meta.is_dir() {
                dirs.push(path.clone());
                stack.push(path);
                continue;
            }
            let dest = to.join(&rel);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if std::fs::rename(&path, &dest).is_err() {
                std::fs::copy(&path, &dest)?;
                if let Ok(t) = meta.modified() {
                    let _ = std::fs::File::options().write(true).open(&dest).and_then(|f| f.set_modified(t));
                }
                std::fs::remove_file(&path)?;
            }
            moved += 1;
        }
    }
    dirs.sort_by_key(|d| std::cmp::Reverse(d.as_os_str().len()));
    for d in dirs {
        let _ = std::fs::remove_dir(d);
    }
    Ok(moved)
}

/// Waits until the service let go of a folder on demand (or lets it go itself).
fn wait_unmounted(path: &Path) {
    #[cfg(target_os = "linux")]
    {
        for _ in 0..50 {
            if !super::drive::mounted(path) {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        super::drive::unmount(path);
    }
    #[cfg(not(target_os = "linux"))]
    let _ = path;
}

/// A folder on demand's files back in it as plain files (what was fetched or kept, and what
/// waited to go up), its cache gone: when it stops being on demand.
pub fn release_demand(pair: &Pair) -> Result<u64> {
    #[cfg(target_os = "linux")]
    {
        wait_unmounted(&pair.local);
        let root = super::drive::cache_root(&pair.id);
        let moved = move_tree(&super::drive::files_dir(&pair.id), &pair.local)?;
        let _ = std::fs::remove_dir_all(root);
        Ok(moved)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pair;
        Ok(0)
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
             [Service]\nExecStart={} sync run\nRestart=on-failure\nRestartSec=15\nTimeoutStopSec=20\n\n\
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

/// Keys read while a pass runs in the foreground: `d` leaves it to the service, Ctrl-C stops.
struct Keys {
    stop: std::sync::Arc<AtomicBool>,
    detach: std::sync::Arc<AtomicBool>,
    cancel: std::sync::Arc<AtomicBool>,
    finished: std::sync::Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Keys {
    fn listen() -> Keys {
        use std::sync::Arc;
        let keys = Keys {
            stop: Arc::new(AtomicBool::new(false)),
            detach: Arc::new(AtomicBool::new(false)),
            cancel: Arc::new(AtomicBool::new(false)),
            finished: Arc::new(AtomicBool::new(false)),
            thread: None,
        };
        if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() || crossterm::terminal::enable_raw_mode().is_err() {
            return keys;
        }
        let (stop, detach, cancel, finished) = (keys.stop.clone(), keys.detach.clone(), keys.cancel.clone(), keys.finished.clone());
        let thread = std::thread::spawn(move || {
            use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
            while !finished.load(Ordering::Relaxed) {
                if !event::poll(Duration::from_millis(100)).unwrap_or(false) {
                    continue;
                }
                let Ok(Event::Key(k)) = event::read() else { continue };
                if k.kind != KeyEventKind::Press {
                    continue;
                }
                match k.code {
                    KeyCode::Char('d') | KeyCode::Char('D') => detach.store(true, Ordering::Relaxed),
                    KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => cancel.store(true, Ordering::Relaxed),
                    _ => continue,
                }
                stop.store(true, Ordering::Relaxed);
            }
        });
        Keys { thread: Some(thread), ..keys }
    }

    fn finish(&mut self) {
        self.finished.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
            let _ = crossterm::terminal::disable_raw_mode();
        }
    }
}

/// A bar, the numbers and the time left, and what's being copied, redrawn in place (three
/// lines; only in a terminal, at most ten times a second).
fn live_bar(hint: &'static str) -> impl FnMut(&Step) {
    let tty = std::io::stdout().is_terminal();
    let mut drawn = false;
    let mut last = Instant::now() - Duration::from_secs(1);
    move |step: &Step| {
        if !tty || last.elapsed() < Duration::from_millis(100) {
            return;
        }
        last = Instant::now();
        let width = 30;
        let f = step.fraction();
        let filled = ((f * width as f64).round() as usize).min(width);
        let bar = format!("{}{}", ui::accent(&"━".repeat(filled)), ui::dim(&"━".repeat(width - filled)));
        let numbers = format!("{} of {} files", step.done, step.total);
        let bytes = if step.bytes_total > 0 { format!(" · {} of {}", human(step.bytes_done), human(step.bytes_total)) } else { String::new() };
        let line1 = format!("  {bar} {:>3}%  {}{}  {}", (f * 100.0) as u32, numbers, ui::dim(&bytes), ui::accent(&format!("· {}", time_left(step.left()))));
        let max = 72;
        let what: String = if step.what.chars().count() > max { format!("…{}", step.what.chars().skip(step.what.chars().count() - (max - 1)).collect::<String>()) } else { step.what.to_string() };
        let line2 = format!("  {}", ui::dim(&what));
        let line3 = format!("  {}", ui::dim(hint));
        let up = if drawn { "\x1b[2F" } else { "" };
        print!("{up}\r\x1b[2K{line1}\r\n\x1b[2K{line2}\r\n\x1b[2K{line3}");
        let _ = std::io::stdout().flush();
        drawn = true;
    }
}

/// Takes the bar away (if one was drawn), leaving the cursor where it started.
fn clear_bar(drawn_any: bool) {
    if std::io::stdout().is_terminal() && drawn_any {
        print!("\r\x1b[2F\x1b[J");
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
        } else if p.on_demand {
            let kept = if p.pinned.is_empty() { String::new() } else { format!(" · {} kept here for good", p.pinned.iter().map(|k| if k.is_empty() { "everything".to_string() } else { k.clone() }).collect::<Vec<_>>().join(", ")) };
            let room = format!("on demand · {} on this computer{kept}", human(s["cached"].as_u64().unwrap_or_else(|| cached_here(p))));
            if s["ok"] == false {
                (ui::warn_text("●"), ui::warn_text(&format!("{room} · {}", s["error"].as_str().unwrap_or("offline"))))
            } else {
                (ui::green("●"), ui::dim(&room))
            }
        } else if let Some(busy) = progress_of(&s) {
            (ui::accent("●"), ui::accent(&busy))
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
    println!("  {}", ui::dim("tpcloud sync add · keep <path> · free <path> · mode <folder> · remove · pause · resume · now"));
    println!();
    Ok(())
}

/// On a Mac, files on demand are the TPCloud app's location in the Finder (its File Provider).
pub const MAC_FINDER: &str = "On a Mac your files are in the Finder: open the TPCloud app, sign in, and they're under Locations.";

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
        on_demand: false,
        pinned: Vec::new(),
    })
}

/// Saved and handed to the service (after its first pass, or with it left to the service).
fn keep_pair(pair: Pair) -> Result<()> {
    let mut pairs = load_pairs();
    pairs.push(pair);
    save_pairs(&pairs)?;
    start_service()
}

/// Its first sync, then saved and handed to the service (the terminal app's «add»).
pub fn start_pair(pair: Pair, remote: &Remote) -> Result<Report> {
    let _ = std::fs::remove_file(state_file(&pair.id)); // a fresh start for this folder
    let outcome = pass(&pair, remote, &mut |_| {}, &AtomicBool::new(false));
    record_status(&pair.id, &outcome);
    let report = outcome?;
    keep_pair(pair)?;
    Ok(report)
}

/// Runs a pass in the foreground with the bar and the keys: (report, left to the service, cancelled).
fn foreground_pass(pair: &Pair, remote: &Remote, hint: &'static str) -> (Result<Report>, bool, bool) {
    let mut keys = Keys::listen();
    let mut drawn = false;
    let mut bar = live_bar(hint);
    let outcome = pass(pair, remote, &mut |step| {
        drawn = true;
        bar(step)
    }, &keys.stop);
    keys.finish();
    clear_bar(drawn);
    (outcome, keys.detach.load(Ordering::Relaxed), keys.cancel.load(Ordering::Relaxed))
}

/// `tpcloud sync add [folder] [--remote path] [--full]`: on demand where it can be (Linux with
/// FUSE), a full copy otherwise or when asked.
pub fn add(folder: Option<PathBuf>, remote_path: Option<String>, full: bool) -> Result<()> {
    let (remote, network) = super::connect()?;
    if !full && cfg!(target_os = "macos") {
        println!();
        println!("  {}", ui::bold(MAC_FINDER));
        println!("  {}", ui::dim("`tpcloud sync add --full` keeps a full copy in a folder instead."));
        println!();
        return Ok(());
    }
    let pair = new_pair(&remote, &network, folder, remote_path)?;
    if !full && demand_possible() {
        return add_on_demand(pair, &remote);
    }
    if !full {
        println!();
        println!("  {}", ui::dim("This computer can't mount folders on demand (no FUSE): this one is a full copy."));
    }
    let shown = pretty(&pair.local);
    println!();
    println!("  {} {}  {}  {} {}", ui::bold("Syncing"), ui::bold(&ui::accent(&shown)), ui::dim("⇄"), pair.network_name, ui::dim(&format!("· {}", remote_label(&pair.remote))));
    println!();
    let _ = std::fs::remove_file(state_file(&pair.id)); // a fresh start for this folder
    let (outcome, detached, cancelled) = foreground_pass(&pair, &remote, "d  carry on in the background    ctrl-c  stop");
    if cancelled {
        let _ = std::fs::remove_file(state_file(&pair.id));
        println!("  {} {}", ui::warn_text("■"), format!("Stopped: {shown} isn't synced. What was copied so far stays."));
        println!();
        return Ok(());
    }
    let report = match outcome {
        Ok(r) => r,
        Err(e) => {
            record_status(&pair.id, &Err(anyhow::anyhow!("{e:#}")));
            return Err(e);
        }
    };
    if !(detached || report.stopped) {
        record_status(&pair.id, &Ok(report));
    }
    keep_pair(pair.clone())?;
    if detached || report.stopped {
        println!("  {} Carrying on in the background {}", ui::green("✓"), ui::dim(&format!("({} so far): {shown} fills up as it goes", report.describe())));
        println!();
        println!("  {} {}", ui::accent("tpcloud sync"), ui::dim("shows how far it's got"));
        println!();
        return Ok(());
    }
    println!("  {} {}", ui::green("✓"), if report.anything() { report.describe() } else { "Both sides were the same already".into() });
    println!("  {} {}", ui::green("✓"), ui::dim("Kept in sync from now on, also after restarting"));
    println!();
    println!("  {} {}", ui::accent("tpcloud sync"), ui::dim("shows how it's doing"));
    println!();
    Ok(())
}

/// The files it already has go up first (only what the cloud lacks or has older), then into the
/// cache, and the folder becomes a window on the cloud.
fn add_on_demand(mut pair: Pair, remote: &Remote) -> Result<()> {
    let shown = pretty(&pair.local);
    println!();
    println!("  {} {}  {}  {} {}", ui::bold("On demand"), ui::bold(&ui::accent(&shown)), ui::dim("⇄"), pair.network_name, ui::dim(&format!("· {}", remote_label(&pair.remote))));
    println!();
    let has_files = std::fs::read_dir(&pair.local).is_ok_and(|mut d| d.next().is_some());
    if has_files {
        let report = push_here(&pair, remote)?;
        if report.anything() {
            println!("  {} {}", ui::green("✓"), ui::dim(&format!("What only this folder had: {}", report.describe())));
        }
        verify_uploaded(&pair, remote)?;
        #[cfg(target_os = "linux")]
        move_tree(&pair.local, &super::drive::files_dir(&pair.id))?;
    }
    let _ = std::fs::remove_file(state_file(&pair.id));
    pair.on_demand = true;
    let local = pair.local.clone();
    keep_pair(pair)?;
    wait_mounted(&local);
    println!("  {} {} {}", ui::green("✓"), ui::bold(&shown), ui::dim("shows all your files; each one is fetched when it's opened and kept a day"));
    println!();
    println!("  {} {}", ui::accent(&format!("tpcloud sync keep {shown}/<folder>")), ui::dim("keeps a folder here for good (it's used a lot)"));
    println!("  {} {}", ui::accent(&format!("tpcloud sync free {shown}")), ui::dim("gives back the room what was fetched takes"));
    println!();
    Ok(())
}

/// Before a folder's files move into the cache: every one of them must be on the server, the
/// same size, or nothing moves (a file only here would vanish from the folder).
fn verify_uploaded(pair: &Pair, remote: &Remote) -> Result<()> {
    let (local, _) = scan_local(&pair.local)?;
    let (there, _) = scan_remote(remote, &pair.remote)?;
    let missing: Vec<&String> = local.iter().filter(|(p, m)| there.get(*p).is_none_or(|r| r.size != m.size)).map(|(p, _)| p).collect();
    if !missing.is_empty() {
        let some: Vec<&str> = missing.iter().take(3).map(|p| p.as_str()).collect();
        bail!("{} file(s) aren't on the server yet ({}{}): nothing was moved; try again", missing.len(), some.join(", "), if missing.len() > 3 { ", …" } else { "" });
    }
    Ok(())
}

/// The terminal app's «add»: on demand where it can be (what the folder had goes up first),
/// a full copy otherwise. Quiet: it says what happened in one line.
pub fn start_folder(pair: Pair, remote: &Remote) -> Result<String> {
    if !demand_possible() {
        let shown = pretty(&pair.local);
        let r = start_pair(pair, remote)?;
        return Ok(format!("{shown}: {}; synced from now on", r.describe()));
    }
    let mut pair = pair;
    let shown = pretty(&pair.local);
    if std::fs::read_dir(&pair.local).is_ok_and(|mut d| d.next().is_some()) {
        let r = pass_with(&pair, remote, &mut |_| {}, &AtomicBool::new(false), true)?;
        let _ = std::fs::remove_file(state_file(&pair.id));
        if r.stopped {
            bail!("stopped: {shown} is as it was");
        }
        verify_uploaded(&pair, remote)?;
        #[cfg(target_os = "linux")]
        move_tree(&pair.local, &super::drive::files_dir(&pair.id))?;
    }
    pair.on_demand = true;
    let local = pair.local.clone();
    keep_pair(pair)?;
    wait_mounted(&local);
    Ok(format!("{shown} shows all your files, fetched when opened"))
}

/// What's only here goes up, with the bar (Ctrl-C stops before anything moves).
fn push_here(pair: &Pair, remote: &Remote) -> Result<Report> {
    let mut keys = Keys::listen();
    let mut drawn = false;
    let mut bar = live_bar("ctrl-c  stop (nothing is moved until this is done)");
    let outcome = pass_with(pair, remote, &mut |step| {
        drawn = true;
        bar(step)
    }, &keys.stop, true);
    keys.finish();
    clear_bar(drawn);
    let _ = std::fs::remove_file(state_file(&pair.id));
    let report = outcome?;
    if report.stopped {
        bail!("stopped: {} is as it was", pretty(&pair.local));
    }
    Ok(report)
}

fn wait_mounted(path: &Path) {
    #[cfg(target_os = "linux")]
    for _ in 0..50 {
        if super::drive::mounted(path) {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    #[cfg(not(target_os = "linux"))]
    let _ = path;
}

/// The synced folder a path is in, and the path inside it.
fn pair_of_path(pairs: &[Pair], path: &Path) -> Option<(usize, String)> {
    let want = expand(path);
    let want = want.canonicalize().unwrap_or(want);
    pairs.iter().position(|p| want.starts_with(&p.local)).map(|i| (i, want.strip_prefix(&pairs[i].local).unwrap().to_string_lossy().into_owned()))
}

/// Keeps a folder (or a file, or the whole folder) on demand here for good, or stops keeping it.
pub fn set_kept(path: &Path, keep: bool) -> Result<String> {
    let mut pairs = load_pairs();
    let (i, rel) = pair_of_path(&pairs, path).with_context(|| format!("{} isn't inside a synced folder", path.display()))?;
    if !pairs[i].on_demand {
        bail!("{} is a full copy already: everything in it is kept here", pretty(&pairs[i].local));
    }
    let pair = &mut pairs[i];
    pair.pinned.retain(|k| k != &rel);
    let shown = if rel.is_empty() { pretty(&pair.local) } else { format!("{}/{rel}", pretty(&pair.local)) };
    if keep {
        pair.pinned.push(rel);
    } else {
        #[cfg(target_os = "linux")]
        super::drive::free(&pair.id, &rel, &pair.pinned);
    }
    save_pairs(&pairs)?;
    Ok(shown)
}

/// `tpcloud sync keep <path>`.
pub fn keep(path: &Path) -> Result<()> {
    let shown = set_kept(path, true)?;
    println!();
    println!("  {} {} {}", ui::green("✓"), ui::bold(&shown), ui::dim("is kept on this computer from now on (fetched in the background)"));
    println!();
    Ok(())
}

/// `tpcloud sync free <path>`: no longer kept for good, and what was fetched gives its room back.
pub fn free(path: &Path) -> Result<()> {
    let pairs = load_pairs();
    let (i, _) = pair_of_path(&pairs, path).with_context(|| format!("{} isn't inside a synced folder", path.display()))?;
    let before = cached_here(&pairs[i]);
    let shown = set_kept(path, false)?;
    let after = cached_here(&load_pairs()[i]);
    println!();
    println!("  {} {} {}", ui::green("✓"), ui::bold(&shown), ui::dim(&format!("takes no room now ({} given back); it's fetched again when opened", human(before.saturating_sub(after)))));
    println!();
    Ok(())
}

/// `tpcloud sync mode <folder> demand|full`, and the terminal app's «m».
pub fn change_mode(folder: &Path, on_demand: bool, progress: bool) -> Result<String> {
    let mut pairs = load_pairs();
    let i = find_pair(&pairs, folder).with_context(|| format!("{} isn't a synced folder (`tpcloud sync` lists them)", folder.display()))?;
    if pairs[i].on_demand == on_demand {
        return Ok(format!("{} is {} already", pretty(&pairs[i].local), if on_demand { "on demand" } else { "a full copy" }));
    }
    let shown = pretty(&pairs[i].local);
    if on_demand {
        if !demand_possible() {
            bail!("this computer can't mount folders on demand (no FUSE)");
        }
        // Everything up to date both ways first, then the files into the cache.
        let (remote, _) = super::connect_network(&pairs[i].network)?;
        let outcome = if progress { foreground_pass(&pairs[i], &remote, "ctrl-c  stop (nothing changes until this is done)").0 } else { pass(&pairs[i], &remote, &mut |_| {}, &AtomicBool::new(false)) };
        let report = outcome?;
        if report.stopped {
            bail!("stopped: {shown} is as it was");
        }
        remove_service(); // nothing else writes while its files move
        verify_uploaded(&pairs[i], &remote).inspect_err(|_| {
            let _ = start_service();
        })?;
        pairs[i].on_demand = true;
        let pair = pairs[i].clone();
        save_pairs(&pairs)?;
        remove_service();
        #[cfg(target_os = "linux")]
        move_tree(&pair.local, &super::drive::files_dir(&pair.id))?;
        let _ = std::fs::remove_file(state_file(&pair.id));
        start_service()?;
        wait_mounted(&pair.local);
        Ok(format!("{shown} is on demand: what was here is kept a day, then given back"))
    } else {
        pairs[i].on_demand = false;
        pairs[i].pinned.clear();
        let pair = pairs[i].clone();
        save_pairs(&pairs)?;
        remove_service();
        release_demand(&pair)?;
        let _ = std::fs::remove_file(state_file(&pair.id));
        start_service()?;
        Ok(format!("{shown} is a full copy: the rest of its files are coming down in the background"))
    }
}

/// `tpcloud sync mode <folder> demand|full`.
pub fn mode(folder: &Path, which: &str) -> Result<()> {
    let on_demand = match which {
        "demand" | "on-demand" => true,
        "full" => false,
        _ => bail!("`tpcloud sync mode <folder> demand` or `… full`"),
    };
    println!();
    let said = change_mode(folder, on_demand, true)?;
    println!("  {} {said}", ui::green("✓"));
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
    if pair.on_demand {
        release_demand(&pair)?;
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
        .filter(|p| !p.paused && !p.on_demand)
        .map(|p| {
            let outcome = super::connect_network(&p.network).and_then(|(remote, _)| pass(&p, &remote, &mut |_| {}, &AtomicBool::new(false)));
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
    for p in pairs.iter().filter(|p| !p.paused && !p.on_demand) {
        println!("  {}", ui::bold(&pretty(&p.local)));
        let (outcome, detached, cancelled) = match super::connect_network(&p.network) {
            Ok((remote, _)) => foreground_pass(p, &remote, "d  leave it to the background    ctrl-c  stop"),
            Err(e) => (Err(e), false, false),
        };
        if let Ok(r) = &outcome {
            if !r.stopped {
                record_status(&p.id, &outcome);
            }
        } else {
            record_status(&p.id, &outcome);
        }
        match outcome {
            Ok(r) if r.stopped => {
                println!("  {} {}", ui::dim("■"), ui::dim(&format!("{} so far; {}", r.describe(), if detached { "the service carries on" } else { "stopped" })));
                if cancelled || detached {
                    println!();
                    return Ok(());
                }
            }
            Ok(r) => println!("  {} {}", ui::green("✓"), ui::dim(&r.describe())),
            Err(e) => println!("  {} {}", ui::warn_text("!"), ui::warn_text(&format!("{e:#}"))),
        }
        println!();
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
static STOPPING: AtomicBool = AtomicBool::new(false);

extern "C" fn on_stop(_: libc::c_int) {
    STOPPING.store(true, Ordering::Relaxed);
}

pub fn run() -> Result<()> {
    // Stopped by systemd/launchd: let go of the folders on demand before leaving.
    unsafe {
        libc::signal(libc::SIGTERM, on_stop as usize);
        libc::signal(libc::SIGINT, on_stop as usize);
    }
    let mut seen: HashMap<String, (Option<(usize, u64, i64)>, Option<Instant>)> = HashMap::new();
    let mut backoff: HashMap<String, Instant> = HashMap::new();
    #[cfg(target_os = "linux")]
    let mut mounts: HashMap<String, (std::sync::Arc<super::drive::Drive>, fuser::BackgroundSession, std::sync::Arc<AtomicBool>)> = HashMap::new();
    loop {
        if STOPPING.load(Ordering::Relaxed) {
            #[cfg(target_os = "linux")]
            for (_, (_, session, stop)) in mounts.drain() {
                stop.store(true, Ordering::Relaxed);
                drop(session);
            }
            return Ok(());
        }
        let pairs = load_pairs();
        if pairs.iter().all(|p| p.paused) {
            println!("No folders to sync: stopping.");
            return Ok(());
        }
        #[cfg(target_os = "linux")]
        {
            // Folders on demand: mounted once, then kept up by their own thread.
            let wanted: Vec<&Pair> = pairs.iter().filter(|p| p.on_demand && !p.paused).collect();
            mounts.retain(|id, (_, _, stop)| {
                let keep = wanted.iter().any(|p| &p.id == id);
                if !keep {
                    stop.store(true, Ordering::Relaxed);
                }
                keep
            });
            for p in wanted {
                match mounts.get(&p.id) {
                    Some((drive, _, _)) => {
                        let mut pinned = drive.pinned.write().unwrap();
                        if *pinned != p.pinned {
                            *pinned = p.pinned.clone();
                            drive.poke.store(true, Ordering::Relaxed);
                        }
                    }
                    None => {
                        let drive = super::drive::Drive::new(p.clone());
                        match super::drive::mount(drive.clone()) {
                            Ok(session) => {
                                println!("{}: mounted on demand", pretty(&p.local));
                                let stop = std::sync::Arc::new(AtomicBool::new(false));
                                super::drive::keep_up(drive.clone(), stop.clone());
                                mounts.insert(p.id.clone(), (drive, session, stop));
                            }
                            Err(e) => {
                                println!("{}: couldn't mount it: {e}", pretty(&p.local));
                                record_status(&p.id, &Err(anyhow::anyhow!("couldn't mount it: {e}")));
                            }
                        }
                    }
                }
            }
        }
        for p in pairs.iter().filter(|p| !p.paused && !p.on_demand) {
            let print = fingerprint(&p.local);
            let entry = seen.entry(p.id.clone()).or_insert((None, None));
            let changed_here = print.is_some() && print != entry.0;
            let due = entry.1.is_none_or(|t| t.elapsed() >= REMOTE_EVERY);
            let waiting = backoff.get(&p.id).is_some_and(|until| Instant::now() < *until);
            if !(changed_here || due) || waiting {
                continue;
            }
            let mut last_write = Instant::now() - Duration::from_secs(10);
            let id = p.id.clone();
            let mut publish = |step: &Step| {
                if last_write.elapsed() >= Duration::from_secs(2) {
                    last_write = Instant::now();
                    record_progress(&id, step);
                }
            };
            let outcome = super::connect_network(&p.network).and_then(|(remote, _)| pass(p, &remote, &mut publish, &AtomicBool::new(false)));
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
        let until = Instant::now() + LOCAL_EVERY;
        while Instant::now() < until && !STOPPING.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(200));
        }
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
        let got = kinds(&plan(&local, &remote, &state, true));
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
        let pair = Pair { id: "test".into(), local: local.clone(), remote: "users/me@test".into(), network: String::new(), network_name: String::new(), paused: false, on_demand: false, pinned: vec![] };
        let run = || pass(&pair, &remote, &mut |_| {}, &AtomicBool::new(false)).unwrap();

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
