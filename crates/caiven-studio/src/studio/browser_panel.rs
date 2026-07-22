//! Cart browser panel: local .cav file list plus the caiven-port online tab.
//! Port requests run on background threads and report back over a shared
//! mpsc channel; the app polls each frame and picks up finished downloads
//! via `take_pending_load`.

use super::theme;
use crate::app::cart_io::CartMeta;
use crate::port_client::{build_multipart, capture_screenshot};
use caiven_vm::VmConfig;
use chrono::{DateTime, Local};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

const PER_PAGE: u32 = 15;
/// Must match `caiven_port::auth::SESSION_COOKIE` — caiven-studio doesn't depend on
/// the caiven-port crate, so this is a small duplicated constant.
const SESSION_COOKIE: &str = "caiven_session";

struct LocalEntry {
    path: PathBuf,
    name: String,
    title: String,
    date: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BrowserTab {
    Local,
    Port,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortOrder {
    New,
    Popular,
    Top,
}

impl SortOrder {
    fn as_str(self) -> &'static str {
        match self {
            SortOrder::New => "new",
            SortOrder::Popular => "popular",
            SortOrder::Top => "top",
        }
    }

    fn label(self) -> &'static str {
        match self {
            SortOrder::New => "NEW",
            SortOrder::Popular => "POPULAR",
            SortOrder::Top => "TOP",
        }
    }
}

#[derive(serde::Deserialize, Clone)]
struct PortCart {
    id: String,
    title: String,
    author: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    tags: Vec<String>,
    downloads: i64,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    rating_avg: f64,
    #[serde(default)]
    rating_count: i64,
    #[serde(default)]
    has_screenshot: bool,
}

#[derive(serde::Deserialize)]
struct PortCartList {
    carts: Vec<PortCart>,
    total: u64,
}

#[derive(serde::Deserialize, Clone)]
struct PortVersion {
    version: i32,
    cart_size: i64,
    #[serde(default)]
    changelog: String,
}

#[derive(serde::Deserialize, Clone)]
struct PortCartDetail {
    #[serde(flatten)]
    cart: PortCart,
    #[serde(default)]
    versions: Vec<PortVersion>,
}

#[derive(serde::Deserialize)]
struct TokenCreated {
    token: String,
}

enum ListState {
    Idle,
    Fetching,
    Loaded {
        carts: Vec<PortCart>,
        total: u64,
        page: u32,
    },
    Error(String),
}

enum DetailState {
    None,
    Fetching(String),
    Loaded(PortCartDetail),
    Error(String),
}

enum PortMsg {
    CartList {
        carts: Vec<PortCart>,
        total: u64,
        page: u32,
    },
    CartListErr(String),
    Detail(PortCartDetail),
    DetailErr(String),
    RomReady(PathBuf),
    RomErr(String),
    Thumbnail {
        id: String,
        png: Vec<u8>,
    },
    LoginOk {
        token: String,
        username: String,
    },
    LoginErr(String),
    PublishOk {
        cart_id: String,
    },
    PublishErr(String),
}

enum PortAction {
    Search(u32),
    SelectDetail(String),
    Download(String, String, Option<i32>),
}

#[derive(Default)]
struct LoginDialog {
    open: bool,
    username: String,
    password: String,
    busy: bool,
    error: Option<String>,
}

struct PublishDialog {
    open: bool,
    cart_path: String,
    title: String,
    author: String,
    description: String,
    tags: String,
    changelog: String,
    target_cart_id: Option<String>,
    frames: u32,
    busy: bool,
    error: Option<String>,
    status: Option<String>,
}

impl Default for PublishDialog {
    fn default() -> Self {
        Self {
            open: false,
            cart_path: String::new(),
            title: String::new(),
            author: String::new(),
            description: String::new(),
            tags: String::new(),
            changelog: String::new(),
            target_cart_id: None,
            frames: 30,
            busy: false,
            error: None,
            status: None,
        }
    }
}

struct PublishJob {
    cart_path: PathBuf,
    port_url: String,
    token: String,
    title: String,
    author: String,
    description: String,
    tags: String,
    changelog: String,
    frames: u32,
    target_cart_id: Option<String>,
}

