use std::path::PathBuf;

use fc_core::{Color, Vec2};
use fc_vm::rendering::{font::Font, screen::ScreenLayer, text::draw_text};
use fc_vm::vm::Vm;
use winit::keyboard::KeyCode;

use super::Editor;

const LIST_TOP: u32 = 10;
const ROW_H: u32 = 8;
const VISIBLE_ROWS: usize = 13; // (120 - 10) / 8 = 13.75
const TAB_BAR_Y: u32 = crate::tabs::TAB_BAR_Y;

fn c_title() -> Color { Color::new_rgb(200, 200, 200) }
fn c_selected_bg() -> Color { Color::new_rgb(30, 50, 90) }
fn c_selected() -> Color { Color::new_rgb(80, 140, 220) }
fn c_normal() -> Color { Color::new_rgb(160, 160, 160) }
fn c_empty() -> Color { Color::new_rgb(80, 80, 80) }
fn c_hint() -> Color { Color::new_rgb(100, 100, 100) }

pub struct BrowserEditor {
    files: Vec<PathBuf>,
    selected: usize,
    scroll: usize,
    pending_load: Option<PathBuf>,
    scan_dir: PathBuf,
}

impl BrowserEditor {
    pub fn new() -> Self {
        let scan_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut b = Self {
            files: Vec::new(),
            selected: 0,
            scroll: 0,
            pending_load: None,
            scan_dir,
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
            let mut files: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("rom"))
                .collect();
            files.sort();
            self.files = files;
        }
        self.selected = 0;
        self.scroll = 0;
    }

    pub fn take_pending_load(&mut self) -> Option<PathBuf> {
        self.pending_load.take()
    }
}

impl Editor for BrowserEditor {
    fn render(&self, layer: &mut ScreenLayer, _vm: &Vm, font: &Font, _cursor: (u32, u32)) {
        draw_text(font, layer, "ROM BROWSER", Vec2::new(1, 1), c_title());

        if self.files.is_empty() {
            draw_text(font, layer, "NO ROM FILES FOUND", Vec2::new(1, LIST_TOP), c_empty());
            draw_text(font, layer, "PLACE .ROM FILES IN", Vec2::new(1, LIST_TOP + ROW_H), c_empty());
            draw_text(font, layer, "CURRENT DIR", Vec2::new(1, LIST_TOP + ROW_H * 2), c_empty());
            draw_text(font, layer, "PRESS R TO RESCAN", Vec2::new(1, LIST_TOP + ROW_H * 4), c_hint());
            return;
        }

        let visible_end = (self.scroll + VISIBLE_ROWS).min(self.files.len());
        for (vis_idx, file_idx) in (self.scroll..visible_end).enumerate() {
            let y = LIST_TOP + vis_idx as u32 * ROW_H;
            let is_sel = file_idx == self.selected;

            if is_sel {
                for px in 0..127u32 {
                    for dy in 0..(ROW_H - 1) {
                        layer.set_pixel(Vec2::new(px, y + dy), c_selected_bg());
                    }
                }
            }

            let name = self.files[file_idx]
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("?");
            let display = if name.len() > 28 { &name[..28] } else { name };
            let col = if is_sel { c_selected() } else { c_normal() };
            draw_text(font, layer, display, Vec2::new(2, y + 1), col);
        }

        // Scroll indicator on right edge
        if self.files.len() > VISIBLE_ROWS {
            let total = self.files.len();
            let track_h = (TAB_BAR_Y - LIST_TOP) as usize;
            let thumb_y = LIST_TOP as usize + self.scroll * track_h / total;
            let thumb_h = (VISIBLE_ROWS * track_h / total).max(2);
            for dy in 0..thumb_h as u32 {
                layer.set_pixel(Vec2::new(127, thumb_y as u32 + dy), c_normal());
            }
        }

        // Hint row at bottom of list area
        let hint_y = TAB_BAR_Y - ROW_H;
        draw_text(font, layer, "ENTER=LOAD  R=RESCAN", Vec2::new(1, hint_y), c_hint());
    }

    fn handle_click(&mut self, _x: u32, y: u32, _vm: &mut Vm) {
        if y < LIST_TOP || y >= TAB_BAR_Y - ROW_H {
            return;
        }
        let vis = ((y - LIST_TOP) / ROW_H) as usize;
        let file_idx = self.scroll + vis;
        if file_idx < self.files.len() {
            if file_idx == self.selected {
                // Double-click (same row clicked twice) — trigger load
                self.pending_load = Some(self.files[file_idx].clone());
            } else {
                self.selected = file_idx;
            }
        }
    }

    fn handle_key(&mut self, key: KeyCode, _vm: &mut Vm) {
        match key {
            KeyCode::ArrowUp => {
                if self.selected > 0 {
                    self.selected -= 1;
                    if self.selected < self.scroll {
                        self.scroll = self.selected;
                    }
                }
            }
            KeyCode::ArrowDown => {
                if self.selected + 1 < self.files.len() {
                    self.selected += 1;
                    if self.selected >= self.scroll + VISIBLE_ROWS {
                        self.scroll = self.selected + 1 - VISIBLE_ROWS;
                    }
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if let Some(path) = self.files.get(self.selected) {
                    self.pending_load = Some(path.clone());
                }
            }
            KeyCode::KeyR => {
                self.rescan();
            }
            _ => {}
        }
    }
}
