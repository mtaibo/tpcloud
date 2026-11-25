//! TPCloud for the Linux desktop: a replica of the web apps' desktop layouts (`agents/design.md`):
//! a Finder-like Files with a translucent sidebar, icon and list views and a path bar; Photos like
//! Apple's; the house; the account; and Settings for the look (light/dark, accent, transparency,
//! text size, density, corners). Native (iced, Wayland or X11), the same API as the web and the
//! iPhone app. Fast first: everything shown comes from the cache at once and is refreshed after.

mod cache;
mod discover;
mod icons;
mod net;
mod store;
mod theme;

use iced::widget::{button, column, container, horizontal_space, image, row, scrollable, slider, svg, text, text_input, Column, Row, Space};
use iced::{keyboard, window, Alignment, Color, Element, Length, Padding, Size, Subscription, Task, Theme};
use net::{Api, Blind, Entry, Host, Me, Photo};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};
use store::Store;
use theme::{u, Palette, Prefs, View, BOLD, MEDIUM, SEMIBOLD};

const FONTS: [&[u8]; 4] = [
    include_bytes!("../assets/fonts/Inter-Regular.ttf"),
    include_bytes!("../assets/fonts/Inter-Medium.ttf"),
    include_bytes!("../assets/fonts/Inter-SemiBold.ttf"),
    include_bytes!("../assets/fonts/Inter-Bold.ttf"),
];

/// Thumbnails asked of the server at once: enough to fill a screen quickly, few enough not to
/// queue the Pi.
const THUMBS_AT_ONCE: usize = 8;

/// Photos: thumbnails asked for per bulk request, bulk requests at once, and how many decoded
/// thumbnails stay in memory (those far from what's on screen go first).
const PHOTO_BATCH: usize = 60;
const PHOTO_BATCHES: usize = 3;
const PHOTO_KEEP: usize = 1500;

/// The sidebar's width, its margin, and the heights around the file list (for working out
/// what fits and what's on screen).
fn sidebar() -> f32 {
    u(256.0)
}
fn toolbar() -> f32 {
    u(64.0)
}
fn pathbar() -> f32 {
    u(36.0)
}
fn list_head() -> f32 {
    u(32.0)
}

fn main() -> iced::Result {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--look") {
        let (networks, waiting) = tokio::runtime::Runtime::new().unwrap().block_on(discover::look());
        println!("networks: {networks:?}\nwaiting to be set up: {waiting:?}");
        return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("--probe") {
        probe(&args[2..]);
        return Ok(());
    }
    // Vulkan through NVIDIA's driver on Wayland draws a black window (an RTX 4070 on Hyprland did):
    // OpenGL draws the same everywhere, so it's the default here unless someone picks otherwise.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WGPU_BACKEND").is_none() {
        std::env::set_var("WGPU_BACKEND", "gl");
    }
    // Drawn solid; see-through is the compositor's job (Settings → Transparency → Window).
    let mut app = iced::application(App::title, App::update, App::view)
        .subscription(App::subscription)
        .theme(App::theme)
        .style(|app: &App, _| iced::application::Appearance { background_color: app.p.window(), text_color: app.p.label })
        .default_font(theme::INTER)
        .window(window::Settings { size: Size::new(1180.0, 760.0), min_size: Some(Size::new(760.0, 480.0)), ..Default::default() });
    for f in FONTS {
        app = app.font(f);
    }
    app.run_with(App::new)
}

/// `tpcloud-app --probe <address> [email password]`: everything the app does with a network,
/// without a window (for checking it from a terminal).
fn probe(args: &[String]) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let Some(address) = args.first() else {
            eprintln!("usage: tpcloud-app --probe <address> [email password]");
            return;
        };
        let (card, anchor) = match net::load_card(address).await {
            Ok(x) => x,
            Err(e) => return eprintln!("card: {e}"),
        };
        println!("card: {} ({}) v{} addresses {:?} anchor {}", card.name, card.id, card.version, card.addresses.iter().map(|a| (&a.url, &a.kind)).collect::<Vec<_>>(), anchor.as_ref().map(|d| net::fingerprint(d)).unwrap_or("none (system-trusted)".into()));
        let locals: Vec<String> = card.addresses.iter().filter(|a| a.kind == "local").map(|a| net::host_of(&a.url)).collect();
        let base = net::normalize(address).unwrap();
        let mut api = Api::new(&base, anchor.as_deref(), &locals, None);
        println!("answers as its id: {}", api.answers_as(&card.id).await);
        let stranger = Api::new(&base, Some(&[0x30, 0x03, 0x02, 0x01, 0x01]), &locals, None);
        println!("with a wrong anchor: {}", if stranger.answers_as(&card.id).await { "ACCEPTED (bad)" } else { "refused" });
        if let (Some(email), Some(password)) = (args.get(1), args.get(2)) {
            match api.login(email, password).await {
                Ok(totp) => println!("login: ok, totp needed: {totp}, token: {}", api.token.is_some()),
                Err(e) => return eprintln!("login: {e}"),
            }
            let me = api.me().await;
            println!("me: {:?}", me.as_ref().map(|m| &m.email));
            if let Ok(me) = me {
                println!("files in users/{}: {:?}", me.email, api.list(&format!("users/{}", me.email)).await.map(|l| l.len()));
            }
            println!("photos: {:?}", api.photos(None).await.map(|p| p.items.len()));
            api.logout().await;
            println!("after logout, me: {}", if api.me().await.is_err() { "refused (good)" } else { "STILL IN (bad)" });
        }
    });
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Place {
    Files,
    Shared,
    Photos,
    House,
    Account,
    Settings,
}

#[derive(Debug, PartialEq)]
enum Screen {
    Loading,
    Networks,
    Login,
    Main,
}

#[derive(Debug, Clone, PartialEq)]
enum ThumbKey {
    File(String, u64),
}

impl ThumbKey {
    fn id(&self) -> String {
        match self {
            ThumbKey::File(p, v) => format!("f:{p}:{v}"),
        }
    }
}

/// The photo grid worked out once (not every frame): its rows, where each starts, and how tall
/// it all is, so only the rows near the screen are built.
struct PhotoLayout {
    key: (usize, i64, usize, u32),
    cell: f32,
    month_h: f32,
    row_h: f32,
    rows: Vec<PhotoRow>,
    tops: Vec<f32>,
    total: f32,
}

enum PhotoRow {
    Month(String),
    /// Photos `start..end` of the library.
    Tiles(usize, usize),
}

struct App {
    prefs: Prefs,
    system_dark: bool,
    p: Palette,
    store: Store,
    screen: Screen,
    api: Option<Api>,
    me: Option<Me>,
    busy: bool,
    error: Option<String>,
    notice: Option<(String, Instant)>,
    width: f32,
    height: f32,
    // Networks and sign-in
    address: String,
    found: Vec<discover::Found>,
    waiting: Vec<discover::Found>,
    looking: bool,
    email: String,
    password: String,
    code: String,
    totp: bool,
    // Files
    place: Place,
    path: Vec<String>,
    back: Vec<(Place, Vec<String>)>,
    forward: Vec<(Place, Vec<String>)>,
    listings: HashMap<String, Vec<Entry>>,
    loading: bool,
    search: String,
    selected: Option<String>,
    last_click: Option<(String, Instant)>,
    /// How far the file list is scrolled and how tall its viewport is, to keep the selection
    /// in sight when the arrows move it.
    files_scroll: (f32, f32),
    // Thumbnails (files and photos)
    thumbs: HashMap<String, image::Handle>,
    queued: HashSet<String>,
    queue: VecDeque<ThumbKey>,
    in_flight: usize,
    // Photos
    photos: Vec<Photo>,
    /// The library's index came from the server in this session (else it's the cached one).
    photos_fresh: bool,
    photo_layout: RefCell<Option<PhotoLayout>>,
    /// How far the grid is scrolled and how tall its viewport is.
    photos_scroll: (f32, f32),
    photo_thumbs: HashMap<i64, image::Handle>,
    photo_loading: HashSet<i64>,
    photo_failed: HashSet<i64>,
    photo_batches: usize,
    // House and account
    blinds: Vec<Blind>,
    host: Option<Host>,
}

