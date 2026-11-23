//! TPCloud for the Linux desktop: networks (found on this network or by address), sign-in, Files,
//! Photos and Home. Native (iced, on Wayland or X11, any desktop), talking to the same API as the
//! web and the iPhone app.

mod discover;
mod net;
mod store;

use iced::widget::{button, column, container, horizontal_space, image, row, scrollable, text, text_input, Column, Space};
use iced::{Alignment, Element, Length, Task, Theme};
use net::{Api, Blind, Entry, Me, Photo, PhotoPage};
use std::collections::HashMap;
use store::Store;

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
    iced::application("TPCloud", App::update, App::view)
        .subscription(App::subscription)
        .theme(|_| Theme::Light)
        .window_size((1100.0, 720.0))
        .run_with(App::new)
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
        // A different anchor must be refused.
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
enum Tab {
    Files,
    Photos,
    Home,
    Account,
}

#[derive(Debug, PartialEq)]
enum Screen {
    Loading,
    Networks,
    Login,
    Main,
}

struct App {
    store: Store,
    screen: Screen,
    api: Option<Api>,
    me: Option<Me>,
    busy: bool,
    error: Option<String>,
    notice: Option<String>,
    // Networks
    address: String,
    found: Vec<discover::Found>,
    waiting: Vec<discover::Found>,
    looking: bool,
    // Sign-in
    email: String,
    password: String,
    code: String,
    totp: bool,
    // Main
    tab: Tab,
    path: Vec<String>,
    entries: Vec<Entry>,
    photos: Vec<Photo>,
    thumbs: HashMap<i64, image::Handle>,
    cursor: Option<String>,
    blinds: Vec<Blind>,
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
    Tab(Tab),
    Open(usize),
    Up,
    Listed(Result<Vec<Entry>, String>),
    Downloaded(Result<std::path::PathBuf, String>),
    Upload,
    Uploaded(Result<usize, String>),
    PhotosLoaded(Result<PhotoPage, String>),
    Thumb(i64, Result<Vec<u8>, String>),
    MorePhotos,
    Blinds(Result<Vec<Blind>, String>),
    BlindCommand(String, &'static str),
    Done(Result<(), String>),
    Tick,
    SignOut,
    MakeHost,
    HostStarted(Result<String, String>),
}

impl App {
    fn new() -> (App, Task<Message>) {
        let store = Store::load();
        let mut app = App {
            store,
            screen: Screen::Loading,
            api: None,
            me: None,
            busy: false,
            error: None,
            notice: None,
            address: String::new(),
            found: vec![],
            waiting: vec![],
            looking: false,
            email: String::new(),
            password: String::new(),
            code: String::new(),
            totp: false,
            tab: Tab::Files,
            path: vec![],
            entries: vec![],
            photos: vec![],
            thumbs: HashMap::new(),
            cursor: None,
            blinds: vec![],
        };
        let task = app.connect();
        (app, task)
    }

    /// Picks the fastest address of the network in use (home's, Tailscale's, then public) and,
    /// with a session kept from before, signs straight in.
    fn connect(&mut self) -> Task<Message> {
        let Some(net) = self.store.current().cloned() else {
            self.screen = Screen::Networks;
            return Task::done(Message::Look);
        };
        self.screen = Screen::Loading;
        self.email = net.email.clone().unwrap_or_default();
        Task::perform(
            async move {
                let anchor = net.anchor_der();
                let locals = net.local_hosts();
                let mut candidates = net.by_preference();
                if let Some(last) = &net.last_address {
                    if !candidates.contains(last) {
                        candidates.push(last.clone());
                    }
                }
                let probes = candidates.iter().map(|url| {
                    let api = Api::new(url, anchor.as_deref(), &locals, net.token.clone());
                    let id = net.id.clone();
                    async move { api.answers_as(&id).await.then_some(api) }
                });
                let results = futures_join(probes.collect()).await;
                let api = results.into_iter().flatten().next().ok_or("None of its addresses answer right now.".to_string())?;
                let me = if api.token.is_some() { api.me().await.ok() } else { None };
                Ok((api, me))
            },
            Message::Connected,
        )
    }

