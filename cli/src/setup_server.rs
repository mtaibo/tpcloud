//! A host waiting to be set up (`TPCLOUD_SETUP=pending`) runs this instead of TPCloud: a setup
//! page and its API on the local network (port 80, or 8090 when that's taken), announced as
//! `_tpcloud-setup._tcp` so the apps find it. The apps and the page drive the same `engine` as the
//! terminal. Once the answers are applied it stops, and the service starts again as TPCloud.
//!
//! Who may set it up: whoever types the 6-digit code shown where the install ran (and in
//! `sudo tpcloud status`). For a machine with no screen and no SSH, for the first 15 minutes it
//! also lets the first device on the local network in without one. Whoever's in gets a token;
//! everyone else is locked out from then on.

use crate::engine::{self, Answers, Domain, Files};
use crate::{discovery, network, paths, storage};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const OPEN_FOR: Duration = Duration::from_secs(15 * 60);
const MAX_BODY: usize = 1 << 20;

pub fn code_file() -> std::path::PathBuf {
    paths::state_dir().join("run/setup-code")
}

/// The pairing code, made once and kept until the setup's done (root only).
pub fn code() -> Result<String> {
    if let Ok(c) = std::fs::read_to_string(code_file()) {
        let c = c.trim().to_string();
        if c.len() == 6 {
            return Ok(c);
        }
    }
    let mut b = [0u8; 4];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut b)?;
    let c = format!("{:06}", u32::from_le_bytes(b) % 1_000_000);
    std::fs::create_dir_all(code_file().parent().unwrap())?;
    std::fs::write(code_file(), &c)?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(code_file(), std::fs::Permissions::from_mode(0o600))?;
    Ok(c)
}

/// Where the page answers: port 80 if it's free, else 8090.
pub fn port_file() -> std::path::PathBuf {
    paths::state_dir().join("run/setup-port")
}

pub fn address() -> String {
    let port = std::fs::read_to_string(port_file()).ok().and_then(|p| p.trim().parse::<u16>().ok()).unwrap_or(80);
    let host = format!("{}.local", network::host_label());
    if port == 80 { format!("http://{host}") } else { format!("http://{host}:{port}") }
}

#[derive(Default)]
struct Run {
    state: &'static str,
    log: Vec<(String, String)>,
    sign_in: Option<String>,
    recovery_key: Option<String>,
    dns_taken: Option<String>,
    error: Option<String>,
    url: Option<String>,
    others: Vec<String>,
}

struct Shared {
    code: String,
    token: Option<String>,
    started: Instant,
    wrong: u32,
    locked_until: Option<Instant>,
    run: Run,
    finished_at: Option<Instant>,
}

