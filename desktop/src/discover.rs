//! TPCloud hosts announcing themselves on this network (Bonjour): set-up networks and machines
//! waiting to be set up. Their TXT record names the host the address is built from.

use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub name: String,
    pub url: String,
    pub network_id: Option<String>,
}

/// Listens for a couple of seconds and returns what answered: (networks, waiting to be set up).
pub async fn look() -> (Vec<Found>, Vec<Found>) {
    tokio::task::spawn_blocking(|| (browse("_tpcloud._tcp.local.", true), browse("_tpcloud-setup._tcp.local.", false)))
        .await
        .unwrap_or_default()
}

fn browse(service: &str, https: bool) -> Vec<Found> {
    let Ok(daemon) = mdns_sd::ServiceDaemon::new() else { return vec![] };
    let Ok(rx) = daemon.browse(service) else { return vec![] };
    let mut found: Vec<Found> = Vec::new();
    let until = Instant::now() + Duration::from_millis(2500);
    while let Some(left) = until.checked_duration_since(Instant::now()) {
        let Ok(event) = rx.recv_timeout(left) else { break };
        if let mdns_sd::ServiceEvent::ServiceResolved(info) = event {
            let txt = info.get_properties();
            let host = txt.get_property_val_str("host").map(str::to_string).unwrap_or_else(|| info.get_hostname().trim_end_matches('.').to_string());
            let port = info.get_port();
            let url = if https {
                format!("https://{host}")
            } else if port == 80 {
                format!("http://{host}")
            } else {
                format!("http://{host}:{port}")
            };
            let name = info.get_fullname().split("._tpcloud").next().unwrap_or("").replace("\\032", " ");
            let item = Found { name, url, network_id: txt.get_property_val_str("id").map(str::to_string) };
            if !found.contains(&item) {
                found.push(item);
            }
        }
    }
    let _ = daemon.shutdown();
    found
}
