//! `tphome`: the house from the terminal. On the server itself it talks to home-api on its local
//! port (whoever runs commands there owns the server, so no session); anywhere else, to the
//! network you're signed in to (`tpcloud login`), with your session.

use crate::ui;
use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde_json::Value;

#[derive(Parser)]
#[command(name = "tphome", version, about = "Control the house's blinds and lights from the terminal.", styles = crate::help::styles())]
#[command(after_help = "Examples:\n  tphome                     every blind and its state\n  tphome --blind B0301 up    raise it (also: down, stop)\n  tphome --blind B0301 50    move it to 50 % open (0 closed, 100 open)")]
pub struct HomeCli {
    /// The blind to act on (e.g. B0301); alone, shows its state.
    #[arg(short, long, value_name = "ID")]
    blind: Option<String>,

    /// What to do with --blind: up, down, stop, or a position from 0 (closed) to 100 (open).
    #[arg(value_name = "ACTION", requires = "blind")]
    action: Option<String>,

    #[command(subcommand)]
    command: Option<HomeCommand>,
}

#[derive(Subcommand)]
enum HomeCommand {
    /// Every blind, with its position and whether it's connected.
    Blinds,
    /// Every light and whether it's connected.
    Lights,
    /// Ask every switch to say it's there (marks the ones that answer as connected).
    Ping,
}

pub fn run(args: impl IntoIterator<Item = String>) -> Result<()> {
    let cli: HomeCli = crate::help::parse(args);
    let api = Api::connect()?;
    match (cli.blind, cli.action, cli.command) {
        (Some(id), Some(action), _) => blind_action(&api, &id, &action),
        (Some(id), None, _) => show_blind(&api, &id),
        (None, _, Some(HomeCommand::Lights)) => list_lights(&api),
        (None, _, Some(HomeCommand::Ping)) => ping_all(&api),
        _ => list_blinds(&api),
    }
}

pub enum Api {
    /// home-api on this server.
    #[cfg_attr(not(feature = "host"), allow(dead_code))]
    Local(String),
    /// The network signed in to, its home-api under /home/api.
    Remote(crate::client::net::Remote),
}

impl Api {
    pub fn connect() -> Result<Self> {
        #[cfg(feature = "host")]
        if crate::paths::env_file().exists() && !crate::client::signed_in() {
            if !crate::install::home_enabled() {
                bail!("Home is off on this server: turn it on with `sudo tpcloud module enable home`");
            }
            return Ok(Api::Local(format!("http://127.0.0.1:{}", crate::services::HOME_PORT)));
        }
        let (remote, _) = crate::client::connect()?;
        let me = remote.get("/auth/passkey/me")?;
        if !me["modules"].as_array().is_some_and(|m| m.iter().any(|x| x == "home")) {
            bail!("Home isn't on in this network");
        }
        Ok(Api::Remote(remote))
    }

    fn get(&self, path: &str) -> Result<Value> {
        let base = match self {
            Api::Remote(r) => return r.get(&format!("/home/api{path}")).map_err(friendly),
            Api::Local(base) => base,
        };
        let body = ureq::get(&format!("{base}{path}"))
            .timeout(std::time::Duration::from_secs(5))
            .call()
            .map_err(api_error)?
            .into_string()?;
        Ok(serde_json::from_str(&body)?)
    }

    fn post(&self, path: &str) -> Result<()> {
        let base = match self {
            Api::Remote(r) => return r.send("POST", &format!("/home/api{path}"), None).map(|_| ()).map_err(friendly),
            Api::Local(base) => base,
        };
        ureq::post(&format!("{base}{path}"))
            .timeout(std::time::Duration::from_secs(5))
            .call()
            .map_err(api_error)?;
        Ok(())
    }
}

fn api_error(err: ureq::Error) -> anyhow::Error {
    match err {
        ureq::Error::Status(code, resp) => {
            let detail = resp
                .into_string()
                .ok()
                .and_then(|b| serde_json::from_str::<Value>(&b).ok())
                .and_then(|v| v["detail"].as_str().map(str::to_string))
                .unwrap_or_default();
            match code {
                404 => anyhow::anyhow!("there's no device with that id{}", suffix(&detail)),
                423 | 409 => anyhow::anyhow!("it's locked{}", suffix(&detail)),
                _ => anyhow::anyhow!("home-api answered {code}{}", suffix(&detail)),
            }
        }
        ureq::Error::Transport(_) => anyhow::anyhow!("home-api isn't answering (is TPCloud running? `tpcloud status`)"),
    }
}

/// The network's answers in tphome's words.
fn friendly(err: anyhow::Error) -> anyhow::Error {
    let text = err.to_string();
    if text.contains("404") || text.to_lowercase().contains("not found") {
        anyhow::anyhow!("there's no device with that id")
    } else if text.contains("423") || text.contains("409") || text.to_lowercase().contains("lock") {
        anyhow::anyhow!("it's locked ({text})")
    } else {
        err
    }
}

