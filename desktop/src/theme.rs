//! The look: the web apps' Apple-style system (`agents/design.md`, `web/ui/styles.css`) drawn in
//! iced, plus the person's own settings for it (like Firefox's): light/dark, accent, how
//! see-through the window and the sidebar are, text size, density, corners.

use iced::widget::{button, container, scrollable, slider, text_input};
use iced::{Background, Border, Color, Font, Shadow, Theme};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};

/// Settings → Size: every length in the app goes through `u` (iced 0.13's own `scale_factor`
/// drew SVGs and images in the wrong place, with both renderers, so it isn't used).
static UI: AtomicU32 = AtomicU32::new(0x3f80_0000); // 1.0

pub fn set_ui(size: f32) {
    UI.store(size.clamp(1.0, 2.0).to_bits(), Ordering::Relaxed);
}

pub fn ui() -> f32 {
    f32::from_bits(UI.load(Ordering::Relaxed))
}

/// A length at the chosen size.
pub fn u(points: f32) -> f32 {
    (points * ui()).round()
}

pub const INTER: Font = Font::with_name("Inter");
pub const MEDIUM: Font = Font { weight: iced::font::Weight::Medium, ..INTER };
pub const SEMIBOLD: Font = Font { weight: iced::font::Weight::Semibold, ..INTER };
pub const BOLD: Font = Font { weight: iced::font::Weight::Bold, ..INTER };

