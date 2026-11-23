//! Disks on Linux: `lsblk` to list them, the bundled btrfs/mkfs.btrfs/cryptsetup to make a pool,
//! systemd units to open and mount it at boot.
//!
//! At boot: `tpcloud-storage-unlock.service` (only when encrypted) opens each disk with the key in
//! the state folder, then `<mount>.mount` mounts the pool, then tpcloud.service starts
//! (`RequiresMountsFor`). None of them is part of the boot itself (`multi-user.target`, not
//! `local-fs.target`): a missing disk leaves TPCloud stopped, never the machine stuck at boot.

use crate::storage::{Disk, Level, Pool};
use crate::{paths, ui};
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The bundled tool if the runtime has it, else the system's.
fn tool(name: &str) -> PathBuf {
    let bundled = paths::runtime_dir().join("bin").join(name);
    if bundled.exists() { bundled } else { PathBuf::from(name) }
}

fn run(program: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new(program).args(args).output().with_context(|| format!("couldn't run {}", program.display()))?;
    if !out.status.success() {
        bail!(
            "{} {} failed: {}",
            program.file_name().unwrap_or_default().to_string_lossy(),
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

const SYSTEM_MOUNTS: [&str; 7] = ["/", "/boot", "/boot/firmware", "/boot/efi", "/usr", "/var", "[SWAP]"];

/// Every disk, each with why it can't join a pool (if it can't).
pub fn disks() -> Result<Vec<Disk>> {
    let out = run(Path::new("lsblk"), &["--json", "--bytes", "--output", "NAME,PATH,SIZE,TYPE,MODEL,FSTYPE,LABEL,MOUNTPOINTS,RM,RO"])?;
    let json: serde_json::Value = serde_json::from_str(&out).context("lsblk's answer wasn't JSON")?;
    let loops = std::env::var("TPCLOUD_STORAGE_LOOPS").as_deref() == Ok("1");
    let mut list = Vec::new();
    for dev in json["blockdevices"].as_array().cloned().unwrap_or_default() {
        let kind = dev["type"].as_str().unwrap_or("");
        let name = dev["name"].as_str().unwrap_or("");
        if !(kind == "disk" || (loops && kind == "loop")) || name.starts_with("zram") || name.contains("boot") || name.contains("rpmb") {
            continue;
        }
        let size = json_u64(&dev["size"]);
        if size < 1_000_000_000 {
            continue;
        }
        let (mut mounts, mut contents, mut holders) = (Vec::new(), Vec::new(), Vec::new());
        walk(&dev, &mut mounts, &mut contents, &mut holders);
        let system = mounts.iter().any(|m| SYSTEM_MOUNTS.contains(&m.as_str()));
        let busy = if super::is_nixos() {
            Some("new storage isn't available on NixOS yet: pick a folder".to_string())
        } else if system {
            Some("the system's disk".to_string())
        } else if !mounts.is_empty() {
            Some(format!("in use, mounted at {}", mounts.join(", ")))
        } else if !holders.is_empty() {
            Some(format!("part of {}", holders.join(", ")))
        } else if json_bool(&dev["ro"]) {
            Some("read-only".to_string())
        } else {
            None
        };
        list.push(Disk {
            path: dev["path"].as_str().unwrap_or("").to_string(),
            size,
            model: dev["model"].as_str().unwrap_or("").trim().to_string(),
            removable: json_bool(&dev["rm"]),
            contents,
            busy,
        });
    }
    Ok(list)
}

fn json_u64(v: &serde_json::Value) -> u64 {
    v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())).unwrap_or(0)
}

fn json_bool(v: &serde_json::Value) -> bool {
    v.as_bool().unwrap_or_else(|| matches!(v.as_str(), Some("1") | Some("true")))
}

/// Collects mount points, what's on it, and anything built on top (RAID, LVM, encryption).
fn walk(dev: &serde_json::Value, mounts: &mut Vec<String>, contents: &mut Vec<String>, holders: &mut Vec<String>) {
    for m in dev["mountpoints"].as_array().into_iter().flatten().filter_map(|m| m.as_str()) {
        mounts.push(m.to_string());
    }
    if let Some(m) = dev["mountpoint"].as_str() {
        mounts.push(m.to_string());
    }
    if let Some(fs) = dev["fstype"].as_str().filter(|f| !f.is_empty()) {
        let label = dev["label"].as_str().filter(|l| !l.is_empty()).map(|l| format!(" «{l}»")).unwrap_or_default();
        match fs {
            "linux_raid_member" => holders.push("a RAID".into()),
            "LVM2_member" => holders.push("LVM".into()),
            _ => contents.push(format!("{fs}{label}")),
        }
    }
    match dev["type"].as_str().unwrap_or("") {
        t if t.starts_with("raid") => holders.push("a RAID".into()),
        "lvm" => holders.push("LVM".into()),
        "crypt" if !dev["name"].as_str().unwrap_or("").starts_with("tpcloud-") => holders.push("an encrypted volume".into()),
        _ => {}
    }
    for child in dev["children"].as_array().into_iter().flatten() {
        walk(child, mounts, contents, holders);
    }
}

/// The kernel can mount it (loads the module if it has to).
fn kernel_has(fs_or_module: &str, check: impl Fn() -> bool) -> bool {
    if check() {
        return true;
    }
    let _ = Command::new("modprobe").arg(fs_or_module).status();
    check()
}

fn has_btrfs() -> bool {
    std::fs::read_to_string("/proc/filesystems").map(|t| t.lines().any(|l| l.trim_end().ends_with("btrfs"))).unwrap_or(false)
}

fn has_dm_crypt() -> bool {
    Path::new("/sys/module/dm_crypt").exists()
}

fn random(n: usize) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut b = vec![0u8; n];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut b)?;
    Ok(b)
}

