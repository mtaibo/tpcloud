//! Reaching the host at the owner's own domain through a Cloudflare Tunnel, made by the setup
//! with nothing pasted: `cloudflared tunnel login` prints a link the owner opens on any device to
//! pick their domain, then the tunnel is created in their account, the hostname's DNS record
//! pointed at it, and the tunnel run with our own ingress (to the gateway's plain-HTTP site).
//!
//! The login leaves a certificate that can manage every tunnel of that domain: it stays in
//! `cloudflare/` (root only); the tunnel process, run as `tpcloud`, only reads its own
//! credentials in `cloudflared/`.

use crate::{paths, services::Settings, ui};
use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use std::process::Command;

fn cloudflared() -> PathBuf {
    paths::runtime_dir().join("bin/cloudflared")
}

/// Where the login's certificate lands (cloudflared writes it to $HOME/.cloudflared/cert.pem).
fn login_home() -> PathBuf {
    paths::state_dir().join("cloudflare")
}

fn origin_cert() -> PathBuf {
    login_home().join(".cloudflared/cert.pem")
}

/// The tunnel's own credentials, all the running tunnel needs.
pub fn credentials() -> PathBuf {
    paths::state_dir().join("cloudflared/tunnel.json")
}

pub fn config_path() -> PathBuf {
    paths::state_dir().join("run/cloudflared.yml")
}

/// The tunnel's ingress: the hostname to the gateway, anything else 404.
pub fn config(id: &str, hostname: &str, origin: &str) -> String {
    format!(
        "# Written by `tpcloud serve` on every start: don't edit.\n\
         tunnel: {id}\ncredentials-file: {}\nno-autoupdate: true\n\
         ingress:\n  - hostname: {hostname}\n    service: {origin}\n  - service: http_status:404\n",
        credentials().display()
    )
}

/// Written before the tunnel starts, from the settings.
pub fn write_config(settings: &Settings) -> Result<()> {
    if let Some(crate::services::Tunnel::Managed { id, hostname }) = settings.tunnel() {
        std::fs::write(config_path(), config(&id, &hostname, &crate::services::http_origin(settings)))?;
    }
    Ok(())
}

fn run(args: &[&str]) -> Result<std::process::Output> {
    Command::new(cloudflared())
        .arg("--origincert")
        .arg(origin_cert())
        .args(args)
        .env("HOME", login_home())
        .output()
        .context("couldn't run cloudflared (is TPCloud installed?)")
}

fn text(out: &std::process::Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).trim().to_string()
}

/// Signs this machine in to the owner's Cloudflare account, unless it already is. cloudflared
/// prints the link itself and waits until it's been approved in the browser.
pub fn login() -> Result<()> {
    if origin_cert().exists() {
        return Ok(());
    }
    std::fs::create_dir_all(login_home())?;
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(login_home(), std::fs::Permissions::from_mode(0o700))?;
    }
    println!("  {}", ui::dim("Open the link below on any device, sign in to Cloudflare and pick your domain."));
    let status = Command::new(cloudflared())
        .args(["tunnel", "login"])
        .env("HOME", login_home())
        .status()
        .context("couldn't run cloudflared")?;
    if !status.success() || !origin_cert().exists() {
        bail!("the Cloudflare sign-in didn't finish");
    }
    Ok(())
}

/// Forgets the login, so the next one can pick another domain.
pub fn logout() {
    let _ = std::fs::remove_file(origin_cert());
}

/// Makes the tunnel, or keeps the one already made (its credentials are here). Returns its id.
pub fn create(name: &str, existing: Option<&str>) -> Result<String> {
    if let Some(id) = existing.filter(|id| !id.is_empty()) {
        if credentials().exists() {
            return Ok(id.to_string());
        }
    }
    std::fs::create_dir_all(credentials().parent().unwrap())?;
    let creds = credentials().display().to_string();
    let out = run(&["tunnel", "create", "--output", "json", "--credentials-file", &creds, name])?;
    if !out.status.success() {
        bail!("couldn't make the tunnel: {}", text(&out));
    }
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).context("cloudflared's answer wasn't JSON")?;
    match json["id"].as_str() {
        Some(id) if !id.is_empty() => Ok(id.to_string()),
        _ => bail!("cloudflared didn't say the tunnel's id"),
    }
}

pub enum Route {
    Done,
    /// The hostname already has a DNS record (another tunnel's, a server's): only replaced if the
    /// owner says so.
    Taken,
}

/// Points the hostname's DNS record at the tunnel.
pub fn route(id: &str, hostname: &str, overwrite: bool) -> Result<Route> {
    let mut args = vec!["tunnel", "route", "dns"];
    if overwrite {
        args.push("--overwrite-dns");
    }
    args.extend([id, hostname]);
    let out = run(&args)?;
    if out.status.success() {
        return Ok(Route::Done);
    }
    let said = text(&out);
    if said.contains("already exists") {
        return Ok(Route::Taken);
    }
    bail!("couldn't point {hostname} at the tunnel: {said}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingress_sends_the_hostname_to_the_gateway_only() {
        let c = config("abc", "cloud.example.com", "http://127.0.0.1:8080");
        assert!(c.contains("tunnel: abc\n"));
        assert!(c.contains("  - hostname: cloud.example.com\n    service: http://127.0.0.1:8080\n  - service: http_status:404\n"));
    }
}