pub fn serve() -> Result<()> {
    let code = code()?;
    let (listener, port) = match TcpListener::bind("0.0.0.0:80") {
        Ok(l) => (l, 80),
        Err(_) => (TcpListener::bind("0.0.0.0:8090").context("neither port 80 nor 8090 is free for the setup page")?, 8090),
    };
    std::fs::write(port_file(), port.to_string())?;
    let _announcement = discovery::announce_setup(port);
    println!("tpcloud: waiting to be set up at {} (pairing code {code})", address());
    let shared = Arc::new(Mutex::new(Shared {
        code,
        token: None,
        started: Instant::now(),
        wrong: 0,
        locked_until: None,
        run: Run { state: "idle", ..Default::default() },
        finished_at: None,
    }));
    listener.set_nonblocking(true)?;
    loop {
        if crate::supervisor::stopping() {
            return Ok(());
        }
        // Done: a little longer for the page to see it, then TPCloud itself starts.
        if shared.lock().unwrap().finished_at.is_some_and(|t| t.elapsed() > Duration::from_secs(20)) {
            let _ = std::fs::remove_file(code_file());
            let _ = std::fs::remove_file(port_file());
            println!("tpcloud: set up; starting TPCloud");
            return Ok(());
        }
        match listener.accept() {
            Ok((stream, peer)) => {
                let shared = shared.clone();
                std::thread::spawn(move || {
                    let _ = stream.set_nonblocking(false);
                    let _ = handle(stream, peer.ip(), &shared);
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => eprintln!("tpcloud: setup page: {e}"),
        }
    }
}

struct Request {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
}

fn read_request(stream: &mut TcpStream) -> Result<Request> {
    stream.set_read_timeout(Some(Duration::from_secs(15)))?;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            anyhow::bail!("closed");
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        if buf.len() > 16 * 1024 {
            anyhow::bail!("headers too long");
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let mut first = lines.next().unwrap_or("").split_whitespace();
    let method = first.next().unwrap_or("").to_string();
    let path = first.next().unwrap_or("/").split('?').next().unwrap_or("/").to_string();
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    let length: usize = headers.iter().find(|(k, _)| k == "content-length").and_then(|(_, v)| v.parse().ok()).unwrap_or(0);
    if length > MAX_BODY {
        anyhow::bail!("body too big");
    }
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < length {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(length);
    Ok(Request { method, path, headers, body })
}

fn respond(stream: &mut TcpStream, status: u16, kind: &str, body: &[u8]) -> Result<()> {
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        429 => "Too Many Requests",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\
         X-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nReferrer-Policy: no-referrer\r\n\
         Content-Security-Policy: default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; frame-ancestors 'none'\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    Ok(())
}

fn send_json(stream: &mut TcpStream, status: u16, v: Value) -> Result<()> {
    respond(stream, status, "application/json", v.to_string().as_bytes())
}

fn private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_loopback() || (v6.segments()[0] & 0xfe00) == 0xfc00 || (v6.segments()[0] & 0xffc0) == 0xfe80,
    }
}

fn handle(mut stream: TcpStream, peer: IpAddr, shared: &Arc<Mutex<Shared>>) -> Result<()> {
    let req = read_request(&mut stream)?;
    // Only the local network: it's how a host with nothing set up is found.
    if !private(peer) {
        return send_json(&mut stream, 403, json!({"detail": "Only from the local network."}));
    }
    if !req.path.starts_with("/api/") {
        return serve_file(&mut stream, &req.path);
    }
    let authorized = {
        let s = shared.lock().unwrap();
        let given = req.header("authorization").and_then(|h| h.strip_prefix("Bearer ")).unwrap_or("");
        s.token.as_deref().is_some_and(|t| !given.is_empty() && constant_eq(t, given))
    };
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/api/setup") => {
            let s = shared.lock().unwrap();
            send_json(&mut stream, 200, json!({
                "tpcloud": 1,
                "version": env!("CARGO_PKG_VERSION"),
                "host": format!("{}.local", network::host_label()),
                "claimed": s.token.is_some(),
                "open": s.token.is_none() && s.started.elapsed() < OPEN_FOR,
            }))
        }
        ("POST", "/api/setup/claim") => claim(&mut stream, &req, shared),
        _ if !authorized => send_json(&mut stream, 401, json!({"detail": "Pair with this machine first."})),
        ("GET", "/api/setup/facts") => {
            let f = engine::facts();
            let a = engine::current(&f);
            send_json(&mut stream, 200, json!({ "facts": facts_json(&f), "answers": answers_json(&a) }))
        }
        ("POST", "/api/setup/check") => match parse_answers(&req.body) {
            Ok(a) => send_json(&mut stream, 200, json!({ "problems": problems_json(&engine::check(&a, &engine::facts())) })),
            Err(e) => send_json(&mut stream, 400, json!({"detail": format!("{e:#}")})),
        },
        ("POST", "/api/setup/apply") => apply(&mut stream, &req, shared),
        ("GET", "/api/setup/progress") => {
            let s = shared.lock().unwrap();
            let r = &s.run;
            send_json(&mut stream, 200, json!({
                "state": r.state,
                "log": r.log.iter().map(|(k, t)| json!({"kind": k, "text": t})).collect::<Vec<_>>(),
                "sign_in": r.sign_in, "recovery_key": r.recovery_key, "dns_taken": r.dns_taken,
                "error": r.error, "url": r.url, "others": r.others,
            }))
        }
        _ => send_json(&mut stream, 404, json!({"detail": "No such thing."})),
    }
}

fn constant_eq(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn claim(stream: &mut TcpStream, req: &Request, shared: &Arc<Mutex<Shared>>) -> Result<()> {
    let body: Value = serde_json::from_slice(&req.body).unwrap_or_default();
    let given = body["code"].as_str().unwrap_or("").trim().to_string();
    let mut s = shared.lock().unwrap();
    if s.locked_until.is_some_and(|t| Instant::now() < t) {
        return send_json(stream, 429, json!({"detail": "Too many wrong codes: wait five minutes."}));
    }
    let open = s.token.is_none() && s.started.elapsed() < OPEN_FOR;
    let right = !given.is_empty() && constant_eq(&given, &s.code);
    if !(right || (open && given.is_empty())) {
        s.wrong += 1;
        if s.wrong >= 10 {
            s.wrong = 0;
            s.locked_until = Some(Instant::now() + Duration::from_secs(300));
        }
        return send_json(stream, 403, json!({"detail": if given.is_empty() { "Type the code shown where TPCloud was installed." } else { "That's not the code." }}));
    }
    // The code always gets in (and takes over from whoever was in); without it, only the first.
    let mut b = [0u8; 24];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut b)?;
    let token: String = b.iter().map(|x| format!("{x:02x}")).collect();
    s.token = Some(token.clone());
    s.wrong = 0;
    send_json(stream, 200, json!({ "token": token }))
}

struct WebProgress {
    shared: Arc<Mutex<Shared>>,
    overwrite: bool,
}

impl engine::Progress for WebProgress {
    fn step(&mut self, text: &str) {
        self.shared.lock().unwrap().run.log.push(("step".into(), text.into()));
    }
    fn done(&mut self, text: &str) {
        self.shared.lock().unwrap().run.log.push(("done".into(), text.into()));
    }
    fn sign_in(&mut self, url: &str) {
        self.shared.lock().unwrap().run.sign_in = Some(url.into());
    }
    fn recovery_key(&mut self, key: &str) {
        self.shared.lock().unwrap().run.recovery_key = Some(key.into());
    }
    fn replace_dns(&mut self, hostname: &str) -> bool {
        if !self.overwrite {
            self.shared.lock().unwrap().run.dns_taken = Some(hostname.into());
        }
        self.overwrite
    }
}

fn apply(stream: &mut TcpStream, req: &Request, shared: &Arc<Mutex<Shared>>) -> Result<()> {
    let answers = match parse_answers(&req.body) {
        Ok(a) => a,
        Err(e) => return send_json(stream, 400, json!({"detail": format!("{e:#}")})),
    };
    let problems = engine::check(&answers, &engine::facts());
    if !problems.is_empty() {
        return send_json(stream, 400, json!({ "problems": problems_json(&problems) }));
    }
    {
        let mut s = shared.lock().unwrap();
        if s.run.state == "running" || s.run.state == "done" {
            return send_json(stream, 409, json!({"detail": "It's being set up already."}));
        }
        s.run = Run { state: "running", ..Default::default() };
    }
    let shared = shared.clone();
    std::thread::spawn(move || {
        let mut progress = WebProgress { shared: shared.clone(), overwrite: answers.overwrite_dns };
        let result = engine::apply(&answers, &mut progress);
        let mut s = shared.lock().unwrap();
        match result {
            Ok(outcome) => {
                s.run.state = "done";
                s.run.url = Some(outcome.url);
                s.run.others = outcome.others;
                s.finished_at = Some(Instant::now());
                // TPCloud itself starts in this unit's place: the extra boot step goes.
                let _ = crate::install::write_unit();
            }
            Err(e) => {
                s.run.state = "failed";
                s.run.error = Some(format!("{e:#}"));
            }
        }
    });
    send_json(stream, 202, json!({"state": "running"}))
}

fn serve_file(stream: &mut TcpStream, path: &str) -> Result<()> {
    let root = paths::runtime_dir().join("web/setup");
    let rel = path.trim_start_matches('/');
    let safe = !rel.split('/').any(|p| p == ".." || p.starts_with('.'));
    let mut file = root.join(if rel.is_empty() || !safe { "index.html" } else { rel });
    if !file.is_file() {
        // The page routes itself: anything that isn't a file is the page.
        file = root.join("index.html");
    }
    let kind = match file.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    };
    match std::fs::read(&file) {
        Ok(body) => respond(stream, 200, kind, &body),
        Err(_) => respond(stream, 404, "text/plain", b"The setup page isn't here: use `sudo tpcloud setup`."),
    }
}