pub struct BrowserState {
    tab: BrowserTab,
    scan_dir: PathBuf,
    files: Vec<LocalEntry>,
    scanned: bool,
    port_url: String,
    port_token: Option<String>,
    port_username: Option<String>,
    query: String,
    sort: SortOrder,
    list: ListState,
    downloading: bool,
    detail: DetailState,
    thumbnails: HashMap<String, egui::TextureHandle>,
    thumb_requested: HashSet<String>,
    login: LoginDialog,
    publish: PublishDialog,
    tx: Sender<PortMsg>,
    rx: Receiver<PortMsg>,
    pending_load: Option<PathBuf>,
    pending_new: bool,
}

fn token_file_path() -> Option<PathBuf> {
    if let Ok(appdata) = std::env::var("APPDATA") {
        return Some(
            PathBuf::from(appdata)
                .join("caiven-studio")
                .join("port_token"),
        );
    }
    if let Ok(home) = std::env::var("HOME") {
        return Some(
            PathBuf::from(home)
                .join(".config")
                .join("caiven-studio")
                .join("port_token"),
        );
    }
    None
}

fn load_saved_token() -> Option<(String, String)> {
    let path = token_file_path()?;
    let content = std::fs::read_to_string(path).ok()?;
    let mut lines = content.lines();
    let username = lines.next()?.to_string();
    let token = lines.next()?.to_string();
    Some((username, token))
}

fn save_token(username: &str, token: &str) {
    let Some(path) = token_file_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, format!("{username}\n{token}"));
}

fn safe_filename(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(32)
        .collect()
}

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn ureq_error_message(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, resp) => {
            let body: serde_json::Value =
                serde_json::from_reader(resp.into_reader()).unwrap_or_default();
            body.get("error")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| format!("HTTP {code}"))
        }
        ureq::Error::Transport(t) => format!("connection failed: {t}"),
    }
}

fn parse_session_cookie(resp: &ureq::Response) -> Option<String> {
    let raw = resp.header("Set-Cookie")?;
    let first = raw.split(';').next()?;
    let (name, value) = first.split_once('=')?;
    if name.trim() == SESSION_COOKIE {
        Some(value.trim().to_string())
    } else {
        None
    }
}

fn decode_png_to_color_image(bytes: &[u8]) -> Option<egui::ColorImage> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [w as usize, h as usize],
        img.as_raw(),
    ))
}

fn run_publish(job: &PublishJob) -> Result<String, String> {
    let cart =
        caiven_cart::load(&job.cart_path).map_err(|e| format!("failed to load cart: {e:#}"))?;
    let cart_bytes =
        std::fs::read(&job.cart_path).map_err(|e| format!("failed to read cart: {e}"))?;
    let filename = job
        .cart_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("cart.cav");

    let boundary = "----CaivenStudioBoundary7x3k9p";
    let (upload_url, body) = if let Some(id) = &job.target_cart_id {
        let meta = serde_json::json!({ "changelog": job.changelog }).to_string();
        let body = build_multipart(
            boundary,
            &[
                ("meta", None, "application/json", meta.as_bytes()),
                (
                    "cart",
                    Some(filename),
                    "application/octet-stream",
                    &cart_bytes,
                ),
            ],
        );
        (
            format!("{}/api/v2/carts/{}/versions", job.port_url, id),
            body,
        )
    } else {
        let tags: Vec<&str> = job
            .tags
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        let meta = serde_json::json!({
            "title": job.title,
            "author": job.author,
            "description": job.description,
            "tags": tags,
        })
        .to_string();
        let body = build_multipart(
            boundary,
            &[
                ("meta", None, "application/json", meta.as_bytes()),
                (
                    "cart",
                    Some(filename),
                    "application/octet-stream",
                    &cart_bytes,
                ),
            ],
        );
        (format!("{}/api/v2/carts", job.port_url), body)
    };

    let content_type = format!("multipart/form-data; boundary={boundary}");
    let resp = ureq::post(&upload_url)
        .set("X-Api-Key", &job.token)
        .set("Content-Type", &content_type)
        .send_bytes(&body)
        .map_err(ureq_error_message)?;

    let cart_id = if let Some(id) = &job.target_cart_id {
        id.clone()
    } else {
        let val: serde_json::Value = serde_json::from_reader(resp.into_reader())
            .map_err(|e| format!("bad upload response: {e}"))?;
        val["id"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| "upload response missing 'id'".to_string())?
    };

    if let Ok(png) = capture_screenshot(&cart, VmConfig::default(), job.frames) {
        let boundary2 = "----CaivenStudioScreenshotBoundary";
        let ss_body = build_multipart(
            boundary2,
            &[("screenshot", Some("screenshot.png"), "image/png", &png)],
        );
        let ct2 = format!("multipart/form-data; boundary={boundary2}");
        let ss_url = format!("{}/api/v2/carts/{}/screenshot", job.port_url, cart_id);
        let _ = ureq::post(&ss_url)
            .set("X-Api-Key", &job.token)
            .set("Content-Type", &ct2)
            .send_bytes(&ss_body);
    }

    Ok(cart_id)
}

