//! `tpcloud` alone on a computer signed in to a network: the network in a terminal app. Overview,
//! the house (blinds), and for admins the accounts, everyone's sessions and the host's version
//! with its update. A worker thread does every request, so keys never wait for the network.

use super::net::Remote;
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
    Home,
    Users,
    Sessions,
    Server,
}

impl Section {
    fn title(self) -> &'static str {
        match self {
            Section::Overview => "Overview",
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
    message: Option<(String, Color, Instant)>,
    loaded: bool,
}

enum Job {
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
    data: Arc<Mutex<Data>>,
    jobs: mpsc::Sender<Job>,
}

pub fn run(remote: Remote, network: Value) -> Result<()> {
    let data = Arc::new(Mutex::new(Data::default()));
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
        if d.me["modules"].as_array().is_some_and(|m| m.iter().any(|x| x == "home")) {
            sections.push(Section::Home);
        }
        if d.me["is_admin"] == true {
            sections.extend([Section::Users, Section::Sessions, Section::Server]);
        } else {
            sections.push(Section::Sessions);
        }
    }
    let mut app = App { sections, menu: ListState::default().with_selected(Some(0)), focused: false, item: ListState::default().with_selected(Some(0)), data, jobs: tx };
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
        let mut d = data.lock().unwrap();
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
                data.lock().unwrap().message = Some((text, color, Instant::now()));
                std::thread::sleep(Duration::from_millis(600));
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

fn items_len(app: &App) -> usize {
    let d = app.data.lock().unwrap();
    match current(app) {
        Section::Home => d.blinds.len(),
        Section::Users => d.users.len(),
        Section::Sessions => d.sessions.len(),
        _ => 0,
    }
}

/// false: quit.
fn handle(app: &mut App, key: KeyCode) -> bool {
    if matches!(key, KeyCode::Char('q')) {
        return false;
    }
    if !app.focused {
        let n = app.sections.len();
        let i = app.menu.selected().unwrap_or(0);
        match key {
            KeyCode::Up => app.menu.select(Some((i + n - 1) % n)),
            KeyCode::Down => app.menu.select(Some((i + 1) % n)),
            KeyCode::Enter | KeyCode::Right if current(app) != Section::Overview => {
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
            Section::Home => d.blinds.get(i).cloned(),
            Section::Users => d.users.get(i).cloned(),
            Section::Sessions => d.sessions.get(i).cloned(),
            _ => None,
        }
        .unwrap_or(Value::Null)
    };
    match (current(app), key) {
        (_, KeyCode::Esc | KeyCode::Left) => app.focused = false,
        (_, KeyCode::Up) => app.item.select(Some((i + n - 1) % n)),
        (_, KeyCode::Down) => app.item.select(Some((i + 1) % n)),
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
            if d.host["available"] == true {
                lines.push(Line::styled(format!("TPCloud {} is out: Server → u", d.host["latest"].as_str().unwrap_or("")), Style::default().fg(ACCENT)));
            }
            f.render_widget(Paragraph::new(lines).block(frame("Overview", false)).wrap(Wrap { trim: true }), area);
        }
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
        (false, _) => "↑↓ choose  ⏎ open  q quit",
        (true, Section::Home) => "↑↓ blind  u up  d down  s stop  0–9 position  ← back",
        (true, Section::Users) => "↑↓ account  a admin on/off  ← back   (add/remove: tpcloud users)",
        (true, Section::Sessions) => "↑↓ session  r close it  ← back",
        (true, Section::Server) => "c look for updates  u update the host  ← back",
        (true, Section::Overview) => "← back",
    };
    let footer = match d.message.as_ref().filter(|m| m.2.elapsed() < Duration::from_secs(5)) {
        Some((text, color, _)) => Line::from(Span::styled(format!(" {text}"), Style::default().fg(*color))),
        None => Line::from(Span::styled(format!(" {hint}"), Style::default().fg(DIM))),
    };
    f.render_widget(Paragraph::new(footer), rows[2]);
}

fn frame(title: &str, focused: bool) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if focused { ACCENT } else { DIM }))
        .title(Span::styled(format!(" {title} "), Style::default().add_modifier(Modifier::BOLD)))
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