#[derive(Debug, Clone)]
enum Message {
    Connected(Result<(Api, Option<Me>), String>),
    Address(String),
    Add,
    Added(Result<(net::Card, Option<Vec<u8>>, String), String>),
    Look,
    Looked((Vec<discover::Found>, Vec<discover::Found>)),
    AddFound(String),
    Use(String),
    Remove(String),
    ShowNetworks,
    Email(String),
    Password(String),
    Code(String),
    SignIn,
    SignedIn(Result<(Api, bool), String>),
    Verify,
    Verified(Result<Me, String>),
    Go(Place),
    Back,
    Forward,
    Crumb(usize),
    Click(String),
    Listed(String, Result<Vec<Entry>, String>),
    Prefetched(String, Result<Vec<Entry>, String>),
    Search(String),
    Refresh,
    Opened(Result<std::path::PathBuf, String>),
    Download,
    Downloaded(Result<std::path::PathBuf, String>),
    Upload,
    Dropped(std::path::PathBuf),
    Uploaded(Result<usize, String>),
    Thumb(String, Result<Vec<u8>, String>),
    PhotoIndex(Result<Vec<Photo>, String>),
    PhotoThumbs(Vec<i64>, Vec<(i64, image::Handle)>),
    PhotosScrolled(scrollable::Viewport),
    FilesScrolled(scrollable::Viewport),
    Blinds(Result<Vec<Blind>, String>),
    BlindCommand(String, &'static str),
    Done(Result<(), String>),
    Tick,
    SignOut,
    MakeHost,
    HostStarted(Result<String, String>),
    HostLoaded(Result<Host, String>),
    HostAction(&'static str),
    HostActed(Result<&'static str, String>),
    Resized(Size),
    Key(keyboard::Key, keyboard::Modifiers),
    Pref(Pref),
    SystemTheme(bool),
}

#[derive(Debug, Clone)]
enum Pref {
    Mode(theme::Mode),
    Accent(theme::Accent),
    WindowOpacity(f32),
    SidebarOpacity(f32),
    TextScale(f32),
    Compact(bool),
    Roundness(f32),
    View(View),
    IconSize(f32),
    Size(f32),
    Reset,
}

impl App {
    fn new() -> (App, Task<Message>) {
        let prefs = Prefs::load();
        let system_dark = theme::system_is_dark();
        let store = Store::load();
        let mut app = App {
            p: Palette::new(&prefs, system_dark),
            prefs,
            system_dark,
            store,
            screen: Screen::Loading,
            api: None,
            me: None,
            busy: false,
            error: None,
            notice: None,
            width: 1180.0,
            height: 760.0,
            address: String::new(),
            found: vec![],
            waiting: vec![],
            looking: false,
            email: String::new(),
            password: String::new(),
            code: String::new(),
            totp: false,
            place: Place::Files,
            path: vec![],
            back: vec![],
            forward: vec![],
            listings: HashMap::new(),
            loading: false,
            search: String::new(),
            selected: None,
            last_click: None,
            files_scroll: (0.0, 0.0),
            thumbs: HashMap::new(),
            queued: HashSet::new(),
            queue: VecDeque::new(),
            in_flight: 0,
            photos: vec![],
            photos_fresh: false,
            photo_layout: RefCell::new(None),
            photos_scroll: (0.0, 0.0),
            photo_thumbs: HashMap::new(),
            photo_loading: HashSet::new(),
            photo_failed: HashSet::new(),
            photo_batches: 0,
            blinds: vec![],
            host: None,
        };
        // Signed in before: straight to the files as they were, while it connects underneath.
        if let Some(net) = app.store.current().cloned() {
            if net.token.is_some() {
                if let Some(me) = cache::me(&net.id) {
                    app.me = Some(me);
                    app.screen = Screen::Main;
                    let root = app.folder();
                    if let Some(l) = cache::listing(&net.id, &root) {
                        app.listings.insert(root, l);
                    }
                    app.photos = cache::photos(&net.id).unwrap_or_default();
                }
            }
        }
        let opacity = app.prefs.window_opacity;
        let task = Task::batch([
            app.connect(),
            // Once the window exists, the compositor makes it as see-through as chosen.
            Task::perform(
                async move {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    theme::apply_window_opacity(opacity);
                },
                |_| Message::Done(Ok(())),
            ),
        ]);
        (app, task)
    }

    fn title(&self) -> String {
        match (&self.screen, self.store.current()) {
            (Screen::Main, Some(n)) => format!("TPCloud · {}", n.name),
            _ => "TPCloud".into(),
        }
    }

    fn theme(&self) -> Theme {
        self.p.iced_theme()
    }

    fn net_id(&self) -> String {
        self.store.current().map(|n| n.id.clone()).unwrap_or_default()
    }

    fn set_pref(&mut self, pref: Pref) {
        let p = &mut self.prefs;
        match pref {
            Pref::Mode(m) => p.mode = m,
            Pref::Accent(a) => p.accent = a,
            Pref::WindowOpacity(v) => {
                p.window_opacity = v;
                theme::apply_window_opacity(v);
            }
            Pref::SidebarOpacity(v) => p.sidebar_opacity = v,
            Pref::TextScale(v) => p.text_scale = v,
            Pref::Compact(v) => p.compact = v,
            Pref::Roundness(v) => p.roundness = v,
            Pref::View(v) => p.view = v,
            Pref::IconSize(v) => p.icon_size = v,
            Pref::Size(v) => p.size = v,
            Pref::Reset => {
                *p = Prefs::default();
                theme::apply_window_opacity(1.0);
            }
        }
        self.p = Palette::new(&self.prefs, self.system_dark);
        self.prefs.save();
    }

    /// The fastest address that answers (home's, Tailscale's, public), taken as soon as one does.
    fn connect(&mut self) -> Task<Message> {
        let Some(net) = self.store.current().cloned() else {
            self.screen = Screen::Networks;
            return Task::done(Message::Look);
        };
        if self.screen != Screen::Main {
            self.screen = Screen::Loading;
        }
        self.email = net.email.clone().unwrap_or_default();
        Task::perform(
            async move {
                let anchor = net.anchor_der();
                let locals = net.local_hosts();
                let mut candidates = net.by_preference();
                if let Some(last) = &net.last_address {
                    if !candidates.contains(last) {
                        candidates.insert(0, last.clone());
                    }
                }
                let (tx, mut rx) = tokio::sync::mpsc::channel(candidates.len().max(1));
                for url in candidates.clone() {
                    let (tx, anchor, locals, token, id) = (tx.clone(), anchor.clone(), locals.clone(), net.token.clone(), net.id.clone());
                    tokio::spawn(async move {
                        let api = Api::new(&url, anchor.as_deref(), &locals, token);
                        let ok = api.answers_as(&id).await;
                        let _ = tx.send(ok.then_some(api)).await;
                    });
                }
                drop(tx);
                let mut api = None;
                while let Some(answer) = rx.recv().await {
                    if answer.is_some() {
                        api = answer;
                        break;
                    }
                }
                let api = api.ok_or("None of its addresses answer right now.".to_string())?;
                let me = if api.token.is_some() { api.me().await.ok() } else { None };
                Ok((api, me))
            },
            Message::Connected,
        )
    }

    // ── Files ──

    fn base_folder(&self) -> String {
        match self.place {
            Place::Shared => "shared".into(),
            _ => format!("users/{}", self.me.as_ref().map(|m| m.email.as_str()).unwrap_or("")),
        }
    }

    fn folder(&self) -> String {
        let mut p = self.base_folder();
        for part in &self.path {
            p.push('/');
            p.push_str(part);
        }
        p
    }

    fn entries(&self) -> &[Entry] {
        self.listings.get(&self.folder()).map(Vec::as_slice).unwrap_or(&[])
    }

    fn list(&mut self) -> Task<Message> {
        let Some(api) = self.api.clone() else { return Task::none() };
        let path = self.folder();
        self.loading = !self.listings.contains_key(&path);
        if self.loading {
            if let Some(l) = cache::listing(&self.net_id(), &path) {
                self.listings.insert(path.clone(), l);
                self.loading = false;
            }
        }
        let task = Task::perform({
            let path = path.clone();
            async move { api.list(&path).await }
        }, move |r| Message::Listed(path.clone(), r));
        Task::batch([task, self.want_thumbs()])
    }

    /// The folders here, listed in the background, so opening one is instant.
    fn prefetch(&mut self, folder: &str) -> Task<Message> {
        let Some(api) = self.api.clone() else { return Task::none() };
        let subs: Vec<String> = self
            .listings
            .get(folder)
            .map(|l| l.iter().filter(|e| e.kind == "directory").take(16).map(|e| format!("{folder}/{}", e.name)).collect())
            .unwrap_or_default();
        Task::batch(subs.into_iter().filter(|s| !self.listings.contains_key(s)).map(|s| {
            let api = api.clone();
            let key = s.clone();
            Task::perform(async move { api.list(&s).await }, move |r| Message::Prefetched(key.clone(), r))
        }))
    }

    fn navigate(&mut self, place: Place, path: Vec<String>) -> Task<Message> {
        if (place, &path) != (self.place, &self.path) {
            self.back.push((self.place, std::mem::take(&mut self.path)));
            self.forward.clear();
        }
        self.place = place;
        self.path = path;
        self.selected = None;
        self.search.clear();
        self.error = None;
        self.load_place()
    }

    fn load_place(&mut self) -> Task<Message> {
        let Some(api) = self.api.clone() else { return Task::none() };
        match self.place {
            Place::Files | Place::Shared => self.list(),
            Place::Photos => {
                if self.photos_fresh {
                    return self.want_photo_thumbs();
                }
                Task::batch([Task::perform(async move { api.photo_index().await }, Message::PhotoIndex), self.want_photo_thumbs()])
            }
            Place::House => Task::perform(async move { api.blinds().await }, Message::Blinds),
            Place::Account => {
                if !self.me.as_ref().is_some_and(|m| m.is_admin) {
                    return Task::none();
                }
                Task::perform(async move { api.host().await }, Message::HostLoaded)
            }
            Place::Settings => Task::none(),
        }
    }

    // ── Thumbnails: what's on screen, cached on disk, a few at a time from the server ──

    fn want_thumbs(&mut self) -> Task<Message> {
        let net = self.net_id();
        let mut keys: Vec<ThumbKey> = Vec::new();
        match self.place {
            Place::Files | Place::Shared if self.prefs.view == View::Icons => {
                let folder = self.folder();
                for e in self.entries() {
                    let (kind, _) = icons::kind_of(&e.name);
                    if e.kind == "file" && (kind == "image" || kind == "video") {
                        keys.push(ThumbKey::File(format!("{folder}/{}", e.name), e.modified.unwrap_or(0.0) as u64 ^ e.size.unwrap_or(0)));
                    }
                }
            }
            _ => {}
        }
        for k in keys {
            let id = k.id();
            if self.thumbs.contains_key(&id) || self.queued.contains(&id) {
                continue;
            }
            if let Some(bytes) = cache::thumb(&net, &id) {
                self.thumbs.insert(id, image::Handle::from_bytes(bytes));
                continue;
            }
            self.queued.insert(id);
            self.queue.push_back(k);
        }
        self.pump()
    }

    fn pump(&mut self) -> Task<Message> {
        let Some(api) = self.api.clone() else { return Task::none() };
        let mut tasks = Vec::new();
        while self.in_flight < THUMBS_AT_ONCE {
            let Some(k) = self.queue.pop_front() else { break };
            self.in_flight += 1;
            let id = k.id();
            let api = api.clone();
            match k {
                ThumbKey::File(path, _) => tasks.push(Task::perform(async move { api.file_thumbnail(&path, 240).await }, move |r| Message::Thumb(id.clone(), r))),
            }
        }
        Task::batch(tasks)
    }

    // ── Photos: laid out once, only what's near the screen built, thumbnails in bulk ──

    fn photo_layout(&self) -> std::cell::Ref<'_, PhotoLayout> {
        let usable = (self.width - sidebar() - u(8.0 + 28.0)).max(200.0);
        let per_row = ((usable / u(150.0)).round() as usize).clamp(2, 24);
        let key = (self.photos.len(), self.photos.first().map(|p| p.id).unwrap_or(0), per_row, (usable * 10.0) as u32 ^ self.p.scale.to_bits());
        if self.photo_layout.borrow().as_ref().map(|l| l.key) != Some(key) {
            let gap = u(2.0);
            let cell = ((usable - (per_row as f32 - 1.0) * gap) / per_row as f32).floor();
            let month_h = u(26.0) + 22.0 * self.p.scale * 1.3;
            let row_h = cell + gap;
            let mut rows = Vec::new();
            let mut i = 0;
            while i < self.photos.len() {
                let month = photo_month(&self.photos[i]);
                let mut end = i + 1;
                while end < self.photos.len() && photo_month(&self.photos[end]) == month {
                    end += 1;
                }
                rows.push(PhotoRow::Month(month));
                let mut start = i;
                while start < end {
                    rows.push(PhotoRow::Tiles(start, (start + per_row).min(end)));
                    start += per_row;
                }
                i = end;
            }
            let mut tops = Vec::with_capacity(rows.len());
            let mut y = 0.0;
            for row in &rows {
                tops.push(y);
                y += if matches!(row, PhotoRow::Month(_)) { month_h } else { row_h };
            }
            *self.photo_layout.borrow_mut() = Some(PhotoLayout { key, cell, month_h, row_h, rows, tops, total: y });
        }
        std::cell::Ref::map(self.photo_layout.borrow(), |l| l.as_ref().unwrap())
    }

    /// The rows from a screen above to two below what's shown (the ones worth building and
    /// loading).
    fn photo_rows_near(&self) -> std::ops::Range<usize> {
        let l = self.photo_layout();
        let (offset, mut view) = self.photos_scroll;
        if view <= 0.0 {
            view = self.height;
        }
        let (lo, hi) = (offset - view, offset + view * 2.0);
        let a = l.tops.partition_point(|&t| t < lo).saturating_sub(1);
        let b = l.tops.partition_point(|&t| t <= hi);
        a..b.max(a)
    }

    /// The photos near the screen whose thumbnails aren't here yet, asked for in bulk (from the
    /// disk cache first, then the server), decoded off the UI thread.
    fn want_photo_thumbs(&mut self) -> Task<Message> {
        if self.place != Place::Photos || self.photos.is_empty() {
            return Task::none();
        }
        let Some(api) = self.api.clone() else { return Task::none() };
        let range = self.photo_rows_near();
        let (first, last) = {
            let l = self.photo_layout();
            let tiles = l.rows[range.clone()].iter().filter_map(|r| if let PhotoRow::Tiles(a, b) = r { Some((*a, *b)) } else { None });
            tiles.fold((usize::MAX, 0), |(lo, hi), (a, b)| (lo.min(a), hi.max(b)))
        };
        if first >= last {
            return Task::none();
        }
        let mut want: Vec<i64> = self.photos[first..last].iter().map(|p| p.id).filter(|id| !self.photo_thumbs.contains_key(id) && !self.photo_loading.contains(id) && !self.photo_failed.contains(id)).collect();
        let mut tasks = Vec::new();
        let net = self.net_id();
        let side = u(170.0).min(340.0) as u32;
        while self.photo_batches < PHOTO_BATCHES && !want.is_empty() {
            let ids: Vec<i64> = want.drain(..want.len().min(PHOTO_BATCH)).collect();
            self.photo_loading.extend(ids.iter().copied());
            self.photo_batches += 1;
            let (api, net, asked) = (api.clone(), net.clone(), ids.clone());
            tasks.push(Task::perform(
                async move {
                    let mut found = Vec::new();
                    let mut missing = Vec::new();
                    for id in ids {
                        match tokio::fs::read(cache::thumb_file(&net, &format!("p:{id}"))).await {
                            Ok(b) => found.push((id, b)),
                            Err(_) => missing.push(id),
                        }
                    }
                    let fresh = if missing.is_empty() { vec![] } else { api.photo_thumbnails(&missing).await.unwrap_or_default() };
                    tokio::task::spawn_blocking(move || {
                        for (id, b) in &fresh {
                            cache::save_thumb(&net, &format!("p:{id}"), b);
                        }
                        found.into_iter().chain(fresh).filter_map(|(id, b)| square_thumb(&b, side).map(|h| (id, h))).collect::<Vec<_>>()
                    })
                    .await
                    .unwrap_or_default()
                },
                move |got| Message::PhotoThumbs(asked.clone(), got),
            ));
        }
        Task::batch(tasks)
    }

    /// Keeps memory bounded: when there are too many, only the thumbnails near the screen stay.
    fn evict_photo_thumbs(&mut self) {
        if self.photo_thumbs.len() <= PHOTO_KEEP {
            return;
        }
        let range = self.photo_rows_near();
        let (first, last) = {
            let l = self.photo_layout();
            let tiles = l.rows[range].iter().filter_map(|r| if let PhotoRow::Tiles(a, b) = r { Some((*a, *b)) } else { None });
            tiles.fold((usize::MAX, 0), |(lo, hi), (a, b)| (lo.min(a), hi.max(b)))
        };
        let margin = PHOTO_KEEP / 3;
        let lo = first.saturating_sub(margin).min(self.photos.len());
        let hi = (last + margin).min(self.photos.len()).max(lo);
        let keep: HashSet<i64> = self.photos[lo..hi].iter().map(|p| p.id).collect();
        self.photo_thumbs.retain(|id, _| keep.contains(id));
    }

    fn signed_out(&mut self) {
        self.screen = Screen::Login;
        self.me = None;
        self.totp = false;
        if let Some(n) = self.store.current_mut() {
            n.token = None;
        }
        self.store.save();
    }

    fn say(&mut self, text: impl Into<String>) {
        self.notice = Some((text.into(), Instant::now()));
    }

    fn open_entry(&mut self, name: &str) -> Task<Message> {
        let Some(entry) = self.entries().iter().find(|e| e.name == name).cloned() else { return Task::none() };
        if entry.kind == "directory" {
            let mut path = self.path.clone();
            path.push(entry.name);
            return self.navigate(self.place, path);
        }
        let Some(api) = self.api.clone() else { return Task::none() };
        let path = format!("{}/{}", self.folder(), entry.name);
        let dir = cache::open_dir(&self.net_id());
        self.say(format!("Opening {}…", entry.name));
        Task::perform(
            async move {
                let file = dir.join(&entry.name);
                let fresh = tokio::fs::metadata(&file).await.is_ok_and(|m| Some(m.len()) == entry.size);
                if !fresh {
                    let data = api.download(&path).await?;
                    tokio::fs::create_dir_all(&dir).await.map_err(|e| e.to_string())?;
                    tokio::fs::write(&file, data).await.map_err(|e| e.to_string())?;
                }
                let _ = std::process::Command::new("xdg-open").arg(&file).spawn();
                Ok(file)
            },
            Message::Opened,
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Connected(Ok((api, me))) => {
                self.error = None;
                if let Some(n) = self.store.current_mut() {
                    n.last_address = Some(api.base.clone());
                }
                self.store.save();
                self.api = Some(api);
                if let Some(me) = me {
                    cache::save_me(&self.net_id(), &me);
                    self.me = Some(me);
                    self.screen = Screen::Main;
                    return self.load_place();
                }
                self.me = None;
                self.screen = Screen::Login;
            }
            Message::Connected(Err(e)) => {
                if self.screen == Screen::Main {
                    // Offline: what's cached stays on screen.
                    self.error = Some(format!("Offline: {e}"));
                } else {
                    self.error = Some(e);
                    self.screen = Screen::Networks;
                    return Task::done(Message::Look);
                }
            }
            Message::Address(a) => self.address = a,
            Message::Add => {
                let address = self.address.clone();
                return self.add(address);
            }
            Message::AddFound(url) => return self.add(url),
            Message::Added(Ok((card, anchor, reached))) => {
                self.busy = false;
                self.address.clear();
                self.store.upsert(&card, anchor, &reached);
                self.reset_network();
                return self.connect();
            }
            Message::Added(Err(e)) | Message::HostStarted(Err(e)) => {
                self.busy = false;
                self.error = Some(e);
            }
            Message::Look => {
                self.looking = true;
                return Task::perform(discover::look(), Message::Looked);
            }
            Message::Looked((found, waiting)) => {
                self.looking = false;
                self.found = found;
                self.waiting = waiting;
            }
            Message::Use(id) => {
                self.store.current = Some(id);
                self.store.save();
                self.reset_network();
                return self.connect();
            }
            Message::Remove(id) => self.store.remove(&id),
            Message::ShowNetworks => {
                self.screen = Screen::Networks;
                return Task::done(Message::Look);
            }
            Message::Email(v) => self.email = v,
            Message::Password(v) => self.password = v,
            Message::Code(v) => {
                self.code = v.chars().filter(char::is_ascii_digit).take(6).collect();
                if self.code.len() == 6 {
                    return Task::done(Message::Verify);
                }
            }
            Message::SignIn => {
                let Some(mut api) = self.api.clone() else { return Task::none() };
                self.busy = true;
                self.error = None;
                let (email, password) = (self.email.trim().to_lowercase(), self.password.clone());
                return Task::perform(async move { api.login(&email, &password).await.map(|totp| (api, totp)) }, Message::SignedIn);
            }
            Message::SignedIn(Ok((api, totp))) => {
                self.busy = false;
                let token = api.token.clone();
                self.api = Some(api.clone());
                let email = self.email.trim().to_lowercase();
                if let Some(n) = self.store.current_mut() {
                    n.token = token;
                    n.email = Some(email);
                }
                self.store.save();
                if totp {
                    self.totp = true;
                    return Task::none();
                }
                return Task::perform(async move { api.me().await }, Message::Verified);
            }
            Message::SignedIn(Err(e)) => {
                self.busy = false;
                self.error = Some(e);
            }
            Message::Verify => {
                let Some(api) = self.api.clone() else { return Task::none() };
                self.busy = true;
                let code = self.code.clone();
                return Task::perform(
                    async move {
                        api.totp(&code).await?;
                        api.me().await
                    },
                    Message::Verified,
                );
            }
            Message::Verified(Ok(me)) => {
                self.busy = false;
                self.password.clear();
                self.code.clear();
                self.totp = false;
                cache::save_me(&self.net_id(), &me);
                self.me = Some(me);
                self.screen = Screen::Main;
                return self.load_place();
            }
            Message::Verified(Err(e)) => {
                self.busy = false;
                self.code.clear();
                self.error = Some(e);
            }
            Message::Go(place) => {
                let path = if place == self.place { vec![] } else { vec![] };
                return self.navigate(place, path);
            }
            Message::Back => {
                if let Some((place, path)) = self.back.pop() {
                    self.forward.push((self.place, std::mem::take(&mut self.path)));
                    self.place = place;
                    self.path = path;
                    self.selected = None;
                    return self.load_place();
                }
            }
            Message::Forward => {
                if let Some((place, path)) = self.forward.pop() {
                    self.back.push((self.place, std::mem::take(&mut self.path)));
                    self.place = place;
                    self.path = path;
                    self.selected = None;
                    return self.load_place();
                }
            }
            Message::Crumb(depth) => {
                let path = self.path[..depth.min(self.path.len())].to_vec();
                return self.navigate(self.place, path);
            }
            Message::Click(name) => {
                // One click selects (folders too), a second within a moment opens, like the
                // Finder and the web.
                let double = self.last_click.as_ref().is_some_and(|(n, t)| *n == name && t.elapsed() < Duration::from_millis(450));
                self.last_click = Some((name.clone(), Instant::now()));
                self.selected = Some(name.clone());
                if double {
                    self.last_click = None;
                    return self.open_entry(&name);
                }
            }
            Message::Listed(path, Ok(list)) => {
                cache::save_listing(&self.net_id(), &path, &list);
                self.listings.insert(path.clone(), list);
                if path == self.folder() {
                    self.loading = false;
                }
                return Task::batch([self.want_thumbs(), self.prefetch(&path)]);
            }
            Message::Prefetched(path, Ok(list)) => {
                cache::save_listing(&self.net_id(), &path, &list);
                self.listings.insert(path, list);
            }
            Message::Prefetched(_, Err(_)) => {}
            Message::Search(s) => self.search = s,
            Message::Refresh => {
                self.listings.remove(&self.folder());
                return self.load_place();
            }
            Message::Opened(Ok(_)) => self.notice = None,
            Message::Download => {
                let (Some(api), Some(name)) = (self.api.clone(), self.selected.clone()) else { return Task::none() };
                let path = format!("{}/{}", self.folder(), name);
                self.say(format!("Downloading {name}…"));
                return Task::perform(
                    async move {
                        let data = api.download(&path).await?;
                        let dir = dirs::download_dir().unwrap_or_else(std::env::temp_dir);
                        let file = dir.join(&name);
                        tokio::fs::write(&file, data).await.map_err(|e| e.to_string())?;
                        Ok(file)
                    },
                    Message::Downloaded,
                );
            }
            Message::Downloaded(Ok(file)) => self.say(format!("Saved in {}", file.display())),
            Message::Upload => {
                let Some(api) = self.api.clone() else { return Task::none() };
                let folder = self.folder();
                return Task::perform(
                    async move {
                        let Some(files) = rfd::AsyncFileDialog::new().set_title("Upload to TPCloud").pick_files().await else { return Ok(0) };
                        for f in &files {
                            api.upload(&folder, &f.file_name(), f.read().await).await?;
                        }
                        Ok(files.len())
                    },
                    Message::Uploaded,
                );
            }
            Message::Dropped(file) => {
                if !matches!(self.place, Place::Files | Place::Shared) || self.screen != Screen::Main {
                    return Task::none();
                }
                let Some(api) = self.api.clone() else { return Task::none() };
                let folder = self.folder();
                self.say(format!("Uploading {}…", file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()));
                return Task::perform(
                    async move {
                        let name = file.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or("not a file")?;
                        let data = tokio::fs::read(&file).await.map_err(|e| e.to_string())?;
                        api.upload(&folder, &name, data).await?;
                        Ok(1)
                    },
                    Message::Uploaded,
                );
            }
            Message::Uploaded(Ok(n)) => {
                if n > 0 {
                    self.say(format!("Uploaded {n} file{}.", if n == 1 { "" } else { "s" }));
                }
                return self.list();
            }
            Message::Thumb(id, result) => {
                self.in_flight = self.in_flight.saturating_sub(1);
                self.queued.remove(&id);
                if let Ok(bytes) = result {
                    cache::save_thumb(&self.net_id(), &id, &bytes);
                    self.thumbs.insert(id, image::Handle::from_bytes(bytes));
                }
                return self.pump();
            }
            Message::PhotoIndex(Ok(photos)) => {
                self.photos_fresh = true;
                cache::save_photos(&self.net_id(), &photos);
                self.photos = photos;
                return self.want_photo_thumbs();
            }
            Message::PhotoIndex(Err(e)) => {
                if e == "signed-out" {
                    self.signed_out();
                } else {
                    self.error = Some(e);
                }
            }
            Message::PhotoThumbs(asked, got) => {
                self.photo_batches = self.photo_batches.saturating_sub(1);
                for id in &asked {
                    self.photo_loading.remove(id);
                }
                let arrived: HashSet<i64> = got.iter().map(|(id, _)| *id).collect();
                // Ones the server had nothing for aren't asked again in this session.
                self.photo_failed.extend(asked.iter().filter(|id| !arrived.contains(id)));
                self.photo_thumbs.extend(got);
                self.evict_photo_thumbs();
                return self.want_photo_thumbs();
            }
            Message::FilesScrolled(viewport) => self.files_scroll = (viewport.absolute_offset().y, viewport.bounds().height),
            Message::PhotosScrolled(viewport) => {
                self.photos_scroll = (viewport.absolute_offset().y, viewport.bounds().height);
                return self.want_photo_thumbs();
            }
            Message::Blinds(Ok(b)) => self.blinds = b,
            Message::BlindCommand(id, command) => {
                let Some(api) = self.api.clone() else { return Task::none() };
                return Task::perform(async move { api.blind(&id, command).await }, Message::Done);
            }
            Message::Done(Ok(())) => {}
            Message::HostLoaded(Ok(h)) => self.host = Some(h),
            Message::HostLoaded(Err(e)) => {
                if !self.host.as_ref().is_some_and(Host::updating) {
                    self.error = Some(e);
                }
            }
            Message::HostAction(action) => {
                let Some(api) = self.api.clone() else { return Task::none() };
                return Task::perform(async move { api.host_action(action).await.map(|_| action) }, Message::HostActed);
            }
            Message::HostActed(Ok(action)) => {
                if action == "update" {
                    if let Some(h) = self.host.as_mut() {
                        h.update = Some(net::HostUpdate { state: "checking".into(), detail: None });
                    }
                }
                let Some(api) = self.api.clone() else { return Task::none() };
                return Task::perform(
                    async move {
                        tokio::time::sleep(Duration::from_millis(2500)).await;
                        api.host().await
                    },
                    Message::HostLoaded,
                );
            }
            Message::HostActed(Err(e)) => self.error = Some(e),
            Message::Tick => {
                if self.notice.as_ref().is_some_and(|(_, t)| t.elapsed() > Duration::from_secs(4)) {
                    self.notice = None;
                }
                if self.screen == Screen::Main && (self.place == Place::House || (self.place == Place::Account && self.host.as_ref().is_some_and(Host::updating))) {
                    return self.load_place();
                }
            }
            Message::SignOut => {
                if let Some(api) = self.api.clone() {
                    self.signed_out();
                    return Task::perform(async move { api.logout().await }, |_| Message::Done(Ok(())));
                }
            }
            Message::MakeHost => {
                self.busy = true;
                self.error = None;
                return Task::perform(make_host(), Message::HostStarted);
            }
            Message::HostStarted(Ok(page)) => {
                self.busy = false;
                self.say(format!("TPCloud is installed here and waiting to be set up at {page}."));
                let _ = std::process::Command::new("xdg-open").arg(&page).spawn();
            }
            Message::Listed(_, Err(e)) | Message::Opened(Err(e)) | Message::Downloaded(Err(e)) | Message::Uploaded(Err(e)) | Message::Blinds(Err(e)) | Message::Done(Err(e)) => {
                self.loading = false;
                if e == "signed-out" {
                    self.signed_out();
                } else {
                    self.error = Some(e);
                }
            }
            Message::Resized(size) => {
                self.width = size.width;
                self.height = size.height;
            }
            Message::Key(key, mods) => return self.key(key, mods),
            Message::Pref(pref) => {
                let reload = matches!(pref, Pref::View(_));
                self.set_pref(pref);
                if reload {
                    return self.want_thumbs();
                }
            }
            Message::SystemTheme(dark) => {
                if dark != self.system_dark {
                    self.system_dark = dark;
                    self.p = Palette::new(&self.prefs, dark);
                }
            }
        }
        Task::none()
    }

    fn key(&mut self, key: keyboard::Key, mods: keyboard::Modifiers) -> Task<Message> {
        use keyboard::key::Named;
        if self.screen != Screen::Main {
            return Task::none();
        }
        match key.as_ref() {
            keyboard::Key::Named(Named::Backspace) if self.search.is_empty() && !self.path.is_empty() => {
                let up = self.path.len() - 1;
                return self.update(Message::Crumb(up));
            }
            keyboard::Key::Named(Named::ArrowLeft) if mods.alt() => return self.update(Message::Back),
            keyboard::Key::Named(Named::ArrowRight) if mods.alt() => return self.update(Message::Forward),
            keyboard::Key::Named(Named::ArrowUp) if mods.alt() && !self.path.is_empty() => {
                let up = self.path.len() - 1;
                return self.update(Message::Crumb(up));
            }
            // Photos: a screen at a time, or to either end of the library.
            keyboard::Key::Named(k @ (Named::PageDown | Named::PageUp | Named::Home | Named::End)) if self.place == Place::Photos && !self.photos.is_empty() => {
                let total = self.photo_layout().total;
                let (offset, view) = self.photos_scroll;
                let view = if view > 0.0 { view } else { self.height - toolbar() };
                let to = match k {
                    Named::PageDown => offset + view * 0.9,
                    Named::PageUp => offset - view * 0.9,
                    Named::Home => 0.0,
                    _ => total,
                }
                .clamp(0.0, (total - view).max(0.0));
                self.photos_scroll = (to, view);
                return Task::batch([scrollable::scroll_to(scrollable::Id::new("photos"), scrollable::AbsoluteOffset { x: 0.0, y: to }), self.want_photo_thumbs()]);
            }
            keyboard::Key::Named(Named::ArrowUp) if !mods.alt() => return self.move_selection(-(self.columns() as isize)),
            keyboard::Key::Named(Named::ArrowDown) if !mods.alt() => return self.move_selection(self.columns() as isize),
            keyboard::Key::Named(Named::ArrowLeft) if self.prefs.view == View::Icons => return self.move_selection(-1),
            keyboard::Key::Named(Named::ArrowRight) if self.prefs.view == View::Icons => return self.move_selection(1),
            keyboard::Key::Named(Named::Enter) => {
                if let Some(name) = self.selected.clone() {
                    return self.open_entry(&name);
                }
            }
            keyboard::Key::Named(Named::Escape) => {
                self.selected = None;
                self.search.clear();
            }
            keyboard::Key::Character("r") if mods.control() => return self.update(Message::Refresh),
            keyboard::Key::Character("f") if mods.control() => return text_input::focus(text_input::Id::new("search")),
            keyboard::Key::Character(",") if mods.control() => return self.update(Message::Go(Place::Settings)),
            // The sidebar's places, like the Finder's ⌘1–⌘4.
            keyboard::Key::Character("1") if mods.control() => return self.update(Message::Go(Place::Files)),
            keyboard::Key::Character("2") if mods.control() => return self.update(Message::Go(Place::Shared)),
            keyboard::Key::Character("3") if mods.control() => return self.update(Message::Go(Place::Photos)),
            keyboard::Key::Character("4") if mods.control() => return self.update(Message::Go(Place::House)),
            keyboard::Key::Character("i") if mods.control() => return self.update(Message::Go(Place::Account)),
            _ => {}
        }
        Task::none()
    }

    /// What the files view shows, filtered by the search.
    fn shown(&self) -> Vec<&Entry> {
        let needle = self.search.to_lowercase();
        self.entries().iter().filter(|e| needle.is_empty() || e.name.to_lowercase().contains(&needle)).collect()
    }

    fn list_row_height(&self) -> f32 {
        u(if self.p.compact { 34.0 } else { 42.0 })
    }

    /// The icon view's tile width and how many fit in a row.
    fn icon_layout(&self) -> (f32, usize) {
        let icon = u(self.prefs.icon_size.clamp(40.0, 128.0));
        let tile = (icon + u(54.0)).max(u(118.0));
        let usable = (self.width - sidebar() - u(8.0 + 24.0)).max(tile);
        (tile, ((usable / (tile + 4.0)).floor() as usize).max(1))
    }

    /// Items per row: the arrows up and down move by this much.
    fn columns(&self) -> usize {
        if self.prefs.view == View::Icons { self.icon_layout().1 } else { 1 }
    }

    /// The arrows: the next item becomes the selection, scrolled into sight.
    fn move_selection(&mut self, step: isize) -> Task<Message> {
        if !matches!(self.place, Place::Files | Place::Shared) {
            return Task::none();
        }
        let names: Vec<String> = self.shown().iter().map(|e| e.name.clone()).collect();
        if names.is_empty() {
            return Task::none();
        }
        let current = self.selected.as_ref().and_then(|s| names.iter().position(|n| n == s));
        let next = match current {
            None => 0,
            Some(i) => (i as isize + step).clamp(0, names.len() as isize - 1) as usize,
        };
        self.selected = Some(names[next].clone());
        self.last_click = None;
        // Where that item sits in the scrolled list, and whether it's already in view.
        let (top, h) = if self.prefs.view == View::Icons {
            let per_row = self.icon_layout().1;
            let row_h = u(self.prefs.icon_size.clamp(40.0, 128.0)) + u(10.0 + 6.0 + 16.0 + 4.0) + 17.0 * self.p.scale;
            (u(6.0) + (next / per_row) as f32 * row_h, row_h)
        } else {
            (next as f32 * self.list_row_height(), self.list_row_height())
        };
        let (offset, mut view) = self.files_scroll;
        if view <= 0.0 {
            view = self.height - toolbar() - pathbar() - if self.prefs.view == View::List { list_head() } else { 0.0 };
        }
        let to = if top < offset { top } else if top + h > offset + view { top + h - view } else { return Task::none() };
        self.files_scroll.0 = to;
        scrollable::scroll_to(scrollable::Id::new("files"), scrollable::AbsoluteOffset { x: 0.0, y: to.max(0.0) })
    }

    fn reset_network(&mut self) {
        self.listings.clear();
        self.photos.clear();
        self.photos_fresh = false;
        self.photo_thumbs.clear();
        self.photo_loading.clear();
        self.photo_failed.clear();
        self.photos_scroll = (0.0, 0.0);
        self.thumbs.clear();
        self.queue.clear();
        self.queued.clear();
        self.path.clear();
        self.back.clear();
        self.forward.clear();
        self.place = Place::Files;
    }

    fn add(&mut self, address: String) -> Task<Message> {
        self.busy = true;
        self.error = None;
        Task::perform(
            async move {
                let (card, anchor) = net::load_card(&address).await?;
                let reached = net::normalize(&address).unwrap_or_default();
                Ok((card, anchor, reached))
            },
            Message::Added,
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![
            window::resize_events().map(|(_, size)| Message::Resized(size)),
            keyboard::on_key_press(|key, mods| Some(Message::Key(key, mods))),
            iced::event::listen_with(|event, _, _| match event {
                iced::Event::Window(window::Event::FileDropped(path)) => Some(Message::Dropped(path)),
                _ => None,
            }),
            // The desktop's light/dark may change while it's open.
            iced::time::every(Duration::from_secs(5)).map(|_| Message::SystemTheme(theme::system_is_dark())),
        ];
        let fast = self.screen == Screen::Main && self.place == Place::Account && self.host.as_ref().is_some_and(Host::updating);
        let every = if fast { Duration::from_millis(1500) } else if self.place == Place::House || self.notice.is_some() { Duration::from_secs(2) } else { Duration::from_secs(30) };
        subs.push(iced::time::every(every).map(|_| Message::Tick));
        Subscription::batch(subs)
    }

    // ── Views ──

    fn view(&self) -> Element<'_, Message> {
        let p = self.p;
        let body: Element<Message> = match self.screen {
            Screen::Loading => center(column![spinner_dot(p), text("Connecting…").size(p.sz(14.0)).color(p.label2)].spacing(u(10.0)).align_x(Alignment::Center).into()),
            Screen::Networks => self.networks_view(),
            Screen::Login => self.login_view(),
            Screen::Main => self.main_view(),
        };
        container(body).width(Length::Fill).height(Length::Fill).style(theme::window_bg(p)).into()
    }

    fn main_view(&self) -> Element<'_, Message> {
        let p = self.p;
        let content: Element<Message> = match self.place {
            Place::Files | Place::Shared => self.files_view(),
            Place::Photos => self.photos_view(),
            Place::House => self.house_view(),
            Place::Account => self.account_view(),
            Place::Settings => self.settings_view(),
        };
        let mut main = column![content].width(Length::Fill).height(Length::Fill);
        if let Some(e) = &self.error {
            main = main.push(banner(p, e, p.red));
        } else if let Some((n, _)) = &self.notice {
            main = main.push(banner(p, n, p.accent));
        }
        row![container(self.sidebar()).padding(Padding { top: u(8.0), right: 0.0, bottom: u(8.0), left: u(8.0) }).height(Length::Fill), main].into()
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let p = self.p;
        let modules = self.me.as_ref().map(|m| m.modules.clone()).unwrap_or_default();
        let store = modules.is_empty() || modules.iter().any(|m| m == "store");
        let home = modules.iter().any(|m| m == "home");
        let item = |icon: &str, label: &str, place: Place| -> Element<Message> {
            let on = self.place == place;
            let h = if p.compact { 32.0 } else { 36.0 };
            // The row fills the button's height so its contents sit in the middle of it.
            button(row![line_icon(icon, 19.0, p.accent), text(label.to_string()).size(p.sz(15.0)).font(if on { SEMIBOLD } else { theme::INTER })].spacing(u(10.0)).height(Length::Fill).align_y(Alignment::Center))
                .width(Length::Fill)
                .height(h)
                .padding([u(0.0), u(10.0)])
                .style(theme::side_item(p, on))
                .on_press(Message::Go(place))
                .into()
        };
        let group = |label: &str| container(text(label.to_string()).size(p.sz(12.0)).font(BOLD).color(p.label3)).padding(Padding { top: u(16.0), right: u(10.0), bottom: u(5.0), left: u(10.0) });
        let name = self.store.current().map(|n| n.name.clone()).unwrap_or_default();
        let brand = row![
            container(line_icon("cloud", 16.0, Color::WHITE)).width(u(30.0)).height(u(30.0)).center(u(30.0)).style(theme::fill(p, p.accent, 8.0)),
            column![text("TPCloud").size(p.sz(16.0)).font(BOLD), text(name).size(p.sz(12.5)).color(p.label2)].spacing(u(0.0)),
        ]
        .spacing(u(10.0))
        .align_y(Alignment::Center);
        let mut nav = column![container(brand).padding(Padding { top: u(6.0), right: u(10.0), bottom: u(8.0), left: u(10.0) })].spacing(u(1.0));
        if store {
            nav = nav.push(group("Files")).push(item("house", "Home", Place::Files)).push(item("person-2", "Shared", Place::Shared)).push(group("Library")).push(item("photo", "Photos", Place::Photos));
        }
        if home {
            nav = nav.push(group("House")).push(item("blinds", "Blinds", Place::House));
        }
        let email = self.me.as_ref().map(|m| m.email.clone()).unwrap_or_default();
        let initial = email.chars().next().map(|c| c.to_ascii_uppercase().to_string()).unwrap_or_default();
        let account = button(
            row![
                container(text(initial).size(p.sz(13.0)).font(SEMIBOLD).color(Color::WHITE)).width(u(30.0)).height(u(30.0)).center(u(30.0)).style(theme::fill(p, p.accent, 15.0)),
                text(ellipsize(&email, 20)).size(p.sz(14.0)).wrapping(text::Wrapping::None).width(Length::Fill),
            ]
            .spacing(u(10.0))
            .height(Length::Fill)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .height(u(46.0))
        .padding([u(0.0), u(8.0)])
        .style(theme::side_item(p, self.place == Place::Account))
        .on_press(Message::Go(Place::Account));
        // Settings is an item with its name, not a lone gear (it read as a light/dark switch).
        let bottom = column![item("gear", "Settings", Place::Settings), container(Space::with_height(u(0.5))).width(Length::Fill).style(theme::separator(p)), account].spacing(u(6.0));
        container(column![scrollable(nav).height(Length::Fill).style(theme::scroll(p)), container(bottom).padding(Padding { top: u(6.0), ..Padding::ZERO })])
            .width(sidebar())
            .height(Length::Fill)
            .padding([u(10.0), u(10.0)])
            .style(theme::sidebar(p))
            .into()
    }

    fn toolbar<'a>(&'a self, title: String, trailing: Element<'a, Message>) -> Element<'a, Message> {
        let p = self.p;
        let nav = |icon: &str, msg: Option<Message>| button(container(line_icon(icon, 18.0, if msg.is_some() { p.label } else { p.label3 })).center(Length::Fill)).width(u(36.0)).height(u(36.0)).padding(u(0.0)).style(theme::glass_button(p)).on_press_maybe(msg);
        container(
            row![
                nav("chevron-left", (!self.back.is_empty()).then_some(Message::Back)),
                nav("chevron-right", (!self.forward.is_empty()).then_some(Message::Forward)),
                Space::with_width(u(8.0)),
                text(title).size(p.sz(21.0)).font(BOLD),
                horizontal_space(),
                trailing,
            ]
            .spacing(u(8.0))
            .align_y(Alignment::Center),
        )
        .padding(Padding { top: 0.0, right: u(18.0), bottom: 0.0, left: u(16.0) })
        .center_y(toolbar())
        .into()
    }

    fn files_view(&self) -> Element<'_, Message> {
        let p = self.p;
        let root = if self.place == Place::Shared { "Shared" } else { "Home" };
        let title = self.path.last().cloned().unwrap_or_else(|| root.into());
        let view_switch = container(
            row![
                button(container(line_icon("grid", 17.0, p.label)).center(Length::Fill)).width(u(38.0)).height(u(30.0)).padding(u(0.0)).style(theme::segment(p, self.prefs.view == View::Icons)).on_press(Message::Pref(Pref::View(View::Icons))),
                button(container(line_icon("list", 17.0, p.label)).center(Length::Fill)).width(u(38.0)).height(u(30.0)).padding(u(0.0)).style(theme::segment(p, self.prefs.view == View::List)).on_press(Message::Pref(Pref::View(View::List))),
            ]
            .spacing(u(2.0)),
        )
        .padding(u(2.0))
        .style(theme::track(p));
        let search = text_input("Search", &self.search).id(text_input::Id::new("search")).on_input(Message::Search).padding([u(8.0), u(12.0)]).size(p.sz(14.0)).width(u(220.0)).style(theme::field(p));
        let mut trailing = row![view_switch, search].spacing(u(8.0)).align_y(Alignment::Center);
        if self.selected.as_ref().is_some_and(|s| self.entries().iter().any(|e| &e.name == s && e.kind == "file")) {
            trailing = trailing.push(icon_button(p, "download", Message::Download));
        }
        trailing = trailing.push(icon_button(p, "refresh", Message::Refresh));
        trailing = trailing.push(button(row![line_icon("upload", 17.0, Color::WHITE), text("Upload").size(p.sz(14.0)).font(SEMIBOLD)].spacing(u(7.0)).height(Length::Fill).align_y(Alignment::Center)).height(u(36.0)).padding([u(0.0), u(16.0)]).style(theme::prominent(p)).on_press(Message::Upload));
        let needle = self.search.to_lowercase();
        let shown = self.shown();
        let body: Element<Message> = if self.loading && shown.is_empty() {
            center(column![spinner_dot(p), text("Loading…").size(p.sz(13.0)).color(p.label2)].spacing(u(8.0)).align_x(Alignment::Center).into())
        } else if shown.is_empty() {
            center(column![line_icon("folder", 44.0, p.label3), text(if needle.is_empty() { "This folder is empty" } else { "Nothing matches" }).size(p.sz(17.0)).font(SEMIBOLD).color(p.label2), text(if needle.is_empty() { "Drop files here, or use Upload." } else { "" }).size(p.sz(14.0)).color(p.label3)].spacing(u(8.0)).align_x(Alignment::Center).into())
        } else if self.prefs.view == View::Icons {
            self.icon_grid(&shown)
        } else {
            self.list_table(&shown)
        };
        column![self.toolbar(title, trailing.into()), body, self.pathbar(shown.len())].into()
    }

    fn icon_grid(&self, shown: &[&Entry]) -> Element<'_, Message> {
        let p = self.p;
        let icon = u(self.prefs.icon_size.clamp(40.0, 128.0));
        let (tile, per_row) = self.icon_layout();
        let folder = self.folder();
        let mut grid = Column::new().spacing(u(4.0)).padding([u(6.0), u(12.0)]);
        for chunk in shown.chunks(per_row) {
            let mut r = Row::new().spacing(u(4.0));
            for e in chunk {
                let picture: Element<Message> = if e.kind == "directory" {
                    svg(icons::folder()).width(icon).height(icon * 0.82).into()
                } else {
                    let key = ThumbKey::File(format!("{folder}/{}", e.name), e.modified.unwrap_or(0.0) as u64 ^ e.size.unwrap_or(0)).id();
                    match self.thumbs.get(&key) {
                        Some(h) => image(h.clone()).width(icon).height(icon).content_fit(iced::ContentFit::Contain).into(),
                        None => svg(icons::file(&e.name)).width(icon * 0.8).height(icon).into(),
                    }
                };
                let selected = self.selected.as_deref() == Some(e.name.as_str());
                let label = text(ellipsize(&e.name, ((tile / 7.5) as usize).max(10))).size(p.sz(13.0)).align_x(Alignment::Center);
                // Selected like web/store's: the picture on fill-3, the name on the accent.
                let label: Element<Message> = if selected {
                    container(label.color(Color::WHITE)).padding([u(1.0), u(6.0)]).style(theme::fill(p, p.accent, 5.0)).into()
                } else {
                    container(label).padding([u(1.0), u(6.0)]).into()
                };
                let pic = container(picture).width(icon + 16.0).height(icon + 10.0).center_x(icon + 16.0).align_y(Alignment::Center);
                let pic = if selected { pic.style(theme::fill(p, p.fill3, 10.0)) } else { pic };
                r = r.push(
                    button(column![pic, label].spacing(u(6.0)).align_x(Alignment::Center))
                        .width(tile)
                        .padding([u(8.0), u(4.0)])
                        .style(theme::item(p, selected, 12.0))
                        .on_press(Message::Click(e.name.clone())),
                );
            }
            grid = grid.push(r);
        }
        scrollable(grid).id(scrollable::Id::new("files")).on_scroll(Message::FilesScrolled).height(Length::Fill).width(Length::Fill).style(theme::scroll(p)).into()
    }

