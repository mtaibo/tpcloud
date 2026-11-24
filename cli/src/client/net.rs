//! Talking to a TPCloud network from the terminal: its card, the certificate checks and the API
//! with the session's bearer token. The same rules as the apps: an address the system's roots
//! trust is used as is; a local address with the host's own authority is trusted the first time
//! once its card, its `/auth/network/ca.crt` and the server's chain all agree, and from then on
//! only against that authority.

use anyhow::{anyhow, bail, Context, Result};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::client::WebPkiServerVerifier;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, RootCertStore, SignatureScheme};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

fn system_roots() -> Arc<RootCertStore> {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    Arc::new(roots)
}

fn webpki(roots: Arc<RootCertStore>) -> Arc<WebPkiServerVerifier> {
    WebPkiServerVerifier::builder_with_provider(roots, provider()).build().expect("a verifier for these roots")
}

#[derive(Debug)]
struct Verifier {
    roots: Arc<WebPkiServerVerifier>,
    anchored: HashMap<String, Arc<WebPkiServerVerifier>>,
    seen: Option<Arc<Mutex<Vec<CertificateDer<'static>>>>>,
}

impl ServerCertVerifier for Verifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp: &[u8],
        now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        if let Some(seen) = &self.seen {
            let mut chain = vec![end_entity.clone().into_owned()];
            chain.extend(intermediates.iter().map(|c| c.clone().into_owned()));
            *seen.lock().unwrap() = chain;
            return Ok(ServerCertVerified::assertion());
        }
        match self.roots.verify_server_cert(end_entity, intermediates, server_name, ocsp, now) {
            Ok(v) => Ok(v),
            Err(err) => {
                let host = match server_name {
                    ServerName::DnsName(d) => d.as_ref().to_lowercase(),
                    other => format!("{other:?}"),
                };
                match self.anchored.get(&host) {
                    Some(anchor) => anchor.verify_server_cert(end_entity, intermediates, server_name, ocsp, now),
                    None => Err(err),
                }
            }
        }
    }

    fn verify_tls12_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(m, c, d, &provider().signature_verification_algorithms)
    }

    fn verify_tls13_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(m, c, d, &provider().signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        provider().signature_verification_algorithms.supported_schemes()
    }
}

// ── Names ending in .local ──
// The Linux command is a static musl program, and musl's resolver doesn't do mDNS (nor does a
// Linux without nss-mdns), so `tpcloud.local` would never resolve: those names are looked up here
// over mDNS when the system can't, and the addresses found while browsing are remembered.

static LOCAL_NAMES: Mutex<Option<HashMap<String, Vec<std::net::IpAddr>>>> = Mutex::new(None);

/// What browsing found for a host, so connecting to it right after needs no second query.
pub fn remember_local(host: &str, ips: Vec<std::net::IpAddr>) {
    if ips.is_empty() {
        return;
    }
    let key = host.trim_end_matches('.').to_ascii_lowercase();
    LOCAL_NAMES.lock().unwrap().get_or_insert_with(HashMap::new).insert(key, ips);
}

fn lookup_local(host: &str) -> Vec<std::net::IpAddr> {
    let key = host.trim_end_matches('.').to_ascii_lowercase();
    if let Some(ips) = LOCAL_NAMES.lock().unwrap().as_ref().and_then(|m| m.get(&key)) {
        return ips.clone();
    }
    let Ok(daemon) = mdns_sd::ServiceDaemon::new() else { return vec![] };
    let mut ips: Vec<std::net::IpAddr> = Vec::new();
    if let Ok(rx) = daemon.resolve_hostname(&format!("{key}."), Some(2500)) {
        let until = std::time::Instant::now() + Duration::from_millis(2500);
        while let Some(left) = until.checked_duration_since(std::time::Instant::now()) {
            match rx.recv_timeout(left) {
                Ok(mdns_sd::HostnameResolutionEvent::AddressesFound(_, found)) => {
                    for ip in found.iter().map(|ip| ip.to_ip_addr()) {
                        if !ips.contains(&ip) {
                            ips.push(ip);
                        }
                    }
                    // IPv4 first: an IPv6 link-local address needs its interface, which a URL can't carry.
                    if ips.iter().any(|ip| ip.is_ipv4()) {
                        break;
                    }
                }
                Ok(mdns_sd::HostnameResolutionEvent::SearchTimeout(_)) | Err(_) => break,
                Ok(_) => {}
            }
        }
    }
    let _ = daemon.shutdown();
    ips.sort_by_key(|ip| !ip.is_ipv4());
    ips.retain(|ip| !matches!(ip, std::net::IpAddr::V6(v6) if (v6.segments()[0] & 0xffc0) == 0xfe80));
    remember_local(&key, ips.clone());
    ips
}

