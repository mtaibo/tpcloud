//! Linux: systemd for the service, the usual FHS folders, shadow-utils for accounts.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub mod storage;

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

    /// systemd runs `<program> serve`, which runs everything else, once the folders it needs are
    /// mounted (Store's files: with their disk missing it would fill the system disk instead).
    pub fn install(program: &Path, needs: &[PathBuf]) -> Result<()> {
        let mounts: String = needs.iter().map(|p| format!("RequiresMountsFor={}\n", p.display())).collect();
        let unit = format!(
            "[Unit]\nDescription=TPCloud\nAfter=network-online.target\nWants=network-online.target\n{mounts}\n\
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

    /// Stopped, not started at boot, and its unit gone.
    pub fn remove() -> Result<()> {
        let _ = Command::new("systemctl").args(["disable", "--now", NAME]).status();
        let unit = unit_path();
        if unit.exists() {
            std::fs::remove_file(&unit)?;
        }
        systemctl(&["daemon-reload"])
    }

    pub fn installed() -> bool {
        unit_path().exists()
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

pub mod discovery {
    use super::*;

    fn service_file() -> PathBuf {
        std::env::var_os("TPCLOUD_AVAHI_SERVICE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/etc/avahi/services/tpcloud.service"))
    }

    /// avahi-daemon is running: it owns mDNS here and already answers for <hostname>.local.
    pub fn system_responder() -> bool {
        Command::new("systemctl").args(["is-active", "--quiet", "avahi-daemon"]).status().map(|s| s.success()).unwrap_or(false)
    }

    fn xml(text: &str) -> String {
        text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
    }

    /// A static service file avahi picks up on its own (it watches the folder).
    pub fn publish(name: &str, service: &str, port: u16, txt: &[(String, String)]) -> Result<()> {
        let records: String = txt.iter().map(|(k, v)| format!("    <txt-record>{}={}</txt-record>\n", xml(k), xml(v))).collect();
        let body = format!(
            "<?xml version=\"1.0\" standalone='no'?>\n<!DOCTYPE service-group SYSTEM \"avahi-service.dtd\">\n\
             <!-- Written by `tpcloud serve` while TPCloud runs: don't edit. -->\n<service-group>\n  <name>{}</name>\n  <service>\n\
             \x20   <type>{service}</type>\n    <port>{port}</port>\n{records}  </service>\n</service-group>\n",
            xml(name)
        );
        let path = service_file();
        if std::fs::read_to_string(&path).ok().as_deref() == Some(body.as_str()) {
            return Ok(());
        }
        let dir = path.parent().context("no folder for avahi's service file")?;
        if !dir.is_dir() {
            bail!("{} doesn't exist", dir.display());
        }
        std::fs::write(&path, body).with_context(|| format!("writing {}", path.display()))
    }

    pub fn withdraw() {
        let _ = std::fs::remove_file(service_file());
    }
}
