//! The network this host serves. Every host gets a permanent id the first time it's set up (apps
//! tell networks apart by it, whatever their addresses become) and a name people see. Both live
//! in tpcloud.env; auth hands them out at /.well-known/tpcloud with the addresses and, for local
//! addresses, the fingerprint of the host's own certificate authority.

use crate::services::Settings;
use crate::{envfile, paths};
use anyhow::{Context, Result};
use std::io::Read;
use std::path::{Path, PathBuf};

pub const ID: &str = "TPCLOUD_NETWORK_ID";
pub const NAME: &str = "TPCLOUD_NETWORK_NAME";
/// `on`: also answer at home, on https://<hostname>.local (and LOCAL_NAMES) with the host's own
/// certificate authority, announced on the network. Hosts from before 0.4 get `off` (it takes
/// port 443), new ones choose in the setup.
pub const LOCAL: &str = "TPCLOUD_LOCAL";
/// More names for the local site, e.g. one set in the router's DNS (cloud.casa).
pub const LOCAL_NAMES: &str = "TPCLOUD_LOCAL_NAMES";
/// The network's name at home: `<this>.local` (default `tpcloud`), whatever the machine is called.
pub const LOCAL_HOST: &str = "TPCLOUD_LOCAL_HOST";
pub const DEFAULT_LOCAL_HOST: &str = "tpcloud";
/// This machine's Tailscale name (host.tailnet.ts.net); the gateway answers there itself, with
/// the certificate Tailscale gives it.
pub const TAILSCALE: &str = "TPCLOUD_TAILSCALE";
/// A tunnel the setup made in the owner's Cloudflare account: its id and public hostname.
pub const TUNNEL_ID: &str = "TPCLOUD_TUNNEL_ID";
pub const TUNNEL_HOSTNAME: &str = "TPCLOUD_TUNNEL_HOSTNAME";

/// Gives the settings an id and a name if they have none (a new host, or one set up before 0.4).
/// The id is never replaced once there.
pub fn ensure(env: &Path) -> Result<()> {
    if envfile::get(env, ID).map_or(true, |v| v.is_empty()) {
        envfile::set(env, ID, &new_id()?)?;
    }
    if envfile::get(env, NAME).map_or(true, |v| v.is_empty()) {
        envfile::set(env, NAME, &default_name())?;
    }
    if envfile::get(env, LOCAL).is_none() {
        // Already local-only before 0.4: that's the local site, keep it.
        let was_local = envfile::get(env, "TPCLOUD_TLS").as_deref() == Some("internal");
        envfile::set(env, LOCAL, if was_local { "on" } else { "off" })?;
    }
    Ok(())
}

/// A label for a `.local` name: lowercase letters, digits and dashes ("" if nothing's left).
pub fn label(text: &str) -> String {
    let name: String = text
        .split('.')
        .next()
        .unwrap_or("")
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    name.trim_matches('-').to_string()
}

/// The network's address at home, `tpcloud.local` unless the setup chose another label.
pub fn local_host() -> String {
    let set = envfile::get(&paths::env_file(), LOCAL_HOST).map(|l| label(&l)).unwrap_or_default();
    format!("{}.local", if set.is_empty() { DEFAULT_LOCAL_HOST.to_string() } else { set })
}

fn host_of(url_or_site: &str) -> &str {
    let rest = url_or_site.split_once("://").map_or(url_or_site, |(_, r)| r);
    let rest = rest.split('/').next().unwrap_or(rest);
    rest.rsplit_once(':').filter(|(_, p)| p.chars().all(|c| c.is_ascii_digit())).map_or(rest, |(h, _)| h)
}

fn push(list: &mut Vec<String>, item: String) {
    if !item.is_empty() && !list.contains(&item) {
        list.push(item);
    }
}

/// The names of the local site: <hostname>.local first (the one announced on the network), then
/// any the owner set in their own DNS. Empty when the local site is off.
pub fn local_names(s: &Settings) -> Vec<String> {
    let mut names = Vec::new();
    if s.get(LOCAL) != Some("on") {
        return names;
    }
    let first = match s.get(LOCAL_HOST).map(label) {
        Some(l) if !l.is_empty() => format!("{l}.local"),
        _ => format!("{DEFAULT_LOCAL_HOST}.local"),
    };
    push(&mut names, first);
    for name in s.list(LOCAL_NAMES) {
        push(&mut names, host_of(&name).to_lowercase());
    }
    names
}

/// Every address the network answers on, the main one (TPCLOUD_URL) first: the tunnel's, then
/// Tailscale's, then the local ones, then any set by hand. What the network card lists.
pub fn addresses(s: &Settings) -> Vec<String> {
    let mut list = Vec::new();
    push(&mut list, s.get("TPCLOUD_URL").unwrap_or("").trim_end_matches('/').to_string());
    if let Some(host) = s.get(TUNNEL_HOSTNAME) {
        push(&mut list, format!("https://{host}"));
    }
    if let Some(host) = s.get(TAILSCALE) {
        push(&mut list, format!("https://{host}"));
    }
    for name in local_names(s) {
        push(&mut list, format!("https://{name}"));
    }
    for extra in s.list("TPCLOUD_ADDRESSES") {
        push(&mut list, extra.trim_end_matches('/').to_string());
    }
    list
}