fn resolve(addr: &str) -> std::io::Result<Vec<std::net::SocketAddr>> {
    use std::net::ToSocketAddrs;
    let system = addr.to_socket_addrs().map(|a| a.collect::<Vec<_>>());
    if matches!(&system, Ok(a) if !a.is_empty()) {
        return system;
    }
    let (host, port) = addr.rsplit_once(':').unwrap_or((addr, "443"));
    if host.to_ascii_lowercase().trim_end_matches('.').ends_with(".local") {
        let port: u16 = port.parse().unwrap_or(443);
        let found: Vec<_> = lookup_local(host).into_iter().map(|ip| std::net::SocketAddr::new(ip, port)).collect();
        if !found.is_empty() {
            return Ok(found);
        }
    }
    system
}

fn agent_with(verifier: Verifier, timeout: Duration) -> ureq::Agent {
    let tls = rustls::ClientConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()
        .expect("TLS versions")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    ureq::AgentBuilder::new()
        .tls_config(Arc::new(tls))
        .resolver(resolve)
        .redirects(0)
        .timeout_connect(Duration::from_secs(5))
        .timeout(timeout)
        .user_agent(&format!("tpcloud/{} ({})", env!("CARGO_PKG_VERSION"), if cfg!(target_os = "macos") { "macOS" } else { "Linux" }))
        .build()
}

pub fn host_of(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let rest = rest.split('/').next().unwrap_or(rest);
    rest.rsplit_once(':').filter(|(_, p)| p.chars().all(|c| c.is_ascii_digit())).map_or(rest, |(h, _)| h).to_lowercase()
}

/// `tpcloud.local`, `cloud.example.com/login` → `https://…` with no path.
pub fn normalize(address: &str) -> Option<String> {
    let text = address.trim();
    if text.is_empty() {
        return None;
    }
    let text = if text.contains("://") { text.to_string() } else { format!("https://{text}") };
    let (scheme, rest) = text.split_once("://")?;
    let hostport = rest.split(['/', '?', '#']).next()?;
    if hostport.is_empty() {
        return None;
    }
    Some(format!("{scheme}://{hostport}"))
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let (mut buf, mut bits) = (0u32, 0);
    for c in s.bytes() {
        if c == b'=' {
            break;
        }
        if c.is_ascii_whitespace() {
            continue;
        }
        buf = (buf << 6) | B64.iter().position(|&x| x == c)? as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}

pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            out.push(if i <= chunk.len() { B64[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
        }
    }
    out
}

fn der_from_pem(pem: &str) -> Option<Vec<u8>> {
    let start = pem.find("-----BEGIN CERTIFICATE-----")? + "-----BEGIN CERTIFICATE-----".len();
    let end = pem.find("-----END CERTIFICATE-----")?;
    base64_decode(&pem[start..end])
}

pub fn fingerprint(der: &[u8]) -> String {
    Sha256::digest(der).iter().map(|b| format!("{b:02x}")).collect()
}

/// What a network says about itself at /.well-known/tpcloud.
pub struct Card {
    pub id: String,
    pub name: String,
    pub version: String,
    /// (url, kind): kind is local, tailscale or public.
    pub addresses: Vec<(String, String)>,
}

fn parse_card(v: &Value) -> Result<Card> {
    if v["tpcloud"].as_u64().unwrap_or(0) < 1 {
        bail!("there's no TPCloud at that address");
    }
    Ok(Card {
        id: v["id"].as_str().unwrap_or("").to_string(),
        name: v["name"].as_str().unwrap_or("").to_string(),
        version: v["version"].as_str().unwrap_or("").to_string(),
        addresses: v["addresses"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|a| (a["url"].as_str().unwrap_or("").to_string(), a["kind"].as_str().unwrap_or("").to_string()))
            .collect(),
    })
}

/// Reads a network's card from an address that may not be trusted yet; returns it and, for an
/// address with the host's own authority, that authority (DER).
pub fn load_card(address: &str) -> Result<(Card, Option<Vec<u8>>)> {
    let base = normalize(address).context("that isn't an address")?;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let probe = agent_with(Verifier { roots: webpki(system_roots()), anchored: HashMap::new(), seen: Some(seen.clone()) }, Duration::from_secs(8));
    let body: Value = probe
        .get(&format!("{base}/.well-known/tpcloud"))
        .call()
        .map_err(|e| anyhow!("couldn't reach {base}: {}", short(e)))
        .and_then(|r| json_of(r).map_err(|_| anyhow!("there's no TPCloud at {base}")))?;
    let card = parse_card(&body)?;
    let chain = seen.lock().unwrap().clone();
    if chain.is_empty() {
        return Ok((card, None));
    }
    let host = host_of(&base);
    let name = ServerName::try_from(host).map_err(|_| anyhow!("bad host name"))?;
    let now = UnixTime::now();
    if webpki(system_roots()).verify_server_cert(&chain[0], &chain[1..], &name, &[], now).is_ok() {
        return Ok((card, None));
    }
    let expected = body["ca_sha256"].as_str().context("its certificate isn't trusted here, and it names no authority of its own")?;
    let pem = probe.get(&format!("{base}/auth/network/ca.crt")).call().map_err(|e| anyhow!(short(e)))?.into_string()?;
    let der = der_from_pem(&pem).context("its certificate authority couldn't be read")?;
    if fingerprint(&der) != expected.to_lowercase() {
        bail!("its certificate isn't the one it says it uses: don't add it if you're not sure where you are");
    }
    let mut roots = RootCertStore::empty();
    roots.add(CertificateDer::from(der.clone()))?;
    webpki(Arc::new(roots))
        .verify_server_cert(&chain[0], &chain[1..], &name, &[], now)
        .map_err(|_| anyhow!("its certificate doesn't check out against its own authority"))?;
    Ok((card, Some(der)))
}

/// A response's body as JSON (ureq is built without its json feature here).
/// A value for a query string: everything but unreserved characters percent-encoded.
pub fn query_value(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for b in text.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn rand_u64() -> u64 {
    let mut bytes = [0u8; 8];
    if std::fs::File::open("/dev/urandom").and_then(|mut f| std::io::Read::read_exact(&mut f, &mut bytes)).is_err() {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        return t.as_nanos() as u64 ^ std::process::id() as u64;
    }
    u64::from_le_bytes(bytes)
}

pub fn json_of(r: ureq::Response) -> Result<Value> {
    Ok(serde_json::from_str(&r.into_string()?)?)
}

fn send_json(r: ureq::Request, body: Value) -> std::result::Result<ureq::Response, ureq::Error> {
    r.set("Content-Type", "application/json").send_string(&body.to_string())
}

fn short(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, _) => format!("it answered {code}"),
        ureq::Error::Transport(t) => t.to_string(),
    }
}

