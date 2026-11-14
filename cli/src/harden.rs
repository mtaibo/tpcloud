//! Who owns what, and as whom things run. Applied on every start, so a server set up by an older
//! version (or adopted from Docker with the clones' owners) ends up the same as a fresh one.
//!
//! - auth, home-api and cloudflared run as the system user `tpcloud` (no shell, no home), owning
//!   only their own folders. store-api and the gateway stay root for now: Store writes the users'
//!   files disk and runs the rsync mirror, and Caddy binds ports 80/443.
//! - Databases and caches are readable by their owner only. Settings, logs and the supervisor's
//!   state are readable by the `tpcloud` group (the people who use `tpcloud status`), never by
//!   other users of the machine.
//! - Settings files can't smuggle loader or interpreter variables into the processes.

use crate::{envfile, paths};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

pub const USER: &str = "tpcloud";
const NOBODY: (u32, u32) = (65534, 65534);

/// uid and gid of the `tpcloud` system user, if it exists.
pub fn service_ids() -> Option<(u32, u32)> {
    let name = CString::new(USER).ok()?;
    let pw = unsafe { libc::getpwnam(name.as_ptr()) };
    if pw.is_null() {
        return None;
    }
    let (uid, gid) = unsafe { ((*pw).pw_uid, (*pw).pw_gid) };
    (uid != 0).then_some((uid, gid))
}

fn group_gid() -> Option<u32> {
    let name = CString::new(USER).ok()?;
    let gr = unsafe { libc::getgrnam(name.as_ptr()) };
    (!gr.is_null()).then(|| unsafe { (*gr).gr_gid })
}

/// The `tpcloud` group (people allowed to read status and logs) and the `tpcloud` system user
/// the services run as, in that group.
pub fn ensure_accounts() {
    let _ = Command::new("groupadd").args(["-f", USER]).status();
    if service_ids().is_none() {
        let _ = Command::new("useradd")
            .args(["--system", "--gid", USER, "--no-create-home", "--home-dir", "/nonexistent", "--shell", "/usr/sbin/nologin", USER])
            .status();
    }
}

fn set(path: &Path, mode: u32, owner: (u32, u32)) {
    if !path.exists() {
        return;
    }
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
    chown(path, owner);
}

fn chown(path: &Path, (uid, gid): (u32, u32)) {
    use std::os::unix::ffi::OsStrExt;
    if let Ok(c) = CString::new(path.as_os_str().as_bytes()) {
        unsafe { libc::lchown(c.as_ptr(), uid, gid) };
    }
}

pub fn chown_tree(path: &Path, owner: (u32, u32)) {
    if path.symlink_metadata().is_err() {
        return;
    }
    chown(path, owner);
    if path.is_dir() && !path.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                chown_tree(&entry.path(), owner);
            }
        }
    }
}

/// Owners and permissions of the state folder and the files disk's top folders.
pub fn secure_state() {
    let state = paths::state_dir();
    let root = (0, 0);
    let group = (0, group_gid().unwrap_or(0));
    let svc = service_ids().unwrap_or(root);

    std::fs::create_dir_all(state.join("cloudflared")).ok();
    // Others may only pass through (Mosquitto, as nobody, needs its folder under home/).
    set(&state, 0o751, group);
    set(&state.join("tpcloud.env"), 0o640, group);
    set(&state.join("gateway-extra"), 0o750, group);
    set(&state.join("logs"), 0o750, group);
    set(&state.join("run"), 0o750, group);
    set(&state.join("caddy"), 0o700, root);
    set(&state.join("data"), 0o711, root);
    set(&state.join("data/store"), 0o700, root);
    set(&state.join("data/auth"), 0o700, svc);
    chown_tree(&state.join("data/auth"), svc);
    set(&state.join("home"), 0o711, root);
    set(&state.join("home/home.env"), 0o640, group);
    set(&state.join("home/db"), 0o700, svc);
    chown_tree(&state.join("home/db"), svc);
    set(&state.join("home/mosquitto"), 0o700, NOBODY);
    chown_tree(&state.join("home/mosquitto"), NOBODY);
    set(&state.join("cloudflared"), 0o700, svc);

    // Store keeps every user apart itself; nobody else on the machine should read them.
    if let Some(files) = envfile::get(&paths::env_file(), "EXTERNAL_DISK_PATH").filter(|f| !f.is_empty()) {
        for area in ["users", "photos", "shared"] {
            set(&Path::new(&files).join(area), 0o700, root);
        }
    }
}

/// Variables a settings file must never set for a process: the dynamic loader's, the
/// interpreters' and the shell's. Settings are root's, but this keeps a mistake (or an old file
/// owned by someone else) from becoming code run by a service.
pub fn is_dangerous(key: &str) -> bool {
    const EXACT: [&str; 9] = ["PATH", "HOME", "SHELL", "ENV", "BASH_ENV", "IFS", "NODE_OPTIONS", "GCONV_PATH", "TMPDIR"];
    const PREFIXES: [&str; 6] = ["LD_", "DYLD_", "PYTHON", "PERL5", "RUBY", "MALLOC_"];
    EXACT.contains(&key) || PREFIXES.iter().any(|p| key.starts_with(p))
}

pub fn filtered(settings: &BTreeMap<String, String>) -> impl Iterator<Item = (&String, &String)> {
    settings.iter().filter(|(k, _)| !is_dangerous(k))
}

/// For `Command::pre_exec`: drop every group and become `owner`. Runs in the child after fork.
pub fn drop_to(owner: (u32, u32)) -> std::io::Result<()> {
    unsafe {
        if libc::setgroups(0, std::ptr::null()) != 0 || libc::setgid(owner.1) != 0 || libc::setuid(owner.0) != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_dangerous;

    #[test]
    fn refuses_loader_and_interpreter_variables() {
        for k in ["LD_PRELOAD", "LD_LIBRARY_PATH", "PYTHONPATH", "PYTHONSTARTUP", "PATH", "BASH_ENV", "PERL5OPT"] {
            assert!(is_dangerous(k), "{k}");
        }
        for k in ["TPCLOUD_URL", "RP_ID", "TUNNEL_TOKEN", "WIFI_SSID", "MQTT_PORT", "TPCLOUD_GEOIP"] {
            assert!(!is_dangerous(k), "{k}");
        }
    }
}
