//! Linux: systemd for the service, the usual FHS folders, shadow-utils for accounts.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub mod storage;

/// NixOS: /etc/systemd/system is the system configuration's, read-only; services are declared
/// in configuration.nix instead of written by programs.
pub fn is_nixos() -> bool {
    Path::new("/etc/NIXOS").exists()
}

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
    /// What NixOS needs in its configuration for TPCloud to run (the program can't write the unit).
    pub const NIXOS_SNIPPET: &str = r#"  systemd.services.tpcloud = {
    description = "TPCloud";
    wantedBy = [ "multi-user.target" ];
    after = [ "network-online.target" ];
    wants = [ "network-online.target" ];
    serviceConfig = {
      ExecStart = "/usr/local/bin/tpcloud serve";
      Restart = "always";
      RestartSec = 3;
      KillMode = "mixed";
      TimeoutStopSec = 30;
      LimitNOFILE = 65536;
    };
  };
  environment.extraInit = ''export PATH="$PATH:/usr/local/bin"'';"#;

    /// Before anything's installed: on NixOS the service must be declared first.
    pub fn preflight() -> Result<()> {
        if is_nixos() && !unit_path().exists() {
            bail!(
                "on NixOS, TPCloud's service goes in your configuration: nothing was installed.\n\n\
                 Add this to configuration.nix, run `sudo nixos-rebuild switch`, then install again:\n\n{NIXOS_SNIPPET}\n"
            );
        }
        Ok(())
    }

    /// `setup_pending`: the setup may make a storage pool, whose kernel modules the sandboxed
    /// service can't load itself (ProtectKernelModules): loaded first, with full privileges.
    pub fn install(program: &Path, needs: &[PathBuf], setup_pending: bool) -> Result<()> {
        let mounts: String = needs.iter().map(|p| format!("RequiresMountsFor={}\n", p.display())).collect();
        let modules = if setup_pending { "ExecStartPre=+/bin/sh -c 'modprobe -qa btrfs dm_crypt || true'\n" } else { "" };
        let unit = format!(
            "[Unit]\nDescription=TPCloud\nAfter=network-online.target\nWants=network-online.target\n{mounts}\n\
             [Service]\n{modules}ExecStart={} serve\nRestart=always\nRestartSec=3\n\
             # SIGTERM to tpcloud, which stops its processes in order; anything left is killed after the timeout.\n\
             KillMode=mixed\nTimeoutStopSec=30\nLimitNOFILE=65536\n\
             NoNewPrivileges=yes\nPrivateTmp=yes\nProtectKernelTunables=yes\nProtectKernelModules=yes\n\
             ProtectControlGroups=yes\nRestrictSUIDSGID=yes\nLockPersonality=yes\n\n\
             [Install]\nWantedBy=multi-user.target\n",
            program.display()
        );
        if is_nixos() {
            // Declared in configuration.nix (preflight checked it's there).
            return Ok(());
        }
        std::fs::write(unit_path(), unit)?;
        systemctl(&["daemon-reload"])
    }

    /// Starts with the machine from now on (doesn't start it now).
    pub fn enable() -> Result<()> {
        if is_nixos() {
            return Ok(()); // wantedBy in configuration.nix
        }
        systemctl(&["enable", NAME])
    }

    /// Enabled and started.
    pub fn start() -> Result<()> {
        if is_nixos() {
            return systemctl(&["start", NAME]);
        }
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
        if is_nixos() {
            let _ = Command::new("systemctl").args(["stop", NAME]).status();
            eprintln!("Stopped. On NixOS, take `systemd.services.tpcloud` out of configuration.nix too.");
            return Ok(());
        }
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

    /// Runs a command as a transient unit of its own, outside this service (so restarting the
    /// service doesn't stop it). Refuses while one of that name is still running.
    pub fn run_detached(unit: &str, program: &Path, args: &[&str]) -> Result<()> {
        if Command::new("systemctl").args(["is-active", "--quiet", unit]).status().map(|s| s.success()).unwrap_or(false) {
            bail!("{unit} is running already");
        }
        let status = Command::new("systemd-run")
            .args(["--unit", unit, "--collect", "--quiet", "--description", "TPCloud update"])
            .arg(program)
            .args(args)
            .status()
            .context("couldn't run systemd-run")?;
        if !status.success() {
            bail!("systemd-run couldn't start {unit}");
        }
        Ok(())
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

    /// avahi's file for one of our services (`tpcloud`, `tpcloud-setup`).
    fn service_file(id: &str) -> PathBuf {
        let dir = std::env::var_os("TPCLOUD_AVAHI_SERVICES").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/etc/avahi/services"));
        dir.join(format!("{id}.service"))
    }

    pub fn withdraw(id: &str) {
        let _ = std::fs::remove_file(service_file(id));
    }

    fn hosts_file() -> PathBuf {
        std::env::var_os("TPCLOUD_AVAHI_HOSTS").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/etc/avahi/hosts"))
    }

    const BEGIN: &str = "# tpcloud: begin (written by `tpcloud serve`, don't edit)";
    const END: &str = "# tpcloud: end";

    /// Takes out the block an older version wrote in avahi's hosts file.
    pub fn withdraw_host() {
        let path = hosts_file();
        let Ok(text) = std::fs::read_to_string(&path) else { return };
        let kept = without_block(&text);
        if kept != text && std::fs::write(&path, kept).is_ok() {
            let _ = Command::new("systemctl").args(["reload", "avahi-daemon"]).status();
        }
    }

    fn without_block(text: &str) -> String {
        let mut out = String::new();
        let mut inside = false;
        for line in text.lines() {
            if line == BEGIN {
                inside = true;
            } else if line == END {
                inside = false;
            } else if !inside {
                out.push_str(line);
                out.push('\n');
            }
        }
        out
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn our_block_comes_and_goes_alone() {
            let mine = format!("1.2.3.4 router.local\n{BEGIN}\n10.0.0.2 tpcloud.local\n{END}\n# theirs\n");
            assert_eq!(without_block(&mine), "1.2.3.4 router.local\n# theirs\n");
            assert_eq!(without_block("a\nb\n"), "a\nb\n");
        }
    }
}
