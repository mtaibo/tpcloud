//! Announcing the network on the local network (Bonjour / mDNS), so the apps find it without an
//! address: a `_tpcloud._tcp` service on port 443 named after the network, whose TXT record carries
//! its id, the version and where the network card is. Only while the local site is on.
//!
//! The responder built in here answers for the network's name at home (tpcloud.local) and its
//! services, also beside avahi (see `start`).

use crate::network;
use crate::platform;
use crate::services::Settings;

pub const SERVICE: &str = "_tpcloud._tcp";
const PORT: u16 = 443;

pub const SETUP_SERVICE: &str = "_tpcloud-setup._tcp";

/// Live until dropped: then it says goodbye on the network.
pub struct Announcement {
    kind: Kind,
}

enum Kind {
    Builtin(mdns_sd::ServiceDaemon, String),
}

/// What the service says about itself, besides its name.
fn txt(settings: &Settings) -> Vec<(String, String)> {
    vec![
        ("id".into(), settings.get(network::ID).unwrap_or("").into()),
        ("v".into(), env!("CARGO_PKG_VERSION").into()),
        ("path".into(), "/.well-known/tpcloud".into()),
        // The name the certificate is for: the apps build https://<host> from it.
        ("host".into(), network::local_host()),
    ]
}

/// The network, while the local site is on (an old announcement is withdrawn when it's off).
pub fn announce(settings: &Settings) -> Option<Announcement> {
    if network::local_names(settings).is_empty() {
        platform::discovery::withdraw("tpcloud");
        return None;
    }
    let name = settings.get(network::NAME).unwrap_or("TPCloud").to_string();
    start("tpcloud", &name, SERVICE, PORT, &txt(settings))
}

/// A host waiting to be set up, for the apps' «Set up a new host».
pub fn announce_setup(port: u16) -> Option<Announcement> {
    let txt = vec![
        ("v".to_string(), env!("CARGO_PKG_VERSION").to_string()),
        ("path".to_string(), "/api/setup".to_string()),
        ("host".to_string(), network::local_host()),
        ("port".to_string(), port.to_string()),
    ];
    start("tpcloud-setup", &network::default_name(), SETUP_SERVICE, port, &txt)
}

/// Always our own responder, also beside avahi (both share port 5353): avahi can't publish a
/// second name for the machine's address (its reverse record collides with the machine's own
/// name: «Local name collision»), and the network's name at home isn't the machine's. Anything an
/// older version left in avahi's folders is taken out.
fn start(id: &'static str, name: &str, service: &str, port: u16, txt: &[(String, String)]) -> Option<Announcement> {
    platform::discovery::withdraw(id);
    platform::discovery::withdraw_host();
    match builtin(name, service, port, txt) {
        Ok(kind) => Some(Announcement { kind }),
        Err(err) => {
            eprintln!("tpcloud: couldn't announce on the local network: {err}");
            None
        }
    }
}

fn builtin(name: &str, service: &str, port: u16, txt: &[(String, String)]) -> Result<Kind, mdns_sd::Error> {
    let daemon = mdns_sd::ServiceDaemon::new()?;
    let props: Vec<(&str, &str)> = txt.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let host = format!("{}.", network::local_host());
    let info = mdns_sd::ServiceInfo::new(&format!("{service}.local."), name, &host, "", port, &props[..])?.enable_addr_auto();
    let fullname = info.get_fullname().to_string();
    daemon.register(info)?;
    Ok(Kind::Builtin(daemon, fullname))
}

impl Drop for Announcement {
    fn drop(&mut self) {
        match &self.kind {
            Kind::Builtin(daemon, fullname) => {
                // Says goodbye on the network, so browsers drop it at once instead of timing out.
                if let Ok(done) = daemon.unregister(fullname) {
                    let _ = done.recv_timeout(std::time::Duration::from_secs(2));
                }
                let _ = daemon.shutdown();
            }
        }
    }
}