/// A recovery key people can write down: 32 letters/digits (160 bits) in groups of four.
fn recovery_key() -> Result<String> {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let bytes = random(32)?;
    let chars: Vec<char> = bytes.iter().map(|b| ALPHABET[(*b as usize) % 32] as char).collect();
    Ok(chars.chunks(4).map(|c| c.iter().collect::<String>()).collect::<Vec<_>>().join("-"))
}

fn mapper_name(luks_uuid: &str) -> String {
    format!("tpcloud-{}", &luks_uuid[..luks_uuid.len().min(8)])
}

/// systemd's name for the mount unit of a path (`/mnt/tpcloud` → `mnt-tpcloud.mount`).
pub fn mount_unit(mount: &Path) -> String {
    let text = mount.to_string_lossy();
    let trimmed = text.trim_matches('/');
    let mut name = String::new();
    for (i, c) in trimmed.chars().enumerate() {
        match c {
            '/' => name.push('-'),
            c if c.is_ascii_alphanumeric() || c == '_' || c == ':' || (c == '.' && i > 0) => name.push(c),
            c => name.push_str(&format!("\\x{:02x}", c as u32)),
        }
    }
    format!("{name}.mount")
}

const UNLOCK_UNIT: &str = "tpcloud-storage-unlock.service";

fn unit_dir() -> PathBuf {
    std::env::var_os("TPCLOUD_UNIT_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/etc/systemd/system"))
}

/// The units that open (if encrypted) and mount the pool at boot.
pub fn write_units(mount: &Path, uuid: &str, luks: &[String]) -> Result<()> {
    let unit = mount_unit(mount);
    let mut options = "noatime,compress=zstd:1".to_string();
    let mut needs = String::new();
    if !luks.is_empty() {
        for id in luks {
            options.push_str(&format!(",device=/dev/mapper/{}", mapper_name(id)));
        }
        // Run on every mount, not only once per boot: a oneshot that stayed "active" wouldn't open
        // disks locked again since (a manual stop, a disk unplugged and back).
        needs = format!("Wants={UNLOCK_UNIT}\nAfter={UNLOCK_UNIT}\n");
        let unlock = format!(
            "[Unit]\nDescription=Unlock TPCloud's storage\nDefaultDependencies=no\nAfter=systemd-udev-settle.service\nBefore={unit}\n\n\
             [Service]\nType=oneshot\nExecStart={} storage unlock\n",
            paths::bin_dir().join("tpcloud").display()
        );
        std::fs::write(unit_dir().join(UNLOCK_UNIT), unlock)?;
    }
    let body = format!(
        "# Written by tpcloud: TPCloud's storage pool.\n[Unit]\nDescription=TPCloud storage\n{needs}\n\
         [Mount]\nWhat=UUID={uuid}\nWhere={}\nType=btrfs\nOptions={options}\n\n[Install]\nWantedBy=multi-user.target\n",
        mount.display()
    );
    std::fs::write(unit_dir().join(&unit), body)?;
    run(Path::new("systemctl"), &["daemon-reload"])?;
    // An unlock left "active" by an older unit file (RemainAfterExit) would never run again.
    if !luks.is_empty() {
        let _ = Command::new("systemctl").args(["stop", UNLOCK_UNIT]).status();
    }
    Ok(())
}