/// One address of a network, with the session's token.
pub struct Remote {
    pub base: String,
    agent: ureq::Agent,
    pub token: Option<String>,
}

impl Remote {
    pub fn new(base: &str, anchor: Option<&[u8]>, local_hosts: &[String], token: Option<String>) -> Remote {
        let mut anchored = HashMap::new();
        if let Some(der) = anchor {
            let mut roots = RootCertStore::empty();
            if roots.add(CertificateDer::from(der.to_vec())).is_ok() {
                let v = webpki(Arc::new(roots));
                for h in local_hosts {
                    anchored.insert(h.to_lowercase(), v.clone());
                }
            }
        }
        let agent = agent_with(Verifier { roots: webpki(system_roots()), anchored, seen: None }, Duration::from_secs(120));
        Remote { base: base.trim_end_matches('/').to_string(), agent, token }
    }

    fn request(&self, method: &str, path: &str) -> ureq::Request {
        let mut r = self.agent.request(method, &format!("{}{path}", self.base)).set("X-TP-Client", "cli");
        if let Some(t) = &self.token {
            r = r.set("Authorization", &format!("Bearer {t}"));
        }
        r
    }

    /// Whether this address answers as this network, quickly (to pick the fastest).
    pub fn answers_as(&self, id: &str) -> bool {
        self.agent
            .get(&format!("{}/.well-known/tpcloud", self.base))
            .timeout(Duration::from_millis(2500))
            .call()
            .ok()
            .and_then(|r| json_of(r).ok())
            .is_some_and(|v| v["id"].as_str() == Some(id))
    }

    fn finish(result: std::result::Result<ureq::Response, ureq::Error>) -> Result<ureq::Response> {
        match result {
            Ok(r) if (300..400).contains(&r.status()) => bail!("signed out: `tpcloud login`"),
            Ok(r) => Ok(r),
            Err(ureq::Error::Status(401, _)) => bail!("signed out: `tpcloud login`"),
            Err(ureq::Error::Status(403, r)) => bail!("{}", detail(r).unwrap_or_else(|| "only an admin can do that".into())),
            Err(ureq::Error::Status(code, r)) => bail!("{}", detail(r).unwrap_or_else(|| format!("the server answered {code}"))),
            Err(ureq::Error::Transport(t)) => bail!("couldn't reach the server: {t}"),
        }
    }

    pub fn get(&self, path: &str) -> Result<Value> {
        json_of(Self::finish(self.request("GET", path).call())?)
    }

    /// A file's bytes into `dest`, through a temporary file beside it (a cut connection never
    /// leaves half a file under the real name). No overall timeout: big files take their time.
    pub fn download_to(&self, path: &str, dest: &std::path::Path) -> Result<()> {
        let res = Self::finish(self.request("GET", path).timeout(Duration::from_secs(24 * 3600)).call())?;
        let name = dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let part = dest.with_file_name(format!(".{name}.tpcloud-part"));
        let result = (|| -> Result<()> {
            let mut out = std::fs::File::create(&part)?;
            std::io::copy(&mut res.into_reader(), &mut out)?;
            out.sync_all()?;
            Ok(())
        })();
        if let Err(err) = result {
            let _ = std::fs::remove_file(&part);
            return Err(err);
        }
        std::fs::rename(&part, dest)?;
        Ok(())
    }