impl Default for BrowserState {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        let env_token = std::env::var("CAIVEN_PORT_API_KEY")
            .ok()
            .filter(|s| !s.is_empty());
        let (port_username, port_token) = if let Some(t) = env_token {
            (None, Some(t))
        } else if let Some((u, t)) = load_saved_token() {
            (Some(u), Some(t))
        } else {
            (None, None)
        };
        Self {
            tab: BrowserTab::Local,
            scan_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            files: Vec::new(),
            scanned: false,
            port_url: std::env::var("CAIVEN_PORT_URL")
                .unwrap_or_else(|_| "http://localhost:8080".into()),
            port_token,
            port_username,
            query: String::new(),
            sort: SortOrder::New,
            list: ListState::Idle,
            downloading: false,
            detail: DetailState::None,
            thumbnails: HashMap::new(),
            thumb_requested: HashSet::new(),
            login: LoginDialog::default(),
            publish: PublishDialog::default(),
            tx,
            rx,
            pending_load: None,
            pending_new: false,
        }
    }
}

impl BrowserState {
    pub fn set_scan_dir(&mut self, dir: PathBuf) {
        if dir != self.scan_dir {
            self.scan_dir = dir;
            self.scanned = false;
        }
    }

    pub fn take_pending_load(&mut self) -> Option<PathBuf> {
        self.pending_load.take()
    }

    pub fn take_pending_new(&mut self) -> bool {
        std::mem::take(&mut self.pending_new)
    }

    pub fn scan_dir(&self) -> &std::path::Path {
        &self.scan_dir
    }