    fn list_table(&self, shown: &[&Entry]) -> Element<'_, Message> {
        let p = self.p;
        let h = self.list_row_height();
        // Name, Date Modified, Size, Kind: the same columns (and the same gaps) in the header and
        // in every row, so they line up.
        let cols = |name: Element<'static, Message>, date: Element<'static, Message>, size: Element<'static, Message>, kind: Element<'static, Message>| {
            row![
                // web/store's columns: the name takes what's left, then 190 / 100 / 160.
                container(name).width(Length::Fill).clip(true),
                container(date).width(u(190.0)),
                container(size).width(u(100.0)).align_x(Alignment::End),
                container(kind).width(u(160.0)).padding(Padding { left: u(16.0), ..Padding::ZERO }),
            ]
            .spacing(u(12.0))
            .height(Length::Fill)
            .align_y(Alignment::Center)
        };
        let head = |label: &str| -> Element<'static, Message> { text(label.to_string()).size(p.sz(12.5)).font(SEMIBOLD).color(p.label2).into() };
        let header = container(cols(row![Space::with_width(u(34.0)), head("Name")].into(), head("Date Modified"), head("Size"), head("Kind")))
            .height(list_head())
            .padding([u(0.0), u(22.0)]);
        let mut list = Column::new().spacing(u(0.0)).padding(Padding { top: u(2.0), right: u(12.0), bottom: u(6.0), left: u(12.0) });
        for (i, e) in shown.iter().enumerate() {
            let selected = self.selected.as_deref() == Some(e.name.as_str());
            // Selected rows are the accent with white text, like the web (and the Finder).
            let (fg, fg2) = if selected { (Color::WHITE, Color::WHITE) } else { (p.label, p.label2) };
            let icon: Element<'static, Message> = if e.kind == "directory" { svg(icons::folder()).width(u(24.0)).height(u(20.0)).into() } else { svg(icons::file(&e.name)).width(u(19.0)).height(u(24.0)).into() };
            let kind = if e.kind == "directory" { "Folder" } else { icons::kind_of(&e.name).1 };
            let date = e.modified.map(when).unwrap_or_default();
            let size = if e.kind == "directory" { "—".to_string() } else { e.size.map(human).unwrap_or_default() };
            let cell = |t: String, c: Color| -> Element<'static, Message> { text(t).size(p.sz(13.5)).color(c).wrapping(text::Wrapping::None).into() };
            let name: Element<'static, Message> = row![container(icon).width(u(24.0)).center_x(u(24.0)), text(e.name.clone()).size(p.sz(15.0)).color(fg).wrapping(text::Wrapping::None)].spacing(u(10.0)).align_y(Alignment::Center).into();
            let line = cols(name, cell(date, fg2), cell(size, fg2), cell(kind.to_string(), fg2));
            list = list.push(button(line).width(Length::Fill).height(h).padding([u(0.0), u(10.0)]).style(theme::list_row(p, selected, i % 2 == 1)).on_press(Message::Click(e.name.clone())));
        }
        column![
            header,
            container(Space::with_height(u(0.5))).width(Length::Fill).style(theme::separator(p)),
            scrollable(list).id(scrollable::Id::new("files")).on_scroll(Message::FilesScrolled).height(Length::Fill).style(theme::scroll(p))
        ]
        .into()
    }

    fn pathbar(&self, count: usize) -> Element<'_, Message> {
        let p = self.p;
        let root = if self.place == Place::Shared { "Shared" } else { "Home" };
        let mut crumbs = Row::new().spacing(u(4.0)).align_y(Alignment::Center);
        crumbs = crumbs.push(button(row![line_icon(if self.place == Place::Shared { "person-2" } else { "house" }, 15.0, p.accent), text(root).size(p.sz(13.0))].spacing(u(6.0)).align_y(Alignment::Center)).padding([u(3.0), u(7.0)]).style(theme::side_item(p, false)).on_press(Message::Crumb(0)));
        for (i, part) in self.path.iter().enumerate() {
            crumbs = crumbs.push(line_icon("chevron-right", 11.0, p.label3));
            crumbs = crumbs.push(button(row![svg(icons::folder()).width(u(16.0)).height(u(13.0)), text(part.clone()).size(p.sz(13.0))].spacing(u(6.0)).align_y(Alignment::Center)).padding([u(3.0), u(7.0)]).style(theme::side_item(p, false)).on_press(Message::Crumb(i + 1)));
        }
        let items = format!("{count} item{}", if count == 1 { "" } else { "s" });
        column![
            container(Space::with_height(u(0.5))).width(Length::Fill).style(theme::separator(p)),
            container(row![crumbs, horizontal_space(), text(items).size(p.sz(12.5)).color(p.label2)].align_y(Alignment::Center)).padding([u(0.0), u(14.0)]).center_y(pathbar()),
        ]
        .into()
    }

    fn photos_view(&self) -> Element<'_, Message> {
        let p = self.p;
        let body: Element<Message> = if self.photos.is_empty() {
            center(column![line_icon("photo", 44.0, p.label3), text("No photos yet").size(p.sz(17.0)).font(SEMIBOLD).color(p.label2), text("The iPhone app backs them up here.").size(p.sz(14.0)).color(p.label3)].spacing(u(8.0)).align_x(Alignment::Center).into())
        } else {
            // Only the rows near the screen are built; the rest are empty space of the right
            // height, so the scroll bar and position stay true (iced builds every widget each
            // frame: 12,000 tiles as one column made the whole app crawl).
            let near = self.photo_rows_near();
            let l = self.photo_layout();
            let mut col = Column::new().padding([u(4.0), u(14.0)]);
            let before = l.tops.get(near.start).copied().unwrap_or(0.0);
            if before > 0.0 {
                col = col.push(Space::with_height(before));
            }
            for row in &l.rows[near.clone()] {
                col = col.push(match row {
                    PhotoRow::Month(label) => Element::from(container(text(label.clone()).size(p.sz(22.0)).font(BOLD)).height(l.month_h).padding(Padding { top: u(18.0), right: 0.0, bottom: u(8.0), left: u(2.0) }).align_y(Alignment::End)),
                    PhotoRow::Tiles(a, b) => {
                        let mut r = Row::new().spacing(u(2.0));
                        for ph in &self.photos[*a..*b] {
                            r = r.push(match self.photo_thumbs.get(&ph.id) {
                                Some(h) => Element::from(image(h.clone()).width(l.cell).height(l.cell).content_fit(iced::ContentFit::Cover)),
                                None => container(Space::new(l.cell, l.cell)).style(theme::fill(p, p.fill4, 0.0)).into(),
                            });
                        }
                        container(r).height(l.row_h).into()
                    }
                });
            }
            let after = l.total - l.tops.get(near.end).copied().unwrap_or(l.total);
            if after > 0.0 {
                col = col.push(Space::with_height(after));
            }
            scrollable(col).id(scrollable::Id::new("photos")).height(Length::Fill).width(Length::Fill).on_scroll(Message::PhotosScrolled).style(theme::scroll(p)).into()
        };
        let count = text(format!("{} photos", self.photos.len())).size(p.sz(13.5)).color(p.label2);
        column![self.toolbar("Photos".into(), count.into()), body].into()
    }

    fn house_view(&self) -> Element<'_, Message> {
        let p = self.p;
        let orange = p.orange;
        let mut grid = Column::new().spacing(u(12.0)).padding([u(4.0), u(18.0)]);
        let usable = (self.width - sidebar() - 8.0 - 36.0).max(260.0);
        let per_row = ((usable / 260.0).floor() as usize).max(1);
        for chunk in self.blinds.chunks(per_row) {
            let mut r = Row::new().spacing(u(12.0));
            for b in chunk {
                let pos = b.state.as_ref().map(|s| s.position).unwrap_or(-1);
                let open = pos.clamp(0, 100) as f32 / 100.0;
                let capsule = container(column![container(Space::with_height(Length::Fill)).height(Length::FillPortion(((1.0 - open) * 100.0) as u16 + 1)).width(Length::Fill).style(theme::fill(p, theme::alpha(orange, 0.85), 10.0)), Space::with_height(Length::FillPortion((open * 100.0) as u16 + 1))])
                    .width(u(44.0))
                    .height(u(86.0))
                    .padding(u(3.0))
                    .style(theme::fill(p, p.fill3, 12.0));
                let ctl = |icon: &str, cmd: &'static str| button(container(line_icon(icon, 15.0, p.label)).center(u(15.0))).width(u(34.0)).height(u(34.0)).style(theme::glass_button(p)).on_press(Message::BlindCommand(b.id.clone(), cmd));
                let state = if pos < 0 { "—".to_string() } else if pos >= 100 { "Open".into() } else if pos <= 0 { "Closed".into() } else { format!("{pos}% open") };
                let card = container(
                    row![
                        capsule,
                        column![text(b.name.clone()).size(p.sz(15.0)).font(SEMIBOLD), text(state).size(p.sz(12.5)).color(p.label2), Space::with_height(u(6.0)), row![ctl("chevron-up", "up"), ctl("stop", "stop"), ctl("chevron-down", "down")].spacing(u(6.0))].spacing(u(2.0)).width(Length::Fill),
                    ]
                    .spacing(u(14.0))
                    .align_y(Alignment::Center),
                )
                .padding(u(14.0))
                .width(Length::Fill)
                .style(theme::card(p));
                r = r.push(card);
            }
            for _ in chunk.len()..per_row {
                r = r.push(Space::with_width(Length::Fill));
            }
            grid = grid.push(r);
        }
        let body: Element<Message> = if self.blinds.is_empty() { center(text("No blinds yet").size(p.sz(15.0)).color(p.label2).into()) } else { scrollable(grid).height(Length::Fill).style(theme::scroll(p)).into() };
        column![self.toolbar("Blinds".into(), Space::with_width(u(0.0)).into()), body].into()
    }

    fn account_view(&self) -> Element<'_, Message> {
        let p = self.p;
        let me = self.me.as_ref();
        let net = self.store.current();
        let mut col = column![
            section(p, "Account", vec![
                kv_row(p, "person", "Email", me.map(|m| m.email.clone()).unwrap_or_default()),
                kv_row(p, "lock", "Role", if me.is_some_and(|m| m.is_admin) { "Administrator".into() } else { "Member".into() }),
            ]),
            section(p, "Network", vec![
                kv_row(p, "cloud", "Name", net.map(|n| n.name.clone()).unwrap_or_default()),
                kv_row(p, "network", "Connected at", self.api.as_ref().map(|a| a.base.clone()).unwrap_or_default()),
                action_row(p, "network", "Other networks…", Message::ShowNetworks, false),
            ]),
        ]
        .spacing(u(4.0));
        if let Some(h) = &self.host {
            let state = h.update.as_ref().map(|u| u.state.as_str());
            let line = match state {
                Some("checking") => "Looking for the new version…".to_string(),
                Some("downloading") => "Downloading…".to_string(),
                Some("installing") => "Installing and restarting…".to_string(),
                Some("done") => format!("Updated to {}.", h.update.as_ref().and_then(|u| u.detail.clone()).unwrap_or_default()),
                Some("failed") => format!("The update failed: {}", h.update.as_ref().and_then(|u| u.detail.clone()).unwrap_or_default()),
                _ if h.available => format!("TPCloud {} is out.", h.latest.clone().unwrap_or_default()),
                _ => "This is the latest version.".to_string(),
            };
            let mut rows = vec![kv_row(p, "server", "Version", format!("TPCloud {}", h.version)), kv_row(p, "info", "Status", line)];
            if !h.updating() {
                rows.push(action_row(p, "refresh", "Look for updates", Message::HostAction("check"), false));
                if h.available && h.can_update {
                    rows.push(action_row(p, "download", &format!("Update to {}", h.latest.clone().unwrap_or_default()), Message::HostAction("update"), false));
                }
            }
            col = col.push(section(p, "Server", rows));
        }
        col = col.push(section(p, "", vec![action_row(p, "sign-out", "Sign out", Message::SignOut, true)]));
        let page = container(col.max_width(u(640.0))).padding([u(8.0), u(24.0)]).center_x(Length::Fill);
        column![self.toolbar("Account".into(), Space::with_width(u(0.0)).into()), scrollable(page).height(Length::Fill).style(theme::scroll(p))].into()
    }

    fn settings_view(&self) -> Element<'_, Message> {
        let p = self.p;
        let pr = &self.prefs;
        let seg = |items: Vec<(&'static str, bool, Message)>| -> Element<Message> {
            let n = items.len();
            let mut r = Row::new().spacing(u(2.0));
            for (label, on, msg) in items {
                r = r.push(button(text(label).size(p.sz(13.5)).font(if on { SEMIBOLD } else { theme::INTER }).align_x(Alignment::Center).width(Length::Fill)).width(Length::Fill).padding([u(6.0), u(10.0)]).style(theme::segment(p, on)).on_press(msg));
            }
            container(r).padding(u(2.0)).width(u(if n > 3 { 380.0 } else { 280.0 })).style(theme::track(p)).into()
        };
        let mut swatches = Row::new().spacing(u(10.0)).align_y(Alignment::Center);
        for a in theme::Accent::ALL {
            let on = pr.accent == a;
            let dot = button(container(if on { Element::from(line_icon("checkmark", 11.0, Color::WHITE)) } else { Space::new(u(11.0), u(11.0)).into() }).center(u(11.0))).width(u(22.0)).height(u(22.0)).style(theme::swatch(p, a.color(p.dark), on)).on_press(Message::Pref(Pref::Accent(a)));
            swatches = swatches.push(dot);
        }
        let range = |value: f32, lo: f32, hi: f32, f: fn(f32) -> Pref, shown: String| -> Element<Message> {
            row![slider(lo..=hi, value, move |v| Message::Pref(f(v))).step(0.01).width(u(200.0)).style(theme::range(p)), text(shown).size(p.sz(13.5)).color(p.label2).width(u(56.0))].spacing(u(12.0)).align_y(Alignment::Center).into()
        };
        let pct = |v: f32| format!("{}%", (v * 100.0).round());
        let col = column![
            section(p, "Appearance", vec![
                setting_row(p, "Size", seg(vec![
                    ("100%", pr.size < 1.12, Message::Pref(Pref::Size(1.0))),
                    ("125%", (1.12..1.37).contains(&pr.size), Message::Pref(Pref::Size(1.25))),
                    ("150%", (1.37..1.62).contains(&pr.size), Message::Pref(Pref::Size(1.5))),
                    ("175%", (1.62..1.87).contains(&pr.size), Message::Pref(Pref::Size(1.75))),
                    ("200%", pr.size >= 1.87, Message::Pref(Pref::Size(2.0))),
                ])),
                setting_row(p, "Theme", seg(vec![("System", pr.mode == theme::Mode::System, Message::Pref(Pref::Mode(theme::Mode::System))), ("Light", pr.mode == theme::Mode::Light, Message::Pref(Pref::Mode(theme::Mode::Light))), ("Dark", pr.mode == theme::Mode::Dark, Message::Pref(Pref::Mode(theme::Mode::Dark)))])),
                setting_row(p, "Accent colour", swatches.into()),
                setting_row(p, "Text size", seg(vec![("Small", pr.text_scale < 0.95, Message::Pref(Pref::TextScale(0.9))), ("Medium", (0.95..1.05).contains(&pr.text_scale), Message::Pref(Pref::TextScale(1.0))), ("Large", pr.text_scale >= 1.05, Message::Pref(Pref::TextScale(1.12)))])),
                setting_row(p, "Density", seg(vec![("Regular", !pr.compact, Message::Pref(Pref::Compact(false))), ("Compact", pr.compact, Message::Pref(Pref::Compact(true)))])),
                setting_row(p, "Corners", range(pr.roundness, 0.2, 1.6, Pref::Roundness, if (pr.roundness - 1.0).abs() < 0.05 { "Default".into() } else { pct(pr.roundness) })),
            ]),
            section(p, "Transparency", vec![
                setting_row(p, "Window", range(pr.window_opacity, 0.3, 1.0, Pref::WindowOpacity, pct(pr.window_opacity))),
                setting_row(p, "Sidebar", range(pr.sidebar_opacity, 0.0, 1.0, Pref::SidebarOpacity, pct(pr.sidebar_opacity))),
            ]),
            container(
                text(if theme::can_set_window_opacity() {
                    "Below 100% the desktop shows through, blurred by Hyprland (if blur is on in its decoration settings)."
                } else {
                    "Window transparency needs a compositor that lets apps set it (Hyprland); here the window stays solid."
                })
                .size(p.sz(13.0))
                .color(p.label2),
            )
            .padding(Padding { top: 0.0, right: u(16.0), bottom: u(10.0), left: u(16.0) }),
            section(p, "Files", vec![
                setting_row(p, "Show as", seg(vec![("Icons", pr.view == View::Icons, Message::Pref(Pref::View(View::Icons))), ("List", pr.view == View::List, Message::Pref(Pref::View(View::List)))])),
                setting_row(p, "Icon size", range(pr.icon_size, 40.0, 128.0, |v| Pref::IconSize((v / 4.0).round() * 4.0), format!("{} pt", pr.icon_size as u32))),
            ]),
            section(p, "", vec![action_row(p, "refresh", "Back to the default look", Message::Pref(Pref::Reset), false)]),
        ]
        .spacing(u(4.0));
        let page = container(col.max_width(u(640.0))).padding([u(8.0), u(24.0)]).center_x(Length::Fill);
        column![self.toolbar("Settings".into(), Space::with_width(u(0.0)).into()), scrollable(page).height(Length::Fill).style(theme::scroll(p))].into()
    }

    fn networks_view(&self) -> Element<'_, Message> {
        let p = self.p;
        let mut rows: Vec<Element<Message>> = Vec::new();
        for n in &self.store.networks {
            let current = self.store.current.as_deref() == Some(&n.id);
            rows.push(
                row![
                    button(row![line_icon("cloud", 18.0, p.accent), column![text(n.name.clone()).size(p.sz(14.0)).font(SEMIBOLD), text(n.last_address.clone().unwrap_or_default()).size(p.sz(12.0)).color(p.label2)].spacing(u(1.0)).width(Length::Fill), if current { Element::from(line_icon("checkmark", 15.0, p.accent)) } else { Space::with_width(u(0.0)).into() }].spacing(u(10.0)).align_y(Alignment::Center))
                        .width(Length::Fill)
                        .padding([u(8.0), u(12.0)])
                        .style(theme::side_item(p, false))
                        .on_press(Message::Use(n.id.clone())),
                    button(line_icon("trash", 15.0, p.label3)).padding(u(8.0)).style(theme::side_item(p, false)).on_press(Message::Remove(n.id.clone())),
                ]
                .align_y(Alignment::Center)
                .into(),
            );
        }
        let known: Vec<&str> = self.store.networks.iter().map(|n| n.id.as_str()).collect();
        let mut found: Vec<Element<Message>> = Vec::new();
        for f in self.found.iter().filter(|f| f.network_id.as_deref().map_or(true, |id| !known.contains(&id))) {
            found.push(button(row![line_icon("network", 18.0, p.accent), column![text(f.name.clone()).size(p.sz(14.0)).font(SEMIBOLD), text(f.url.clone()).size(p.sz(12.0)).color(p.label2)].spacing(u(1.0))].spacing(u(10.0)).align_y(Alignment::Center)).width(Length::Fill).padding([u(8.0), u(12.0)]).style(theme::side_item(p, false)).on_press(Message::AddFound(f.url.clone())).into());
        }
        for w in &self.waiting {
            found.push(button(row![line_icon("server", 18.0, p.orange), column![text(format!("{} · waiting to be set up", w.name)).size(p.sz(14.0)).font(SEMIBOLD), text(w.url.clone()).size(p.sz(12.0)).color(p.label2)].spacing(u(1.0))].spacing(u(10.0)).align_y(Alignment::Center)).width(Length::Fill).padding([u(8.0), u(12.0)]).style(theme::side_item(p, false)).on_press(Message::HostStarted(Ok(w.url.clone()))).into());
        }
        if found.is_empty() {
            found.push(container(text(if self.looking { "Looking on this network…" } else { "Nothing found here." }).size(p.sz(13.0)).color(p.label2)).padding([u(8.0), u(12.0)]).into());
        }
        let add = row![
            text_input("tpcloud.local or cloud.example.com", &self.address).on_input(Message::Address).on_submit(Message::Add).padding([u(9.0), u(12.0)]).size(p.sz(14.0)).style(theme::field(p)),
            button(text(if self.busy { "Adding…" } else { "Add" }).size(p.sz(14.0)).font(SEMIBOLD)).padding([u(9.0), u(18.0)]).style(theme::prominent(p)).on_press_maybe((!self.busy && !self.address.trim().is_empty()).then_some(Message::Add)),
        ]
        .spacing(u(8.0))
        .align_y(Alignment::Center);
        let mut col = column![
            row![container(line_icon("cloud", 22.0, Color::WHITE)).width(u(40.0)).height(u(40.0)).center(u(40.0)).style(theme::fill(p, p.accent, 11.0)), column![text("TPCloud").size(p.sz(28.0)).font(BOLD), text("Your files, photos and home.").size(p.sz(14.0)).color(p.label2)].spacing(u(0.0))].spacing(u(14.0)).align_y(Alignment::Center),
            Space::with_height(u(8.0)),
        ]
        .spacing(u(10.0))
        .max_width(u(560.0));
        if !rows.is_empty() {
            col = col.push(group_card(p, "Your networks", rows));
        }
        col = col.push(group_card(p, "On this network", found)).push(row![horizontal_space(), button(text("Look again").size(p.sz(13.0))).style(theme::plain(p)).on_press(Message::Look)]);
        col = col.push(group_card(p, "Add one by its address", vec![container(add).padding(u(10.0)).into()]));
        col = col.push(group_card(p, "This computer", vec![row![column![text("Make it the host").size(p.sz(14.0)).font(SEMIBOLD), text("Installs TPCloud here (asks for your password) and opens its setup.").size(p.sz(12.0)).color(p.label2)].spacing(u(2.0)).width(Length::Fill), button(text("Set up").size(p.sz(13.0)).font(SEMIBOLD)).padding([u(7.0), u(14.0)]).style(theme::glass_button(p)).on_press_maybe((!self.busy).then_some(Message::MakeHost))].align_y(Alignment::Center).padding([u(10.0), u(12.0)]).into()]));
        if let Some(e) = &self.error {
            col = col.push(text(e.clone()).size(p.sz(13.0)).color(p.red));
        }
        container(scrollable(container(col).padding(u(36.0)).center_x(Length::Fill)).style(theme::scroll(p))).height(Length::Fill).into()
    }

    fn login_view(&self) -> Element<'_, Message> {
        let p = self.p;
        let name = self.store.current().map(|n| n.name.clone()).unwrap_or_default();
        let field = |hint: &str, value: &str| text_input(hint, value).padding([u(11.0), u(14.0)]).size(p.sz(15.0)).style(theme::field(p));
        let form: Element<Message> = if self.totp {
            column![
                text("Enter the 6-digit code from your authenticator app.").size(p.sz(13.5)).color(p.label2),
                text_input("123456", &self.code).on_input(Message::Code).on_submit(Message::Verify).padding([u(12.0), u(14.0)]).size(p.sz(22.0)).style(theme::field(p)),
                button(text("Continue").size(p.sz(15.0)).font(SEMIBOLD).align_x(Alignment::Center).width(Length::Fill)).padding([u(12.0), u(18.0)]).width(Length::Fill).style(theme::prominent(p)).on_press_maybe((self.code.len() == 6 && !self.busy).then_some(Message::Verify)),
            ]
            .spacing(u(10.0))
            .into()
        } else {
            column![
                field("Email", &self.email).on_input(Message::Email),
                field("Password", &self.password).on_input(Message::Password).on_submit(Message::SignIn).secure(true),
                button(text(if self.busy { "Signing in…" } else { "Sign in" }).size(p.sz(15.0)).font(SEMIBOLD).align_x(Alignment::Center).width(Length::Fill)).padding([u(12.0), u(18.0)]).width(Length::Fill).style(theme::prominent(p)).on_press_maybe((!self.busy && !self.email.is_empty() && !self.password.is_empty()).then_some(Message::SignIn)),
            ]
            .spacing(u(10.0))
            .into()
        };
        let mut col = column![
            container(line_icon("cloud", 26.0, Color::WHITE)).width(u(52.0)).height(u(52.0)).center(u(52.0)).style(theme::fill(p, p.accent, 14.0)),
            text("Sign in").size(p.sz(26.0)).font(BOLD),
            text(format!("to {name}")).size(p.sz(14.0)).color(p.label2),
            Space::with_height(u(6.0)),
            form,
            button(text("Another network").size(p.sz(13.0))).style(theme::plain(p)).on_press(Message::ShowNetworks),
        ]
        .spacing(u(8.0))
        .align_x(Alignment::Center);
        if let Some(e) = &self.error {
            col = col.push(text(e.clone()).size(p.sz(13.0)).color(p.red));
        }
        center(container(col).width(u(360.0)).padding(u(28.0)).style(theme::glass(p)).into())
    }
}

