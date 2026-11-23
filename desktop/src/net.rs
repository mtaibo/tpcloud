//! Talking to a TPCloud network: its card, the certificate checks, and the API.
//!
//! Certificates: an address the system's roots trust is used as is. A local address serves a
//! certificate from the host's own authority; the first time it's met, the card names that
//! authority's fingerprint, the authority comes from `/auth/network/ca.crt`, both must match and
//! the server's chain must check out against it (trust on first use, like the iPhone app). From
//! then on that authority is the anchor for the network's local addresses, and nothing else is.

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::client::WebPkiServerVerifier;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, RootCertStore, SignatureScheme};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Address {
    pub url: String,
    /// local, tailscale or public.
    pub kind: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Card {
    pub tpcloud: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub addresses: Vec<Address>,
    #[allow(dead_code)] // read from /auth/passkey/me once signed in
    pub modules: Vec<String>,
    pub ca_sha256: Option<String>,
}

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

/// The system's roots, plus the host's own authority for the hosts it's pinned to.
#[derive(Debug)]
struct Verifier {
    roots: Arc<WebPkiServerVerifier>,
    anchored: HashMap<String, Arc<WebPkiServerVerifier>>,
    /// Probe mode: accept anything and keep the chain, to be checked against the card after.
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

    fn verify_tls12_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &provider().signature_verification_algorithms)
    }

    fn verify_tls13_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &provider().signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        provider().signature_verification_algorithms.supported_schemes()
    }
}

fn client_with(verifier: Verifier, timeout: Duration) -> reqwest::Client {
    let tls = rustls::ClientConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()
        .expect("TLS versions")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("X-TP-Client", reqwest::header::HeaderValue::from_static("linux"));
    reqwest::Client::builder()
        .use_preconfigured_tls(tls)
        .default_headers(headers)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .timeout(timeout)
        .build()
        .expect("an HTTP client")
}

pub fn host_of(url: &str) -> String {
    reqwest::Url::parse(url).ok().and_then(|u| u.host_str().map(str::to_lowercase)).unwrap_or_default()
}

/// `tphome.local`, `cloud.example.com/login` → `https://…` with no path.
pub fn normalize(address: &str) -> Option<String> {
    let text = address.trim();
    if text.is_empty() {
        return None;
    }
    let text = if text.contains("://") { text.to_string() } else { format!("https://{text}") };
    let mut url = reqwest::Url::parse(&text).ok()?;
    url.host_str()?;
    url.set_path("");
    url.set_query(None);
    Some(url.as_str().trim_end_matches('/').to_string())
}

fn der_from_pem(pem: &str) -> Option<Vec<u8>> {
    let start = pem.find("-----BEGIN CERTIFICATE-----")? + "-----BEGIN CERTIFICATE-----".len();
    let end = pem.find("-----END CERTIFICATE-----")?;
    base64_decode(&pem[start..end].chars().filter(|c| !c.is_whitespace()).collect::<String>())
}

