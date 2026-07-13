use std::io::Read;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

use chrono::{DateTime, Local};
use fc_core::Vec2;
use fc_vm::rendering::{font::Font, screen::ScreenLayer, text::draw_text};
use fc_vm::vm::Vm;
use winit::keyboard::KeyCode;

use super::Editor;
use super::util::{fill_rect, theme};

const LIST_TOP: u32 = 10;
const ROW_H: u32 = 8;
const VISIBLE_ROWS: usize = 13;
const TAB_BAR_Y: u32 = crate::tabs::TAB_BAR_Y;
const PER_PAGE: u32 = VISIBLE_ROWS as u32;

// Font charset: " 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ!?"'()+-=.:,[]<>"
// to_display filters + uppercases strings to renderable chars.
fn to_display(s: &str, max: usize) -> String {
    const FONT_CHARS: &str = " 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ!?\"'()+-=.:,[]<>";
    s.chars()
        .map(|c| c.to_ascii_uppercase())
        .filter(|c| FONT_CHARS.contains(*c))
        .take(max)
        .collect()
}

#[derive(Clone)]
struct RomEntry {
    path: PathBuf,
    title: String,
    date: String, // "MM-DD" from mtime
}

#[derive(PartialEq, Clone, Copy)]
enum BrowserTab {
    Local,
    Online,
}

#[derive(serde::Deserialize, Clone)]
struct HubCart {
    id: String,
    title: String,
    author: String,
    downloads: i64,
}

#[derive(serde::Deserialize)]
struct HubCartList {
    carts: Vec<HubCart>,
    total: u64,
}

enum OnlineState {
    Idle,
    Fetching,
    Loaded {
        carts: Vec<HubCart>,
        total: u64,
        page: u32,
    },
    Error(String),
}

enum HubMsg {
    CartList {
        carts: Vec<HubCart>,
        total: u64,
        page: u32,
    },
    RomReady(PathBuf),
    Err(String),
}

pub struct BrowserEditor {
    // local tab
    files: Vec<RomEntry>,
    local_selected: usize,
    local_scroll: usize,
    // online tab
    online_state: OnlineState,
    online_selected: usize,
    online_scroll: usize,
    downloading: bool,
    // shared
    tab: BrowserTab,
    pending_load: Option<PathBuf>,
    scan_dir: PathBuf,
    hub_url: String,
    hub_rx: Option<Receiver<HubMsg>>,
}

impl BrowserEditor {
    pub fn new() -> Self {
        let scan_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let hub_url =
            std::env::var("FC_HUB_URL").unwrap_or_else(|_| "http://localhost:8080".into());
        let mut b = Self {
            files: Vec::new(),
            local_selected: 0,
            local_scroll: 0,
            online_state: OnlineState::Idle,
            online_selected: 0,
            online_scroll: 0,
            downloading: false,
            tab: BrowserTab::Local,
            pending_load: None,
            scan_dir,
            hub_url,
            hub_rx: None,
        };
        b.rescan();
        b
    }

    pub fn set_scan_dir(&mut self, dir: PathBuf) {
        self.scan_dir = dir;
        self.rescan();
    }

