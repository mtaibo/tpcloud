//! What runs, read from the settings: the same pieces the compose files ran, as plain processes on
//! local ports. Each one gets only the settings it used to get from its container.

use crate::{envfile, paths};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const AUTH_PORT: u16 = 8401;
pub const STORE_PORT: u16 = 8402;
pub const HOME_PORT: u16 = 8403;

pub struct Service {
    pub name: &'static str,
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: PathBuf,
}

pub struct Settings {
    pub cloud: BTreeMap<String, String>,
    pub home: BTreeMap<String, String>,
}

impl Settings {
    pub fn load() -> Self {
        Settings { cloud: read_all(&paths::env_file()), home: read_all(&paths::home_env_file()) }
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.cloud.get(key).map(String::as_str).filter(|v| !v.is_empty())
    }

    fn list(&self, key: &str) -> Vec<String> {
        self.get(key).unwrap_or("").split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    }

    pub fn modules(&self) -> Vec<String> {
        self.list("TPCLOUD_MODULES")
    }

    pub fn has_module(&self, module: &str) -> bool {
        self.modules().iter().any(|m| m == module)
    }

    /// The tunnel runs when its profile is on (the compose files' name for it) and there's a token.
    pub fn tunnel(&self) -> Option<String> {
        if self.list("COMPOSE_PROFILES").iter().any(|p| p == "tunnel") {
            self.get("TUNNEL_TOKEN").map(str::to_string)
        } else {
            None
        }
    }
}

fn read_all(path: &Path) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let Ok(text) = std::fs::read_to_string(path) else { return map };
    for line in text.lines() {
        let line = line.trim_start();
        if line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            if !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                map.insert(key.to_string(), envfile::strip_comment(value).trim_matches('"').to_string());
            }
        }
    }
    map
}

/// "127.0.0.1:8080" → ("127.0.0.1", 8080); "80" → ("0.0.0.0", 80).
fn split_publish(value: &str, default: (&str, u16)) -> (String, u16) {
    let value = value.trim();
    if value.is_empty() {
        return (default.0.to_string(), default.1);
    }
    match value.rsplit_once(':') {
        Some((host, port)) => (host.to_string(), port.parse().unwrap_or(default.1)),
        None => ("0.0.0.0".to_string(), value.parse().unwrap_or(default.1)),
    }
}

/// The gateway's site address and bind host. A plain-HTTP site (behind a tunnel or `tailscale
/// serve`) listened on port 80 inside its container, published on the host as
/// TPCLOUD_HTTP_PUBLISH; here Caddy listens there directly. An HTTPS site binds 80 and 443 itself.
pub fn gateway_site(settings: &Settings) -> (String, String) {
    let site = settings.get("TPCLOUD_SITE").unwrap_or(":80").to_string();
    let (bind, port) = split_publish(settings.get("TPCLOUD_HTTP_PUBLISH").unwrap_or(""), ("127.0.0.1", 8080));
    let plain = site.starts_with("http://") || site.starts_with(':');
    if !plain {
        return (site, "0.0.0.0".into());
    }
    let without_port = match site.strip_prefix("http://") {
        Some(host) => format!("http://{}", host.split(':').next().unwrap_or(host)),
        None => String::new(),
    };
    (format!("{without_port}:{port}"), bind)
}

