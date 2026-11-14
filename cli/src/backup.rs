//! Copies of TPCloud's own data: the settings, every database (accounts, Store's index, albums and
//! share links, the house), Home's firmware and broker state. Not the files themselves (those are
//! on their own disk, with their own backup) and not the caches (thumbnails and previews are made
//! again). A few MB each, made every night by the service into TPCLOUD_BACKUP_DIR, which should
//! be on another disk; the last TPCLOUD_BACKUP_KEEP (14) are kept.

use crate::{envfile, install, paths, ui};
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// In every copy, so `restore` only ever takes a folder that really is one.
const MANIFEST: &str = "TPCLOUD-BACKUP";

/// Never copied: what's made again (logs, the supervisor's state, thumbnails and previews).
const SKIP: [&str; 4] = ["logs", "run", "data/store/media", "data/store/thumbs"];

pub fn dir() -> Option<PathBuf> {
    envfile::get(&paths::env_file(), "TPCLOUD_BACKUP_DIR").filter(|d| !d.is_empty()).map(PathBuf::from)
}

fn keep() -> usize {
    envfile::get(&paths::env_file(), "TPCLOUD_BACKUP_KEEP").and_then(|k| k.parse().ok()).filter(|&k| k > 0).unwrap_or(14)
}

fn last_file() -> PathBuf {
    paths::state_dir().join("run/backup.json")
}

pub fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Local time as (year, month, day, hour, minute).
pub fn local_time(secs: u64) -> (i32, u32, u32, u32, u32) {
    #[allow(deprecated)] // 64-bit on every system TPCloud is built for
    let t = secs as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&t, &mut tm) };
    (tm.tm_year + 1900, tm.tm_mon as u32 + 1, tm.tm_mday as u32, tm.tm_hour as u32, tm.tm_min as u32)
}

/// What the last backup did, for `tpcloud status` and for deciding when the next one is due.
pub struct Last {
    pub at: u64,
    pub ok: bool,
    pub detail: String,
}

pub fn last() -> Option<Last> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(last_file()).ok()?).ok()?;
    Some(Last {
        at: v["at"].as_u64()?,
        ok: v["ok"].as_bool().unwrap_or(false),
        detail: v["detail"].as_str().unwrap_or_default().to_string(),
    })
}

fn last_ok_at() -> Option<u64> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(last_file()).ok()?).ok()?;
    v["last_ok"].as_u64()
}

fn record(ok: bool, detail: &str) {
    let last_ok = if ok { Some(now_secs()) } else { last_ok_at() };
    let body = serde_json::json!({ "at": now_secs(), "ok": ok, "detail": detail, "last_ok": last_ok });
    let path = last_file();
    let _ = std::fs::create_dir_all(path.parent().unwrap());
    let _ = std::fs::write(path, body.to_string());
}

/// The nightly one: between 3 and 6 in the morning, once a day; or straight away when the last
/// good one is two days old (the machine was off at night, or it kept failing).
pub fn due() -> bool {
    if dir().is_none() {
        return false;
    }
    let now = now_secs();
    let Some(last_ok) = last_ok_at() else { return true };
    let age = now.saturating_sub(last_ok);
    let tried_recently = last().is_some_and(|l| now.saturating_sub(l.at) < 3600);
    if tried_recently {
        return false;
    }
    let (_, _, _, hour, _) = local_time(now);
    (age > 20 * 3600 && (3..6).contains(&hour)) || age > 48 * 3600
}

/// Makes one now, if a folder is set. Returns the copy's folder.
pub fn run() -> Result<PathBuf> {
    let Some(root) = dir() else { bail!("no backup folder set: `sudo tpcloud backup --to <folder>`") };
    let result = make(&root);
    match &result {
        Ok(path) => record(true, &path.display().to_string()),
        Err(err) => record(false, &format!("{err:#}")),
    }
    result
}

fn make(root: &Path) -> Result<PathBuf> {
    // One at a time (the service's nightly one and a `tpcloud backup` could meet).
    let lock = paths::state_dir().join("run/backup.lock");
    let _ = std::fs::create_dir_all(lock.parent().unwrap());
    if let Ok(meta) = std::fs::metadata(&lock) {
        let old = meta.modified().ok().and_then(|m| m.elapsed().ok()).is_some_and(|e| e.as_secs() > 3600);
        if !old {
            bail!("another backup is running");
        }
    }
    std::fs::write(&lock, std::process::id().to_string())?;
    let result = make_locked(root);
    let _ = std::fs::remove_file(&lock);
    result
}