    pub fn rescan(&mut self) {
        self.files.clear();
        if let Ok(entries) = std::fs::read_dir(&self.scan_dir) {
            let mut files: Vec<RomEntry> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("rom"))
                .map(|path| {
                    let title = fc_rom::load(&path)
                        .ok()
                        .map(|r| r.header.title)
                        .filter(|t| !t.is_empty())
                        .unwrap_or_else(|| {
                            path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("?")
                                .to_string()
                        });
                    let date = path
                        .metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(|t| {
                            let dt: DateTime<Local> = t.into();
                            dt.format("%m-%d").to_string()
                        })
                        .unwrap_or_default();
                    RomEntry { path, title, date }
                })
                .collect();
            files.sort_by(|a, b| a.path.cmp(&b.path));
            self.files = files;
        }
        self.local_selected = 0;
        self.local_scroll = 0;
    }

    pub fn take_pending_load(&mut self) -> Option<PathBuf> {
        self.pending_load.take()
    }

    /// Poll background hub thread; call from App::about_to_wait.
    pub fn poll_hub(&mut self) {
        let msg = self.hub_rx.as_ref().and_then(|rx| rx.try_recv().ok());
        let Some(msg) = msg else { return };
        self.hub_rx = None;
        match msg {
            HubMsg::CartList { carts, total, page } => {
                self.online_selected = 0;
                self.online_scroll = 0;
                self.online_state = OnlineState::Loaded { carts, total, page };
            }
            HubMsg::RomReady(path) => {
                self.downloading = false;
                self.pending_load = Some(path);
            }
            HubMsg::Err(e) => {
                self.downloading = false;
                self.online_state = OnlineState::Error(e);
            }
        }
    }

    fn fetch_page(&mut self, page: u32) {
        let (tx, rx) = mpsc::channel();
        self.hub_rx = Some(rx);
        self.online_state = OnlineState::Fetching;
        let url = format!(
            "{}/api/carts?page={}&per_page={}",
            self.hub_url, page, PER_PAGE
        );
        std::thread::spawn(move || {
            let result = ureq::get(&url)
                .call()
                .map_err(|e| format!("CONNECTION FAILED: {e}"))
                .and_then(|resp| {
                    serde_json::from_reader::<_, HubCartList>(resp.into_reader())
                        .map_err(|e| format!("PARSE ERROR: {e}"))
                });
            match result {
                Ok(list) => {
                    let _ = tx.send(HubMsg::CartList {
                        carts: list.carts,
                        total: list.total,
                        page,
                    });
                }
                Err(e) => {
                    let _ = tx.send(HubMsg::Err(e));
                }
            }
        });
    }

    fn download_rom(&mut self, id: String, title: String) {
        let (tx, rx) = mpsc::channel();
        self.hub_rx = Some(rx);
        self.downloading = true;
        let url = format!("{}/api/carts/{}/rom", self.hub_url, id);
        let safe: String = title
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .take(32)
            .collect();
        let tmp_path = std::env::temp_dir()
            .join("fc-hub")
            .join(format!("{}.rom", safe));
        std::thread::spawn(move || {
            let dir = tmp_path.parent().expect("temp path has parent");
            if std::fs::create_dir_all(dir).is_err() {
                let _ = tx.send(HubMsg::Err("CANNOT CREATE TEMP DIR".into()));
                return;
            }
            let result = ureq::get(&url)
                .call()
                .map_err(|e| format!("CONNECTION FAILED: {e}"))
                .and_then(|resp| {
                    let mut buf = Vec::new();
                    resp.into_reader()
                        .read_to_end(&mut buf)
                        .map_err(|e| format!("READ ERROR: {e}"))?;
                    std::fs::write(&tmp_path, &buf).map_err(|e| format!("WRITE ERROR: {e}"))?;
                    Ok(tmp_path)
                });
            match result {
                Ok(path) => {
                    let _ = tx.send(HubMsg::RomReady(path));
                }
                Err(e) => {
                    let _ = tx.send(HubMsg::Err(e));
                }
            }
        });
    }

    fn draw_selected_row(layer: &mut ScreenLayer, y: u32) {
        fill_rect(layer, 0, y, 127, ROW_H - 1, theme::SEL_BG);
    }

    fn draw_scroll_thumb(layer: &mut ScreenLayer, scroll: usize, total: usize) {
        if total <= VISIBLE_ROWS {
            return;
        }
        let track_h = (TAB_BAR_Y - LIST_TOP) as usize;
        let thumb_y = LIST_TOP as usize + scroll * track_h / total;
        let thumb_h = (VISIBLE_ROWS * track_h / total).max(2);
        for dy in 0..thumb_h as u32 {
            layer.set_pixel(Vec2::new(127, thumb_y as u32 + dy), theme::DIM);
        }
    }

    fn render_local(&self, layer: &mut ScreenLayer, font: &Font) {
        if self.files.is_empty() {
            draw_text(
                font,
                layer,
                "NO ROM FILES FOUND",
                Vec2::new(1, LIST_TOP),
                theme::EMPTY,
            );
            draw_text(
                font,
                layer,
                "PLACE .ROM FILES IN CURRENT DIR",
                Vec2::new(1, LIST_TOP + ROW_H),
                theme::EMPTY,
            );
            draw_text(
                font,
                layer,
                "PRESS R TO RESCAN",
                Vec2::new(1, LIST_TOP + ROW_H * 3),
                theme::HINT,
            );
            return;
        }

        let visible_end = (self.local_scroll + VISIBLE_ROWS).min(self.files.len());
        for (vis_idx, file_idx) in (self.local_scroll..visible_end).enumerate() {
            let y = LIST_TOP + vis_idx as u32 * ROW_H;
            let is_sel = file_idx == self.local_selected;
            if is_sel {
                Self::draw_selected_row(layer, y);
            }
            let entry = &self.files[file_idx];
            let display = to_display(&entry.title, 22);
            let date = to_display(&entry.date, 5);
            draw_text(
                font,
                layer,
                &display,
                Vec2::new(2, y + 1),
                if is_sel { theme::SELECTED } else { theme::DIM },
            );
            if !date.is_empty() {
                draw_text(font, layer, &date, Vec2::new(92, y + 1), theme::HINT);
            }
        }

        Self::draw_scroll_thumb(layer, self.local_scroll, self.files.len());
        let hint_y = TAB_BAR_Y - ROW_H;
        draw_text(
            font,
            layer,
            "ENTER=LOAD  R=RESCAN",
            Vec2::new(1, hint_y),
            theme::HINT,
        );
    }

    fn render_online(&self, layer: &mut ScreenLayer, font: &Font) {
        if self.downloading {
            draw_text(
                font,
                layer,
                "DOWNLOADING...",
                Vec2::new(1, LIST_TOP),
                theme::DIM,
            );
            return;
        }

        match &self.online_state {
            OnlineState::Idle => {
                draw_text(
                    font,
                    layer,
                    "PRESS R TO BROWSE HUB",
                    Vec2::new(1, LIST_TOP),
                    theme::EMPTY,
                );
                draw_text(
                    font,
                    layer,
                    "SET FC-HUB-URL ENV VAR",
                    Vec2::new(1, LIST_TOP + ROW_H),
                    theme::HINT,
                );
                draw_text(
                    font,
                    layer,
                    "DEFAULT: LOCALHOST:8080",
                    Vec2::new(1, LIST_TOP + ROW_H * 2),
                    theme::HINT,
                );
            }
            OnlineState::Fetching => {
                draw_text(
                    font,
                    layer,
                    "LOADING...",
                    Vec2::new(1, LIST_TOP),
                    theme::DIM,
                );
            }
            OnlineState::Error(e) => {
                draw_text(font, layer, "ERROR:", Vec2::new(1, LIST_TOP), theme::ERROR);
                let msg = to_display(e, 26);
                draw_text(
                    font,
                    layer,
                    &msg,
                    Vec2::new(1, LIST_TOP + ROW_H),
                    theme::ERROR,
                );
                draw_text(
                    font,
                    layer,
                    "PRESS R TO RETRY",
                    Vec2::new(1, LIST_TOP + ROW_H * 3),
                    theme::HINT,
                );
            }
            OnlineState::Loaded { carts, total, page } => {
                let visible_end = (self.online_scroll + VISIBLE_ROWS).min(carts.len());
                for (vis_idx, cart_idx) in (self.online_scroll..visible_end).enumerate() {
                    let y = LIST_TOP + vis_idx as u32 * ROW_H;
                    let is_sel = cart_idx == self.online_selected;
                    if is_sel {
                        Self::draw_selected_row(layer, y);
                    }
                    let title = to_display(&carts[cart_idx].title, 24);
                    draw_text(
                        font,
                        layer,
                        &title,
                        Vec2::new(2, y + 1),
                        if is_sel { theme::SELECTED } else { theme::DIM },
                    );
                }

                Self::draw_scroll_thumb(layer, self.online_scroll, carts.len());

                let hint_y = TAB_BAR_Y - ROW_H;
                if let Some(cart) = carts.get(self.online_selected) {
                    let author = to_display(&cart.author, 12);
                    let dl = cart.downloads;
                    let total_pages = (*total as u32).div_ceil(PER_PAGE);
                    let hint = format!(
                        "{}  {}DL  PG {} OF {}",
                        author,
                        dl,
                        page + 1,
                        total_pages.max(1)
                    );
                    draw_text(font, layer, &hint, Vec2::new(1, hint_y), theme::HINT);
                }
            }
        }
    }
}

