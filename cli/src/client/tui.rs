//! `tpcloud` alone on a computer signed in to a network: the network in a terminal app. Overview,
//! your files (browse, download, upload, share), the folders synced here, your share links, the
//! house (blinds), and for admins the accounts, everyone's sessions and the host's version with
//! its update. A worker thread does every request, so keys never wait for the network.

use super::net::{query_value, Remote};
use super::sync;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;
use serde_json::Value;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const ACCENT: Color = Color::Rgb(106, 175, 255);
const GREEN: Color = Color::Rgb(120, 200, 120);
const ORANGE: Color = Color::Rgb(255, 170, 90);
const DIM: Color = Color::DarkGray;

#[derive(Clone, Copy, PartialEq)]
enum Section {
    Overview,
    Files,
    Sync,
    Shares,
    Home,
    Users,
    Sessions,
    Server,
}

impl Section {
    fn title(self) -> &'static str {
        match self {
            Section::Overview => "Overview",
            Section::Files => "Files",
            Section::Sync => "Sync",
            Section::Shares => "Shares",
            Section::Home => "Home",
            Section::Users => "Accounts",
            Section::Sessions => "Sessions",
            Section::Server => "Server",
        }
    }
}

#[derive(Default)]
struct Data {
    me: Value,
    card: Value,
    blinds: Vec<Value>,
    users: Vec<Value>,
    sessions: Vec<Value>,
    host: Value,
    network: Value,
    /// The folder shown in Files ("" = the two roots: yours and shared) and what's in it.
    cwd: String,
    files: Vec<Value>,
    shares: Vec<Value>,
    message: Option<(String, Color, Instant)>,
    loaded: bool,
}

/// Something typed or confirmed in the footer.
#[derive(Clone)]
enum Prompt {
    Mkdir,
    Rename(String),
    Upload,
    AddSync,
    Delete(String, String),
    Revoke(String),
    Unsync(String, String),
}

enum Job {
    List(String),
    Download(String, String),
    Delete(String),
    Mkdir(String),
    Rename(String, String),
    Upload(String, std::path::PathBuf),
    Share(String),
    Unshare(String),
    SyncAdd(String),
    SyncNow,
    SyncPause(bool),
    SyncRemove(String),
    SyncMode(std::path::PathBuf, bool),
    SyncFree(std::path::PathBuf),
    SyncKeep(std::path::PathBuf, bool),
    Blind(String, String),
    Revoke(i64, bool),
    ToggleAdmin(i64, String),
    HostCheck,
    HostUpdate,
}

struct App {
    sections: Vec<Section>,
    menu: ListState,
    focused: bool,
    item: ListState,
    prompt: Option<(Prompt, String)>,
    data: Arc<Mutex<Data>>,
    jobs: mpsc::Sender<Job>,
}

pub fn run(remote: Remote, network: Value) -> Result<()> {
    let data = Arc::new(Mutex::new(Data { network: network.clone(), ..Data::default() }));
    let (tx, rx) = mpsc::channel::<Job>();
    // The worker: the first load, then every few seconds, and whatever a key asked for.
    {
        let data = data.clone();
        std::thread::spawn(move || worker(remote, data, rx));
    }
    // Wait a moment for the first load, so the menu knows what this account may see.
    let started = Instant::now();
    while !data.lock().unwrap().loaded && started.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(30));
    }
    let mut sections = vec![Section::Overview];
    {
        let d = data.lock().unwrap();
        if d.me["modules"].as_array().is_some_and(|m| m.iter().any(|x| x == "store")) {
            sections.extend([Section::Files, Section::Sync, Section::Shares]);
        }
        if d.me["modules"].as_array().is_some_and(|m| m.iter().any(|x| x == "home")) {
            sections.push(Section::Home);
        }
        if d.me["is_admin"] == true {
            sections.extend([Section::Users, Section::Sessions, Section::Server]);
        } else {
            sections.push(Section::Sessions);
        }
    }
    let mut app = App { sections, menu: ListState::default().with_selected(Some(0)), focused: false, item: ListState::default().with_selected(Some(0)), prompt: None, data, jobs: tx };
    let mut terminal = ratatui::init();
    let result = (|| -> Result<()> {
        loop {
            terminal.draw(|f| draw(f, &mut app, &network))?;
            if !event::poll(Duration::from_millis(250))? {
                continue;
            }
            let Event::Key(key) = event::read()? else { continue };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if !handle(&mut app, key.code) {
                return Ok(());
            }
        }
    })();
    ratatui::restore();
    result
}

