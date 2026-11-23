//! The network this host serves. Every host gets a permanent id the first time it's set up (apps
//! tell networks apart by it, whatever their addresses become) and a name people see. Both live
//! in tpcloud.env; auth hands them out at /.well-known/tpcloud with the addresses and, for local
//! addresses, the fingerprint of the host's own certificate authority.

use crate::{envfile, paths};
use anyhow::{Context, Result};
use std::io::Read;
use std::path::{Path, PathBuf};

pub const ID: &str = "TPCLOUD_NETWORK_ID";
pub const NAME: &str = "TPCLOUD_NETWORK_NAME";

/// Gives the settings an id and a name if they have none (a new host, or one set up before 0.4).
/// The id is never replaced once there.
pub fn ensure(env: &Path) -> Result<()> {
    if envfile::get(env, ID).map_or(true, |v| v.is_empty()) {
        envfile::set(env, ID, &new_id()?)?;
    }
    if envfile::get(env, NAME).map_or(true, |v| v.is_empty()) {
        envfile::set(env, NAME, &default_name())?;
    }
    Ok(())
}

/// A random UUID (version 4), from the kernel's random source.
pub fn new_id() -> Result<String> {
    let mut b = [0u8; 16];
    std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).context("reading /dev/urandom")?;
    Ok(format_uuid(b))
}

fn format_uuid(mut b: [u8; 16]) -> String {
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

/// The machine's name, as a first guess at the network's.
pub fn default_name() -> String {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "TPCloud".into())
}

/// Caddy's local certificate authority, made the first time the gateway serves a local address.
fn caddy_ca() -> PathBuf {
    paths::state_dir().join("caddy/data/caddy/pki/authorities/local/root.crt")
}

/// Where auth (not root, so it can't look inside Caddy's folder) reads that certificate from.
pub fn published_ca() -> PathBuf {
    paths::state_dir().join("run/local-ca.crt")
}

/// Copies Caddy's root certificate where auth can read it, whenever it's new or changed. Called by
/// the supervisor every few seconds; costs a stat when nothing changed.
pub fn publish_ca() {
    let src = caddy_ca();
    let Ok(cert) = std::fs::read(&src) else { return };
    let dst = published_ca();
    if std::fs::read(&dst).ok().as_deref() == Some(cert.as_slice()) {
        return;
    }
    let tmp = dst.with_extension("tmp");
    if std::fs::write(&tmp, &cert).is_ok() {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o644));
        let _ = std::fs::rename(&tmp, &dst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_is_version_4() {
        let id = format_uuid([0xff; 16]);
        assert_eq!(id, "ffffffff-ffff-4fff-bfff-ffffffffffff");
        let id = format_uuid([0; 16]);
        assert_eq!(id, "00000000-0000-4000-8000-000000000000");
        assert_eq!(new_id().unwrap().len(), 36);
    }

    #[test]
    fn ensure_keeps_an_existing_id() {
        let dir = std::env::temp_dir().join(format!("tpcloud-net-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let env = dir.join("tpcloud.env");
        std::fs::write(&env, "TPCLOUD_URL=https://x\n").unwrap();
        ensure(&env).unwrap();
        let id = envfile::get(&env, ID).unwrap();
        assert_eq!(id.len(), 36);
        assert!(!envfile::get(&env, NAME).unwrap().is_empty());
        envfile::set(&env, NAME, "Casa").unwrap();
        ensure(&env).unwrap();
        assert_eq!(envfile::get(&env, ID).unwrap(), id);
        assert_eq!(envfile::get(&env, NAME).unwrap(), "Casa");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