pub fn plan(settings: &Settings) -> Vec<Service> {
    let rt = paths::runtime_dir();
    let state = paths::state_dir();
    let python = rt.join("python/bin/python3");
    let path_var = format!("{}:/usr/local/bin:/usr/bin:/bin", rt.join("bin").display());
    let url = settings.get("TPCLOUD_URL").unwrap_or("").to_string();
    let mut services = Vec::new();

    let base = |pythonpath: PathBuf| -> BTreeMap<String, String> {
        let mut env = BTreeMap::new();
        env.insert("PATH".into(), path_var.clone());
        env.insert("PYTHONPATH".into(), pythonpath.display().to_string());
        env.insert("PYTHONDONTWRITEBYTECODE".into(), "1".into());
        env.insert("PYTHONUNBUFFERED".into(), "1".into());
        env.insert("LANG".into(), "C.UTF-8".into());
        env
    };
    let uvicorn = |module: &str, port: u16, app_dir: &Path| -> Vec<String> {
        ["-m", "uvicorn", module, "--host", "127.0.0.1", "--port", &port.to_string(), "--app-dir"]
            .iter()
            .map(|s| s.to_string())
            .chain([app_dir.display().to_string()])
            .collect()
    };

    let home_on = settings.has_module("home");
    if home_on {
        services.push(Service {
            name: "mqtt-broker",
            program: rt.join("bin/mosquitto"),
            args: vec!["-c".into(), state.join("run/mosquitto.conf").display().to_string()],
            env: BTreeMap::new(),
            cwd: state.join("home/mosquitto"),
        });
    }

    // auth got the whole settings file in its container; it still does.
    let mut env = base(rt.join("py/auth"));
    for (k, v) in &settings.cloud {
        env.entry(k.clone()).or_insert_with(|| v.clone());
    }
    env.insert("DATABASE_URL".into(), format!("sqlite:///{}", state.join("data/auth/tpauth.db").display()));
    services.push(Service {
        name: "auth",
        program: python.clone(),
        args: uvicorn("main:app", AUTH_PORT, &rt.join("code/auth")),
        env,
        cwd: state.join("data/auth"),
    });

    if settings.has_module("store") {
        let mut env = base(rt.join("py/store"));
        env.insert("TPCLOUD_URL".into(), url.clone());
        env.insert("AUTH_SERVICE_URL".into(), format!("http://127.0.0.1:{AUTH_PORT}"));
        env.insert("EXTERNAL_DISK_PATH".into(), settings.get("EXTERNAL_DISK_PATH").unwrap_or("/mnt/external").to_string());
        env.insert("DATA_DIR".into(), state.join("data/store").display().to_string());
        if let Some(tz) = settings.get("PHOTO_TIMEZONE") {
            env.insert("PHOTO_TIMEZONE".into(), tz.to_string());
        }
        if let Some(mount) = settings.get("TPSTORE_BACKUP_MOUNT") {
            let mount = mount.trim_end_matches('/');
            env.insert("BACKUP_SOURCE".into(), format!("{mount}/storage/"));
            env.insert("BACKUP_DEST".into(), format!("{mount}/backup/"));
        }
        services.push(Service {
            name: "store-api",
            program: python.clone(),
            args: uvicorn("app.main:app", STORE_PORT, &rt.join("code/store")),
            env,
            cwd: state.join("data/store"),
        });
    }

    if home_on {
        let mut env = base(rt.join("py/home"));
        for (k, v) in &settings.home {
            env.entry(k.clone()).or_insert_with(|| v.clone());
        }
        env.insert("DATABASE_URL".into(), format!("sqlite:///{}", state.join("home/db/tphome.db").display()));
        env.insert("FIRMWARE_DIR".into(), state.join("home/db/firmwares").display().to_string());
        env.insert("MQTT_BROKER".into(), "127.0.0.1".into());
        env.insert("AUTH_URL".into(), format!("http://127.0.0.1:{AUTH_PORT}"));
        services.push(Service {
            name: "home-api",
            program: python.clone(),
            args: uvicorn("main:app", HOME_PORT, &rt.join("code/home")),
            env,
            cwd: state.join("home/db"),
        });
    }

    let (site, bind) = gateway_site(settings);
    let (ota_bind, ota_port) = split_publish(settings.get("TPCLOUD_OTA_PUBLISH").unwrap_or(""), ("127.0.0.1", 8081));
    let mut env = BTreeMap::new();
    env.insert("PATH".into(), path_var.clone());
    env.insert("HOME".into(), state.join("caddy").display().to_string());
    env.insert("XDG_DATA_HOME".into(), state.join("caddy/data").display().to_string());
    env.insert("XDG_CONFIG_HOME".into(), state.join("caddy/config").display().to_string());
    env.insert("TPCLOUD_SITE".into(), site);
    env.insert("TPCLOUD_BIND".into(), bind);
    env.insert("TPCLOUD_TLS".into(), settings.get("TPCLOUD_TLS").unwrap_or("none").to_string());
    env.insert("TPCLOUD_OTA_SITE".into(), format!(":{ota_port}"));
    env.insert("TPCLOUD_OTA_BIND".into(), ota_bind);
    env.insert("TPCLOUD_AUTH_UPSTREAM".into(), format!("127.0.0.1:{AUTH_PORT}"));
    env.insert("TPCLOUD_STORE_UPSTREAM".into(), format!("127.0.0.1:{STORE_PORT}"));
    env.insert("TPCLOUD_HOME_UPSTREAM".into(), format!("127.0.0.1:{HOME_PORT}"));
    env.insert("TPCLOUD_WEB_ROOT".into(), rt.join("web").display().to_string());
    env.insert("TPCLOUD_CADDY_EXTRA".into(), state.join("gateway-extra").display().to_string());
    services.push(Service {
        name: "gateway",
        program: rt.join("bin/caddy"),
        args: ["run", "--adapter", "caddyfile", "--config"].iter().map(|s| s.to_string()).chain([rt.join("Caddyfile").display().to_string()]).collect(),
        env,
        cwd: state.join("caddy"),
    });

    if let Some(token) = settings.tunnel() {
        let mut env = BTreeMap::new();
        env.insert("TUNNEL_TOKEN".into(), token);
        env.insert("HOME".into(), state.join("caddy").display().to_string());
        services.push(Service {
            name: "cloudflared",
            program: rt.join("bin/cloudflared"),
            args: ["tunnel", "--no-autoupdate", "run"].iter().map(|s| s.to_string()).collect(),
            env,
            cwd: state.clone(),
        });
    }
    services
}

/// Mosquitto's settings, written next to the other runtime files on every start.
pub fn mosquitto_conf() -> String {
    format!(
        "listener 1883\nallow_anonymous true\npersistence true\npersistence_location {}/\nlog_dest stdout\n",
        paths::state_dir().join("home/mosquitto").display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(pairs: &[(&str, &str)]) -> Settings {
        Settings { cloud: pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(), home: BTreeMap::new() }
    }

    #[test]
    fn plain_sites_listen_where_the_container_was_published() {
        assert_eq!(gateway_site(&settings(&[("TPCLOUD_SITE", "http://cloud.example.com")])), ("http://cloud.example.com:8080".into(), "127.0.0.1".into()));
        assert_eq!(gateway_site(&settings(&[("TPCLOUD_SITE", ":80")])), (":8080".into(), "127.0.0.1".into()));
        assert_eq!(gateway_site(&settings(&[("TPCLOUD_SITE", ":80"), ("TPCLOUD_HTTP_PUBLISH", "0.0.0.0:80")])), (":80".into(), "0.0.0.0".into()));
        assert_eq!(gateway_site(&settings(&[("TPCLOUD_SITE", "cloud.example.com")])), ("cloud.example.com".into(), "0.0.0.0".into()));
    }

    #[test]
    fn publish_values() {
        assert_eq!(split_publish("80", ("127.0.0.1", 8081)), ("0.0.0.0".into(), 80));
        assert_eq!(split_publish("", ("127.0.0.1", 8081)), ("127.0.0.1".into(), 8081));
        assert_eq!(split_publish("127.0.0.1:8080", ("x", 1)), ("127.0.0.1".into(), 8080));
    }
}