impl Editor for BrowserEditor {
    fn handle_scroll(&mut self, _dx: f32, dy: f32, _vm: &mut Vm) {
        match self.tab {
            BrowserTab::Local => {
                if dy < 0.0 {
                    if self.local_scroll + VISIBLE_ROWS < self.files.len() {
                        self.local_scroll += 1;
                        if self.local_selected < self.local_scroll {
                            self.local_selected = self.local_scroll;
                        }
                    }
                } else if dy > 0.0 && self.local_scroll > 0 {
                    self.local_scroll -= 1;
                    if self.local_selected >= self.local_scroll + VISIBLE_ROWS {
                        self.local_selected = self.local_scroll + VISIBLE_ROWS - 1;
                    }
                }
            }
            BrowserTab::Online => {
                let len = if let OnlineState::Loaded { carts, .. } = &self.online_state {
                    carts.len()
                } else {
                    0
                };
                if dy < 0.0 {
                    if self.online_scroll + VISIBLE_ROWS < len {
                        self.online_scroll += 1;
                        if self.online_selected < self.online_scroll {
                            self.online_selected = self.online_scroll;
                        }
                    }
                } else if dy > 0.0 && self.online_scroll > 0 {
                    self.online_scroll -= 1;
                    if self.online_selected >= self.online_scroll + VISIBLE_ROWS {
                        self.online_selected = self.online_scroll + VISIBLE_ROWS - 1;
                    }
                }
            }
        }
    }

