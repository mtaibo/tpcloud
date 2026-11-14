//! `tpcloud` on its own: a terminal app with a menu (status, the house, accounts, modules, logs
//! and the guided setup), refreshed while it's open.

use crate::{home, install, logs, setup, status, users};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use std::time::{Duration, Instant};

const ACCENT: Color = Color::Rgb(106, 175, 255);
const GREEN: Color = Color::Rgb(120, 200, 120);
const ORANGE: Color = Color::Rgb(255, 170, 90);
const DIM: Color = Color::DarkGray;

#[derive(Clone, Copy, PartialEq)]
enum Section {
    Status,
    Home,
    Users,
    Modules,
    Logs,
    Setup,
}

const SECTIONS: [(Section, &str); 6] = [
    (Section::Status, "Status"),
    (Section::Home, "Home"),
    (Section::Users, "Accounts"),
    (Section::Modules, "Modules"),
    (Section::Logs, "Logs"),
    (Section::Setup, "Setup"),
];

struct App {
    menu: ListState,
    /// Keys go to the section on the right instead of the menu.
    inside: bool,
    status: status::Status,
    blinds: Vec<serde_json::Value>,
    blind: ListState,
    users: Vec<users::User>,
    module: usize,
    log_service: usize,
    message: Option<(String, Color)>,
    refreshed: Instant,
    quit: bool,
    run_setup: bool,
}

impl App {
    fn section(&self) -> Section {
        SECTIONS[self.menu.selected().unwrap_or(0)].0
    }

    fn refresh(&mut self) {
        self.status = status::read();
        match self.section() {
            Section::Home if install::home_enabled() => {
                match home::Api::connect().and_then(|api| home::blinds(&api)) {
                    Ok(list) => self.blinds = list,
                    Err(err) => self.say(&format!("{err:#}"), ORANGE),
                }
                if self.blind.selected().is_none() && !self.blinds.is_empty() {
                    self.blind.select(Some(0));
                }
            }
            Section::Users => {
                if let Ok(list) = users::list() {
                    self.users = list;
                }
            }
            _ => {}
        }
        self.refreshed = Instant::now();
    }

    fn say(&mut self, text: &str, color: Color) {
        self.message = Some((text.to_string(), color));
    }
}

pub fn run() -> Result<()> {
    let mut app = App {
        menu: ListState::default().with_selected(Some(0)),
        inside: false,
        status: status::read(),
        blinds: Vec::new(),
        blind: ListState::default(),
        users: Vec::new(),
        module: 0,
        log_service: 0,
        message: None,
        refreshed: Instant::now() - Duration::from_secs(10),
        quit: false,
        run_setup: false,
    };
    loop {
        let mut terminal = ratatui::init();
        let result = event_loop(&mut terminal, &mut app);
        ratatui::restore();
        result?;
        if app.run_setup {
            app.run_setup = false;
            if let Err(err) = setup::run() {
                crate::ui::warn(&format!("{err:#}"));
            }
            println!("\nPress enter to go back to TPCloud.");
            let mut line = String::new();
            let _ = std::io::stdin().read_line(&mut line);
            app.refreshed = Instant::now() - Duration::from_secs(10);
            continue;
        }
        if app.quit {
            return Ok(());
        }
    }
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    while !app.quit && !app.run_setup {
        if app.refreshed.elapsed() > Duration::from_secs(2) {
            app.refresh();
        }
        terminal.draw(|f| draw(f, app))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        handle_key(app, key.code);
    }
    Ok(())
}

fn handle_key(app: &mut App, code: KeyCode) {
    if matches!(code, KeyCode::Char('q')) {
        app.quit = true;
        return;
    }
    if !app.inside {
        let i = app.menu.selected().unwrap_or(0);
        match code {
            KeyCode::Up | KeyCode::Char('k') => app.menu.select(Some(i.saturating_sub(1))),
            KeyCode::Down | KeyCode::Char('j') => app.menu.select(Some((i + 1).min(SECTIONS.len() - 1))),
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                if app.section() == Section::Setup {
                    app.run_setup = true;
                } else if app.section() != Section::Status {
                    app.inside = true;
                }
            }
            KeyCode::Esc => app.quit = true,
            _ => return,
        }
        app.message = None;
        app.refreshed = Instant::now() - Duration::from_secs(10);
        return;
    }
    if matches!(code, KeyCode::Esc | KeyCode::Left) {
        app.inside = false;
        return;
    }
    match app.section() {
        Section::Home => home_key(app, code),
        Section::Modules => match code {
            KeyCode::Up => app.module = app.module.saturating_sub(1),
            KeyCode::Down => app.module = (app.module + 1).min(1),
            KeyCode::Enter | KeyCode::Char(' ') => {
                let name = ["store", "home"][app.module];
                let on = !app.status.modules.iter().any(|m| m == name);
                if unsafe { libc::geteuid() } != 0 {
                    app.say("Changing modules needs root: open it with `sudo tpcloud`.", ORANGE);
                } else {
                    match install::set_module(name, on) {
                        Ok(()) => app.say(&format!("{name} is {}.", if on { "on" } else { "off" }), GREEN),
                        Err(err) => app.say(&format!("{err:#}"), ORANGE),
                    }
                    app.refresh();
                }
            }
            _ => {}
        },
        Section::Logs => match code {
            KeyCode::Up | KeyCode::Char('k') => app.log_service = app.log_service.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => app.log_service = (app.log_service + 1).min(logs::NAMES.len() - 1),
            _ => {}
        },
        _ => {}
    }
}