    /// Polls the background port threads; call once per frame.
    pub fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                PortMsg::CartList { carts, total, page } => {
                    for cart in &carts {
                        if cart.has_screenshot {
                            self.fetch_thumbnail(cart.id.clone());
                        }
                    }
                    self.list = ListState::Loaded { carts, total, page };
                }
                PortMsg::CartListErr(e) => self.list = ListState::Error(e),
                PortMsg::Detail(d) => {
                    if d.cart.has_screenshot {
                        self.fetch_thumbnail(d.cart.id.clone());
                    }
                    self.detail = DetailState::Loaded(d);
                }
                PortMsg::DetailErr(e) => self.detail = DetailState::Error(e),
                PortMsg::RomReady(path) => {
                    self.downloading = false;
                    self.pending_load = Some(path);
                }
                PortMsg::RomErr(e) => {
                    self.downloading = false;
                    self.list = ListState::Error(e);
                }
                PortMsg::Thumbnail { id, png } => {
                    if let Some(img) = decode_png_to_color_image(&png) {
                        let tex = ctx.load_texture(
                            format!("port-thumb-{id}"),
                            img,
                            egui::TextureOptions::NEAREST,
                        );
                        self.thumbnails.insert(id, tex);
                    }
                }
                PortMsg::LoginOk { token, username } => {
                    save_token(&username, &token);
                    self.port_token = Some(token);
                    self.port_username = Some(username);
                    self.login.busy = false;
                    self.login.open = false;
                    self.login.error = None;
                    self.login.password.clear();
                }
                PortMsg::LoginErr(e) => {
                    self.login.busy = false;
                    self.login.error = Some(e);
                }
                PortMsg::PublishOk { cart_id } => {
                    self.publish.busy = false;
                    self.publish.status = Some(format!("published: {cart_id}"));
                    self.publish.error = None;
                    if let DetailState::Loaded(d) = &self.detail
                        && (d.cart.id == cart_id
                            || self.publish.target_cart_id.as_deref() == Some(&cart_id))
                    {
                        self.fetch_detail(cart_id);
                    }
                }
                PortMsg::PublishErr(e) => {
                    self.publish.busy = false;
                    self.publish.error = Some(e);
                }
            }
        }
    }

    fn rescan(&mut self) {
        self.files.clear();
        self.scanned = true;
        let Ok(entries) = std::fs::read_dir(&self.scan_dir) else {
            return;
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "cav" {
                continue;
            }
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let title = caiven_cart::load(&path)
                .ok()
                .map(|r| r.header.title)
                .unwrap_or_default();
            let date = path
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| {
                    let dt: DateTime<Local> = t.into();
                    dt.format("%Y-%m-%d %H:%M").to_string()
                })
                .unwrap_or_default();
            self.files.push(LocalEntry {
                path,
                name,
                title,
                date,
            });
        }
        self.files.sort_by(|a, b| a.name.cmp(&b.name));
    }

    fn fetch_list(&mut self, page: u32) {
        self.list = ListState::Fetching;
        let tx = self.tx.clone();
        let mut url = format!(
            "{}/api/v2/carts?page={}&per_page={}&sort={}",
            self.port_url,
            page,
            PER_PAGE,
            self.sort.as_str()
        );
        if !self.query.trim().is_empty() {
            url.push_str(&format!("&q={}", url_encode(self.query.trim())));
        }
        std::thread::spawn(move || {
            let result = ureq::get(&url)
                .call()
                .map_err(ureq_error_message)
                .and_then(|resp| {
                    serde_json::from_reader::<_, PortCartList>(resp.into_reader())
                        .map_err(|e| format!("parse error: {e}"))
                });
            let _ = tx.send(match result {
                Ok(list) => PortMsg::CartList {
                    carts: list.carts,
                    total: list.total,
                    page,
                },
                Err(e) => PortMsg::CartListErr(e),
            });
        });
    }

    fn fetch_detail(&mut self, id: String) {
        self.detail = DetailState::Fetching(id.clone());
        let tx = self.tx.clone();
        let url = format!("{}/api/v2/carts/{}", self.port_url, id);
        let token = self.port_token.clone();
        std::thread::spawn(move || {
            let mut req = ureq::get(&url);
            if let Some(t) = &token {
                req = req.set("X-Api-Key", t);
            }
            let result = req.call().map_err(ureq_error_message).and_then(|resp| {
                serde_json::from_reader::<_, PortCartDetail>(resp.into_reader())
                    .map_err(|e| format!("parse error: {e}"))
            });
            let _ = tx.send(match result {
                Ok(d) => PortMsg::Detail(d),
                Err(e) => PortMsg::DetailErr(e),
            });
        });
    }

    fn fetch_thumbnail(&mut self, id: String) {
        if !self.thumb_requested.insert(id.clone()) {
            return;
        }
        let tx = self.tx.clone();
        let url = format!("{}/api/v2/carts/{}/screenshot", self.port_url, id);
        std::thread::spawn(move || {
            if let Ok(resp) = ureq::get(&url).call() {
                let mut buf = Vec::new();
                if resp.into_reader().read_to_end(&mut buf).is_ok() {
                    let _ = tx.send(PortMsg::Thumbnail { id, png: buf });
                }
            }
        });
    }

    fn download_cart(&mut self, id: String, title: String, version: Option<i32>) {
        self.downloading = true;
        let tx = self.tx.clone();
        let mut url = format!("{}/api/v2/carts/{}/cart", self.port_url, id);
        if let Some(v) = version {
            url.push_str(&format!("?version={v}"));
        }
        let safe = safe_filename(&title);
        let tmp_path = std::env::temp_dir()
            .join("caiven-port")
            .join(format!("{safe}.cav"));
        std::thread::spawn(move || {
            let dir = tmp_path.parent().expect("temp path has parent");
            if std::fs::create_dir_all(dir).is_err() {
                let _ = tx.send(PortMsg::RomErr("cannot create temp dir".into()));
                return;
            }
            let result = ureq::get(&url)
                .call()
                .map_err(ureq_error_message)
                .and_then(|resp| {
                    let mut buf = Vec::new();
                    resp.into_reader()
                        .read_to_end(&mut buf)
                        .map_err(|e| format!("read error: {e}"))?;
                    std::fs::write(&tmp_path, &buf).map_err(|e| format!("write error: {e}"))?;
                    Ok(tmp_path)
                });
            let _ = tx.send(match result {
                Ok(path) => PortMsg::RomReady(path),
                Err(e) => PortMsg::RomErr(e),
            });
        });
    }

    fn submit_login(&mut self) {
        self.login.busy = true;
        self.login.error = None;
        let tx = self.tx.clone();
        let port_url = self.port_url.clone();
        let username = self.login.username.trim().to_string();
        let password = self.login.password.clone();
        std::thread::spawn(move || {
            let login_url = format!("{port_url}/api/v2/auth/login");
            let creds =
                serde_json::json!({ "username": username, "password": password }).to_string();
            let resp = match ureq::post(&login_url)
                .set("Content-Type", "application/json")
                .send_string(&creds)
            {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(PortMsg::LoginErr(ureq_error_message(e)));
                    return;
                }
            };
            let Some(session) = parse_session_cookie(&resp) else {
                let _ = tx.send(PortMsg::LoginErr(
                    "login succeeded but no session cookie returned".into(),
                ));
                return;
            };

            let token_url = format!("{port_url}/api/v2/auth/tokens");
            let token_body = serde_json::json!({ "name": "Studio" }).to_string();
            let token_resp = ureq::post(&token_url)
                .set("Cookie", &format!("{SESSION_COOKIE}={session}"))
                .set("Content-Type", "application/json")
                .send_string(&token_body);
            let _ = tx.send(match token_resp {
                Ok(r) => match serde_json::from_reader::<_, TokenCreated>(r.into_reader()) {
                    Ok(t) => PortMsg::LoginOk {
                        token: t.token,
                        username,
                    },
                    Err(e) => PortMsg::LoginErr(format!("bad token response: {e}")),
                },
                Err(e) => PortMsg::LoginErr(ureq_error_message(e)),
            });
        });
    }

    fn logout(&mut self) {
        self.port_token = None;
        self.port_username = None;
        if let Some(path) = token_file_path() {
            let _ = std::fs::remove_file(path);
        }
    }

    fn submit_publish(&mut self, job: PublishJob) {
        self.publish.busy = true;
        self.publish.error = None;
        self.publish.status = None;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(match run_publish(&job) {
                Ok(cart_id) => PortMsg::PublishOk { cart_id },
                Err(e) => PortMsg::PublishErr(e),
            });
        });
    }
}