    fn render(&self, layer: &mut ScreenLayer, _vm: &Vm, font: &Font, _cursor: (u32, u32)) {
        // Sub-tab header: "LOCAL" at x=1, "ONLINE" at x=29 (gap of 8px between them)
        let local_col = if self.tab == BrowserTab::Local {
            theme::SELECTED
        } else {
            theme::HINT
        };
        let online_col = if self.tab == BrowserTab::Online {
            theme::SELECTED
        } else {
            theme::HINT
        };
        draw_text(font, layer, "LOCAL", Vec2::new(1, 1), local_col);
        draw_text(font, layer, "ONLINE", Vec2::new(29, 1), online_col);

        match self.tab {
            BrowserTab::Local => self.render_local(layer, font),
            BrowserTab::Online => self.render_online(layer, font),
        }
    }

    fn handle_click(&mut self, x: u32, y: u32, _vm: &mut Vm) {
        // Sub-tab header area (y=1..LIST_TOP)
        if (1..LIST_TOP).contains(&y) {
            if x < 24 {
                self.tab = BrowserTab::Local;
            } else if x >= 29 {
                self.tab = BrowserTab::Online;
            }
            return;
        }

        if !(LIST_TOP..TAB_BAR_Y - ROW_H).contains(&y) {
            return;
        }

        match self.tab {
            BrowserTab::Local => {
                let vis = ((y - LIST_TOP) / ROW_H) as usize;
                let file_idx = self.local_scroll + vis;
                if file_idx < self.files.len() {
                    if file_idx == self.local_selected {
                        self.pending_load = Some(self.files[file_idx].path.clone());
                    } else {
                        self.local_selected = file_idx;
                    }
                }
            }
            BrowserTab::Online => {
                if self.downloading {
                    return;
                }
                let vis = ((y - LIST_TOP) / ROW_H) as usize;
                let cart_idx = self.online_scroll + vis;
                let cart = if let OnlineState::Loaded { carts, .. } = &self.online_state {
                    carts.get(cart_idx).cloned()
                } else {
                    None
                };
                if let Some(c) = cart {
                    if cart_idx == self.online_selected {
                        self.download_rom(c.id, c.title);
                    } else {
                        self.online_selected = cart_idx;
                    }
                }
            }
        }
    }