// ── Pieces ──

fn line_icon<'a>(name: &str, size: f32, color: Color) -> Element<'a, Message> {
    svg(icons::line(name)).width(u(size)).height(u(size)).style(move |_, _| svg::Style { color: Some(color) }).into()
}

fn icon_button<'a>(p: Palette, icon: &str, msg: Message) -> Element<'a, Message> {
    button(container(line_icon(icon, 17.0, p.label)).center(Length::Fill)).width(u(36.0)).height(u(36.0)).padding(u(0.0)).style(theme::glass_button(p)).on_press(msg).into()
}

fn center(content: Element<'_, Message>) -> Element<'_, Message> {
    container(content).center(Length::Fill).into()
}

fn spinner_dot<'a>(p: Palette) -> Element<'a, Message> {
    container(Space::new(u(10.0), u(10.0))).style(theme::fill(p, theme::alpha(p.accent, 0.8), 5.0)).into()
}

fn banner<'a>(p: Palette, message: &str, color: Color) -> Element<'a, Message> {
    container(row![container(Space::new(u(8.0), u(8.0))).style(theme::fill(p, color, 4.0)), text(message.to_string()).size(p.sz(13.5)).color(p.label)].spacing(u(8.0)).align_y(Alignment::Center)).padding([u(10.0), u(18.0)]).width(Length::Fill).into()
}

/// A grouped-list section: its header, then the rows in a card with separators between them.
fn section<'a>(p: Palette, title: &str, rows: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    group_card(p, title, rows)
}

