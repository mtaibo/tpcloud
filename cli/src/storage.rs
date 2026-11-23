//! Where Store keeps the files: a folder on any disk that's already set up (never touched), or a
//! storage pool TPCloud makes on empty disks: Btrfs on one disk, or two or more as a mirror
//! (RAID1, RAID10 from four), optionally encrypted with LUKS. The OS-specific work (listing disks,
//! formatting, mounting at boot) is in `platform::storage`.

use crate::{envfile, paths, platform, ui};
use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

/// The pool's mount point, its Btrfs UUID and, when encrypted, the LUKS UUID of each disk.
pub const POOL: &str = "TPCLOUD_POOL";
pub const POOL_UUID: &str = "TPCLOUD_POOL_UUID";
pub const POOL_LUKS: &str = "TPCLOUD_POOL_LUKS";

pub const DEFAULT_MOUNT: &str = "/mnt/tpcloud";

#[derive(Clone, Debug)]
pub struct Disk {
    pub path: String,
    pub size: u64,
    pub model: String,
    pub removable: bool,
    /// What's on it now, e.g. "ext4 «Fotos»": shown before it's erased.
    pub contents: Vec<String>,
    /// Why it can't be used for a pool (system disk, mounted, read-only…), if it can't.
    pub busy: Option<String>,
}