/// `tpcloud storage unlock` (run at boot): opens every disk of the encrypted pool not open yet.
pub fn unlock(luks: &[String], key: &Path) -> Result<()> {
    for id in luks {
        let name = mapper_name(id);
        if Path::new("/dev/mapper").join(&name).exists() {
            continue;
        }
        let dev = format!("/dev/disk/by-uuid/{id}");
        // A USB disk can show up a little after the boot gets here.
        for _ in 0..30 {
            if Path::new(&dev).exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        run(&tool("cryptsetup"), &["open", "--key-file", &key.to_string_lossy(), "--allow-discards", &dev, &name])?;
    }
    let _ = run(&tool("btrfs"), &["device", "scan"]);
    Ok(())
}

fn luks_format(disk: &str, key: &Path, recovery: Option<&str>) -> Result<String> {
    let crypt = tool("cryptsetup");
    let key = key.to_string_lossy().to_string();
    run(&crypt, &["luksFormat", "--type", "luks2", "--batch-mode", "--key-file", &key, disk])?;
    if let Some(recovery) = recovery {
        // The second key, for a person: given on stdin, never written anywhere.
        use std::io::Write;
        let mut child = Command::new(&crypt)
            .args(["luksAddKey", "--key-file", &key, "--batch-mode", disk, "-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        child.stdin.take().unwrap().write_all(recovery.as_bytes())?;
        let out = child.wait_with_output()?;
        if !out.status.success() {
            bail!("adding the recovery key failed: {}", String::from_utf8_lossy(&out.stderr).trim());
        }
    }
    let uuid = run(&crypt, &["luksUUID", disk])?.trim().to_string();
    run(&crypt, &["open", "--key-file", &key, "--allow-discards", disk, &mapper_name(&uuid)])?;
    Ok(uuid)
}

pub struct Made {
    pub uuid: String,
    pub luks: Vec<String>,
    pub recovery: Option<String>,
}

/// Erases the disks and makes the pool, mounted at `mount` from now on, with a `files` subvolume
/// for Store. Everything that can be checked is checked before the first disk is touched.
pub fn create_pool(chosen: &[String], level: Level, encrypt: bool, mount: &Path) -> Result<Made> {
    if chosen.is_empty() {
        bail!("no disks chosen");
    }
    let now = disks()?;
    for path in chosen {
        match now.iter().find(|d| &d.path == path) {
            None => bail!("{path} isn't here any more"),
            Some(d) if d.busy.is_some() => bail!("{path} can't be used: {}", d.busy.as_deref().unwrap_or("")),
            _ => {}
        }
    }
    if !kernel_has("btrfs", has_btrfs) {
        bail!("this system's kernel can't mount Btrfs (no btrfs module): nothing was erased");
    }
    if encrypt && !kernel_has("dm_crypt", has_dm_crypt) {
        bail!("this system's kernel has no disk encryption (dm_crypt): nothing was erased");
    }
    if mount.exists() && std::fs::read_dir(mount).map(|mut d| d.next().is_some()).unwrap_or(false) {
        bail!("{} already has files in it: pick another place for the pool", mount.display());
    }

    let mut devices = Vec::new();
    let mut luks = Vec::new();
    let mut recovery = None;
    if encrypt {
        let key = crate::storage::key_file();
        std::fs::create_dir_all(key.parent().unwrap())?;
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(key.parent().unwrap(), std::fs::Permissions::from_mode(0o700))?;
        }
        if key.exists() {
            // Never lose a key that might open something: kept aside under a dated name.
            let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            std::fs::rename(&key, key.with_extension(format!("key.{stamp}")))?;
        }
        std::fs::write(&key, random(64)?)?;
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o400))?;
        }
        recovery = Some(recovery_key()?);
    }
    for disk in chosen {
        ui::step(&format!("Erasing {disk}"));
        let _ = Command::new("wipefs").args(["--all", "--quiet", disk]).status();
        if encrypt {
            let uuid = luks_format(disk, &crate::storage::key_file(), recovery.as_deref())?;
            devices.push(format!("/dev/mapper/{}", mapper_name(&uuid)));
            luks.push(uuid);
        } else {
            devices.push(disk.clone());
        }
    }
    ui::step("Making the pool");
    let (data, meta) = level.btrfs();
    let mut args = vec!["-f", "-q", "-L", "TPCloud", "-d", data, "-m", meta];
    args.extend(devices.iter().map(String::as_str));
    run(&tool("mkfs.btrfs"), &args)?;
    let shown = run(&tool("btrfs"), &["filesystem", "show", &devices[0]])?;
    let uuid = shown.split("uuid:").nth(1).and_then(|r| r.split_whitespace().next()).context("couldn't read the pool's UUID")?.to_string();

    std::fs::create_dir_all(mount)?;
    write_units(mount, &uuid, &luks)?;
    let _ = run(&tool("btrfs"), &["device", "scan"]);
    run(Path::new("systemctl"), &["enable", "--now", &mount_unit(mount)])?;
    let files = mount.join("files");
    run(&tool("btrfs"), &["subvolume", "create", &files.to_string_lossy()])?;
    Ok(Made { uuid, luks, recovery })
}