// ── Answers and facts as JSON (what the page and the apps send and get) ──

fn level_name(l: storage::Level) -> &'static str {
    match l {
        storage::Level::Single => "single",
        storage::Level::Raid1 => "raid1",
        storage::Level::Raid10 => "raid10",
    }
}

pub fn answers_json(a: &Answers) -> Value {
    json!({
        "name": a.name,
        "files": match &a.files {
            Files::Folder(p) => json!({"kind": "folder", "path": p}),
            Files::Pool { disks, level, encrypt } => json!({"kind": "pool", "disks": disks, "level": level_name(*level), "encrypt": encrypt}),
        },
        "backup_dir": a.backup_dir,
        "store": a.store,
        "home": a.home,
        "local": a.local,
        "local_names": a.local_names,
        "tailscale": a.tailscale.is_some(),
        "domain": match &a.domain {
            Domain::None => json!({"kind": "none"}),
            Domain::Cloudflare { hostname, new } => json!({"kind": "cloudflare", "hostname": hostname, "new": new}),
            Domain::OpenPort { hostname } => json!({"kind": "open_port", "hostname": hostname}),
        },
        "owner": Value::Null,
        "overwrite_dns": a.overwrite_dns,
    })
}

pub fn parse_answers(body: &[u8]) -> Result<Answers> {
    let v: Value = serde_json::from_slice(body).context("not JSON")?;
    let s = |v: &Value| v.as_str().unwrap_or("").trim().to_string();
    let files = match v["files"]["kind"].as_str() {
        Some("pool") => Files::Pool {
            disks: v["files"]["disks"].as_array().into_iter().flatten().map(s).filter(|d| !d.is_empty()).collect(),
            level: match v["files"]["level"].as_str() {
                Some("raid10") => storage::Level::Raid10,
                Some("single") => storage::Level::Single,
                _ => storage::Level::Raid1,
            },
            encrypt: v["files"]["encrypt"].as_bool().unwrap_or(true),
        },
        _ => Files::Folder(s(&v["files"]["path"])),
    };
    let host = |h: &Value| s(h).trim_start_matches("https://").trim_end_matches('/').to_lowercase();
    let domain = match v["domain"]["kind"].as_str() {
        Some("cloudflare") => Domain::Cloudflare { hostname: host(&v["domain"]["hostname"]), new: v["domain"]["new"].as_bool().unwrap_or(true) },
        Some("open_port") => Domain::OpenPort { hostname: host(&v["domain"]["hostname"]) },
        _ => Domain::None,
    };
    let tailscale = if v["tailscale"].as_bool().unwrap_or(false) { Some(engine::tailscale_name()?) } else { None };
    let owner = v["owner"]["email"].as_str().map(|e| (e.trim().to_lowercase(), v["owner"]["password"].as_str().unwrap_or("").to_string()));
    Ok(Answers {
        name: s(&v["name"]),
        files,
        backup_dir: s(&v["backup_dir"]),
        store: v["store"].as_bool().unwrap_or(true),
        home: v["home"].as_bool().unwrap_or(false),
        local: v["local"].as_bool().unwrap_or(true),
        local_names: v["local_names"].as_array().into_iter().flatten().map(host).filter(|n| !n.is_empty()).collect(),
        tailscale,
        domain,
        owner,
        overwrite_dns: v["overwrite_dns"].as_bool().unwrap_or(false),
    })
}