impl Disk {
    pub fn describe(&self) -> String {
        let model = if self.model.is_empty() { "disk".to_string() } else { self.model.clone() };
        let mut text = format!("{}  {} {}", self.path, human(self.size), model);
        if self.removable {
            text.push_str(" (removable)");
        }
        text
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Level {
    /// All the space, every file once: a failing disk loses what was on it.
    Single,
    /// Every file on two disks: one can fail. Usable ≈ half the total.
    Raid1,
    /// Striped mirrors, from four disks: one per pair can fail, faster.
    Raid10,
}

impl Level {
    pub fn btrfs(self) -> (&'static str, &'static str) {
        match self {
            // One disk keeps two copies of the metadata (Btrfs' default), so a bad sector can't
            // lose the folder tree.
            Level::Single => ("single", "dup"),
            Level::Raid1 => ("raid1", "raid1"),
            Level::Raid10 => ("raid10", "raid10"),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Level::Single => "single",
            Level::Raid1 => "RAID1",
            Level::Raid10 => "RAID10",
        }
    }
}

pub fn human(bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < units.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 { format!("{bytes} B") } else if value >= 100.0 { format!("{value:.0} {}", units[unit]) } else { format!("{value:.1} {}", units[unit]) }
}

/// Free and total bytes of the filesystem a path is (or would be) on.
pub fn space(path: &Path) -> Option<(u64, u64)> {
    let mut at = path.to_path_buf();
    while !at.exists() {
        at = at.parent()?.to_path_buf();
    }
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(at.as_os_str().as_bytes()).ok()?;
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
        return None;
    }
    let block = st.f_frsize as u64;
    Some((st.f_bavail as u64 * block, st.f_blocks as u64 * block))
}

/// Usable space of a pool made of these disks at that level (Btrfs' own arithmetic, roughly).
pub fn usable(sizes: &[u64], level: Level) -> u64 {
    let total: u64 = sizes.iter().sum();
    match level {
        Level::Single => total,
        Level::Raid1 | Level::Raid10 => {
            // Each block goes on two different disks: the biggest disk can't hold more than the
            // rest together.
            let largest = sizes.iter().copied().max().unwrap_or(0);
            (total / 2).min(total - largest)
        }
    }
}

/// The levels possible with this many disks, the safest first.
pub fn levels_for(count: usize) -> Vec<Level> {
    match count {
        0 => vec![],
        1 => vec![Level::Single],
        2 | 3 => vec![Level::Raid1, Level::Single],
        _ => vec![Level::Raid1, Level::Raid10, Level::Single],
    }
}

/// What the settings say about the pool, if there is one.
pub struct Pool {
    pub mount: PathBuf,
    pub uuid: String,
    pub luks: Vec<String>,
}

pub fn configured(env: &Path) -> Option<Pool> {
    let mount = envfile::get(env, POOL).filter(|m| !m.is_empty())?;
    Some(Pool {
        mount: PathBuf::from(mount),
        uuid: envfile::get(env, POOL_UUID).unwrap_or_default(),
        luks: envfile::get_list(env, POOL_LUKS),
    })
}

/// Where the encrypted pool's key lives: on the system disk, so the pool opens by itself at boot
/// (it protects disks that leave the house without the machine). Root only.
pub fn key_file() -> PathBuf {
    paths::state_dir().join("keys/pool.key")
}

/// One line about the pool for `tpcloud status`.
pub fn status_line(env: &Path) -> Option<(String, bool)> {
    let pool = configured(env)?;
    let space_text = space(&pool.mount).map(|(free, total)| format!(", {} free of {}", human(free), human(total))).unwrap_or_default();
    match platform::storage::health(&pool.mount) {
        Some(h) => {
            let ok = h.missing == 0 && h.errors == 0;
            let state = if h.missing > 0 {
                format!("{} disk(s) missing", h.missing)
            } else if h.errors > 0 {
                format!("{} bad blocks found on its disks (a mirror repairs them): check the disks, then `sudo tpcloud storage clear`", h.errors)
            } else {
                "healthy".into()
            };
            Some((format!("{} on {} disk(s){space_text}, {state}", h.level, h.devices), ok))
        }
        None => Some((format!("{} isn't mounted", pool.mount.display()), false)),
    }
}

/// `tpcloud storage`: the disks here, which could make a pool, and the pool's state.
pub fn print() -> Result<()> {
    let env = paths::env_file();
    if let Some(files) = envfile::get(&env, "EXTERNAL_DISK_PATH").filter(|f| !f.is_empty()) {
        let space_text = space(Path::new(&files)).map(|(free, total)| format!("  {} free of {}", human(free), human(total))).unwrap_or_default();
        println!("{}  {}{}", ui::dim("Files"), files, ui::dim(&space_text));
    }
    if let Some((line, ok)) = status_line(&env) {
        println!("{}  {}", ui::dim("Pool "), if ok { line } else { ui::warn_text(&line) });
    }
    println!();
    for d in platform::storage::disks()? {
        let state = match &d.busy {
            Some(why) => ui::dim(why),
            None if d.contents.is_empty() => ui::green("empty: can join a pool"),
            None => ui::warn_text(&format!("can join a pool (erasing {})", d.contents.join(", "))),
        };
        println!("  {}  {}", d.describe(), state);
    }
    Ok(())
}

/// `sudo tpcloud storage add <disk>`: one more disk in the pool, then its data spread again.
pub fn add(disk: &str) -> Result<()> {
    crate::install::require_root("Adding a disk")?;
    let env = paths::env_file();
    let Some(pool) = configured(&env) else { bail!("there's no storage pool here: `sudo tpcloud setup` makes one") };
    if platform::storage::health(&pool.mount).is_none() {
        bail!("the pool isn't mounted at {}: start TPCloud first", pool.mount.display());
    }
    let Some(d) = platform::storage::disks()?.into_iter().find(|d| d.path == disk) else { bail!("{disk} isn't a disk here (`tpcloud storage` lists them)") };
    if let Some(why) = &d.busy {
        bail!("{disk} can't be used: {why}");
    }
    println!("This erases everything on {}{}.", d.describe(), if d.contents.is_empty() { String::new() } else { format!(" ({})", d.contents.join(", ")) });
    print!("Type ERASE to go on: ");
    use std::io::Write;
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    if line.trim() != "ERASE" {
        bail!("nothing changed");
    }
    // A pool of one disk can become a mirror now that there are two.
    let convert = match platform::storage::health(&pool.mount) {
        Some(h) if h.devices == 1 => {
            print!("Keep every file on both disks from now on (RAID1: one can fail, half the space)? [Y/n]: ");
            std::io::stdout().flush()?;
            let mut answer = String::new();
            std::io::stdin().read_line(&mut answer)?;
            (!answer.trim().eq_ignore_ascii_case("n")).then_some(Level::Raid1)
        }
        _ => None,
    };
    let luks = platform::storage::add_disk(&pool, disk, convert)?;
    if let Some(uuid) = luks {
        let mut list = pool.luks.clone();
        list.push(uuid);
        envfile::set_list(&env, POOL_LUKS, &list)?;
        platform::storage::write_units(&pool.mount, &pool.uuid, &list)?;
    }
    ui::done(&format!("{disk} is part of the pool; its data is spread over every disk now."));
    Ok(())
}

/// `tpcloud storage unlock`, run at boot before the pool is mounted.
pub fn unlock() -> Result<()> {
    let Some(pool) = configured(&paths::env_file()) else { return Ok(()) };
    if pool.luks.is_empty() {
        return Ok(());
    }
    platform::storage::unlock(&pool.luks, &key_file())
}

fn scrub_file() -> PathBuf {
    paths::state_dir().join("run/scrub.json")
}

/// The last scrub: when (unix seconds) and whether it found nothing wrong.
pub fn last_scrub() -> Option<(u64, bool, String)> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(scrub_file()).ok()?).ok()?;
    Some((v["at"].as_u64()?, v["ok"].as_bool()?, v["detail"].as_str().unwrap_or("").to_string()))
}