fn worker(remote: Remote, data: Arc<Mutex<Data>>, jobs: mpsc::Receiver<Job>) {
    let refresh = |data: &Arc<Mutex<Data>>| {
        let me = remote.get("/auth/passkey/me").unwrap_or(Value::Null);
        let admin = me["is_admin"] == true;
        let home = me["modules"].as_array().is_some_and(|m| m.iter().any(|x| x == "home"));
        let card = remote.get("/.well-known/tpcloud").unwrap_or(Value::Null);
        let blinds = if home { remote.get("/home/api/devices/blinds").ok().and_then(|v| v.as_array().cloned()).unwrap_or_default() } else { vec![] };
        let users = if admin { remote.get("/auth/admin/users").ok().and_then(|v| v.as_array().cloned()).unwrap_or_default() } else { vec![] };
        let sessions = if admin { remote.get("/auth/admin/sessions") } else { remote.get("/auth/account/profile").map(|p| p["sessions"].clone()) }.ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
        let host = if admin { remote.get("/auth/admin/host").unwrap_or(Value::Null) } else { Value::Null };
        let store = me["modules"].as_array().is_some_and(|m| m.iter().any(|x| x == "store"));
        let cwd = data.lock().unwrap().cwd.clone();
        let files = if store && !cwd.is_empty() {
            remote.get(&format!("/store/api/files/list?path={}", query_value(&cwd))).ok().and_then(|v| v["entries"].as_array().cloned())
        } else {
            Some(vec![])
        };
        let shares = if store { remote.get("/store/api/shares").ok().and_then(|v| v.as_array().cloned()).unwrap_or_default() } else { vec![] };
        let mut d = data.lock().unwrap();
        // The folder may have changed while this was asked: only keep an answer for the one shown.
        if let Some(files) = files.filter(|_| d.cwd == cwd) {
            d.files = files.into_iter().filter(|e| e["type"] == "file" || e["type"] == "directory").collect();
        }
        d.shares = shares;
        if !me.is_null() {
            d.me = me;
        }
        if !card.is_null() {
            d.card = card;
        }
        d.blinds = blinds;
        d.users = users;
        d.sessions = sessions;
        // While the host restarts after an update it doesn't answer: keep what was there.
        if !host.is_null() {
            d.host = host;
        }
        d.loaded = true;
    };
    refresh(&data);
    let mut last = Instant::now();
    loop {
        match jobs.recv_timeout(Duration::from_millis(400)) {
            Ok(job) => {
                let said = match job {
                    Job::List(path) => {
                        let mut d = data.lock().unwrap();
                        d.cwd = path;
                        d.files.clear();
                        Ok((String::new(), DIM))
                    }
                    Job::Download(path, name) => {
                        let dir = super::sync::downloads_dir();
                        let dest = dir.join(&name);
                        std::fs::create_dir_all(&dir)
                            .map_err(anyhow::Error::from)
                            .and_then(|_| remote.download_to(&format!("/store/api/files/download?path={}", query_value(&path)), &dest))
                            .map(|_| (format!("Downloaded to {}", sync::pretty(&dest)), GREEN))
                    }
                    Job::Delete(path) => remote.send("DELETE", &format!("/store/api/files?path={}", query_value(&path)), None).map(|_| ("Deleted.".to_string(), GREEN)),
                    Job::Mkdir(path) => remote.send("POST", "/store/api/files/mkdir", Some(serde_json::json!({ "path": path }))).map(|_| ("Folder made.".to_string(), GREEN)),
                    Job::Rename(path, name) => remote
                        .send("POST", "/store/api/files/rename", Some(serde_json::json!({ "path": path, "new_name": name })))
                        .map(|_| ("Renamed.".to_string(), GREEN)),
                    Job::Upload(dir, file) => remote
                        .upload_file(&format!("/store/api/files/upload?path={}", query_value(&dir)), "files", &file)
                        .map(|_| (format!("Sent {}", file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()), GREEN)),
                    Job::Share(path) => remote
                        .send("POST", "/store/api/shares", Some(serde_json::json!({ "location": "external", "path": path })))
                        .map(|v| {
                            let url = full_url(&remote.base, v["url"].as_str().unwrap_or(""));
                            let copied = copy_to_clipboard(&url);
                            (format!("{url}{}", if copied { "  (copied)" } else { "" }), GREEN)
                        }),
                    Job::Unshare(token) => remote.send("DELETE", &format!("/store/api/shares/{token}"), None).map(|_| ("Link revoked: it doesn't open any more.".to_string(), GREEN)),
                    Job::SyncAdd(_) if cfg!(target_os = "macos") => Ok((sync::MAC_FINDER.to_string(), ACCENT)),
                    Job::SyncAdd(folder) => {
                        let folder = if folder.trim().is_empty() { None } else { Some(std::path::PathBuf::from(folder.trim())) };
                        data.lock().unwrap().message = Some(("Syncing for the first time…".into(), ACCENT, Instant::now()));
                        let network = data.lock().unwrap().network.clone();
                        sync::new_pair(&remote, &network, folder, None)
                            .and_then(|pair| sync::start_folder(pair, &remote).map(|said| (said, GREEN)))
                    }
                    Job::SyncNow => {
                        let results = sync::sync_all_now();
                        let failed: Vec<String> = results.iter().filter_map(|(p, r)| r.as_ref().err().map(|e| format!("{}: {e:#}", sync::pretty(&p.local)))).collect();
                        if failed.is_empty() { Ok(("Every folder is up to date.".to_string(), GREEN)) } else { Err(anyhow::anyhow!(failed.join("; "))) }
                    }
                    Job::SyncPause(paused) => sync::pause_all(paused).map(|_| ((if paused { "Paused." } else { "Syncing again." }).to_string(), GREEN)),
                    Job::SyncRemove(id) => sync::drop_pair(&id).map(|p| (format!("{} isn't synced any more (its files stay).", sync::pretty(&p.local)), GREEN)),
                    Job::SyncMode(folder, on_demand) => {
                        data.lock().unwrap().message = Some((if on_demand { "Making it on demand…".into() } else { "Making it a full copy…".into() }, ACCENT, Instant::now()));
                        sync::change_mode(&folder, on_demand, false).map(|said| (said, GREEN))
                    }
                    Job::SyncFree(folder) => {
                        let before = sync::load_pairs().iter().find(|p| p.local == folder).map(sync::cached_here).unwrap_or(0);
                        sync::set_kept(&folder, false).map(|shown| {
                            let after = sync::load_pairs().iter().find(|p| p.local == folder).map(sync::cached_here).unwrap_or(0);
                            (format!("{shown}: {} given back", sync::human(before.saturating_sub(after))), GREEN)
                        })
                    }
                    Job::SyncKeep(path, keep) => sync::set_kept(&path, keep).map(|shown| (if keep { format!("{shown} is kept here from now on.") } else { format!("{shown} is fetched when opened again.") }, GREEN)),
                    Job::Blind(id, cmd) => remote.send("POST", &format!("/home/api/commands/{id}/{cmd}"), None).map(|_| (format!("{id}: {cmd}"), GREEN)),
                    Job::Revoke(id, all) => remote
                        .send("DELETE", &if all { format!("/auth/admin/sessions/{id}") } else { format!("/auth/account/session/{id}") }, None)
                        .map(|_| (format!("Session {id} closed."), GREEN)),
                    Job::ToggleAdmin(id, email) => remote
                        .send("PATCH", &format!("/auth/admin/users/{id}"), None)
                        .map(|v| (format!("{email} is {}.", if v["is_admin"] == true { "an admin now" } else { "no longer an admin" }), GREEN)),
                    Job::HostCheck => remote.send("POST", "/auth/admin/host/check", None).map(|_| ("Looking for a newer TPCloud…".to_string(), ACCENT)),
                    Job::HostUpdate => remote.send("POST", "/auth/admin/host/update", None).map(|_| ("Updating the host: it restarts by itself.".to_string(), ACCENT)),
                };
                let (text, color) = match said {
                    Ok(m) => m,
                    Err(e) => (format!("{e:#}"), ORANGE),
                };
                let quick = text.is_empty();
                if !quick {
                    data.lock().unwrap().message = Some((text, color, Instant::now()));
                    std::thread::sleep(Duration::from_millis(600));
                }
                refresh(&data);
                last = Instant::now();
            }
            Err(mpsc::RecvTimeoutError::Timeout) if last.elapsed() > Duration::from_secs(3) => {
                refresh(&data);
                last = Instant::now();
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
            Err(_) => {}
        }
    }
}

fn current(app: &App) -> Section {
    app.sections[app.menu.selected().unwrap_or(0)]
}

/// What Files shows: the two roots, or the folder's entries, each with its full path.
fn file_rows(d: &Data) -> Vec<Value> {
    if d.cwd.is_empty() {
        let email = d.me["email"].as_str().unwrap_or("");
        return vec![
            serde_json::json!({"name": "Your files", "type": "directory", "path": format!("users/{email}")}),
            serde_json::json!({"name": "Shared", "type": "directory", "path": "shared"}),
        ];
    }
    d.files
        .iter()
        .map(|e| {
            let mut e = e.clone();
            e["path"] = Value::String(format!("{}/{}", d.cwd, e["name"].as_str().unwrap_or("")));
            e
        })
        .collect()
}

/// Where a file in the cloud shows on this computer, inside a folder on demand, and whether
/// it's kept here for good.
fn local_of(remote: &str) -> Option<(std::path::PathBuf, bool)> {
    sync::load_pairs().into_iter().filter(|p| p.on_demand).find_map(|p| {
        let rel = if remote == p.remote { String::new() } else { remote.strip_prefix(&format!("{}/", p.remote))?.to_string() };
        let kept = p.pinned.iter().any(|k| k.is_empty() || rel == *k || rel.starts_with(&format!("{k}/")));
        Some((if rel.is_empty() { p.local.clone() } else { p.local.join(&rel) }, kept))
    })
}

/// Up a folder from Files' current one ("" from a root).
fn parent_folder(d: &Data) -> String {
    let email = d.me["email"].as_str().unwrap_or("");
    if d.cwd == "shared" || d.cwd == format!("users/{email}") {
        return String::new();
    }
    d.cwd.rsplit_once('/').map(|(p, _)| p.to_string()).unwrap_or_default()
}

fn items_len(app: &App) -> usize {
    let d = app.data.lock().unwrap();
    match current(app) {
        Section::Files => file_rows(&d).len(),
        Section::Sync => sync::load_pairs().len(),
        Section::Shares => d.shares.len(),
        Section::Home => d.blinds.len(),
        Section::Users => d.users.len(),
        Section::Sessions => d.sessions.len(),
        _ => 0,
    }
}

fn say(app: &App, text: &str, color: Color) {
    app.data.lock().unwrap().message = Some((text.to_string(), color, Instant::now()));
}

/// A key while something is being typed or confirmed in the footer.
fn handle_prompt(app: &mut App, key: KeyCode) {
    let Some((prompt, mut typed)) = app.prompt.take() else { return };
    let confirm = matches!(prompt, Prompt::Delete(..) | Prompt::Revoke(_) | Prompt::Unsync(..));
    if confirm {
        if matches!(key, KeyCode::Char('y') | KeyCode::Char('Y')) {
            let _ = app.jobs.send(match prompt {
                Prompt::Delete(path, _) => Job::Delete(path),
                Prompt::Revoke(token) => Job::Unshare(token),
                Prompt::Unsync(id, _) => Job::SyncRemove(id),
                _ => unreachable!(),
            });
        }
        return;
    }
    match key {
        KeyCode::Esc => return,
        KeyCode::Backspace => {
            typed.pop();
        }
        KeyCode::Char(c) => typed.push(c),
        KeyCode::Enter => {
            let cwd = app.data.lock().unwrap().cwd.clone();
            let text = typed.trim().to_string();
            let job = match prompt {
                Prompt::Mkdir if !text.is_empty() => Some(Job::Mkdir(format!("{cwd}/{text}"))),
                Prompt::Rename(path) if !text.is_empty() => Some(Job::Rename(path, text)),
                Prompt::Upload if !text.is_empty() => {
                    let file = std::path::PathBuf::from(match text.strip_prefix("~/") {
                        Some(rest) => format!("{}/{rest}", std::env::var("HOME").unwrap_or_default()),
                        None => text,
                    });
                    if file.is_file() {
                        Some(Job::Upload(cwd, file))
                    } else {
                        say(app, "That isn't a file here.", ORANGE);
                        None
                    }
                }
                Prompt::AddSync => Some(Job::SyncAdd(text)),
                _ => None,
            };
            if let Some(job) = job {
                let _ = app.jobs.send(job);
            }
            return;
        }
        _ => {}
    }
    app.prompt = Some((prompt, typed));
}

/// false: quit.
fn handle(app: &mut App, key: KeyCode) -> bool {
    if app.prompt.is_some() {
        handle_prompt(app, key);
        return true;
    }
    if matches!(key, KeyCode::Char('q')) {
        return false;
    }
    if !app.focused {
        let n = app.sections.len();
        let i = app.menu.selected().unwrap_or(0);
        match key {
            KeyCode::Up | KeyCode::Char('k') => app.menu.select(Some((i + n - 1) % n)),
            KeyCode::Down | KeyCode::Char('j') => app.menu.select(Some((i + 1) % n)),
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') if current(app) != Section::Overview => {
                app.focused = true;
                app.item.select(Some(0));
            }
            KeyCode::Esc => return false,
            _ => {}
        }
        return true;
    }
    let n = items_len(app).max(1);
    let i = app.item.selected().unwrap_or(0).min(n - 1);
    let selected = |app: &App| -> Value {
        let d = app.data.lock().unwrap();
        match current(app) {
            Section::Files => file_rows(&d).get(i).cloned(),
            Section::Shares => d.shares.get(i).cloned(),
            Section::Home => d.blinds.get(i).cloned(),
            Section::Users => d.users.get(i).cloned(),
            Section::Sessions => d.sessions.get(i).cloned(),
            _ => None,
        }
        .unwrap_or(Value::Null)
    };
    let section = current(app);
    // Files: left goes up a folder before it leaves the section.
    if section == Section::Files && matches!(key, KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace) {
        let (cwd, parent) = {
            let d = app.data.lock().unwrap();
            (d.cwd.clone(), parent_folder(&d))
        };
        if cwd.is_empty() {
            app.focused = false;
        } else {
            let _ = app.jobs.send(Job::List(parent));
            app.item.select(Some(0));
        }
        return true;
    }
    match (section, key) {
        (_, KeyCode::Esc | KeyCode::Left | KeyCode::Char('h')) => app.focused = false,
        (_, KeyCode::Up | KeyCode::Char('k')) => app.item.select(Some((i + n - 1) % n)),
        (_, KeyCode::Down | KeyCode::Char('j')) => app.item.select(Some((i + 1) % n)),
        (Section::Files, KeyCode::Enter | KeyCode::Right | KeyCode::Char('l')) => {
            let e = selected(app);
            let path = e["path"].as_str().unwrap_or("").to_string();
            if e["type"] == "directory" {
                let _ = app.jobs.send(Job::List(path));
                app.item.select(Some(0));
            } else if !path.is_empty() {
                let _ = app.jobs.send(Job::Download(path, e["name"].as_str().unwrap_or("file").to_string()));
            }
        }
        (Section::Files, KeyCode::Char('d')) => {
            let e = selected(app);
            if e["type"] == "file" {
                let _ = app.jobs.send(Job::Download(e["path"].as_str().unwrap_or("").to_string(), e["name"].as_str().unwrap_or("file").to_string()));
            } else {
                say(app, "Folders: sync them (Sync → a) or download them from the web.", ORANGE);
            }
        }
        (Section::Files, KeyCode::Char(c @ ('x' | 'r' | 's'))) => {
            let in_root = app.data.lock().unwrap().cwd.is_empty();
            let e = selected(app);
            let (path, name) = (e["path"].as_str().unwrap_or("").to_string(), e["name"].as_str().unwrap_or("").to_string());
            if in_root || path.is_empty() {
                say(app, "Open a folder first.", ORANGE);
            } else {
                match c {
                    'x' => app.prompt = Some((Prompt::Delete(path, name), String::new())),
                    'r' => app.prompt = Some((Prompt::Rename(path), name)),
                    _ => {
                        let _ = app.jobs.send(Job::Share(path));
                    }
                }
            }
        }
        (Section::Files, KeyCode::Char('o')) => {
            let path = selected(app)["path"].as_str().unwrap_or("").to_string();
            match local_of(&path) {
                Some((local, kept)) => {
                    let _ = app.jobs.send(Job::SyncKeep(local, !kept));
                }
                None => say(app, "Only inside a folder on demand (Sync → a, then m).", ORANGE),
            }
        }
        (Section::Files, KeyCode::Char(c @ ('n' | 'u'))) => {
            if app.data.lock().unwrap().cwd.is_empty() {
                say(app, "Open a folder first.", ORANGE);
            } else {
                app.prompt = Some((if c == 'n' { Prompt::Mkdir } else { Prompt::Upload }, String::new()));
            }
        }
        (Section::Sync, KeyCode::Char('a')) => app.prompt = Some((Prompt::AddSync, "~/TPCloud".into())),
        (Section::Sync, KeyCode::Char('n')) => {
            say(app, "Syncing every folder…", ACCENT);
            let _ = app.jobs.send(Job::SyncNow);
        }
        (Section::Sync, KeyCode::Char('p')) => {
            let paused = sync::load_pairs().iter().any(|p| p.paused);
            let _ = app.jobs.send(Job::SyncPause(!paused));
        }
        (Section::Sync, KeyCode::Char('m')) => {
            if let Some(p) = sync::load_pairs().get(i) {
                if !p.on_demand && !sync::demand_possible() {
                    say(app, "This computer can't mount folders on demand.", ORANGE);
                } else {
                    let _ = app.jobs.send(Job::SyncMode(p.local.clone(), !p.on_demand));
                }
            }
        }
        (Section::Sync, KeyCode::Char('f')) => {
            if let Some(p) = sync::load_pairs().get(i) {
                if p.on_demand {
                    let _ = app.jobs.send(Job::SyncFree(p.local.clone()));
                } else {
                    say(app, "A full copy keeps everything here: m makes it on demand.", ORANGE);
                }
            }
        }
        (Section::Sync, KeyCode::Char('x')) => {
            if let Some(p) = sync::load_pairs().get(i) {
                app.prompt = Some((Prompt::Unsync(p.id.clone(), sync::pretty(&p.local)), String::new()));
            }
        }
        (Section::Shares, KeyCode::Char('x')) => {
            if let Some(token) = selected(app)["token"].as_str() {
                app.prompt = Some((Prompt::Revoke(token.to_string()), String::new()));
            }
        }
        (Section::Shares, KeyCode::Char('c') | KeyCode::Enter) => {
            if let Some(url) = selected(app)["url"].as_str() {
                let url = &full_url(&app.data.lock().unwrap().network["last_address"].as_str().unwrap_or("").to_string(), url);
                let copied = copy_to_clipboard(url);
                say(app, &format!("{url}{}", if copied { "  (copied)" } else { "" }), GREEN);
            }
        }
        (Section::Home, KeyCode::Char(c @ ('u' | 'd' | 's'))) => {
            if let Some(id) = selected(app)["id"].as_str() {
                let cmd = match c { 'u' => "up", 'd' => "down", _ => "stop" };
                let _ = app.jobs.send(Job::Blind(id.to_string(), cmd.into()));
            }
        }
        (Section::Home, KeyCode::Char(c)) if c.is_ascii_digit() => {
            if let Some(id) = selected(app)["id"].as_str() {
                let pos = c.to_digit(10).unwrap() * 10;
                let _ = app.jobs.send(Job::Blind(id.to_string(), format!("set/{}", if c == '0' { 0 } else { pos })));
            }
        }
        (Section::Users, KeyCode::Char('a')) => {
            let u = selected(app);
            if let (Some(id), Some(email)) = (u["id"].as_i64(), u["email"].as_str()) {
                let _ = app.jobs.send(Job::ToggleAdmin(id, email.to_string()));
            }
        }
        (Section::Sessions, KeyCode::Char('r')) => {
            let s = selected(app);
            let all = app.data.lock().unwrap().me["is_admin"] == true;
            if s["current"] == true {
                app.data.lock().unwrap().message = Some(("That's this one: `tpcloud logout` instead.".into(), ORANGE, Instant::now()));
            } else if let Some(id) = s["id"].as_i64() {
                let _ = app.jobs.send(Job::Revoke(id, all));
            }
        }
        (Section::Server, KeyCode::Char('c')) => {
            let _ = app.jobs.send(Job::HostCheck);
        }
        (Section::Server, KeyCode::Char('u')) => {
            let _ = app.jobs.send(Job::HostUpdate);
        }
        _ => {}
    }
    true
}