fn make_locked(root: &Path) -> Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    if root.starts_with(paths::state_dir()) {
        bail!("the backup folder can't be inside {} (it would copy itself)", paths::state_dir().display());
    }
    std::fs::create_dir_all(root).with_context(|| format!("creating {}", root.display()))?;
    // The settings hold the tunnel's token: only root may read the copies.
    std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700))?;
    let (y, mo, d, h, mi) = local_time(now_secs());
    let mut name = format!("{y:04}-{mo:02}-{d:02}_{h:02}{mi:02}");
    let mut n = 2;
    while root.join(&name).exists() {
        name = format!("{y:04}-{mo:02}-{d:02}_{h:02}{mi:02}-{n}");
        n += 1;
    }
    let tmp = root.join(format!(".{name}.partial"));
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }
    std::fs::create_dir_all(&tmp)?;

    let state = paths::state_dir();
    let mut databases = Vec::new();
    copy_tree(&state, Path::new(""), &tmp, &mut databases)?;
    snapshot_databases(&databases)?;
    std::fs::write(
        tmp.join(MANIFEST),
        format!("TPCloud {}\nmade {}\nfrom {}\n", env!("CARGO_PKG_VERSION"), name, state.display()),
    )?;
    let done = root.join(&name);
    std::fs::rename(&tmp, &done)?;
    prune(root)?;
    Ok(done)
}

/// Copies `src/rel` into `dst/rel`, leaving out SKIP and SQLite's side files; databases are only
/// listed, to be copied through SQLite itself (a plain copy of a live one can be torn).
fn copy_tree(src: &Path, rel: &Path, dst: &Path, databases: &mut Vec<(PathBuf, PathBuf)>) -> Result<()> {
    std::fs::create_dir_all(dst.join(rel))?;
    for entry in std::fs::read_dir(src.join(rel))? {
        let entry = entry?;
        let child = rel.join(entry.file_name());
        let text = child.to_string_lossy();
        if SKIP.iter().any(|s| text == *s) {
            continue;
        }
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_tree(src, &child, dst, databases)?;
        } else if kind.is_file() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with("-wal") || name.ends_with("-shm") || name.ends_with("-journal") {
                continue;
            }
            if is_sqlite(&src.join(&child)) {
                databases.push((src.join(&child), dst.join(&child)));
            } else {
                std::fs::copy(src.join(&child), dst.join(&child)).with_context(|| format!("copying {}", child.display()))?;
            }
        }
    }
    Ok(())
}

/// By its header, not its name: Mosquitto keeps its state in a `mosquitto.db` of its own format.
fn is_sqlite(path: &Path) -> bool {
    use std::io::Read;
    let mut head = [0u8; 16];
    std::fs::File::open(path).and_then(|mut f| f.read_exact(&mut head)).is_ok() && &head == b"SQLite format 3\0"
}