fn scrub(pool: &Pool) -> Result<()> {
    let result = platform::storage::scrub(&pool.mount);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let detail = match &result {
        Ok(()) => String::new(),
        Err(e) => format!("{e:#}"),
    };
    let _ = std::fs::write(scrub_file(), serde_json::json!({ "at": now, "ok": result.is_ok(), "detail": detail }).to_string());
    result
}

/// `sudo tpcloud storage clear`: forget the error counts once the disks have been looked at.
pub fn clear() -> Result<()> {
    crate::install::require_root("Clearing the counts")?;
    let Some(pool) = configured(&paths::env_file()) else { bail!("there's no storage pool here") };
    platform::storage::clear_errors(&pool.mount)?;
    ui::done("Error counts back to zero.");
    Ok(())
}

/// `sudo tpcloud storage scrub`.
pub fn scrub_now() -> Result<()> {
    crate::install::require_root("Checking the pool")?;
    let Some(pool) = configured(&paths::env_file()) else { bail!("there's no storage pool here") };
    ui::step(&format!("Checking every block of {} (this takes a while)", pool.mount.display()));
    scrub(&pool)?;
    ui::done("Checked: nothing wrong, or repaired from the mirror.");
    Ok(())
}

/// Called by the supervisor's side thread: a scrub once a month.
pub fn scrub_if_due() {
    let Some(pool) = configured(&paths::env_file()) else { return };
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    if last_scrub().is_some_and(|(at, _, _)| now.saturating_sub(at) < 30 * 86400) {
        return;
    }
    if platform::storage::health(&pool.mount).is_none() {
        return;
    }
    match scrub(&pool) {
        Ok(()) => println!("tpcloud: storage pool checked"),
        Err(err) => eprintln!("tpcloud: storage check failed: {err:#}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(human(999), "999 B");
        assert_eq!(human(1_000_204_886_016), "1.0 TB");
        assert_eq!(human(256_060_514_304), "256 GB");
    }

    #[test]
    fn usable_space() {
        let tb = 1_000_000_000_000;
        assert_eq!(usable(&[tb, tb], Level::Raid1), tb);
        assert_eq!(usable(&[tb, 2 * tb], Level::Raid1), tb);
        assert_eq!(usable(&[tb, tb, 2 * tb], Level::Raid1), 2 * tb);
        assert_eq!(usable(&[tb, 2 * tb], Level::Single), 3 * tb);
        assert_eq!(levels_for(2), vec![Level::Raid1, Level::Single]);
        assert_eq!(levels_for(4)[1], Level::Raid10);
    }
}