    fn handle_key(&mut self, key: KeyCode, _vm: &mut Vm) {
        match key {
            KeyCode::Tab => {
                self.tab = match self.tab {
                    BrowserTab::Local => BrowserTab::Online,
                    BrowserTab::Online => BrowserTab::Local,
                };
            }
            KeyCode::ArrowUp => match self.tab {
                BrowserTab::Local => {
                    if self.local_selected > 0 {
                        self.local_selected -= 1;
                        if self.local_selected < self.local_scroll {
                            self.local_scroll = self.local_selected;
                        }
                    }
                }
                BrowserTab::Online => {
                    if self.online_selected > 0 {
                        self.online_selected -= 1;
                        if self.online_selected < self.online_scroll {
                            self.online_scroll = self.online_selected;
                        }
                    }
                }
            },
            KeyCode::ArrowDown => match self.tab {
                BrowserTab::Local => {
                    if self.local_selected + 1 < self.files.len() {
                        self.local_selected += 1;
                        if self.local_selected >= self.local_scroll + VISIBLE_ROWS {
                            self.local_scroll = self.local_selected + 1 - VISIBLE_ROWS;
                        }
                    }
                }
                BrowserTab::Online => {
                    let len = if let OnlineState::Loaded { carts, .. } = &self.online_state {
                        carts.len()
                    } else {
                        0
                    };
                    if self.online_selected + 1 < len {
                        self.online_selected += 1;
                        if self.online_selected >= self.online_scroll + VISIBLE_ROWS {
                            self.online_scroll = self.online_selected + 1 - VISIBLE_ROWS;
                        }
                    }
                }
            },
            KeyCode::Enter | KeyCode::NumpadEnter => match self.tab {
                BrowserTab::Local => {
                    if let Some(entry) = self.files.get(self.local_selected) {
                        self.pending_load = Some(entry.path.clone());
                    }
                }
                BrowserTab::Online => {
                    if self.downloading {
                        return;
                    }
                    let cart = if let OnlineState::Loaded { carts, .. } = &self.online_state {
                        carts.get(self.online_selected).cloned()
                    } else {
                        None
                    };
                    if let Some(c) = cart {
                        self.download_rom(c.id, c.title);
                    }
                }
            },
            KeyCode::KeyR => match self.tab {
                BrowserTab::Local => self.rescan(),
                BrowserTab::Online => {
                    if self.downloading {
                        return;
                    }
                    let page = if let OnlineState::Loaded { page, .. } = &self.online_state {
                        *page
                    } else {
                        0
                    };
                    self.fetch_page(page);
                }
            },
            // Online pagination: left/right arrow
            KeyCode::ArrowLeft if self.tab == BrowserTab::Online => {
                if let OnlineState::Loaded { page, .. } = &self.online_state
                    && *page > 0
                {
                    let p = *page - 1;
                    self.fetch_page(p);
                }
            }
            KeyCode::ArrowRight if self.tab == BrowserTab::Online => {
                if let OnlineState::Loaded { page, total, .. } = &self.online_state {
                    let total_pages = (*total as u32).div_ceil(PER_PAGE);
                    if page + 1 < total_pages {
                        let p = page + 1;
                        self.fetch_page(p);
                    }
                }
            }
            _ => {}
        }
    }
}