fn draw(f: &mut Frame, app: &mut App, network: &Value) {
    let rows = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(3), Constraint::Min(5), Constraint::Length(1)]).split(f.area());
    let cols = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Length(20), Constraint::Min(30)]).split(rows[1]);
    let d = app.data.lock().unwrap();
    // Header.
    let header = Line::from(vec![
        Span::styled(" TPCloud ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(network["name"].as_str().unwrap_or("").to_string(), Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(format!("  {}", d.card["version"].as_str().unwrap_or("")), Style::default().fg(DIM)),
        Span::styled(format!("  {}", d.me["email"].as_str().unwrap_or("")), Style::default().fg(DIM)),
    ]);
    f.render_widget(Paragraph::new(header).block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(DIM))), rows[0]);
    // Menu.
    let items: Vec<ListItem> = app.sections.iter().map(|s| ListItem::new(format!(" {}", s.title()))).collect();
    let menu = List::new(items).block(frame("Menu", !app.focused)).highlight_style(Style::default().fg(Color::Black).bg(ACCENT).add_modifier(Modifier::BOLD));
    f.render_stateful_widget(menu, cols[0], &mut app.menu);
    // Content.
    let section = current(app);
    let area = cols[1];
    match section {
        Section::Overview => {
            let modules: Vec<&str> = d.me["modules"].as_array().into_iter().flatten().filter_map(|m| m.as_str()).collect();
            let mut lines = vec![
                kv("Network ", network["name"].as_str().unwrap_or("")),
                kv("Address ", network["last_address"].as_str().unwrap_or("")),
                kv("Account ", &format!("{}{}", d.me["email"].as_str().unwrap_or(""), if d.me["is_admin"] == true { "  (admin)" } else { "" })),
                kv("Modules ", &if modules.is_empty() { "none".into() } else { modules.join(", ") }),
                kv("Version ", d.card["version"].as_str().unwrap_or("")),
            ];
            if modules.contains(&"store") {
                let pairs = sync::load_pairs();
                let synced = match pairs.len() {
                    0 => "no folder yet (Sync → a)".to_string(),
                    _ => pairs.iter().map(|p| sync::pretty(&p.local)).collect::<Vec<_>>().join(", "),
                };
                lines.push(kv("Synced  ", &synced));
            }
            if d.host["available"] == true {
                lines.push(Line::styled(format!("TPCloud {} is out: Server → u", d.host["latest"].as_str().unwrap_or("")), Style::default().fg(ACCENT)));
            }
            f.render_widget(Paragraph::new(lines).block(frame("Overview", false)).wrap(Wrap { trim: true }), area);
        }
        Section::Files => {
            let rows = file_rows(&d);
            let title = if d.cwd.is_empty() { "Files".to_string() } else { format!("Files · {}", sync::remote_label(&d.cwd)) };
            list(f, area, &title, app.focused, &mut app.item, rows.iter().map(|e| {
                let dir = e["type"] == "directory";
                let size = e["size"].as_u64().map(human).unwrap_or_default();
                let kept = local_of(e["path"].as_str().unwrap_or("")).is_some_and(|(_, k)| k);
                Line::from(vec![
                    Span::styled(if kept { " ↓ " } else if dir { " ▸ " } else { "   " }, Style::default().fg(if kept { GREEN } else { ACCENT })),
                    Span::styled(format!("{:<40}", fit(e["name"].as_str().unwrap_or(""), 39)), if dir { Style::default().add_modifier(Modifier::BOLD) } else { Style::default() }),
                    Span::styled(format!("{size:>9}"), Style::default().fg(DIM)),
                ])
            }).collect());
        }
        Section::Sync => {
            let pairs = sync::load_pairs();
            let running = sync::service_running();
            list(f, area, if running || pairs.is_empty() { "Synced folders" } else { "Synced folders · service stopped" }, app.focused, &mut app.item, pairs.iter().map(|p| {
                let st = sync::status_of(&p.id);
                let (dot, color, state) = if p.paused {
                    ("○", DIM, "paused".to_string())
                } else if p.on_demand {
                    let kept = if p.pinned.is_empty() { String::new() } else { format!(" · {} kept", p.pinned.len()) };
                    ("●", if st["ok"] == false { ORANGE } else { GREEN }, format!("on demand · {} here{kept}", sync::human(st["cached"].as_u64().unwrap_or(0))))
                } else if let Some(busy) = sync::progress_of(&st) {
                    ("●", ACCENT, busy)
                } else if st["ok"] == false {
                    ("●", ORANGE, st["error"].as_str().unwrap_or("failed").to_string())
                } else if let Some(at) = st["at"].as_i64() {
                    ("●", GREEN, format!("up to date · {}", sync::ago(at)))
                } else {
                    ("●", ACCENT, "waiting for its first sync".to_string())
                };
                Line::from(vec![
                    Span::styled(format!(" {dot} "), Style::default().fg(color)),
                    Span::styled(format!("{:<22}", sync::pretty(&p.local)), Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(format!("⇄ {} · {:<14}", p.network_name, sync::remote_label(&p.remote)), Style::default().fg(DIM)),
                    Span::styled(format!("  {state}"), Style::default().fg(color)),
                ])
            }).collect());
        }
        Section::Shares => list(f, area, "Share links", app.focused, &mut app.item, d.shares.iter().map(|sh| {
            let mut flags = Vec::new();
            if sh["has_password"] == true {
                flags.push("password");
            }
            if sh["editable"] == true {
                flags.push("editable");
            }
            Line::from(vec![
                Span::styled(if sh["is_file"] == true { "   " } else { " ▸ " }, Style::default().fg(ACCENT)),
                Span::raw(format!("{:<36}", fit(&sync::remote_label(sh["path"].as_str().unwrap_or("")), 35))),
                Span::styled(full_url(d.network["last_address"].as_str().unwrap_or(""), sh["url"].as_str().unwrap_or("")), Style::default().fg(ACCENT)),
                Span::styled(if flags.is_empty() { String::new() } else { format!("  {}", flags.join(" · ")) }, Style::default().fg(DIM)),
            ])
        }).collect()),
        Section::Home => list(f, area, "Home", app.focused, &mut app.item, d.blinds.iter().map(|b| {
            let pos = b["state"]["position"].as_i64().map(|p| format!("{p:>3}%")).unwrap_or_else(|| "  —".into());
            let online = b["connection"]["online"] == true;
            Line::from(vec![
                Span::styled(if online { " ● " } else { " ○ " }, Style::default().fg(if online { GREEN } else { DIM })),
                Span::raw(format!("{:<8}", b["id"].as_str().unwrap_or(""))),
                Span::raw(format!("{:<22}", b["name"].as_str().unwrap_or(""))),
                Span::styled(pos, Style::default().fg(DIM)),
            ])
        }).collect()),
        Section::Users => list(f, area, "Accounts", app.focused, &mut app.item, d.users.iter().map(|u| {
            Line::from(vec![
                Span::styled(if u["is_admin"] == true { " ● " } else { " ○ " }, Style::default().fg(if u["is_admin"] == true { ACCENT } else { DIM })),
                Span::raw(format!("{:<34}", u["email"].as_str().unwrap_or(""))),
                Span::styled(format!("{}{}", if u["is_admin"] == true { "admin  " } else { "" }, if u["totp_enabled"] == true { "2FA" } else { "" }), Style::default().fg(DIM)),
            ])
        }).collect()),
        Section::Sessions => list(f, area, "Sessions", app.focused, &mut app.item, d.sessions.iter().map(|s| {
            let who = s["user_email"].as_str().map(|e| format!("{e}  ")).unwrap_or_default();
            Line::from(vec![
                Span::raw(format!(" {who}{}", s["device"]["label"].as_str().or(s["device_info"].as_str()).unwrap_or("?"))),
                Span::styled(format!("  {}", s["last_ip"].as_str().unwrap_or("")), Style::default().fg(DIM)),
                Span::styled(if s["current"] == true { "  this one" } else { "" }, Style::default().fg(ACCENT)),
            ])
        }).collect()),
        Section::Server => {
            let h = &d.host;
            let state = match h["update"]["state"].as_str() {
                Some(s @ ("checking" | "downloading" | "installing")) => Line::styled(format!("Updating ({s})…"), Style::default().fg(ACCENT)),
                Some("failed") => Line::styled(format!("The last update failed: {}", h["update"]["detail"].as_str().unwrap_or("")), Style::default().fg(ORANGE)),
                _ if h["available"] == true => Line::styled(format!("TPCloud {} is out: press u to update", h["latest"].as_str().unwrap_or("")), Style::default().fg(ACCENT)),
                _ => Line::styled("This is the latest version.", Style::default().fg(DIM)),
            };
            let lines = vec![kv("Version ", h["version"].as_str().unwrap_or("")), kv("Newest  ", h["latest"].as_str().unwrap_or("—")), Line::raw(""), state];
            f.render_widget(Paragraph::new(lines).block(frame("Server", app.focused)), area);
        }
    }
    // Footer: the last action, or the keys.
    let hint = match (app.focused, section) {
        (false, _) => "↑↓ jk choose  ⏎ l open  q quit",
        (true, Section::Files) => "jk move  l open  h up  d download  u upload  n folder  r rename  s share  o keep here  x delete",
        (true, Section::Sync) => "jk folder  a add  m on demand/full copy  f free up space  n sync now  p pause  x stop syncing  ← h",
        (true, Section::Shares) => "jk link  c copy  x revoke  ← h back   (new ones: Files → s)",
        (true, Section::Home) => "↑↓ blind  u up  d down  s stop  0–9 position  ← h back",
        (true, Section::Users) => "↑↓ account  a admin on/off  ← h back   (add/remove: tpcloud users)",
        (true, Section::Sessions) => "↑↓ session  r close it  ← h back",
        (true, Section::Server) => "c look for updates  u update the host  ← h back",
        (true, Section::Overview) => "← h back",
    };
    let prompt = app.prompt.as_ref().map(|(p, typed)| match p {
        Prompt::Mkdir => format!(" New folder's name: {typed}▏  (⏎ make it, esc cancel)"),
        Prompt::Rename(_) => format!(" New name: {typed}▏  (⏎ rename, esc cancel)"),
        Prompt::Upload => format!(" File to send here: {typed}▏  (⏎ send, esc cancel)"),
        Prompt::AddSync => format!(" Folder to show your files in: {typed}▏  (⏎ start, esc cancel)"),
        Prompt::Delete(_, name) => format!(" Delete {name} for good? y / anything else keeps it"),
        Prompt::Revoke(_) => " Revoke this link (it stops opening)? y / anything else keeps it".to_string(),
        Prompt::Unsync(_, folder) => format!(" Stop syncing {folder} (its files stay)? y / anything else keeps it"),
    });
    let footer = match (prompt, d.message.as_ref().filter(|m| m.2.elapsed() < Duration::from_secs(5))) {
        (Some(text), _) => Line::from(Span::styled(text, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
        (None, m) => match m {
            Some((text, color, _)) => Line::from(Span::styled(format!(" {text}"), Style::default().fg(*color))),
            None => Line::from(Span::styled(format!(" {hint}"), Style::default().fg(DIM))),
        },
    };
    f.render_widget(Paragraph::new(footer), rows[2]);
}

/// Puts a link on the clipboard, with whichever tool this computer has.
fn copy_to_clipboard(text: &str) -> bool {
    let tools: &[(&str, &[&str])] = &[("pbcopy", &[]), ("wl-copy", &[]), ("xclip", &["-selection", "clipboard"]), ("xsel", &["--clipboard", "--input"])];
    tools.iter().any(|(tool, args)| {
        std::process::Command::new(tool)
            .args(*args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .and_then(|mut child| {
                if let Some(mut stdin) = child.stdin.take() {
                    std::io::Write::write_all(&mut stdin, text.as_bytes())?;
                }
                child.wait()
            })
            .is_ok_and(|s| s.success())
    })
}

fn frame(title: &str, focused: bool) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if focused { ACCENT } else { DIM }))
        .title(Span::styled(format!(" {title} "), Style::default().add_modifier(Modifier::BOLD)))
}

