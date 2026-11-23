//! The networks this computer knows, the one in use and each one's session, in
//! ~/.config/tpcloud/networks.json (readable by this user only).

use crate::net::{self, Address};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Saved {
    pub id: String,
    pub name: String,
    pub addresses: Vec<Address>,
    /// The host's own certificate authority (DER, base64), for its local addresses.
    #[serde(default)]
    pub anchor: Option<String>,
    #[serde(default)]
    pub last_address: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

impl Saved {
    pub fn anchor_der(&self) -> Option<Vec<u8>> {
        self.anchor.as_deref().and_then(|a| {
            // Stored as base64 of the DER.
            let pem = format!("-----BEGIN CERTIFICATE-----\n{a}\n-----END CERTIFICATE-----");
            net_der(&pem)
        })
    }

    pub fn local_hosts(&self) -> Vec<String> {
        self.addresses.iter().filter(|a| a.kind == "local").map(|a| net::host_of(&a.url)).collect()
    }

    /// The addresses, home's first, then Tailscale's, then public ones.
    pub fn by_preference(&self) -> Vec<String> {
        let rank = |k: &str| match k {
            "local" => 0,
            "tailscale" => 1,
            _ => 2,
        };
        let mut list = self.addresses.clone();
        list.sort_by_key(|a| rank(&a.kind));
        list.into_iter().map(|a| a.url).collect()
    }
}

fn net_der(pem: &str) -> Option<Vec<u8>> {
    let body: String = pem.lines().filter(|l| !l.starts_with("-----")).collect();
    crate::net::base64_decode_pub(&body)
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Store {
    pub networks: Vec<Saved>,
    pub current: Option<String>,
}

fn file() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("tpcloud/networks.json")
}

impl Store {
    pub fn load() -> Store {
        std::fs::read_to_string(file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        let path = file();
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, serde_json::to_string_pretty(self).unwrap_or_default()).is_ok() {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
            let _ = std::fs::rename(tmp, path);
        }
    }

    pub fn current(&self) -> Option<&Saved> {
        let id = self.current.as_ref()?;
        self.networks.iter().find(|n| &n.id == id)
    }

    pub fn current_mut(&mut self) -> Option<&mut Saved> {
        let id = self.current.clone()?;
        self.networks.iter_mut().find(|n| n.id == id)
    }

    pub fn upsert(&mut self, card: &net::Card, anchor: Option<Vec<u8>>, reached: &str) {
        let anchor = anchor.map(|d| net::base64_encode(&d));
        match self.networks.iter_mut().find(|n| n.id == card.id) {
            Some(n) => {
                n.name = card.name.clone();
                n.addresses = card.addresses.clone();
                if anchor.is_some() {
                    n.anchor = anchor;
                }
                n.last_address = Some(reached.to_string());
            }
            None => self.networks.push(Saved {
                id: card.id.clone(),
                name: card.name.clone(),
                addresses: card.addresses.clone(),
                anchor,
                last_address: Some(reached.to_string()),
                token: None,
                email: None,
            }),
        }
        self.current = Some(card.id.clone());
        self.save();
    }

    pub fn remove(&mut self, id: &str) {
        self.networks.retain(|n| n.id != id);
        if self.current.as_deref() == Some(id) {
            self.current = self.networks.first().map(|n| n.id.clone());
        }
        self.save();
    }
}