fn facts_json(f: &engine::Facts) -> Value {
    json!({
        "first": f.first,
        "host": f.host,
        "disks": f.disks.iter().map(|d| json!({
            "path": d.path, "size": d.size, "size_text": storage::human(d.size), "model": d.model,
            "removable": d.removable, "contents": d.contents, "busy": d.busy,
        })).collect::<Vec<_>>(),
        "pool": f.pool,
        "tailscale": f.tailscale,
        "has_token": f.has_token,
        "needs_owner": f.needs_owner,
        "accounts": f.accounts,
        "state_dir": paths::state_dir().display().to_string(),
    })
}

fn problems_json(p: &[engine::Problem]) -> Value {
    Value::Array(p.iter().map(|x| json!({"field": x.field, "message": x.message})).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_round_trip() {
        let body = br#"{"name":"Casa","files":{"kind":"pool","disks":["/dev/sdb","/dev/sdc"],"level":"raid1","encrypt":true},
            "backup_dir":"","store":true,"home":false,"local":true,"local_names":["https://Cloud.Casa/"],"tailscale":false,
            "domain":{"kind":"cloudflare","hostname":"cloud.example.com","new":true},"owner":{"email":"Me@Example.com","password":"correct-horse-battery"}}"#;
        let a = parse_answers(body).unwrap();
        assert_eq!(a.local_names, vec!["cloud.casa"]);
        assert_eq!(a.owner.as_ref().unwrap().0, "me@example.com");
        assert!(matches!(a.files, Files::Pool { ref disks, level: storage::Level::Raid1, encrypt: true } if disks.len() == 2));
        let back = answers_json(&a);
        assert_eq!(back["domain"]["kind"], "cloudflare");
        assert_eq!(back["owner"], Value::Null, "a password never goes back out");
    }

    #[test]
    fn only_private_peers() {
        assert!(private("192.168.1.20".parse().unwrap()));
        assert!(private("fe80::1".parse().unwrap()));
        assert!(!private("8.8.8.8".parse().unwrap()));
    }
}