pub fn base64_decode_pub(s: &str) -> Option<Vec<u8>> {
    base64_decode(s)
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
    for c in s.bytes() {
        if c == b'=' {
            break;
        }
        let v = T.iter().position(|&x| x == c)? as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}

pub fn base64_encode(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub fn fingerprint(der: &[u8]) -> String {
    Sha256::digest(der).iter().map(|b| format!("{b:02x}")).collect()
}

/// Reads a network's card from an address that may not be trusted yet (see the module's notes).
/// Returns the card and, for an address with the host's own authority, that authority (DER).
pub async fn load_card(address: &str) -> Result<(Card, Option<Vec<u8>>)> {
    let base = normalize(address).ok_or("That isn't an address.")?;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let probe = client_with(Verifier { roots: webpki(system_roots()), anchored: HashMap::new(), seen: Some(seen.clone()) }, Duration::from_secs(8));
    let res = probe.get(format!("{base}/.well-known/tpcloud")).send().await.map_err(|e| format!("Couldn't reach it: {e}"))?;
    if !res.status().is_success() {
        return Err("There's no TPCloud at that address.".into());
    }
    let card: Card = res.json().await.map_err(|_| "There's no TPCloud at that address.".to_string())?;
    if card.tpcloud < 1 {
        return Err("There's no TPCloud at that address.".into());
    }
    let chain = seen.lock().unwrap().clone();
    if chain.is_empty() {
        return Ok((card, None)); // plain HTTP
    }
    let host = host_of(&base);
    let name = ServerName::try_from(host.clone()).map_err(|_| "Bad host name.".to_string())?;
    let now = UnixTime::now();
    if webpki(system_roots()).verify_server_cert(&chain[0], &chain[1..], &name, &[], now).is_ok() {
        return Ok((card, None));
    }
    let expected = card.ca_sha256.clone().ok_or("Its certificate isn't one this computer trusts, and it names no authority of its own.")?;
    let pem = probe
        .get(format!("{base}/auth/network/ca.crt"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    let der = der_from_pem(&pem).ok_or("Its certificate authority couldn't be read.")?;
    if fingerprint(&der) != expected.to_lowercase() {
        return Err("Its certificate isn't the one it says it uses. Don't add it if you're not sure where you are.".into());
    }
    let mut roots = RootCertStore::empty();
    roots.add(CertificateDer::from(der.clone())).map_err(|e| e.to_string())?;
    webpki(Arc::new(roots))
        .verify_server_cert(&chain[0], &chain[1..], &name, &[], now)
        .map_err(|_| "Its certificate doesn't check out against its own authority.".to_string())?;
    Ok((card, Some(der)))
}

/// A signed-in (or signing-in) connection to one address of a network.
#[derive(Clone, Debug)]
pub struct Api {
    pub base: String,
    http: reqwest::Client,
    pub token: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Me {
    pub email: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub modules: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Entry {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub size: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Photo {
    pub id: i64,
    pub path: String,
    pub location: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PhotoPage {
    pub items: Vec<Photo>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BlindState {
    pub position: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Blind {
    pub id: String,
    pub name: String,
    pub state: Option<BlindState>,
}

impl Api {
    /// `anchor`: the host's own authority (DER), trusted for the network's local addresses.
    pub fn new(base: &str, anchor: Option<&[u8]>, local_hosts: &[String], token: Option<String>) -> Api {
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
        let http = client_with(Verifier { roots: webpki(system_roots()), anchored, seen: None }, Duration::from_secs(60));
        Api { base: base.trim_end_matches('/').to_string(), http, token }
    }

    fn req(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let mut r = self.http.request(method, format!("{}{}", self.base, path));
        if let Some(t) = &self.token {
            r = r.bearer_auth(t);
        }
        r
    }

    async fn check(res: reqwest::Response) -> Result<reqwest::Response> {
        let status = res.status();
        if status.is_redirection() || status == 401 {
            return Err("signed-out".into());
        }
        if !status.is_success() {
            #[derive(Deserialize)]
            struct E {
                detail: Option<String>,
            }
            let detail = res.json::<E>().await.ok().and_then(|e| e.detail);
            return Err(detail.unwrap_or_else(|| format!("The server said {status}.")));
        }
        Ok(res)
    }

    /// Is this address answering as this network (for picking the fastest)?
    pub async fn answers_as(&self, id: &str) -> bool {
        let Ok(res) = self.req(reqwest::Method::GET, "/.well-known/tpcloud").timeout(Duration::from_millis(2500)).send().await else { return false };
        res.json::<Card>().await.map(|c| c.id == id || id.starts_with("legacy:")).unwrap_or(false)
    }

    /// Password sign-in; Ok(true) when a TOTP code is needed next. Keeps the session token.
    pub async fn login(&mut self, email: &str, password: &str) -> Result<bool> {
        let res = self
            .req(reqwest::Method::POST, "/auth/password/login")
            .json(&serde_json::json!({"email": email, "password": password}))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if let Some(t) = res.headers().get("x-tp-session").and_then(|v| v.to_str().ok()) {
            self.token = Some(t.to_string());
        }
        let res = Self::check(res).await.map_err(|e| if e == "signed-out" { "Incorrect email or password.".into() } else { e })?;
        let body: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
        Ok(body["totp_required"].as_bool().unwrap_or(false))
    }

    pub async fn totp(&self, code: &str) -> Result<()> {
        let res = self.req(reqwest::Method::POST, "/auth/password/totp/verify").json(&serde_json::json!({"code": code})).send().await.map_err(|e| e.to_string())?;
        Self::check(res).await.map(|_| ())
    }

    pub async fn me(&self) -> Result<Me> {
        let res = self.req(reqwest::Method::GET, "/auth/passkey/me").send().await.map_err(|e| e.to_string())?;
        Self::check(res).await?.json().await.map_err(|e| e.to_string())
    }

    pub async fn logout(&self) {
        let _ = self.req(reqwest::Method::POST, "/auth/passkey/logout").send().await;
    }

    pub async fn list(&self, path: &str) -> Result<Vec<Entry>> {
        let res = self.req(reqwest::Method::GET, "/store/api/files/list").query(&[("path", path), ("location", "external")]).send().await.map_err(|e| e.to_string())?;
        #[derive(Deserialize)]
        struct Listing {
            entries: Vec<Entry>,
        }
        let mut list = Self::check(res).await?.json::<Listing>().await.map_err(|e| e.to_string())?.entries;
        list.sort_by(|a, b| (a.kind != "directory").cmp(&(b.kind != "directory")).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        Ok(list)
    }

    pub async fn download(&self, path: &str) -> Result<Vec<u8>> {
        let res = self.req(reqwest::Method::GET, "/store/api/files/download").query(&[("path", path), ("location", "external")]).send().await.map_err(|e| e.to_string())?;
        Self::check(res).await?.bytes().await.map(|b| b.to_vec()).map_err(|e| e.to_string())
    }

    pub async fn upload(&self, folder: &str, name: &str, data: Vec<u8>) -> Result<()> {
        let part = reqwest::multipart::Part::bytes(data).file_name(name.to_string());
        let form = reqwest::multipart::Form::new().part("files", part);
        let res = self
            .req(reqwest::Method::POST, "/store/api/files/upload")
            .query(&[("path", folder), ("location", "external")])
            .multipart(form)
            .timeout(Duration::from_secs(3600))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Self::check(res).await.map(|_| ())
    }

    pub async fn photos(&self, cursor: Option<&str>) -> Result<PhotoPage> {
        let mut q = vec![("limit", "120".to_string())];
        if let Some(c) = cursor {
            q.push(("cursor", c.to_string()));
        }
        let res = self.req(reqwest::Method::GET, "/store/api/photos").query(&q).send().await.map_err(|e| e.to_string())?;
        Self::check(res).await?.json().await.map_err(|e| e.to_string())
    }

    pub async fn thumbnail(&self, photo: &Photo) -> Result<Vec<u8>> {
        let res = self
            .req(reqwest::Method::GET, "/store/api/files/thumbnail")
            .query(&[("path", photo.path.as_str()), ("location", photo.location.as_str()), ("size", "256")])
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Self::check(res).await?.bytes().await.map(|b| b.to_vec()).map_err(|e| e.to_string())
    }

    pub async fn blinds(&self) -> Result<Vec<Blind>> {
        let res = self.req(reqwest::Method::GET, "/home/api/devices/blinds").send().await.map_err(|e| e.to_string())?;
        Self::check(res).await?.json().await.map_err(|e| e.to_string())
    }

    pub async fn blind(&self, id: &str, command: &str) -> Result<()> {
        let res = self.req(reqwest::Method::POST, &format!("/home/api/commands/{id}/{command}")).send().await.map_err(|e| e.to_string())?;
        Self::check(res).await.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses() {
        assert_eq!(normalize("tphome.local").as_deref(), Some("https://tphome.local"));
        assert_eq!(normalize(" https://cloud.example.com/login?x=1 ").as_deref(), Some("https://cloud.example.com"));
        assert_eq!(normalize("http://box.local:8090").as_deref(), Some("http://box.local:8090"));
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
