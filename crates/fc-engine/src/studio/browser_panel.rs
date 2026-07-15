//! Cart browser panel: local .fc/.rom file list plus the fc-hub online tab.
//! Hub requests run on background threads and report back over mpsc; the
//! app polls each frame and picks up finished downloads via `take_pending_load`.

use super::theme;
use chrono::{DateTime, Local};
use std::io::Read;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

const PER_PAGE: u32 = 15;

struct LocalEntry {
    path: PathBuf,
    name: String,
    title: String,
    date: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BrowserTab {
    Local,
    Hub,
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

enum HubState {
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

enum HubAction {
    Fetch(u32),
    Download(String, String),
}

pub struct BrowserState {
    tab: BrowserTab,
    scan_dir: PathBuf,
    files: Vec<LocalEntry>,
    scanned: bool,
    hub_url: String,
    hub: HubState,
    downloading: bool,
    hub_rx: Option<Receiver<HubMsg>>,
    pending_load: Option<PathBuf>,
}

impl Default for BrowserState {
    fn default() -> Self {
        Self {
            tab: BrowserTab::Local,
            scan_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            files: Vec::new(),
            scanned: false,
            hub_url: std::env::var("FC_HUB_URL")
                .unwrap_or_else(|_| "http://localhost:8080".into()),
            hub: HubState::Idle,
            downloading: false,
            hub_rx: None,
            pending_load: None,
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

    /// Polls the background hub thread; call once per frame.
    pub fn poll(&mut self) {
        let msg = self.hub_rx.as_ref().and_then(|rx| rx.try_recv().ok());
        let Some(msg) = msg else { return };
        self.hub_rx = None;
        match msg {
            HubMsg::CartList { carts, total, page } => {
                self.hub = HubState::Loaded { carts, total, page };
            }
            HubMsg::RomReady(path) => {
                self.downloading = false;
                self.pending_load = Some(path);
            }
            HubMsg::Err(e) => {
                self.downloading = false;
                self.hub = HubState::Error(e);
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
            if ext != "rom" && ext != "fc" {
                continue;
            }
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let title = if ext == "rom" {
                fc_rom::load(&path)
                    .ok()
                    .map(|r| r.header.title)
                    .unwrap_or_default()
            } else {
                String::new()
            };
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

    fn fetch_page(&mut self, page: u32) {
        let (tx, rx) = mpsc::channel();
        self.hub_rx = Some(rx);
        self.hub = HubState::Fetching;
        let url = format!(
            "{}/api/carts?page={}&per_page={}",
            self.hub_url, page, PER_PAGE
        );
        std::thread::spawn(move || {
            let result = ureq::get(&url)
                .call()
                .map_err(|e| format!("connection failed: {e}"))
                .and_then(|resp| {
                    serde_json::from_reader::<_, HubCartList>(resp.into_reader())
                        .map_err(|e| format!("parse error: {e}"))
                });
            let msg = match result {
                Ok(list) => HubMsg::CartList {
                    carts: list.carts,
                    total: list.total,
                    page,
                },
                Err(e) => HubMsg::Err(e),
            };
            let _ = tx.send(msg);
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
            .join(format!("{safe}.rom"));
        std::thread::spawn(move || {
            let dir = tmp_path.parent().expect("temp path has parent");
            if std::fs::create_dir_all(dir).is_err() {
                let _ = tx.send(HubMsg::Err("cannot create temp dir".into()));
                return;
            }
            let result = ureq::get(&url)
                .call()
                .map_err(|e| format!("connection failed: {e}"))
                .and_then(|resp| {
                    let mut buf = Vec::new();
                    resp.into_reader()
                        .read_to_end(&mut buf)
                        .map_err(|e| format!("read error: {e}"))?;
                    std::fs::write(&tmp_path, &buf).map_err(|e| format!("write error: {e}"))?;
                    Ok(tmp_path)
                });
            let msg = match result {
                Ok(path) => HubMsg::RomReady(path),
                Err(e) => HubMsg::Err(e),
            };
            let _ = tx.send(msg);
        });
    }
}

pub fn show(ui: &mut egui::Ui, state: &mut BrowserState) {
    if !state.scanned {
        state.rescan();
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.tab, BrowserTab::Local, "LOCAL");
        ui.selectable_value(&mut state.tab, BrowserTab::Hub, "HUB");
    });
    ui.separator();

    match state.tab {
        BrowserTab::Local => show_local(ui, state),
        BrowserTab::Hub => show_hub(ui, state),
    }
}

fn show_local(ui: &mut egui::Ui, state: &mut BrowserState) {
    ui.horizontal(|ui| {
        if ui.button("RESCAN").clicked() {
            state.rescan();
        }
        ui.colored_label(theme::DIM, state.scan_dir.display().to_string());
    });
    ui.add_space(4.0);

    if state.files.is_empty() {
        ui.colored_label(theme::DIM, "no .fc or .rom files in this folder");
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

fn show_hub(ui: &mut egui::Ui, state: &mut BrowserState) {
    let busy = state.downloading || matches!(state.hub, HubState::Fetching);
    let mut action: Option<HubAction> = None;

    ui.horizontal(|ui| {
        if ui.add_enabled(!busy, egui::Button::new("REFRESH")).clicked() {
            let page = match &state.hub {
                HubState::Loaded { page, .. } => *page,
                _ => 0,
            };
            action = Some(HubAction::Fetch(page));
        }
        ui.colored_label(theme::DIM, &state.hub_url);
    });
    ui.add_space(4.0);

    if state.downloading {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.colored_label(theme::DIM, "downloading…");
        });
    } else {
        match &state.hub {
            HubState::Idle => {
                ui.colored_label(theme::DIM, "press REFRESH to browse the hub");
                ui.colored_label(theme::DIM, "hub address comes from the FC_HUB_URL env var");
            }
            HubState::Fetching => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.colored_label(theme::DIM, "loading…");
                });
            }
            HubState::Error(e) => {
                ui.colored_label(theme::ERROR, format!("error: {e}"));
                ui.colored_label(theme::DIM, "press REFRESH to retry");
            }
            HubState::Loaded { carts, total, page } => {
                let total_pages = (*total as u32).div_ceil(PER_PAGE).max(1);
                ui.horizontal(|ui| {
                    if ui.add_enabled(*page > 0, egui::Button::new("<")).clicked() {
                        action = Some(HubAction::Fetch(page - 1));
                    }
                    ui.label(format!("page {} / {}", page + 1, total_pages));
                    if ui
                        .add_enabled(page + 1 < total_pages, egui::Button::new(">"))
                        .clicked()
                    {
                        action = Some(HubAction::Fetch(page + 1));
                    }
                    ui.colored_label(theme::DIM, format!("{total} carts"));
                });
                ui.add_space(4.0);

                if carts.is_empty() {
                    ui.colored_label(theme::DIM, "hub has no carts yet");
                } else {
                    ui.colored_label(theme::DIM, "double-click to download and run");
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            for cart in carts {
                                let text = format!(
                                    "{:<28} {:<20} {} DL",
                                    cart.title, cart.author, cart.downloads
                                );
                                let resp = ui.selectable_label(false, text);
                                if resp.double_clicked() {
                                    action = Some(HubAction::Download(
                                        cart.id.clone(),
                                        cart.title.clone(),
                                    ));
                                }
                            }
                        });
                }
            }
        }
    }

    match action {
        Some(HubAction::Fetch(page)) => state.fetch_page(page),
        Some(HubAction::Download(id, title)) => state.download_rom(id, title),
        None => {}
    }
}