fn home_key(app: &mut App, code: KeyCode) {
    let i = app.blind.selected().unwrap_or(0);
    let action = match code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.blind.select(Some(i.saturating_sub(1)));
            return;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.blind.select(Some((i + 1).min(app.blinds.len().saturating_sub(1))));
            return;
        }
        KeyCode::Char('u') => "up".to_string(),
        KeyCode::Char('d') => "down".to_string(),
        KeyCode::Char('s') => "stop".to_string(),
        KeyCode::Char(c @ '0'..='9') => ((c as u8 - b'0') as u32 * 10).to_string(),
        _ => return,
    };
    let Some(id) = app.blinds.get(i).and_then(|b| b["id"].as_str()).map(str::to_string) else { return };
    let result = home::Api::connect().and_then(|api| home::send(&api, &id, home::parse_action(&action)?));
    match result {
        Ok((name, said)) => app.say(&format!("{name}: {said}"), GREEN),
        Err(err) => app.say(&format!("{err:#}"), ORANGE),
    }
    app.refreshed = Instant::now() - Duration::from_millis(1500);
}

fn draw(f: &mut Frame, app: &mut App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5), Constraint::Length(1)])
        .split(f.area());
    draw_header(f, app, rows[0]);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(20), Constraint::Min(30)])
        .split(rows[1]);
    draw_menu(f, app, cols[0]);
    let body = cols[1];
    match app.section() {
        Section::Status => draw_status(f, app, body),
        Section::Home => draw_home(f, app, body),
        Section::Users => draw_users(f, app, body),
        Section::Modules => draw_modules(f, app, body),
        Section::Logs => draw_logs(f, app, body),
        Section::Setup => draw_setup(f, body),
    }
    let hint = match (app.inside, app.section()) {
        (false, _) => "↑↓ choose · enter open · q quit",
        (true, Section::Home) => "↑↓ blind · u up · d down · s stop · 0–9 position (×10 %) · ← back",
        (true, Section::Modules) => "↑↓ module · enter turn on/off · ← back",
        (true, Section::Logs) => "↑↓ service · ← back",
        (true, _) => "← back · q quit",
    };
    let footer = match &app.message {
        Some((text, color)) => Line::from(Span::styled(format!(" {text}"), Style::default().fg(*color))),
        None => Line::from(Span::styled(format!(" {hint}"), Style::default().fg(DIM))),
    };
    f.render_widget(Paragraph::new(footer), rows[2]);
}

fn card(title: &str, focused: bool) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if focused { ACCENT } else { DIM }))
        .title(Span::styled(format!(" {title} "), Style::default().add_modifier(Modifier::BOLD)))
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let st = &app.status;
    let state = if st.active { Span::styled("● running", Style::default().fg(GREEN)) } else { Span::styled("○ stopped", Style::default().fg(ORANGE)) };
    let line = Line::from(vec![
        Span::styled(" TPCloud ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(st.version.clone().unwrap_or_default(), Style::default().fg(DIM)),
        Span::raw("   "),
        Span::raw(if st.url.is_empty() { "no address yet".to_string() } else { st.url.clone() }),
        Span::raw("   "),
        state,
    ]);
    f.render_widget(Paragraph::new(line).block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(DIM))), area);
}

fn draw_menu(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = SECTIONS.iter().map(|(_, name)| ListItem::new(format!(" {name}"))).collect();
    let list = List::new(items)
        .block(card("Menu", !app.inside))
        .highlight_style(Style::default().fg(Color::Black).bg(ACCENT).add_modifier(Modifier::BOLD));
    f.render_stateful_widget(list, area, &mut app.menu);
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let st = &app.status;
    let mut lines = vec![
        Line::from(vec![Span::styled("Modules  ", Style::default().fg(DIM)), Span::raw(if st.modules.is_empty() { "none".into() } else { st.modules.join(", ") })]),
        Line::from(vec![Span::styled("Files    ", Style::default().fg(DIM)), Span::raw(st.files.clone())]),
        Line::from(vec![Span::styled("Data     ", Style::default().fg(DIM)), Span::raw(crate::paths::state_dir().display().to_string())]),
        Line::raw(""),
    ];
    if !st.active {
        lines.push(Line::styled("Nothing is running: `sudo tpcloud start`.", Style::default().fg(ORANGE)));
    }
    for p in &st.processes {
        let (dot, color, detail) = if p.running {
            ("●", GREEN, format!("up {}{}", status::ago(p.since), if p.restarts > 0 { format!(" · restarted {}×", p.restarts) } else { String::new() }))
        } else {
            ("●", ORANGE, format!("down {}", p.last_exit.clone().unwrap_or_default()))
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {dot} "), Style::default().fg(color)),
            Span::raw(format!("{:<13}", p.name)),
            Span::styled(detail, Style::default().fg(DIM)),
        ]));
    }
    f.render_widget(Paragraph::new(lines).block(card("Status", false)), area);
}