fn group_card<'a>(p: Palette, title: &str, rows: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let mut inner = Column::new();
    let n = rows.len();
    for (i, r) in rows.into_iter().enumerate() {
        inner = inner.push(r);
        if i + 1 < n {
            inner = inner.push(container(container(Space::with_height(u(0.5))).width(Length::Fill).style(theme::separator(p))).padding(Padding { left: u(12.0), ..Padding::ZERO }));
        }
    }
    let mut col = Column::new().spacing(u(6.0));
    if !title.is_empty() {
        col = col.push(container(text(title.to_string()).size(p.sz(13.5)).font(MEDIUM).color(p.label2)).padding(Padding { top: u(16.0), right: 0.0, bottom: 0.0, left: u(16.0) }));
    } else {
        col = col.push(Space::with_height(u(14.0)));
    }
    col.push(container(inner).width(Length::Fill).style(theme::card(p))).into()
}

fn kv_row<'a>(p: Palette, icon: &str, label: &str, value: String) -> Element<'a, Message> {
    row![line_icon(icon, 18.0, p.accent), text(label.to_string()).size(p.sz(15.0)).width(Length::Fill), text(value).size(p.sz(15.0)).color(p.label2)].spacing(u(12.0)).align_y(Alignment::Center).padding([u(0.0), u(16.0)]).height(u(46.0)).into()
}

