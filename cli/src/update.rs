//! Getting TPCloud's latest release from GitHub: `tpcloud update`, and `tpcloud install` run from an
//! older file. The new file is downloaded next to the state, checked against the SHA-256 GitHub
//! publishes for it, and run as `install`, which puts itself and its runtime in place.

use crate::{paths, runtime, ui};
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Duration;

const LATEST: &str = "https://api.github.com/repos/mtaibo/tpcloud/releases/latest";
/// Set on the downloaded file's run, so it installs itself instead of looking again.
pub const FROM_UPDATE: &str = "TPCLOUD_FROM_UPDATE";

pub struct Release {
    pub version: String,
    url: String,
    size: u64,
    sha256: Option<String>,
}

fn asset_name() -> Result<String> {
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "amd64",
        other => bail!("there's no TPCloud build for {other}"),
    };
    Ok(format!("tpcloud-linux-{arch}"))
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(30))
        .user_agent(&format!("tpcloud/{}", env!("CARGO_PKG_VERSION")))
        .build()
}

pub fn latest() -> Result<Release> {
    let body = agent()
        .get(LATEST)
        .set("Accept", "application/vnd.github+json")
        .call()
        .context("couldn't reach GitHub")?
        .into_string()?;
    let json: serde_json::Value = serde_json::from_str(&body)?;
    let version = json["tag_name"].as_str().context("GitHub's answer has no version")?.trim_start_matches('v').to_string();
    let name = asset_name()?;
    let asset = json["assets"]
        .as_array()
        .and_then(|list| list.iter().find(|a| a["name"].as_str() == Some(name.as_str())))
        .with_context(|| format!("TPCloud {version} has no {name}"))?;
    Ok(Release {
        version,
        url: asset["browser_download_url"].as_str().unwrap_or_default().to_string(),
        size: asset["size"].as_u64().unwrap_or(0),
        sha256: asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).map(str::to_string),
    })
}

fn parts(version: &str) -> Vec<u64> {
    version.trim_start_matches('v').split(['.', '-']).map(|p| p.parse().unwrap_or(0)).collect()
}

/// Whether `a` is a later version than `b`.
pub fn newer(a: &str, b: &str) -> bool {
    parts(a) > parts(b)
}

/// Downloads the release, checks it, and runs it with `args` in place of this program.
pub fn run_release(release: &Release, args: &[String]) -> Result<()> {
    let dir = paths::state_dir().join("run");
    std::fs::create_dir_all(&dir)?;
    let file: PathBuf = dir.join(format!("tpcloud-{}", release.version));
    let tmp = file.with_extension("part");
    ui::step(&format!("Downloading TPCloud {} ({:.0} MB)", release.version, release.size as f64 / 1_048_576.0));
    let response = agent().get(&release.url).call().context("downloading the release")?;
    let mut reader = response.into_reader();
    let mut out = std::fs::File::create(&tmp)?;
    let mut hash = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    let mut got: u64 = 0;
    let tty = std::io::IsTerminal::is_terminal(&std::io::stdout());
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        hash.update(&buf[..n]);
        got += n as u64;
        if tty && release.size > 0 {
            print!("\r  {:>3} %", got * 100 / release.size);
            std::io::stdout().flush()?;
        }
    }
    if tty {
        println!("\r       ");
    }
    out.sync_all()?;
    drop(out);
    if release.size > 0 && got != release.size {
        let _ = std::fs::remove_file(&tmp);
        bail!("the download stopped at {got} of {} bytes", release.size);
    }
    let digest: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    match &release.sha256 {
        Some(expected) if *expected != digest => {
            let _ = std::fs::remove_file(&tmp);
            bail!("the download doesn't match GitHub's checksum; nothing was changed");
        }
        Some(_) => ui::done("Checksum matches GitHub's."),
        None => ui::warn("GitHub didn't give a checksum for this file; installing it anyway."),
    }
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
    }
    std::fs::rename(&tmp, &file)?;
    write_state("installing", &release.version);

    use std::os::unix::process::CommandExt;
    let err = std::process::Command::new(&file).args(args).env(FROM_UPDATE, &file).exec();
    bail!("couldn't run the new version: {err}")
}

/// Where an update tells how it's going (read by auth for the apps' Server screen): `checking`,
/// `downloading`, `installing`, then `done`, `latest` or `failed`.
pub fn state_file() -> PathBuf {
    paths::state_dir().join("run/update.json")
}

pub fn write_state(state: &str, detail: &str) {
    let body = serde_json::json!({ "state": state, "detail": detail, "at": crate::backup::now_secs() });
    let _ = std::fs::write(state_file(), body.to_string());
}

/// Where auth leaves an admin's request (`update`, `check`): a folder it may write in, nothing
/// else; the supervisor, root, acts on it.
pub fn requests_dir() -> PathBuf {
    paths::state_dir().join("requests")
}

/// Called by the supervisor every few seconds: a check or an update an admin asked for from an
/// app. The update runs as a unit of its own (`systemd-run`), so restarting this service at the
/// end doesn't stop it halfway.
pub fn serve_requests() {
    let dir = requests_dir();
    if std::fs::remove_file(dir.join("check")).is_ok() {
        std::thread::spawn(check_quietly);
    }
    if std::fs::remove_file(dir.join("update")).is_ok() {
        write_state("checking", "");
        if let Err(err) = crate::platform::service::run_detached("tpcloud-update", &paths::bin_dir().join("tpcloud"), &["update"]) {
            write_state("failed", &format!("{err:#}"));
        }
    }
}

/// `tpcloud update`.
pub fn command(force: bool) -> Result<()> {
    let result = command_inner(force);
    if let Err(err) = &result {
        write_state("failed", &format!("{err:#}"));
    }
    result
}

fn command_inner(force: bool) -> Result<()> {
    crate::install::require_root("Updating")?;
    write_state("checking", "");
    let installed = runtime::installed_version();
    if installed.is_none() {
        bail!("TPCloud isn't installed here yet: `sudo ./tpcloud install`");
    }
    let installed = installed.unwrap();
    ui::step("Looking for a newer TPCloud");
    let release = latest()?;
    if !force && !newer(&release.version, &installed) {
        write_state("latest", &installed);
        ui::done(&format!("TPCloud {installed} is the latest."));
        return Ok(());
    }
    write_state("downloading", &release.version);
    run_release(&release, &["install".to_string()])
}

/// What the service last found on GitHub, if newer than what's installed (for `tpcloud status`).
pub fn available() -> Option<String> {
    let text = std::fs::read_to_string(paths::state_dir().join("run/latest.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let latest = v["version"].as_str()?.to_string();
    let installed = runtime::installed_version()?;
    newer(&latest, &installed).then_some(latest)
}

/// The service's daily look, kept in run/latest.json.
pub fn check_quietly() {
    if let Ok(release) = latest() {
        let body = serde_json::json!({ "version": release.version, "checked": crate::backup::now_secs() });
        let _ = std::fs::write(paths::state_dir().join("run/latest.json"), body.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::newer;

    #[test]
    fn compares_versions() {
        assert!(newer("0.3.2", "0.3.1"));
        assert!(newer("v0.10.0", "0.9.9"));
        assert!(newer("1.0.0", "0.99.99"));
        assert!(!newer("0.3.1", "0.3.1"));
        assert!(!newer("0.3.0", "0.3.1"));
    }
}
