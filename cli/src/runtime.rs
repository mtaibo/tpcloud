//! The runtime this binary carries: Python with every service's packages, the services' code, the
//! web apps and static caddy, mosquitto, cloudflared, ffmpeg, ffprobe and rsync
//! (cli/build-runtime.sh makes it).

use crate::paths;
use anyhow::{Context, Result};

static RUNTIME: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/runtime.tar.zst"));

/// Unpacks next to the current runtime and swaps it in, so a failed unpack leaves the old one.
pub fn unpack() -> Result<()> {
    let app = paths::app_dir();
    std::fs::create_dir_all(&app)?;
    let fresh = app.join("runtime.new");
    let old = app.join("runtime.old");
    for dir in [&fresh, &old] {
        if dir.exists() {
            std::fs::remove_dir_all(dir)?;
        }
    }
    let mut decoder = zstd::stream::read::Decoder::new(RUNTIME)?;
    decoder.window_log_max(27)?;
    // The archive's top folder is runtime/; unpack it into a scratch folder and move it.
    let scratch = app.join(".unpack");
    if scratch.exists() {
        std::fs::remove_dir_all(&scratch)?;
    }
    std::fs::create_dir_all(&scratch)?;
    let mut archive = tar::Archive::new(decoder);
    archive.set_preserve_permissions(true);
    archive.unpack(&scratch).context("unpacking the runtime")?;
    std::fs::rename(scratch.join("runtime"), &fresh)?;
    std::fs::remove_dir_all(&scratch)?;

    let current = paths::runtime_dir();
    if current.exists() {
        std::fs::rename(&current, &old)?;
    }
    std::fs::rename(&fresh, &current)?;
    if old.exists() {
        std::fs::remove_dir_all(&old)?;
    }
    std::fs::write(app.join("VERSION"), format!("{}\n", env!("CARGO_PKG_VERSION")))?;
    Ok(())
}

pub fn installed_version() -> Option<String> {
    std::fs::read_to_string(paths::app_dir().join("VERSION")).ok().map(|v| v.trim().to_string())
}