fn setting_row<'a>(p: Palette, label: &str, control: Element<'a, Message>) -> Element<'a, Message> {
    row![text(label.to_string()).size(p.sz(15.0)).width(Length::Fill), control].spacing(u(12.0)).align_y(Alignment::Center).padding([u(8.0), u(16.0)]).height(Length::Shrink).into()
}

fn action_row<'a>(p: Palette, icon: &str, label: &str, msg: Message, danger: bool) -> Element<'a, Message> {
    let color = if danger { p.red } else { p.accent };
    button(row![line_icon(icon, 18.0, color), text(label.to_string()).size(p.sz(15.0)).color(color).width(Length::Fill)].spacing(u(12.0)).height(Length::Fill).align_y(Alignment::Center))
        .width(Length::Fill)
        .height(u(46.0))
        .padding([u(0.0), u(16.0)])
        .style(theme::side_item(p, false))
        .on_press(msg)
        .into()
}

fn ellipsize(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_string();
    }
    let keep = max.saturating_sub(1);
    let head: String = chars[..keep * 2 / 3].iter().collect();
    let tail: String = chars[chars.len() - (keep - keep * 2 / 3)..].iter().collect();
    format!("{head}…{tail}")
}

fn human(bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1000.0 && u < units.len() - 1 {
        v /= 1000.0;
        u += 1;
    }
    if u == 0 { format!("{bytes} B") } else { format!("{v:.1} {}", units[u]) }
}