/// SQLite's online backup, with the bundled Python, then a quick check of each copy.
fn snapshot_databases(pairs: &[(PathBuf, PathBuf)]) -> Result<()> {
    if pairs.is_empty() {
        return Ok(());
    }
    let script = "import sqlite3, sys\n\
        a = sys.argv[1:]\n\
        for s, d in zip(a[::2], a[1::2]):\n\
        \x20   src = sqlite3.connect(s, timeout=30)\n\
        \x20   dst = sqlite3.connect(d)\n\
        \x20   src.backup(dst)\n\
        \x20   src.close()\n\
        \x20   ok = dst.execute('pragma quick_check').fetchone()[0]\n\
        \x20   dst.close()\n\
        \x20   if ok != 'ok':\n\
        \x20       sys.exit(f'{d}: {ok}')\n";
    let mut cmd = Command::new(paths::runtime_dir().join("python/bin/python3"));
    cmd.arg("-c").arg(script).env_clear().env("PYTHONDONTWRITEBYTECODE", "1");
    for (s, d) in pairs {
        cmd.arg(s).arg(d);
    }
    let out = cmd.output().context("couldn't run the bundled Python")?;
    if !out.status.success() {
        bail!("copying the databases: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(())
}

/// The copies in a folder, oldest first.
pub fn list(root: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(root)
        .map(|entries| entries.flatten().map(|e| e.path()).filter(|p| p.join(MANIFEST).is_file()).collect())
        .unwrap_or_default();
    found.sort();
    found
}

fn prune(root: &Path) -> Result<()> {
    let copies = list(root);
    let keep = keep();
    if copies.len() > keep {
        for old in &copies[..copies.len() - keep] {
            std::fs::remove_dir_all(old)?;
        }
    }
    Ok(())
}

/// `tpcloud backup`: sets the folder if asked, then makes one.
pub fn command(to: Option<PathBuf>, off: bool) -> Result<()> {
    install::require_root("Backing up")?;
    let env = paths::env_file();
    if !env.exists() {
        bail!("TPCloud isn't set up here yet: `sudo tpcloud install`");
    }
    if off {
        envfile::set(&env, "TPCLOUD_BACKUP_DIR", "")?;
        ui::done("Nightly backups are off. The copies already made are kept.");
        return Ok(());
    }
    if let Some(to) = to {
        let to = if to.is_absolute() { to } else { std::env::current_dir()?.join(to) };
        envfile::set(&env, "TPCLOUD_BACKUP_DIR", &to.display().to_string())?;
        ui::step(&format!("Backups go to {} from now on, every night", to.display()));
    }
    ui::step("Backing up TPCloud's data");
    let path = run()?;
    let size = du(&path);
    ui::done(&format!("Backed up to {} ({}).", path.display(), human(size)));
    Ok(())
}

pub fn print_list() -> Result<()> {
    let Some(root) = dir() else { bail!("no backup folder set: `sudo tpcloud backup --to <folder>`") };
    let copies = list(&root);
    if copies.is_empty() {
        println!("No backups yet in {}: `sudo tpcloud backup`.", root.display());
    }
    for c in copies.iter().rev() {
        println!("  {}  {}", c.display(), ui::dim(&human(du(c))));
    }
    Ok(())
}

/// `tpcloud restore <copy>`: puts a copy's settings and databases back. What's there now is
/// backed up first (when a folder is set), so a restore can itself be undone.
pub fn restore(copy: &Path) -> Result<()> {
    install::require_root("Restoring")?;
    if !copy.join(MANIFEST).is_file() {
        bail!("{} isn't a TPCloud backup (it has no {MANIFEST} file)", copy.display());
    }
    let state = paths::state_dir();
    if dir().is_some() && paths::env_file().exists() {
        ui::step("Backing up what's here now first");
        let before = run()?;
        ui::done(&format!("Saved as {}", before.display()));
    }
    let was_running = Command::new("systemctl").args(["is-active", "--quiet", "tpcloud"]).status().map(|s| s.success()).unwrap_or(false);
    if was_running {
        ui::step("Stopping TPCloud");
        install::systemctl(&["stop", "tpcloud"])?;
    }
    ui::step(&format!("Restoring {}", copy.display()));
    let result = put_back(copy, Path::new(""), &state);
    if was_running {
        ui::step("Starting TPCloud");
        install::systemctl(&["start", "tpcloud"])?;
    }
    result?;
    ui::done("Restored.");
    Ok(())
}

fn put_back(copy: &Path, rel: &Path, state: &Path) -> Result<()> {
    std::fs::create_dir_all(state.join(rel))?;
    for entry in std::fs::read_dir(copy.join(rel))? {
        let entry = entry?;
        let child = rel.join(entry.file_name());
        if child.as_os_str() == MANIFEST {
            continue;
        }
        if entry.file_type()?.is_dir() {
            put_back(copy, &child, state)?;
        } else {
            let target = state.join(&child);
            if target.extension().is_some_and(|e| e == "db") {
                // The old database's side files would be replayed onto the restored one.
                for side in ["-wal", "-shm", "-journal"] {
                    let _ = std::fs::remove_file(format!("{}{side}", target.display()));
                }
            }
            let tmp = target.with_file_name(format!(".{}.restore", entry.file_name().to_string_lossy()));
            std::fs::copy(entry.path(), &tmp).with_context(|| format!("restoring {}", child.display()))?;
            std::fs::rename(&tmp, &target)?;
        }
    }
    Ok(())
}

fn du(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else { return 0 };
    entries
        .flatten()
        .map(|e| match e.file_type() {
            Ok(t) if t.is_dir() => du(&e.path()),
            _ => e.metadata().map(|m| m.len()).unwrap_or(0),
        })
        .sum()
}

pub fn human(bytes: u64) -> String {
    match bytes {
        0..=1023 => format!("{bytes} B"),
        1024..=1_048_575 => format!("{:.0} KB", bytes as f64 / 1024.0),
        _ => format!("{:.1} MB", bytes as f64 / 1_048_576.0),
    }
}

#[cfg(test)]
mod tests {
    use super::is_sqlite;

    #[test]
    fn tells_sqlite_from_other_db_files() {
        let dir = std::env::temp_dir().join(format!("tpcloud-backup-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sqlite = dir.join("a.db");
        let mut bytes = b"SQLite format 3\0".to_vec();
        bytes.extend([0u8; 84]);
        std::fs::write(&sqlite, bytes).unwrap();
        let mosquitto = dir.join("mosquitto.db");
        std::fs::write(&mosquitto, [0x00, 0xb5, 0x00, 0x6d, 0x6f, 0x73, 0x71, 0x75, 0x69, 0x74, 0x74, 0x6f, 0x20, 0x64, 0x62, 0x00, 0x01]).unwrap();
        let short = dir.join("empty.db");
        std::fs::write(&short, b"").unwrap();
        assert!(is_sqlite(&sqlite));
        assert!(!is_sqlite(&mosquitto));
        assert!(!is_sqlite(&short));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