    /// One file sent as a multipart form, streamed from disk (its size is known, so no chunks).
    pub fn upload_file(&self, path: &str, field: &str, file: &std::path::Path) -> Result<Value> {
        let name = file.file_name().map(|n| n.to_string_lossy().replace('"', "%22")).unwrap_or_default();
        let boundary = format!("tpcloud{:016x}", rand_u64());
        let head = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{field}\"; filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n");
        let tail = format!("\r\n--{boundary}--\r\n");
        let f = std::fs::File::open(file)?;
        let len = f.metadata()?.len();
        let body = std::io::Read::chain(std::io::Read::chain(std::io::Cursor::new(head.clone().into_bytes()), f), std::io::Cursor::new(tail.clone().into_bytes()));
        let res = Self::finish(
            self.request("POST", path)
                .timeout(Duration::from_secs(24 * 3600))
                .set("Content-Type", &format!("multipart/form-data; boundary={boundary}"))
                .set("Content-Length", &(head.len() as u64 + len + tail.len() as u64).to_string())
                .send(body),
        )?;
        Ok(json_of(res).unwrap_or(Value::Null))
    }

    pub fn send(&self, method: &str, path: &str, body: Option<Value>) -> Result<Value> {
        let r = self.request(method, path);
        let res = Self::finish(match body {
            Some(b) => send_json(r, b),
            None => r.call(),
        })?;
        Ok(json_of(res).unwrap_or(Value::Null))
    }

    /// Starts a sign-in someone approves from a browser or a phone already signed in (the web's
    /// QR sign-in): returns the page that approves it, and the secret to collect the session with.
    pub fn start_approval(&self) -> Result<(String, String)> {
        let res = Self::finish(self.request("POST", "/auth/qr/start").call())?;
        let cookie = res
            .all("set-cookie")
            .into_iter()
            .find(|c| c.contains("tpcloud_qr="))
            .and_then(|c| c.split(';').next())
            .map(str::to_string)
            .context("the server didn't start the sign-in")?;
        let body = json_of(res)?;
        Ok((body["url"].as_str().unwrap_or("").to_string(), cookie))
    }

    /// How that sign-in is going: pending, scanned, approved (then the token is kept), denied
    /// or expired.
    pub fn poll_approval(&mut self, cookie: &str) -> Result<String> {
        let res = Self::finish(self.request("GET", "/auth/qr/poll").set("Cookie", cookie).call())?;
        if let Some(t) = res.header("x-tp-session") {
            self.token = Some(t.to_string());
        }
        let body = json_of(res)?;
        Ok(body["status"].as_str().unwrap_or("expired").to_string())
    }

    /// Password sign-in: keeps the session token; Ok(true) when a TOTP code is needed next.
    pub fn login(&mut self, email: &str, password: &str) -> Result<bool> {
        let res = send_json(self.request("POST", "/auth/password/login"), serde_json::json!({ "email": email, "password": password }));
        let res = match res {
            Err(ureq::Error::Status(401, r)) => bail!("{}", detail(r).unwrap_or_else(|| "incorrect email or password".into())),
            other => Self::finish(other)?,
        };
        if let Some(t) = res.header("x-tp-session") {
            self.token = Some(t.to_string());
        }
        let body = json_of(res)?;
        Ok(body["totp_required"].as_bool().unwrap_or(false))
    }
}

fn detail(r: ureq::Response) -> Option<String> {
    json_of(r).ok().and_then(|v| v["detail"].as_str().map(str::to_string))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses() {
        assert_eq!(normalize("tpcloud.local").as_deref(), Some("https://tpcloud.local"));
        assert_eq!(normalize(" https://cloud.example.com/login?x=1 ").as_deref(), Some("https://cloud.example.com"));
        assert_eq!(normalize("http://box.local:8090").as_deref(), Some("http://box.local:8090"));
        assert_eq!(host_of("https://Box.Local:8443/x"), "box.local");
        assert_eq!(normalize(""), None);
    }

    #[test]
    fn base64_round_trip() {
        for data in [&b""[..], b"a", b"ab", b"abc", b"\x30\x03\x02\x01\x01"] {
            assert_eq!(base64_decode(&base64_encode(data)).unwrap(), data);
        }
        assert_eq!(der_from_pem("-----BEGIN CERTIFICATE-----\nMAMCAQE=\n-----END CERTIFICATE-----").unwrap(), vec![0x30, 3, 2, 1, 1]);
    }
}