fn draw_home(f: &mut Frame, app: &mut App, area: Rect) {
    if !install::home_enabled() {
        f.render_widget(Paragraph::new("Home is off. Turn it on in Modules.").block(card("Home", app.inside)), area);
        return;
    }
    let width = app.blinds.iter().map(|b| b["name"].as_str().unwrap_or("").chars().count()).max().unwrap_or(4);
    let items: Vec<ListItem> = app
        .blinds
        .iter()
        .map(|b| {
            let id = b["id"].as_str().unwrap_or("?");
            let online = b["connection"]["online"].as_bool() == Some(true);
            let position = b["state"]["position"].as_i64();
            let bar = position.map(|p| {
                let filled = (p.clamp(0, 100) as usize) / 10;
                format!("{}{} {:>3} %", "█".repeat(filled), "░".repeat(10 - filled), p)
            });
            let mut spans = vec![
                Span::styled(if online { " ● " } else { " ○ " }, Style::default().fg(if online { GREEN } else { DIM })),
                Span::styled(format!("{id:<6} "), Style::default().fg(DIM)),
                Span::raw(format!("{:<width$}  ", b["name"].as_str().unwrap_or(id))),
                Span::styled(bar.unwrap_or_else(|| "—".into()), Style::default().fg(ACCENT)),
            ];
            if b["locked"].as_bool() == Some(true) {
                spans.push(Span::styled("  locked", Style::default().fg(ORANGE)));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();
    let list = List::new(items)
        .block(card("Home · blinds", app.inside))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    f.render_stateful_widget(list, area, &mut app.blind);
}

fn draw_users(f: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = app
        .users
        .iter()
        .map(|u| {
            Line::from(vec![
                Span::styled(" ● ", Style::default().fg(if u.admin { ACCENT } else { DIM })),
                Span::raw(u.email.clone()),
                Span::styled(format!("{}{}", if u.admin { "  admin" } else { "" }, if u.totp { "  2FA" } else { "" }), Style::default().fg(DIM)),
            ])
        })
        .collect();
    lines.push(Line::raw(""));
    lines.push(Line::styled("New accounts: sudo tpcloud users add <email> [--admin]", Style::default().fg(DIM)));
    f.render_widget(Paragraph::new(lines).block(card("Accounts", app.inside)), area);
}

fn draw_modules(f: &mut Frame, app: &App, area: Rect) {
    let rows = [("store", "Files, photo backup from the app and share links"), ("home", "Blinds and lights, with tphome's switches; adds the tphome command")];
    let lines: Vec<Line> = rows
        .iter()
        .enumerate()
        .map(|(i, (name, what))| {
            let on = app.status.modules.iter().any(|m| m == name);
            let selected = app.inside && i == app.module;
            let style = if selected { Style::default().add_modifier(Modifier::REVERSED) } else { Style::default() };
            Line::from(vec![
                Span::styled(if on { " ● on  " } else { " ○ off " }, Style::default().fg(if on { GREEN } else { DIM })),
                Span::styled(format!("{name:<6}"), style.add_modifier(Modifier::BOLD)),
                Span::styled(format!("  {what}"), Style::default().fg(DIM)),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(lines).block(card("Modules", app.inside)), area);
}

fn draw_logs(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(16), Constraint::Min(20)])
        .split(area);
    let names: Vec<Line> = logs::NAMES
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let style = if i == app.log_service { Style::default().fg(ACCENT).add_modifier(Modifier::BOLD) } else { Style::default().fg(DIM) };
            Line::styled(format!(" {n}"), style)
        })
        .collect();
    f.render_widget(Paragraph::new(names).block(card("Service", app.inside)), cols[0]);
    let height = cols[1].height.saturating_sub(2) as usize;
    let lines: Vec<Line> = logs::tail(logs::NAMES[app.log_service], height).into_iter().map(Line::raw).collect();
    f.render_widget(Paragraph::new(lines).block(card(logs::NAMES[app.log_service], false)).wrap(Wrap { trim: false }), cols[1]);
}

fn draw_setup(f: &mut Frame, area: Rect) {
    let lines = vec![
        Line::raw("The guided setup: where files go, which modules run,"),
        Line::raw("how you'll reach TPCloud and the first account."),
        Line::raw(""),
        Line::styled("Enter starts it (needs root: `sudo tpcloud`).", Style::default().fg(DIM)),
    ];
    f.render_widget(Paragraph::new(lines).block(card("Setup", false)), area);
}