// ── Settings ──

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Mode {
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Accent {
    Blue,
    Purple,
    Pink,
    Red,
    Orange,
    Yellow,
    Green,
    Graphite,
}

impl Accent {
    pub const ALL: [Accent; 8] = [Accent::Blue, Accent::Purple, Accent::Pink, Accent::Red, Accent::Orange, Accent::Yellow, Accent::Green, Accent::Graphite];

    pub fn color(self, dark: bool) -> Color {
        let hex = match (self, dark) {
            (Accent::Blue, false) => 0x007aff,
            (Accent::Blue, true) => 0x0a84ff,
            (Accent::Purple, false) => 0xaf52de,
            (Accent::Purple, true) => 0xbf5af2,
            (Accent::Pink, false) => 0xff2d55,
            (Accent::Pink, true) => 0xff375f,
            (Accent::Red, false) => 0xff3b30,
            (Accent::Red, true) => 0xff453a,
            (Accent::Orange, false) => 0xff9500,
            (Accent::Orange, true) => 0xff9f0a,
            (Accent::Yellow, false) => 0xffcc00,
            (Accent::Yellow, true) => 0xffd60a,
            (Accent::Green, false) => 0x34c759,
            (Accent::Green, true) => 0x30d158,
            (Accent::Graphite, _) => 0x8e8e93,
        };
        rgb(hex)
    }

    #[allow(dead_code)]
    pub fn name(self) -> &'static str {
        match self {
            Accent::Blue => "Blue",
            Accent::Purple => "Purple",
            Accent::Pink => "Pink",
            Accent::Red => "Red",
            Accent::Orange => "Orange",
            Accent::Yellow => "Yellow",
            Accent::Green => "Green",
            Accent::Graphite => "Graphite",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum View {
    Icons,
    List,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub mode: Mode,
    pub accent: Accent,
    /// How opaque the window is: 1 = solid (the web's look), lower lets the desktop show through
    /// (the compositor blurs it: Hyprland, KWin).
    pub window_opacity: f32,
    /// How opaque the sidebar is over the window (the web's is 0.7).
    pub sidebar_opacity: f32,
    pub text_scale: f32,
    pub compact: bool,
    /// 1 = the web's corners; 0.4 squarer, 1.4 rounder.
    pub roundness: f32,
    pub view: View,
    /// Icon size in the icon view, in points.
    pub icon_size: f32,
    /// How big the whole app is drawn, on top of the display's own scale (1 = 100 %).
    pub size: f32,
}

impl Default for Prefs {
    fn default() -> Prefs {
        Prefs { mode: Mode::System, accent: Accent::Blue, window_opacity: 1.0, sidebar_opacity: 0.7, text_scale: 1.0, compact: false, roundness: 1.0, view: View::List, icon_size: 64.0, size: 1.0 }
    }
}

fn prefs_file() -> std::path::PathBuf {
    dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("tpcloud/app.json")
}

impl Prefs {
    pub fn load() -> Prefs {
        std::fs::read_to_string(prefs_file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        let file = prefs_file();
        if let Some(dir) = file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let tmp = file.with_extension("tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(tmp, file);
            }
        }
    }
}

/// Whether this desktop lets the app make its window see-through (Hyprland does, through its
/// own per-window opacity, which also blurs what's behind).
pub fn can_set_window_opacity() -> bool {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}

/// Makes this app's window as opaque as asked (1 = solid), by asking the compositor.
pub fn apply_window_opacity(opacity: f32) {
    if !can_set_window_opacity() {
        return;
    }
    let pid = format!("pid:{}", std::process::id());
    let value = format!("{:.2}", opacity.clamp(0.3, 1.0));
    for prop in ["alpha", "alphainactive"] {
        let _ = std::process::Command::new("hyprctl").args(["dispatch", "setprop", &pid, prop, &value]).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status();
    }
}

/// The desktop's own light/dark choice (the portal's or GNOME's setting; GTK_THEME as a hint).
pub fn system_is_dark() -> bool {
    let portal = std::process::Command::new("busctl")
        .args(["--user", "call", "org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop", "org.freedesktop.portal.Settings", "ReadOne", "ss", "org.freedesktop.appearance", "color-scheme"])
        .output();
    if let Ok(out) = portal {
        // «v u 1» = prefers dark, «v u 2» = prefers light.
        let s = String::from_utf8_lossy(&out.stdout);
        if s.trim().ends_with(" 1") {
            return true;
        }
        if s.trim().ends_with(" 2") {
            return false;
        }
    }
    if let Ok(out) = std::process::Command::new("gsettings").args(["get", "org.gnome.desktop.interface", "color-scheme"]).output() {
        if String::from_utf8_lossy(&out.stdout).contains("dark") {
            return true;
        }
    }
    std::env::var("GTK_THEME").is_ok_and(|t| t.to_lowercase().contains("dark"))
}

// ── The palette: web/ui/styles.css's tokens ──

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub dark: bool,
    pub bg: Color,
    #[allow(dead_code)]
    pub cell: Color,
    pub label: Color,
    pub label2: Color,
    pub label3: Color,
    pub separator: Color,
    pub fill: Color,
    pub fill2: Color,
    pub fill3: Color,
    pub fill4: Color,
    pub accent: Color,
    pub red: Color,
    pub green: Color,
    pub orange: Color,
    pub window_opacity: f32,
    pub sidebar_opacity: f32,
    pub scale: f32,
    pub compact: bool,
    pub round: f32,
}

pub fn rgb(hex: u32) -> Color {
    Color::from_rgb8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color::from_rgba8(r, g, b, a)
}

pub fn alpha(c: Color, a: f32) -> Color {
    Color { a: c.a * a, ..c }
}

/// `top` laid over `under`, as one opaque colour. Drawing translucent layers left the parts
/// that weren't repainted dark on Wayland (with both renderers), so what doesn't need to be
/// see-through is mixed here instead.
pub fn over(under: Color, top: Color) -> Color {
    let a = top.a;
    Color { r: top.r * a + under.r * (1.0 - a), g: top.g * a + under.g * (1.0 - a), b: top.b * a + under.b * (1.0 - a), a: 1.0 }
}

impl Palette {
    pub fn new(prefs: &Prefs, system_dark: bool) -> Palette {
        set_ui(prefs.size);
        let dark = match prefs.mode {
            Mode::System => system_dark,
            Mode::Light => false,
            Mode::Dark => true,
        };
        let common = |p: Palette| Palette {
            accent: prefs.accent.color(dark),
            // The window is always drawn solid; its transparency is the compositor's (see
            // `apply_window_opacity`): translucent drawing left unrepainted patches on Wayland.
            window_opacity: 1.0,
            sidebar_opacity: prefs.sidebar_opacity.clamp(0.0, 1.0),
            scale: prefs.text_scale.clamp(0.8, 1.4) * prefs.size.clamp(1.0, 2.0),
            compact: prefs.compact,
            round: prefs.roundness.clamp(0.2, 1.6),
            ..p
        };
        let base = if dark {
            Palette {
                dark,
                bg: rgb(0x000000),
                cell: rgb(0x1c1c1e),
                label: rgb(0xffffff),
                label2: rgba(235, 235, 245, 0.6),
                label3: rgba(235, 235, 245, 0.3),
                separator: rgba(84, 84, 88, 0.6),
                fill: rgba(120, 120, 128, 0.36),
                fill2: rgba(120, 120, 128, 0.32),
                fill3: rgba(118, 118, 128, 0.24),
                fill4: rgba(118, 118, 128, 0.18),
                accent: rgb(0x0a84ff),
                red: rgb(0xff453a),
                green: rgb(0x30d158),
                orange: rgb(0xff9f0a),
                window_opacity: 1.0,
                sidebar_opacity: 0.7,
                scale: 1.0,
                compact: false,
                round: 1.0,
            }
        } else {
            Palette {
                dark,
                bg: rgb(0xf2f2f7),
                cell: rgb(0xffffff),
                label: rgb(0x000000),
                label2: rgba(60, 60, 67, 0.6),
                label3: rgba(60, 60, 67, 0.3),
                separator: rgba(60, 60, 67, 0.29),
                fill: rgba(120, 120, 128, 0.2),
                fill2: rgba(120, 120, 128, 0.16),
                fill3: rgba(118, 118, 128, 0.12),
                fill4: rgba(116, 116, 128, 0.08),
                accent: rgb(0x007aff),
                red: rgb(0xff3b30),
                green: rgb(0x34c759),
                orange: rgb(0xff9500),
                window_opacity: 1.0,
                sidebar_opacity: 0.7,
                scale: 1.0,
                compact: false,
                round: 1.0,
            }
        };
        common(base)
    }

    /// A text size, scaled by the person's choice.
    pub fn sz(&self, points: f32) -> f32 {
        (points * self.scale).round()
    }

    pub fn r(&self, radius: f32) -> f32 {
        radius * self.round
    }

    /// The main area: the window's background (see-through as chosen).
    pub fn window(&self) -> Color {
        alpha(if self.dark { rgb(0x1e1e1e) } else { rgb(0xffffff) }, self.window_opacity)
    }

    pub fn iced_theme(&self) -> Theme {
        let palette = iced::theme::Palette { background: self.bg, text: self.label, primary: self.accent, success: self.green, danger: self.red };
        Theme::custom("TPCloud".to_string(), palette)
    }
}

// ── Styles ──
//
// No shadows: iced 0.13 (both renderers) left the areas under a shadow badly repainted after the
// first frame (dark or grey patches over sidebars and cards). Hairline borders do their job.

pub fn window_bg(p: Palette) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: Some(Background::Color(p.window())), text_color: Some(p.label), ..Default::default() }
}