/// One more disk in the pool (encrypted like the others), the data spread again in the
/// background; `convert` turns a one-disk pool into a mirror. Returns its LUKS UUID if encrypted.
pub fn add_disk(pool: &Pool, disk: &str, convert: Option<Level>) -> Result<Option<String>> {
    if health(&pool.mount).is_none() {
        bail!("the pool isn't mounted at {}: nothing was erased", pool.mount.display());
    }
    let _ = Command::new("wipefs").args(["--all", "--quiet", disk]).status();
    let (device, luks) = if pool.luks.is_empty() {
        (disk.to_string(), None)
    } else {
        // Only the machine's key opens the new disk: a recovery key written down before can't.
        let uuid = luks_format(disk, &crate::storage::key_file(), None)?;
        (format!("/dev/mapper/{}", mapper_name(&uuid)), Some(uuid))
    };
    let mount = pool.mount.to_string_lossy().to_string();
    run(&tool("btrfs"), &["device", "add", "-f", &device, &mount])?;
    let mut args = vec!["balance".to_string(), "start".into(), "--bg".into()];
    match convert {
        Some(level) => {
            let (d, m) = level.btrfs();
            args.push(format!("-dconvert={d}"));
            args.push(format!("-mconvert={m}"));
        }
        None => args.push("--full-balance".into()),
    }
    args.push(mount);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run(&tool("btrfs"), &refs)?;
    Ok(luks)
}

pub struct Health {
    pub level: String,
    pub devices: usize,
    pub missing: usize,
    pub errors: u64,
}

/// The pool's state, if it's mounted.
pub fn health(mount: &Path) -> Option<Health> {
    let mounts = std::fs::read_to_string("/proc/self/mounts").ok()?;
    let target = mount.to_string_lossy();
    if !mounts.lines().any(|l| {
        let mut f = l.split_whitespace();
        let (_, at, fs) = (f.next(), f.next(), f.next());
        at == Some(&target) && fs == Some("btrfs")
    }) {
        return None;
    }
    let btrfs = tool("btrfs");
    let show = run(&btrfs, &["filesystem", "show", &target]).unwrap_or_default();
    let devices = show.lines().filter(|l| l.trim_start().starts_with("devid")).count();
    let missing = show.lines().filter(|l| l.contains("missing")).count();
    let stats = Command::new(&btrfs).args(["device", "stats", &target]).output().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
    let errors = stats.lines().filter_map(|l| l.split_whitespace().last()?.parse::<u64>().ok()).sum();
    let df = run(&btrfs, &["filesystem", "df", &target]).unwrap_or_default();
    let level = df
        .lines()
        .find(|l| l.starts_with("Data,"))
        .and_then(|l| l.split(',').nth(1))
        .and_then(|r| r.split(':').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Btrfs".into());
    Some(Health { level: if level == "single" { "one copy".into() } else { level }, devices, missing, errors })
}

pub fn clear_errors(mount: &Path) -> Result<()> {
    run(&tool("btrfs"), &["device", "stats", "-z", &mount.to_string_lossy()]).map(|_| ())
}

/// Reads every block and checks it against its checksum, repairing from the other copy on a
/// mirror. Blocks until done (hours on a big pool): run from the supervisor's side thread.
pub fn scrub(mount: &Path) -> Result<()> {
    run(&tool("btrfs"), &["scrub", "start", "-B", "-d", &mount.to_string_lossy()]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_names() {
        assert_eq!(mount_unit(Path::new("/mnt/tpcloud")), "mnt-tpcloud.mount");
        assert_eq!(mount_unit(Path::new("/srv/my disk")), "srv-my\\x20disk.mount");
        assert_eq!(mount_unit(Path::new("/mnt/a-b")), "mnt-a\\x2db.mount");
    }

    #[test]
    fn recovery_keys_are_grouped() {
        let k = recovery_key().unwrap();
        assert_eq!(k.len(), 32 + 7);
        assert_eq!(k.split('-').count(), 8);
    }

    #[test]
    fn walks_a_disk_tree() {
        let dev: serde_json::Value = serde_json::from_str(r#"{"name":"sda","type":"disk","children":[
            {"name":"sda1","type":"part","fstype":"vfat","label":"BOOT","mountpoints":[null]},
            {"name":"sda2","type":"part","fstype":"ext4","label":"Fotos","mountpoints":["/mnt/storage"]}]}"#).unwrap();
        let (mut m, mut c, mut h) = (vec![], vec![], vec![]);
        walk(&dev, &mut m, &mut c, &mut h);
        assert_eq!(m, vec!["/mnt/storage"]);
        assert_eq!(c, vec!["vfat «BOOT»", "ext4 «Fotos»"]);
        assert!(h.is_empty());
    }
}