/// «24 Nov 2026 at 18:30», in local time.
fn when(secs: f64) -> String {
    let t = secs as libc_time_t;
    let mut tm: Tm = unsafe { std::mem::zeroed() };
    unsafe { localtime_r(&t, &mut tm) };
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    format!("{} {} {} at {:02}:{:02}", tm.tm_mday, MONTHS[(tm.tm_mon as usize).min(11)], tm.tm_year + 1900, tm.tm_hour, tm.tm_min)
}

/// A photo's month as shown over its rows: «November 2026», local time.
fn photo_month(photo: &Photo) -> String {
    const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    if let Some(ms) = photo.taken_ms {
        let t = (ms / 1000) as libc_time_t;
        let mut tm: Tm = unsafe { std::mem::zeroed() };
        unsafe { localtime_r(&t, &mut tm) };
        return format!("{} {}", MONTHS[(tm.tm_mon as usize).min(11)], tm.tm_year + 1900);
    }
    photo.taken_at.as_deref().map(month_label).unwrap_or_else(|| "Undated".into())
}

/// A thumbnail decoded here (not by the renderer on the UI thread when it's first drawn),
/// cropped square and shrunk to the tile's size so a screenful costs little memory.
fn square_thumb(bytes: &[u8], side: u32) -> Option<image::Handle> {
    let img = ::image::load_from_memory(bytes).ok()?;
    let (w, h) = (img.width(), img.height());
    let s = w.min(h).max(1);
    let img = img.crop_imm((w - s) / 2, (h - s) / 2, s, s);
    let side = side.min(s);
    let img = img.resize_exact(side, side, ::image::imageops::FilterType::Triangle).to_rgba8();
    Some(image::Handle::from_rgba(side, side, img.into_raw()))
}

