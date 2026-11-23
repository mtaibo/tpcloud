//! Where an installed TPCloud keeps things. The runtime is replaced on every install or update;
//! the state (settings, databases, caches, logs) never is.

use crate::platform;
use std::path::PathBuf;

fn from_env(key: &str, default: &str) -> PathBuf {
    std::env::var_os(key).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(default))
}

/// /opt/tpcloud: holds runtime/ (Python, packages, code, web apps, binaries) and VERSION.
pub fn app_dir() -> PathBuf {
    from_env("TPCLOUD_APP_DIR", platform::APP_DIR)
}

pub fn runtime_dir() -> PathBuf {
    app_dir().join("runtime")
}

/// /var/lib/tpcloud: tpcloud.env, data/, home/, gateway-extra/, caddy/, logs/, run/.
pub fn state_dir() -> PathBuf {
    from_env("TPCLOUD_STATE_DIR", platform::STATE_DIR)
}

/// Where the `tpcloud` command (and `tphome`, a link to it) live.
pub fn bin_dir() -> PathBuf {
    from_env("TPCLOUD_BIN_DIR", platform::BIN_DIR)
}

pub fn env_file() -> PathBuf {
    state_dir().join("tpcloud.env")
}

pub fn home_env_file() -> PathBuf {
    state_dir().join("home").join("home.env")
}

pub fn logs_dir() -> PathBuf {
    state_dir().join("logs")
}

/// What the supervisor writes about its processes, read by `tpcloud status`.
pub fn status_file() -> PathBuf {
    state_dir().join("run").join("status.json")
}