/// The floating, translucent sidebar (web/store's Sidebar.vue).
pub fn sidebar(p: Palette) -> impl Fn(&Theme) -> container::Style {
    move |_| {
        let base = if p.dark { rgba(44, 44, 46, 1.0) } else { p.bg };
        let tint = alpha(base, p.sidebar_opacity.max(0.15));
        // Opaque unless the window itself is see-through (then it shows the desktop too).
        let bg = if p.window_opacity >= 0.995 { over(p.window(), tint) } else { alpha(base, p.sidebar_opacity.max(0.15) * p.window_opacity) };
        container::Style {
            background: Some(Background::Color(bg)),
            border: Border { color: if p.dark { rgba(255, 255, 255, 0.08) } else { p.separator }, width: 0.5, radius: p.r(20.0).into() },
            shadow: Shadow::default(),
            text_color: Some(p.label),
        }
    }
}

/// A grouped-list card (web/ui's .card).
pub fn card(p: Palette) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        // Light: a step greyer than the white window, like the Mac's System Settings.
        background: Some(Background::Color({
            let c = if p.dark { rgb(0x2c2c2e) } else { rgb(0xf5f5f7) };
            if p.window_opacity >= 0.995 { c } else { alpha(c, p.window_opacity.max(0.85)) }
        })),
        border: Border { color: if p.dark { rgba(255, 255, 255, 0.06) } else { rgba(0, 0, 0, 0.06) }, width: 0.5, radius: p.r(14.0).into() },
        shadow: Shadow::default(),
        text_color: Some(p.label),
    }
}

