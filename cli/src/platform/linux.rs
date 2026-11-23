//! Linux: systemd for the service, the usual FHS folders, shadow-utils for accounts.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const APP_DIR: &str = "/opt/tpcloud";
pub const STATE_DIR: &str = "/var/lib/tpcloud";
pub const BIN_DIR: &str = "/usr/local/bin";

pub mod service {
    use super::*;

    const NAME: &str = "tpcloud";

    pub fn unit_path() -> PathBuf {
        std::env::var_os("TPCLOUD_SYSTEMD_UNIT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/etc/systemd/system/tpcloud.service"))
    }

    /// systemd runs `<program> serve`, which runs everything else.
    pub fn install(program: &Path) -> Result<()> {
        let unit = format!(
            "[Unit]\nDescription=TPCloud\nAfter=network-online.target\nWants=network-online.target\n\n\
             [Service]\nExecStart={} serve\nRestart=always\nRestartSec=3\n\
             # SIGTERM to tpcloud, which stops its processes in order; anything left is killed after the timeout.\n\
             KillMode=mixed\nTimeoutStopSec=30\nLimitNOFILE=65536\n\
             NoNewPrivileges=yes\nPrivateTmp=yes\nProtectKernelTunables=yes\nProtectKernelModules=yes\n\
             ProtectControlGroups=yes\nRestrictSUIDSGID=yes\nLockPersonality=yes\n\n\
             [Install]\nWantedBy=multi-user.target\n",
            program.display()
        );
        std::fs::write(unit_path(), unit)?;
        systemctl(&["daemon-reload"])
    }

    /// Starts with the machine from now on (doesn't start it now).
    pub fn enable() -> Result<()> {
        systemctl(&["enable", NAME])
    }

    /// Enabled and started.
    pub fn start() -> Result<()> {
        systemctl(&["enable", "--now", NAME])
    }

    /// Started once, as it is (enabled or not).
    pub fn start_once() -> Result<()> {
        systemctl(&["start", NAME])
    }

    pub fn stop() -> Result<()> {
        systemctl(&["stop", NAME])
    }

    pub fn restart() -> Result<()> {
        systemctl(&["restart", NAME])
    }

    pub fn running() -> bool {
        Command::new("systemctl").args(["is-active", "--quiet", NAME]).status().map(|s| s.success()).unwrap_or(false)
    }

    fn systemctl(args: &[&str]) -> Result<()> {
        let status = Command::new("systemctl").args(args).status().context("couldn't run systemctl")?;
        if !status.success() {
            bail!("systemctl {} failed", args.join(" "));
        }
        Ok(())
    }
}

pub mod accounts {
    use super::*;

    /// The group `name` and a system user of the same name in it (no shell, no home).
    pub fn ensure_group_and_user(name: &str, user_exists: bool) {
        let _ = Command::new("groupadd").args(["-f", name]).status();
        if !user_exists {
            let _ = Command::new("useradd")
                .args(["--system", "--gid", name, "--no-create-home", "--home-dir", "/nonexistent", "--shell", "/usr/sbin/nologin", name])
                .status();
        }
    }

    /// Adds a person to a group (takes effect from their next login).
    pub fn add_to_group(user: &str, group: &str) {
        let _ = Command::new("usermod").args(["-aG", group, user]).status();
    }
}