/// Caddy sites for the addresses beyond the main one (TPCLOUD_SITE), all serving the same
/// TPCloud: the local names with the host's own authority, Tailscale's name with Tailscale's
/// certificate. Written to run/caddy-sites/ before the gateway starts.
pub fn extra_sites(s: &Settings) -> String {
    let main = host_of(s.get("TPCLOUD_SITE").unwrap_or("")).to_lowercase();
    let main_is_https = s.get("TPCLOUD_SITE").is_some_and(|site| !site.starts_with("http://") && !site.starts_with(':'));
    let taken = |host: &str| main_is_https && host == main;
    let mut out = String::from("# Written by `tpcloud serve` from the settings on every start: don't edit.\n");
    let local: Vec<String> = local_names(s).into_iter().filter(|n| !taken(n)).map(|n| format!("https://{n}")).collect();
    if !local.is_empty() {
        out.push_str(&format!("\n{} {{\n\tbind 0.0.0.0\n\ttls internal\n\timport tpcloud\n}}\n", local.join(", ")));
    }
    if let Some(host) = s.get(TAILSCALE).filter(|h| !taken(h)) {
        out.push_str(&format!("\nhttps://{host} {{\n\tbind 0.0.0.0\n\timport tpcloud\n}}\n"));
    }
    // The tunnel delivers to the plain-HTTP site: there must be one even when the main site is
    // an HTTPS one.
    if main_is_https && s.tunnel().is_some() {
        let origin = crate::services::http_origin(s);
        let hostport = origin.trim_start_matches("http://");
        let (host, port) = hostport.rsplit_once(':').unwrap_or((hostport, "8080"));
        out.push_str(&format!("\nhttp://:{port} {{\n\tbind {host}\n\timport tpcloud\n}}\n"));
    }
    out
}

pub fn caddy_sites_dir() -> PathBuf {
    paths::state_dir().join("run/caddy-sites")
}

/// Writes the gateway's extra sites (see `extra_sites`).
pub fn write_sites(s: &Settings) -> Result<()> {
    let dir = caddy_sites_dir();
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("tpcloud.caddy"), extra_sites(s))?;
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
    use std::collections::BTreeMap;

    fn settings(pairs: &[(&str, &str)]) -> Settings {
        Settings { cloud: pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(), home: BTreeMap::new() }
    }

    #[test]
    fn hosts() {
        assert_eq!(host_of("https://cloud.example.com/"), "cloud.example.com");
        assert_eq!(host_of("http://cloud.example.com:8080"), "cloud.example.com");
        assert_eq!(host_of("tphome.local"), "tphome.local");
        assert_eq!(host_of(":80"), "");
    }

    #[test]
    fn pi_like_tunnel_only() {
        let s = settings(&[("TPCLOUD_URL", "https://cloud.example.com"), ("TPCLOUD_SITE", "http://cloud.example.com"), (LOCAL, "off")]);
        assert_eq!(addresses(&s), vec!["https://cloud.example.com"]);
        assert!(!extra_sites(&s).contains("import tpcloud"));
    }

    #[test]
    fn tunnel_plus_local_plus_tailscale() {
        let s = settings(&[
            ("TPCLOUD_URL", "https://cloud.example.com"),
            ("TPCLOUD_SITE", "http://cloud.example.com"),
            (TUNNEL_HOSTNAME, "cloud.example.com"),
            (TAILSCALE, "box.tail1.ts.net"),
            (LOCAL, "on"),
            (LOCAL_NAMES, "cloud.casa, https://cloud.casa/"),
        ]);
        let local = "https://tpcloud.local".to_string();
        assert_eq!(addresses(&s), vec!["https://cloud.example.com".to_string(), "https://box.tail1.ts.net".into(), local.clone(), "https://cloud.casa".into()]);
        let sites = extra_sites(&s);
        assert!(sites.contains(&format!("{local}, https://cloud.casa {{\n\tbind 0.0.0.0\n\ttls internal")), "{sites}");
        assert!(sites.contains("https://box.tail1.ts.net {\n\tbind 0.0.0.0\n\timport tpcloud"), "{sites}");
    }

    #[test]
    fn a_tunnel_always_has_its_plain_site() {
        let s = settings(&[("TPCLOUD_SITE", "https://box.local"), ("TPCLOUD_TLS", "internal"), ("COMPOSE_PROFILES", "tunnel"), ("TUNNEL_TOKEN", "t")]);
        assert!(extra_sites(&s).contains("http://:8080 {\n\tbind 127.0.0.1\n\timport tpcloud"), "{}", extra_sites(&s));
        let s = settings(&[("TPCLOUD_SITE", "http://cloud.example.com"), ("COMPOSE_PROFILES", "tunnel"), ("TUNNEL_TOKEN", "t")]);
        assert!(!extra_sites(&s).contains("http://:"));
    }

    #[test]
    fn local_only_main_site_isnt_repeated() {
        let local = "tpcloud.local".to_string();
        let s = settings(&[("TPCLOUD_URL", &format!("https://{local}")), ("TPCLOUD_SITE", &format!("https://{local}")), ("TPCLOUD_TLS", "internal"), (LOCAL, "on")]);
        assert_eq!(addresses(&s), vec![format!("https://{local}")]);
        assert!(!extra_sites(&s).contains("import tpcloud"));
    }

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
