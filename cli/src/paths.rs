//! Where an installed TPCloud keeps things. The code is replaced on every install or update; the
//! state (settings, databases, caches) never is.

use std::path::PathBuf;

fn from_env(key: &str, default: &str) -> PathBuf {
    std::env::var_os(key).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(default))
}

/// The compose files and sources the images are built from (out of the embedded bundle).
pub fn app_dir() -> PathBuf {
    from_env("TPCLOUD_APP_DIR", "/opt/tpcloud")
}

/// Settings and data: tpcloud.env, data/, gateway-extra/ and home/.
pub fn state_dir() -> PathBuf {
    from_env("TPCLOUD_STATE_DIR", "/var/lib/tpcloud")
}

/// Where the `tpcloud` command (and `tphome`, a link to it) live.
pub fn bin_dir() -> PathBuf {
    from_env("TPCLOUD_BIN_DIR", "/usr/local/bin")
}

pub fn env_file() -> PathBuf {
    state_dir().join("tpcloud.env")
}

pub fn home_env_file() -> PathBuf {
    state_dir().join("home").join("home.env")
}