pub fn show(
    ui: &mut egui::Ui,
    state: &mut BrowserState,
    ctx: &egui::Context,
    loaded_cart: Option<&CartMeta>,
) {
    if !state.scanned {
        state.rescan();
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.tab, BrowserTab::Local, "LOCAL");
        ui.selectable_value(&mut state.tab, BrowserTab::Port, "PORT");
    });
    ui.separator();

    match state.tab {
        BrowserTab::Local => show_local(ui, state),
        BrowserTab::Port => show_port(ui, state, loaded_cart),
    }

    show_login_window(ctx, state);
    show_publish_window(ctx, state);
}

fn show_local(ui: &mut egui::Ui, state: &mut BrowserState) {
    ui.horizontal(|ui| {
        if ui.button("RESCAN").clicked() {
            state.rescan();
        }
        if ui.button("NEW CART").clicked() {
            state.pending_new = true;
        }
        ui.colored_label(theme::DIM, state.scan_dir.display().to_string());
    });
    ui.add_space(4.0);

    if state.files.is_empty() {
        ui.colored_label(theme::DIM, "no .cav files in this folder");
        return;
    }

    ui.colored_label(theme::DIM, "double-click to open");
    let mut load: Option<PathBuf> = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for entry in &state.files {
                let text = format!("{:<28} {:<24} {}", entry.name, entry.title, entry.date);
                let resp = ui.selectable_label(false, text);
                if resp.double_clicked() {
                    load = Some(entry.path.clone());
                }
            }
        });
    if load.is_some() {
        state.pending_load = load;
    }
}