    fn root(&self) -> String {
        let mut p = format!("users/{}", self.me.as_ref().map(|m| m.email.as_str()).unwrap_or(""));
        for part in &self.path {
            p.push('/');
            p.push_str(part);
        }
        p
    }

    fn load_tab(&mut self) -> Task<Message> {
        let Some(api) = self.api.clone() else { return Task::none() };
        match self.tab {
            Tab::Files => {
                let path = self.root();
                Task::perform(async move { api.list(&path).await }, Message::Listed)
            }
            Tab::Photos => {
                if !self.photos.is_empty() {
                    return Task::none();
                }
                Task::perform(async move { api.photos(None).await }, Message::PhotosLoaded)
            }
            Tab::Home => Task::perform(async move { api.blinds().await }, Message::Blinds),
            Tab::Account => Task::none(),
        }
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
                    self.me = Some(me);
                    self.screen = Screen::Main;
                    return self.load_tab();
                }
                self.screen = Screen::Login;
            }
            Message::Connected(Err(e)) => {
                self.error = Some(e);
                self.screen = Screen::Networks;
                return Task::done(Message::Look);
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
                self.photos.clear();
                self.thumbs.clear();
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
                self.photos.clear();
                self.thumbs.clear();
                self.path.clear();
                return self.connect();
            }
            Message::Remove(id) => {
                self.store.remove(&id);
            }
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
                self.me = Some(me);
                self.screen = Screen::Main;
                return self.load_tab();
            }
            Message::Verified(Err(e)) => {
                self.busy = false;
                self.code.clear();
                self.error = Some(e);
            }
            Message::Tab(tab) => {
                self.tab = tab;
                self.error = None;
                return self.load_tab();
            }
            Message::Open(i) => {
                let Some(entry) = self.entries.get(i).cloned() else { return Task::none() };
                if entry.kind == "directory" {
                    self.path.push(entry.name);
                    return self.load_tab();
                }
                let Some(api) = self.api.clone() else { return Task::none() };
                let path = format!("{}/{}", self.root(), entry.name);
                self.notice = Some(format!("Opening {}…", entry.name));
                return Task::perform(
                    async move {
                        let data = api.download(&path).await?;
                        let dir = dirs::download_dir().unwrap_or_else(std::env::temp_dir);
                        let file = dir.join(&entry.name);
                        tokio::fs::write(&file, data).await.map_err(|e| e.to_string())?;
                        let _ = std::process::Command::new("xdg-open").arg(&file).spawn();
                        Ok(file)
                    },
                    Message::Downloaded,
                );
            }
            Message::Up => {
                self.path.pop();
                return self.load_tab();
            }
            Message::Listed(Ok(list)) => self.entries = list,
            Message::Downloaded(Ok(file)) => self.notice = Some(format!("Saved in {}", file.display())),
            Message::Upload => {
                let Some(api) = self.api.clone() else { return Task::none() };
                let folder = self.root();
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
            Message::Uploaded(Ok(n)) => {
                if n > 0 {
                    self.notice = Some(format!("Uploaded {n} file(s)."));
                }
                return self.load_tab();
            }
            Message::PhotosLoaded(Ok(page)) => {
                let Some(api) = self.api.clone() else { return Task::none() };
                self.cursor = page.next_cursor;
                let tasks: Vec<Task<Message>> = page
                    .items
                    .iter()
                    .map(|p| {
                        let (api, photo) = (api.clone(), p.clone());
                        let id = photo.id;
                        Task::perform(async move { api.thumbnail(&photo).await }, move |r| Message::Thumb(id, r))
                    })
                    .collect();
                self.photos.extend(page.items);
                return Task::batch(tasks);
            }
            Message::Thumb(id, Ok(bytes)) => {
                self.thumbs.insert(id, image::Handle::from_bytes(bytes));
            }
            Message::Thumb(_, Err(_)) => {}
            Message::MorePhotos => {
                let (Some(api), Some(cursor)) = (self.api.clone(), self.cursor.clone()) else { return Task::none() };
                return Task::perform(async move { api.photos(Some(&cursor)).await }, Message::PhotosLoaded);
            }
            Message::Blinds(Ok(b)) => self.blinds = b,
            Message::BlindCommand(id, command) => {
                let Some(api) = self.api.clone() else { return Task::none() };
                return Task::perform(async move { api.blind(&id, command).await }, Message::Done);
            }
            Message::Done(Ok(())) => {}
            Message::Tick => {
                if self.screen == Screen::Main && self.tab == Tab::Home {
                    return self.load_tab();
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
                self.notice = Some(format!("TPCloud is installed here and waiting to be set up at {page}. Once it's done, add it below."));
                let _ = std::process::Command::new("xdg-open").arg(&page).spawn();
            }
            Message::Listed(Err(e)) | Message::Downloaded(Err(e)) | Message::Uploaded(Err(e)) | Message::PhotosLoaded(Err(e)) | Message::Blinds(Err(e)) | Message::Done(Err(e)) => {
                if e == "signed-out" {
                    self.signed_out();
                } else {
                    self.error = Some(e);
                }
            }
        }
        Task::none()
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

    fn subscription(&self) -> iced::Subscription<Message> {
        if self.screen == Screen::Main && self.tab == Tab::Home {
            iced::time::every(std::time::Duration::from_secs(4)).map(|_| Message::Tick)
        } else {
            iced::Subscription::none()
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let body: Element<Message> = match self.screen {
            Screen::Loading => center(text("Connecting…").size(16).into()),
            Screen::Networks => self.networks_view(),
            Screen::Login => self.login_view(),
            Screen::Main => self.main_view(),
        };
        let mut page = column![body].height(Length::Fill);
        if let Some(n) = &self.notice {
            page = page.push(container(text(n).size(13)).padding([6, 16]));
        }
        page.into()
    }

    fn networks_view(&self) -> Element<'_, Message> {
        let mut list = Column::new().spacing(6);
        for n in &self.store.networks {
            let current = self.store.current.as_deref() == Some(&n.id);
            list = list.push(
                row![
                    button(column![text(&n.name).size(16), text(n.last_address.clone().unwrap_or_default()).size(12)].spacing(2))
                        .on_press(Message::Use(n.id.clone()))
                        .width(Length::Fill)
                        .style(if current { button::primary } else { button::secondary }),
                    button(text("Remove").size(13)).on_press(Message::Remove(n.id.clone())).style(button::text),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        let known: Vec<&str> = self.store.networks.iter().map(|n| n.id.as_str()).collect();
        let mut found = Column::new().spacing(6);
        for f in self.found.iter().filter(|f| f.network_id.as_deref().map_or(true, |id| !known.contains(&id))) {
            found = found.push(button(column![text(&f.name).size(15), text(&f.url).size(12)]).width(Length::Fill).on_press(Message::AddFound(f.url.clone())).style(button::secondary));
        }
        for w in &self.waiting {
            found = found.push(
                button(column![text(format!("{} · waiting to be set up", w.name)).size(15), text(&w.url).size(12)])
                    .width(Length::Fill)
                    .style(button::secondary)
                    .on_press(Message::HostStarted(Ok(w.url.clone()))),
            );
        }
        let content = column![
            text("TPCloud").size(34),
            text("Your files, photos and home. Add your network: the one at home, or one at its address.").size(15),
            Space::with_height(12),
            list,
            text("Add a network").size(18),
            row![
                text_input("tphome.local or cloud.example.com", &self.address).on_input(Message::Address).on_submit(Message::Add).padding(10),
                button(text(if self.busy { "Adding…" } else { "Add" })).on_press_maybe((!self.busy && !self.address.trim().is_empty()).then_some(Message::Add)).padding(10),
            ]
            .spacing(8),
            row![text(if self.looking { "Looking on this network…" } else { "On this network" }).size(14), horizontal_space(), button(text("Look again").size(13)).on_press(Message::Look).style(button::text)].align_y(Alignment::Center),
            found,
            Space::with_height(8),
            row![
                column![text("Make this computer the host").size(16), text("Installs TPCloud here (asks for your password) and opens its setup.").size(12)].width(Length::Fill),
                button(text("Set up")).on_press_maybe((!self.busy).then_some(Message::MakeHost)).style(button::secondary),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(10)
        .max_width(560);
        let content = match &self.error {
            Some(e) => content.push(text(e).size(13).color([0.85, 0.2, 0.2])),
            None => content,
        };
        container(scrollable(container(content).padding(32).center_x(Length::Fill))).height(Length::Fill).into()
    }

    fn login_view(&self) -> Element<'_, Message> {
        let name = self.store.current().map(|n| n.name.clone()).unwrap_or_default();
        let form: Element<Message> = if self.totp {
            column![
                text("Enter the 6-digit code from your authenticator app.").size(14),
                text_input("123456", &self.code).on_input(Message::Code).on_submit(Message::Verify).padding(12).size(22),
                button(text("Continue")).on_press_maybe((self.code.len() == 6 && !self.busy).then_some(Message::Verify)).padding(12).width(Length::Fill),
            ]
            .spacing(10)
            .into()
        } else {
            column![
                text_input("Email", &self.email).on_input(Message::Email).padding(12),
                text_input("Password", &self.password).on_input(Message::Password).on_submit(Message::SignIn).secure(true).padding(12),
                button(text(if self.busy { "Signing in…" } else { "Sign in" })).on_press_maybe((!self.busy && !self.email.is_empty() && !self.password.is_empty()).then_some(Message::SignIn)).padding(12).width(Length::Fill),
            ]
            .spacing(10)
            .into()
        };
        let mut col = column![
            text("TPCloud").size(34),
            text(format!("Sign in to {name}")).size(15),
            Space::with_height(8),
            form,
            button(text("Another network").size(13)).on_press(Message::ShowNetworks).style(button::text),
        ]
        .spacing(10)
        .max_width(380)
        .align_x(Alignment::Center);
        if let Some(e) = &self.error {
            col = col.push(text(e).size(13).color([0.85, 0.2, 0.2]));
        }
        center(col.into())
    }

    fn main_view(&self) -> Element<'_, Message> {
        let modules = self.me.as_ref().map(|m| m.modules.clone()).unwrap_or_default();
        let tab = |label: &'static str, t: Tab| button(text(label).size(15)).width(Length::Fill).padding(10).style(if self.tab == t { button::primary } else { button::text }).on_press(Message::Tab(t));
        let mut side = column![text(self.store.current().map(|n| n.name.clone()).unwrap_or_default()).size(18), Space::with_height(10)].spacing(4).width(200);
        if modules.is_empty() || modules.iter().any(|m| m == "store") {
            side = side.push(tab("Files", Tab::Files)).push(tab("Photos", Tab::Photos));
        }
        if modules.iter().any(|m| m == "home") {
            side = side.push(tab("Home", Tab::Home));
        }
        side = side.push(tab("Account", Tab::Account));
        let content: Element<Message> = match self.tab {
            Tab::Files => self.files_view(),
            Tab::Photos => self.photos_view(),
            Tab::Home => self.home_view(),
            Tab::Account => self.account_view(),
        };
        let mut main = column![content].height(Length::Fill).width(Length::Fill);
        if let Some(e) = &self.error {
            main = main.push(text(e).size(13).color([0.85, 0.2, 0.2]));
        }
        row![container(side).padding(16).height(Length::Fill), container(main).padding(16)].into()
    }

    fn files_view(&self) -> Element<'_, Message> {
        let title = self.path.last().cloned().unwrap_or_else(|| "Home".into());
        let mut header = row![text(title).size(26), horizontal_space(), button(text("Upload…")).on_press(Message::Upload)].spacing(8).align_y(Alignment::Center);
        if !self.path.is_empty() {
            header = row![button(text("‹ Back")).on_press(Message::Up).style(button::text)].push(header);
        }
        let mut list = Column::new().spacing(2);
        for (i, e) in self.entries.iter().enumerate() {
            let size = e.size.map(human).unwrap_or_default();
            let label = if e.kind == "directory" { format!("📁  {}", e.name) } else { format!("📄  {}", e.name) };
            list = list.push(button(row![text(label).size(15), horizontal_space(), text(size).size(13)]).width(Length::Fill).padding(8).style(button::text).on_press(Message::Open(i)));
        }
        if self.entries.is_empty() {
            list = list.push(text("Nothing here yet.").size(14));
        }
        column![header, scrollable(list).height(Length::Fill)].spacing(12).into()
    }

    fn photos_view(&self) -> Element<'_, Message> {
        let per_row = 6;
        let mut grid = Column::new().spacing(4);
        for chunk in self.photos.chunks(per_row) {
            let mut r = row![].spacing(4);
            for p in chunk {
                let cell: Element<Message> = match self.thumbs.get(&p.id) {
                    Some(h) => image(h.clone()).width(140).height(140).content_fit(iced::ContentFit::Cover).into(),
                    None => container(Space::new(140, 140)).style(container::rounded_box).into(),
                };
                r = r.push(cell);
            }
            grid = grid.push(r);
        }
        if self.cursor.is_some() {
            grid = grid.push(button(text("More")).on_press(Message::MorePhotos).style(button::secondary));
        }
        column![text("Photos").size(26), scrollable(grid).height(Length::Fill)].spacing(12).into()
    }

    fn home_view(&self) -> Element<'_, Message> {
        let mut list = Column::new().spacing(8);
        for b in &self.blinds {
            let pos = b.state.as_ref().map(|s| format!("{}%", s.position)).unwrap_or_else(|| "—".into());
            list = list.push(
                row![
                    text(&b.name).size(16).width(Length::Fill),
                    text(pos).size(14),
                    button(text("▲")).on_press(Message::BlindCommand(b.id.clone(), "up")),
                    button(text("■")).on_press(Message::BlindCommand(b.id.clone(), "stop")).style(button::secondary),
                    button(text("▼")).on_press(Message::BlindCommand(b.id.clone(), "down")),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        column![text("Home").size(26), scrollable(list).height(Length::Fill)].spacing(12).into()
    }

    fn account_view(&self) -> Element<'_, Message> {
        let me = self.me.as_ref();
        column![
            text("Account").size(26),
            text(me.and_then(|m| m.display_name.clone()).filter(|n| !n.is_empty()).unwrap_or_default()).size(18),
            text(me.map(|m| m.email.clone()).unwrap_or_default()).size(14),
            text(format!("Connected at {}", self.api.as_ref().map(|a| a.base.clone()).unwrap_or_default())).size(13),
            Space::with_height(12),
            row![button(text("Networks")).on_press(Message::ShowNetworks).style(button::secondary), button(text("Sign out")).on_press(Message::SignOut).style(button::danger)].spacing(8),
        ]
        .spacing(8)
        .into()
    }
}

fn center(content: Element<'_, Message>) -> Element<'_, Message> {
    container(content).center(Length::Fill).into()
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

/// Every future at once, results in the same order.
async fn futures_join<F: std::future::Future + Send + 'static>(futures: Vec<F>) -> Vec<F::Output>
where
    F::Output: Send + 'static,
{
    let handles: Vec<_> = futures.into_iter().map(tokio::spawn).collect();
    let mut out = Vec::new();
    for h in handles {
        if let Ok(v) = h.await {
            out.push(v);
        }
    }
    out
}

/// Downloads the server program for this machine and installs it with pkexec (the system asks for
/// the password). With no terminal it starts as the setup page, which is where it's set up.
async fn make_host() -> Result<String, String> {
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "amd64",
        other => return Err(format!("There's no TPCloud server for {other} yet.")),
    };
    let bytes = reqwest::get(format!("https://tpcloud.migueltaibo.com/download/linux/{arch}"))
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
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