/// A floating glass panel (sheets, the sign-in card).
pub fn glass(p: Palette) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(if p.dark { rgba(44, 44, 46, 0.82) } else { rgba(255, 255, 255, 0.86) })),
        border: Border { color: if p.dark { rgba(255, 255, 255, 0.12) } else { rgba(255, 255, 255, 0.9) }, width: 0.5, radius: p.r(22.0).into() },
        shadow: Shadow::default(),
        text_color: Some(p.label),
    }
}

pub fn separator(p: Palette) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: Some(Background::Color(p.separator)), ..Default::default() }
}

pub fn fill(p: Palette, c: Color, radius: f32) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: Some(Background::Color(c)), border: Border { radius: p.r(radius).into(), ..Default::default() }, ..Default::default() }
}

/// A sidebar item: nothing, a faint fill on hover, a stronger one when it's the place shown.
pub fn side_item(p: Palette, active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: match (active, status) {
            (true, _) => Some(Background::Color(p.fill2)),
            (false, button::Status::Hovered) => Some(Background::Color(p.fill4)),
            (false, button::Status::Pressed) => Some(Background::Color(p.fill3)),
            _ => None,
        },
        text_color: p.label,
        border: Border { radius: p.r(9.0).into(), ..Default::default() },
        shadow: Shadow::default(),
    }
}

/// A row of the list view, like web/store's `.list-row`: every other one faintly banded (the
/// label colour at 3.5 %), the selected one filled with the accent and its text white.
pub fn list_row(p: Palette, selected: bool, zebra: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: match (selected, status) {
            (true, _) => Some(Background::Color(p.accent)),
            (false, button::Status::Hovered) => Some(Background::Color(p.fill4)),
            (false, button::Status::Pressed) => Some(Background::Color(p.fill3)),
            _ if zebra => Some(Background::Color(alpha(p.label, 0.035))),
            _ => None,
        },
        text_color: if selected { Color::WHITE } else { p.label },
        border: Border { radius: p.r(8.0).into(), ..Default::default() },
        shadow: Shadow::default(),
    }
}

/// A tile of the icon view (web/store's `.icon-item`): no fill of its own; when selected its
/// picture sits on fill-3 and its name on the accent (drawn by the tile itself).
pub fn item(p: Palette, selected: bool, radius: f32) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: match (selected, status) {
            (true, _) => None,
            (false, button::Status::Hovered) => Some(Background::Color(p.fill4)),
            (false, button::Status::Pressed) => Some(Background::Color(p.fill3)),
            _ => None,
        },
        text_color: p.label,
        border: Border { radius: p.r(radius).into(), ..Default::default() },
        shadow: Shadow::default(),
    }
}

/// The accent-filled pill (web/ui's .btn-prominent).
pub fn prominent(p: Palette) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let c = match status {
            button::Status::Hovered => lighten(p.accent, 0.08),
            button::Status::Pressed => darken(p.accent, 0.08),
            button::Status::Disabled => alpha(p.accent, 0.4),
            button::Status::Active => p.accent,
        };
        button::Style { background: Some(Background::Color(c)), text_color: Color::WHITE, border: Border { radius: 999.0.into(), ..Default::default() }, shadow: Shadow::default() }
    }
}