/// A server without its public address set answers share links as a path: made whole here.
fn full_url(base: &str, url: &str) -> String {
    if url.starts_with('/') {
        format!("{}{url}", base.trim_end_matches('/'))
    } else {
        url.to_string()
    }
}

/// At most `width` characters, «…» in the middle of what doesn't fit.
fn fit(text: &str, width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= width {
        return text.to_string();
    }
    let keep = width.saturating_sub(1);
    let head: String = chars[..keep / 2 + keep % 2].iter().collect();
    let tail: String = chars[chars.len() - keep / 2..].iter().collect();
    format!("{head}…{tail}")
}

fn human(bytes: u64) -> String {
    match bytes {
        0..=1023 => format!("{bytes} B"),
        1024..=1_048_575 => format!("{:.0} KB", bytes as f64 / 1024.0),
        1_048_576..=1_073_741_823 => format!("{:.1} MB", bytes as f64 / 1_048_576.0),
        _ => format!("{:.1} GB", bytes as f64 / 1_073_741_824.0),
    }
}

fn kv<'a>(k: &'a str, v: &str) -> Line<'a> {
    Line::from(vec![Span::styled(k, Style::default().fg(DIM)), Span::raw(v.to_string())])
}

fn list(f: &mut Frame, area: Rect, title: &str, focused: bool, state: &mut ListState, lines: Vec<Line<'static>>) {
    let empty = lines.is_empty();
    let items: Vec<ListItem> = if empty { vec![ListItem::new(Line::styled(" Nothing here yet.", Style::default().fg(DIM)))] } else { lines.into_iter().map(ListItem::new).collect() };
    let widget = List::new(items).block(frame(title, focused)).highlight_style(if focused && !empty { Style::default().fg(Color::Black).bg(ACCENT) } else { Style::default() });
    f.render_stateful_widget(widget, area, state);
}