fn show_port(ui: &mut egui::Ui, state: &mut BrowserState, loaded_cart: Option<&CartMeta>) {
    ui.horizontal(|ui| {
        ui.colored_label(theme::DIM, &state.port_url);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            match state.port_username.clone() {
                Some(name) => {
                    if ui.button("LOG OUT").clicked() {
                        state.logout();
                    }
                    ui.colored_label(theme::OK, name);
                }
                None => {
                    if ui.button("LOG IN").clicked() {
                        state.login.open = true;
                        state.login.error = None;
                    }
                }
            }
            if ui.button("PUBLISH").clicked() {
                state.publish.open = true;
                state.publish.target_cart_id = None;
                state.publish.error = None;
                state.publish.status = None;
                if state.publish.cart_path.is_empty()
                    && let Some(meta) = loaded_cart
                {
                    state.publish.cart_path = meta.path.display().to_string();
                    state.publish.title = meta.header.title.clone();
                    state.publish.author = meta.header.author.clone();
                }
            }
        });
    });

    let busy = state.downloading || matches!(state.list, ListState::Fetching);
    let mut action: Option<PortAction> = None;

    ui.horizontal(|ui| {
        let resp = ui.add_enabled(
            !busy,
            egui::TextEdit::singleline(&mut state.query).hint_text("search…"),
        );
        let search_now = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

        egui::ComboBox::from_id_salt("port_sort")
            .selected_text(state.sort.label())
            .show_ui(ui, |ui| {
                for opt in [SortOrder::New, SortOrder::Popular, SortOrder::Top] {
                    ui.selectable_value(&mut state.sort, opt, opt.label());
                }
            });

        if ui.add_enabled(!busy, egui::Button::new("SEARCH")).clicked() || search_now {
            action = Some(PortAction::Search(0));
        }
    });
    ui.add_space(4.0);

    if state.downloading {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.colored_label(theme::DIM, "downloading…");
        });
    }

    let has_detail = matches!(
        state.detail,
        DetailState::Loaded(_) | DetailState::Fetching(_) | DetailState::Error(_)
    );
    if has_detail {
        egui::SidePanel::right("port_detail")
            .resizable(true)
            .default_width(300.0)
            .show_inside(ui, |ui| {
                show_detail_pane(ui, state);
            });
    }

    egui::CentralPanel::default().show_inside(ui, |ui| match &state.list {
        ListState::Idle => {
            ui.colored_label(theme::DIM, "press SEARCH to browse the port");
        }
        ListState::Fetching => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.colored_label(theme::DIM, "loading…");
            });
        }
        ListState::Error(e) => {
            ui.colored_label(theme::ERROR, format!("error: {e}"));
            ui.colored_label(theme::DIM, "press SEARCH to retry");
        }
        ListState::Loaded { carts, total, page } => {
            let total_pages = (*total as u32).div_ceil(PER_PAGE).max(1);
            let page = *page;
            ui.horizontal(|ui| {
                if ui.add_enabled(page > 0, egui::Button::new("<")).clicked() {
                    action = Some(PortAction::Search(page - 1));
                }
                ui.label(format!("page {} / {}", page + 1, total_pages));
                if ui
                    .add_enabled(page + 1 < total_pages, egui::Button::new(">"))
                    .clicked()
                {
                    action = Some(PortAction::Search(page + 1));
                }
                ui.colored_label(theme::DIM, format!("{total} carts"));
            });
            ui.add_space(4.0);

            if carts.is_empty() {
                ui.colored_label(theme::DIM, "no carts found");
            } else {
                ui.colored_label(
                    theme::DIM,
                    "click for details · double-click to download and run",
                );
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        for cart in carts {
                            ui.horizontal(|ui| {
                                if let Some(tex) = state.thumbnails.get(&cart.id) {
                                    let size = egui::vec2(24.0, 24.0);
                                    ui.add(egui::Image::from_texture(tex).fit_to_exact_size(size));
                                } else {
                                    ui.add_space(24.0);
                                }
                                let text = format!(
                                    "{:<28} {:<20} ★{:.1} {} DL",
                                    cart.title, cart.author, cart.rating_avg, cart.downloads
                                );
                                let resp = ui.selectable_label(false, text);
                                if resp.clicked() {
                                    action = Some(PortAction::SelectDetail(cart.id.clone()));
                                }
                                if resp.double_clicked() {
                                    action = Some(PortAction::Download(
                                        cart.id.clone(),
                                        cart.title.clone(),
                                        None,
                                    ));
                                }
                            });
                        }
                    });
            }
        }
    });

    match action {
        Some(PortAction::Search(page)) => state.fetch_list(page),
        Some(PortAction::SelectDetail(id)) => state.fetch_detail(id),
        Some(PortAction::Download(id, title, version)) => state.download_cart(id, title, version),
        None => {}
    }
}