/// The glass pill and the round toolbar controls (web/ui's .btn-glass, store's .glass-control).
pub fn glass_button(p: Palette) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let bg = if p.dark { rgba(58, 58, 60, 0.82) } else { rgba(255, 255, 255, 0.86) };
        let bg = match status {
            button::Status::Hovered => if p.dark { rgba(72, 72, 74, 0.9) } else { rgba(255, 255, 255, 1.0) },
            button::Status::Pressed => p.fill,
            button::Status::Disabled => alpha(bg, 0.5),
            button::Status::Active => bg,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: if status == button::Status::Disabled { p.label3 } else { p.label },
            border: Border { color: if p.dark { rgba(255, 255, 255, 0.1) } else { rgba(0, 0, 0, 0.08) }, width: 0.5, radius: 999.0.into() },
            shadow: Shadow::default(),
        }
    }
}

/// Text-only, in the accent (web/ui's .btn-plain).
pub fn plain(p: Palette) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: if status == button::Status::Hovered { Some(Background::Color(p.fill4)) } else { None },
        text_color: if status == button::Status::Disabled { p.label3 } else { p.accent },
        border: Border { radius: p.r(8.0).into(), ..Default::default() },
        shadow: Shadow::default(),
    }
}

/// A segment of the segmented control: the thumb is white (dark: #636366) on the fill-3 track.
pub fn segment(p: Palette, on: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: if on {
            Some(Background::Color(if p.dark { rgb(0x636366) } else { Color::WHITE }))
        } else if status == button::Status::Hovered {
            Some(Background::Color(p.fill4))
        } else {
            None
        },
        text_color: p.label,
        border: Border { radius: p.r(7.0).into(), ..Default::default() },
        shadow: Shadow::default(),
    }
}

pub fn track(p: Palette) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: Some(Background::Color(p.fill3)), border: Border { radius: p.r(9.0).into(), ..Default::default() }, ..Default::default() }
}

/// A colour swatch for the accent choice.
pub fn swatch(p: Palette, c: Color, on: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, _| button::Style {
        background: Some(Background::Color(c)),
        text_color: Color::WHITE,
        border: Border { color: if on { p.label2 } else { Color::TRANSPARENT }, width: if on { 2.5 } else { 0.0 }, radius: 999.0.into() },
        shadow: Shadow::default(),
    }
}

/// Search and form fields: filled, no outline until focused.
pub fn field(p: Palette) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_, status| text_input::Style {
        background: Background::Color(p.fill3),
        border: Border {
            color: if matches!(status, text_input::Status::Focused) { alpha(p.accent, 0.6) } else { Color::TRANSPARENT },
            width: if matches!(status, text_input::Status::Focused) { 2.0 } else { 0.0 },
            radius: p.r(9.0).into(),
        },
        icon: p.label2,
        placeholder: p.label3,
        value: p.label,
        selection: alpha(p.accent, 0.3),
    }
}

pub fn scroll(p: Palette) -> impl Fn(&Theme, scrollable::Status) -> scrollable::Style {
    move |_, status| {
        let scroller = |hover: bool| scrollable::Scroller { color: if hover { p.label3 } else { p.fill }, border: Border { radius: 999.0.into(), ..Default::default() } };
        let hover = matches!(status, scrollable::Status::Hovered { .. } | scrollable::Status::Dragged { .. });
        let rail = scrollable::Rail { background: None, border: Border::default(), scroller: scroller(hover) };
        scrollable::Style { container: container::Style::default(), vertical_rail: rail, horizontal_rail: rail, gap: None }
    }
}

pub fn range(p: Palette) -> impl Fn(&Theme, slider::Status) -> slider::Style {
    move |_, _| slider::Style {
        rail: slider::Rail { backgrounds: (Background::Color(p.accent), Background::Color(p.fill)), width: 4.0, border: Border { radius: 2.0.into(), ..Default::default() } },
        handle: slider::Handle { shape: slider::HandleShape::Circle { radius: 9.0 }, background: Background::Color(Color::WHITE), border_width: 0.5, border_color: rgba(0, 0, 0, 0.15) },
    }
}

fn lighten(c: Color, by: f32) -> Color {
    Color { r: (c.r + by).min(1.0), g: (c.g + by).min(1.0), b: (c.b + by).min(1.0), a: c.a }
}

fn darken(c: Color, by: f32) -> Color {
    Color { r: (c.r - by).max(0.0), g: (c.g - by).max(0.0), b: (c.b - by).max(0.0), a: c.a }
}