fn suffix(detail: &str) -> String {
    if detail.is_empty() { String::new() } else { format!(" ({detail})") }
}

pub enum Action {
    Up,
    Down,
    Stop,
    Set(u8),
}

pub fn parse_action(text: &str) -> Result<Action> {
    match text.to_ascii_lowercase().as_str() {
        "up" | "open" | "subir" => Ok(Action::Up),
        "down" | "close" | "bajar" => Ok(Action::Down),
        "stop" | "parar" => Ok(Action::Stop),
        other => match other.trim_end_matches('%').parse::<u8>() {
            Ok(n) if n <= 100 => Ok(Action::Set(n)),
            _ => bail!("{text:?} isn't an action: use up, down, stop or a position from 0 to 100"),
        },
    }
}

pub fn blinds(api: &Api) -> Result<Vec<Value>> {
    Ok(api.get("/devices/blinds")?.as_array().cloned().unwrap_or_default())
}

fn find_blind(api: &Api, id: &str) -> Result<Value> {
    let id = id.to_ascii_uppercase();
    blinds(api)?
        .into_iter()
        .find(|b| b["id"].as_str() == Some(id.as_str()))
        .with_context(|| format!("there's no blind {id} (`tphome` lists them)"))
}

pub fn blind_action(api: &Api, id: &str, action: &str) -> Result<()> {
    let (name, said) = send(api, id, parse_action(action)?)?;
    ui::done(&format!("{name} ({}): {said}", id.to_ascii_uppercase()));
    Ok(())
}

/// Sends an action to a blind; returns its name and what it's doing, for a message.
pub fn send(api: &Api, id: &str, action: Action) -> Result<(String, String)> {
    let blind = find_blind(api, id)?;
    let id = blind["id"].as_str().unwrap_or_default().to_string();
    let name = blind["name"].as_str().unwrap_or(&id).to_string();
    if blind["locked"].as_bool() == Some(true) {
        bail!("{name} ({id}) is locked: unlock it from Home first");
    }

    let (path, said) = match action {
        Action::Up => ("up".to_string(), "raising".to_string()),
        Action::Down => ("down".to_string(), "lowering".to_string()),
        Action::Stop => ("stop".to_string(), "stopped".to_string()),
        Action::Set(n) => (format!("set/{n}"), format!("moving to {n} % open")),
    };
    api.post(&format!("/commands/{id}/{path}"))?;
    Ok((name, said))
}

fn show_blind(api: &Api, id: &str) -> Result<()> {
    let blind = find_blind(api, id)?;
    print_blinds(&[blind]);
    Ok(())
}

fn list_blinds(api: &Api) -> Result<()> {
    let list = blinds(api)?;
    if list.is_empty() {
        println!("No blinds set up yet.");
        return Ok(());
    }
    print_blinds(&list);
    Ok(())
}

fn print_blinds(list: &[Value]) {
    let width = list.iter().map(|b| b["name"].as_str().unwrap_or("").chars().count()).max().unwrap_or(4).max(4);
    for b in list {
        let id = b["id"].as_str().unwrap_or("?");
        let name = b["name"].as_str().unwrap_or(id);
        let online = b["connection"]["online"].as_bool() == Some(true);
        let position = match b["state"]["position"].as_i64() {
            Some(p) => format!("{p:>3} %"),
            None => "  —  ".into(),
        };
        let motion = match b["state"]["motor_state"].as_i64() {
            Some(1) => " waiting",
            Some(2) => " moving",
            Some(3) => " stopping",
            _ => "",
        };
        let lock = if b["locked"].as_bool() == Some(true) { " · locked" } else { "" };
        println!(
            "  {} {id:<6} {name:<width$}  {position}{motion}{lock}",
            if online { ui::green("●") } else { ui::dim("○") },
        );
    }
}

fn list_lights(api: &Api) -> Result<()> {
    let list = api.get("/devices/lights")?.as_array().cloned().unwrap_or_default();
    if list.is_empty() {
        println!("No lights set up yet.");
    }
    for l in &list {
        let id = l["id"].as_str().unwrap_or("?");
        let online = l["connection"]["online"].as_bool() == Some(true);
        println!("  {} {id:<6} {}", if online { ui::green("●") } else { ui::dim("○") }, l["name"].as_str().unwrap_or(id));
    }
    Ok(())
}

fn ping_all(api: &Api) -> Result<()> {
    let list = blinds(api)?;
    for b in &list {
        if let Some(id) = b["id"].as_str() {
            if b["connection"].is_object() {
                api.post(&format!("/commands/{id}/ping"))?;
            }
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(1500));
    list_blinds(api)
}