fn show_detail_pane(ui: &mut egui::Ui, state: &mut BrowserState) {
    match &state.detail {
        DetailState::None => {}
        DetailState::Fetching(id) => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.colored_label(theme::DIM, id.as_str());
            });
            return;
        }
        DetailState::Error(e) => {
            ui.colored_label(theme::ERROR, e.clone());
            return;
        }
        DetailState::Loaded(_) => {}
    }
    let DetailState::Loaded(detail) = &state.detail else {
        return;
    };
    let detail = detail.clone();
    let tex = state.thumbnails.get(&detail.cart.id).cloned();

    ui.heading(&detail.cart.title);
    ui.colored_label(theme::DIM, format!("by {}", detail.cart.author));
    ui.add_space(6.0);

    if let Some(tex) = tex {
        let size = tex.size_vec2();
        let scale = (256.0 / size.x.max(1.0)).min(4.0);
        ui.add(egui::Image::from_texture(&tex).fit_to_exact_size(size * scale));
        ui.add_space(6.0);
    } else if detail.cart.has_screenshot {
        ui.colored_label(theme::DIM, "(loading screenshot…)");
    }

    if !detail.cart.description.is_empty() {
        ui.label(&detail.cart.description);
    }
    if !detail.cart.tags.is_empty() {
        ui.colored_label(theme::DIM, detail.cart.tags.join(", "));
    }
    ui.horizontal(|ui| {
        ui.label(format!(
            "★ {:.1} ({})",
            detail.cart.rating_avg, detail.cart.rating_count
        ));
        ui.colored_label(theme::DIM, format!("{} downloads", detail.cart.downloads));
    });

    ui.add_space(6.0);
    ui.separator();
    ui.colored_label(theme::DIM, "VERSIONS");
    egui::ScrollArea::vertical()
        .max_height(180.0)
        .show(ui, |ui| {
            for v in &detail.versions {
                ui.horizontal(|ui| {
                    ui.label(format!("v{}", v.version));
                    ui.colored_label(theme::DIM, format!("{} B", v.cart_size));
                    if ui.small_button("DOWNLOAD").clicked() {
                        state.download_cart(
                            detail.cart.id.clone(),
                            detail.cart.title.clone(),
                            Some(v.version),
                        );
                    }
                });
                if !v.changelog.is_empty() {
                    ui.colored_label(theme::DIM, &v.changelog);
                }
            }
        });

    ui.add_space(6.0);
    ui.separator();
    let is_owner = detail.cart.owner.is_some() && detail.cart.owner == state.port_username;
    if is_owner && ui.button("PUBLISH NEW VERSION").clicked() {
        state.publish.open = true;
        state.publish.target_cart_id = Some(detail.cart.id.clone());
        state.publish.changelog.clear();
        state.publish.error = None;
        state.publish.status = None;
    }
}