/// «November 2026» from the photo's ISO date.
fn month_label(iso: &str) -> String {
    const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    let year = iso.get(0..4).unwrap_or("");
    let month: usize = iso.get(5..7).and_then(|m| m.parse().ok()).unwrap_or(1);
    format!("{} {year}", MONTHS[month.clamp(1, 12) - 1])
}

#[allow(non_camel_case_types)]
type libc_time_t = i64;

#[repr(C)]
struct Tm {
    tm_sec: i32,
    tm_min: i32,
    tm_hour: i32,
    tm_mday: i32,
    tm_mon: i32,
    tm_year: i32,
    tm_wday: i32,
    tm_yday: i32,
    tm_isdst: i32,
    tm_gmtoff: i64,
    tm_zone: *const i8,
}

extern "C" {
    fn localtime_r(t: *const libc_time_t, tm: *mut Tm) -> *mut Tm;
}

/// Downloads the server program for this machine and installs it with pkexec (the system asks for
/// the password). With no terminal it starts as the setup page, which is where it's set up.
async fn make_host() -> Result<String, String> {
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "amd64",
        other => return Err(format!("There's no TPCloud server for {other} yet.")),
    };
    let bytes = reqwest::get(format!("https://tpcloud.migueltaibo.com/download/linux/{arch}")).await.map_err(|e| e.to_string())?.bytes().await.map_err(|e| e.to_string())?;
    let file = std::env::temp_dir().join("tpcloud-server");
    tokio::fs::write(&file, &bytes).await.map_err(|e| e.to_string())?;
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    let status = tokio::process::Command::new("pkexec")
        .arg(&file)
        .arg("install")
        .stdin(std::process::Stdio::null())
        .status()
        .await
        .map_err(|e| format!("Couldn't ask for permission (is polkit running?): {e}"))?;
    let _ = std::fs::remove_file(&file);
    if !status.success() {
        return Err("Installing was cancelled or failed.".into());
    }
    for port in [80, 8090] {
        let url = if port == 80 { "http://localhost".to_string() } else { format!("http://localhost:{port}") };
        if reqwest::get(format!("{url}/api/setup")).await.map(|r| r.status().is_success()).unwrap_or(false) {
            return Ok(url);
        }
    }
    Ok("http://localhost".into())
}