fn show_login_window(ctx: &egui::Context, state: &mut BrowserState) {
    if !state.login.open {
        return;
    }
    let mut open = true;
    let mut close_requested = false;
    egui::Window::new("PORT LOGIN")
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(ctx, |ui| {
            egui::Grid::new("login_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label("USERNAME");
                    ui.text_edit_singleline(&mut state.login.username);
                    ui.end_row();
                    ui.label("PASSWORD");
                    ui.add(egui::TextEdit::singleline(&mut state.login.password).password(true));
                    ui.end_row();
                });

            if let Some(err) = &state.login.error {
                ui.colored_label(theme::ERROR, err);
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let ready =
                    !state.login.username.trim().is_empty() && !state.login.password.is_empty();
                if ui
                    .add_enabled(
                        ready && !state.login.busy,
                        egui::Button::new(if state.login.busy {
                            "LOGGING IN…"
                        } else {
                            "LOG IN"
                        }),
                    )
                    .clicked()
                {
                    state.submit_login();
                }
                if ui.button("CANCEL").clicked() {
                    close_requested = true;
                }
            });
            ui.colored_label(
                theme::DIM,
                "no account? register at the port's web page, then log in here",
            );
        });
    if !open || close_requested {
        state.login.open = false;
    }
}

fn show_publish_window(ctx: &egui::Context, state: &mut BrowserState) {
    if !state.publish.open {
        return;
    }
    let mut open = true;
    let mut close_requested = false;
    let is_version = state.publish.target_cart_id.is_some();
    let title = if is_version {
        "PUBLISH NEW VERSION"
    } else {
        "PUBLISH TO PORT"
    };
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(ctx, |ui| {
            if state.port_token.is_none() {
                ui.colored_label(theme::ERROR, "log in to the port first");
                if ui.button("CLOSE").clicked() {
                    close_requested = true;
                }
                return;
            }

            egui::Grid::new("publish_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label("CART FILE");
                    ui.text_edit_singleline(&mut state.publish.cart_path);
                    ui.end_row();

                    if is_version {
                        ui.label("CHANGELOG");
                        ui.add(
                            egui::TextEdit::multiline(&mut state.publish.changelog).desired_rows(3),
                        );
                        ui.end_row();
                    } else {
                        ui.label("TITLE");
                        ui.add(egui::TextEdit::singleline(&mut state.publish.title).char_limit(64));
                        ui.end_row();
                        ui.label("AUTHOR");
                        ui.add(
                            egui::TextEdit::singleline(&mut state.publish.author).char_limit(64),
                        );
                        ui.end_row();
                        ui.label("DESCRIPTION");
                        ui.add(
                            egui::TextEdit::multiline(&mut state.publish.description)
                                .desired_rows(3)
                                .char_limit(512),
                        );
                        ui.end_row();
                        ui.label("TAGS");
                        ui.add(
                            egui::TextEdit::singleline(&mut state.publish.tags)
                                .hint_text("comma, separated"),
                        );
                        ui.end_row();
                    }

                    ui.label("SCREENSHOT FRAMES");
                    ui.add(egui::DragValue::new(&mut state.publish.frames).range(0..=600));
                    ui.end_row();
                });

            ui.add_space(6.0);
            if let Some(err) = &state.publish.error {
                ui.colored_label(theme::ERROR, err);
            }
            if let Some(status) = &state.publish.status {
                ui.colored_label(theme::OK, status);
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let ready = !state.publish.cart_path.trim().is_empty()
                    && (is_version
                        || (!state.publish.title.trim().is_empty()
                            && !state.publish.author.trim().is_empty()));
                if ui
                    .add_enabled(
                        ready && !state.publish.busy,
                        egui::Button::new(if state.publish.busy {
                            "PUBLISHING…"
                        } else {
                            "PUBLISH"
                        }),
                    )
                    .clicked()
                {
                    let job = PublishJob {
                        cart_path: PathBuf::from(state.publish.cart_path.trim()),
                        port_url: state.port_url.clone(),
                        token: state.port_token.clone().unwrap_or_default(),
                        title: state.publish.title.clone(),
                        author: state.publish.author.clone(),
                        description: state.publish.description.clone(),
                        tags: state.publish.tags.clone(),
                        changelog: state.publish.changelog.clone(),
                        frames: state.publish.frames,
                        target_cart_id: state.publish.target_cart_id.clone(),
                    };
                    state.submit_publish(job);
                }
                if ui.button("CLOSE").clicked() {
                    close_requested = true;
                }
            });
        });
    if !open || close_requested {
        state.publish.open = false;
    }
}
