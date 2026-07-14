use fc_core::{Color, Vec2};
use fc_vm::rendering::text::{draw_character, draw_text};
use fc_vm::rendering::{font::Font, screen::ScreenLayer};
use fc_vm::vm::Vm;
use std::path::PathBuf;
use winit::keyboard::KeyCode;

use super::util::{clear_panel, fill_rect, key_to_char};
use super::{Editor, button_hit, draw_button};

const VISIBLE_ROWS: usize = 13;
const CODE_Y: u32 = 8;
const INFO_Y: u32 = 112;
const GUTTER_W: u32 = 14;
const CHAR_W: u32 = 4;

const KEYWORDS: &[&str] = &[
    "if", "then", "else", "elseif", "end", "while", "do", "for", "in", "repeat", "until",
    "function", "fn", "return", "local", "break", "not", "and", "or", "true", "false", "nil",
];

const BUILTINS: &[&str] = &[
    "spr", "pset", "pget", "cls", "txt", "key", "btn", "sin", "cos", "abs", "sqrt", "max", "min",
    "strlen", "tostring", "print", "rnd", "flr", "camera", "btn",
];

pub enum CodeEditorAction {
    None,
    CompileAndRun,
    Save,
}

pub struct CodeEditor {
    lines: Vec<String>,
    cursor_line: usize,
    cursor_col: usize,
    scroll_line: usize,
    pub source_path: Option<PathBuf>,
    pub error_msg: Option<String>,
    pub pending_action: Option<CodeEditorAction>,
}

impl CodeEditor {
    pub fn new() -> Self {
        CodeEditor {
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            scroll_line: 0,
            source_path: None,
            error_msg: None,
            pending_action: None,
        }
    }

    pub fn set_source_path(&mut self, path: PathBuf) {
        if path.exists()
            && let Ok(text) = std::fs::read_to_string(&path)
        {
            self.lines = text.lines().map(|l| l.to_string()).collect();
            if self.lines.is_empty() {
                self.lines.push(String::new());
            }
            self.cursor_line = 0;
            self.cursor_col = 0;
            self.scroll_line = 0;
            self.error_msg = None;
        }
        self.source_path = Some(path);
    }

    pub fn get_source(&self) -> String {
        self.lines.join("\n")
    }

    /// Jump the cursor to a 1-based source line (e.g. from a compile error).
    pub fn goto_line(&mut self, line: usize) {
        self.cursor_line = line.saturating_sub(1).min(self.lines.len().saturating_sub(1));
        self.cursor_col = 0;
        self.scroll_to_cursor();
    }

    pub fn save(&self) -> bool {
        if let Some(path) = &self.source_path {
            return std::fs::write(path, self.get_source()).is_ok();
        }
        false
    }

    pub fn handle_key_direct(&mut self, key: KeyCode, shift: bool, ctrl: bool) -> CodeEditorAction {
        if ctrl {
            return match key {
                KeyCode::KeyR => CodeEditorAction::CompileAndRun,
                KeyCode::KeyS => CodeEditorAction::Save,
                _ => CodeEditorAction::None,
            };
        }

        match key {
            KeyCode::Enter => self.enter(),
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete => self.delete_char(),
            KeyCode::Tab => {
                self.insert_char(' ');
                self.insert_char(' ');
            }
            KeyCode::ArrowUp => {
                if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.clamp_cursor_col();
                }
            }
            KeyCode::ArrowDown => {
                if self.cursor_line + 1 < self.lines.len() {
                    self.cursor_line += 1;
                    self.clamp_cursor_col();
                }
            }
            KeyCode::ArrowLeft => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.cursor_col = self.lines[self.cursor_line].len();
                }
            }
            KeyCode::ArrowRight => {
                let line_len = self.lines[self.cursor_line].len();
                if self.cursor_col < line_len {
                    self.cursor_col += 1;
                } else if self.cursor_line + 1 < self.lines.len() {
                    self.cursor_line += 1;
                    self.cursor_col = 0;
                }
            }
            KeyCode::Home => self.cursor_col = 0,
            KeyCode::End => self.cursor_col = self.lines[self.cursor_line].len(),
            KeyCode::PageUp => {
                self.scroll_line = self.scroll_line.saturating_sub(VISIBLE_ROWS);
                if self.cursor_line >= self.scroll_line + VISIBLE_ROWS {
                    self.cursor_line = self.scroll_line;
                    self.clamp_cursor_col();
                }
            }
            KeyCode::PageDown => {
                let max_scroll = self.lines.len().saturating_sub(VISIBLE_ROWS);
                self.scroll_line = (self.scroll_line + VISIBLE_ROWS).min(max_scroll);
                if self.cursor_line < self.scroll_line {
                    self.cursor_line = self.scroll_line;
                    self.clamp_cursor_col();
                }
            }
            _ => {
                if let Some(ch) = key_to_char(key, shift) {
                    self.insert_char(ch);
                    self.error_msg = None;
                }
            }
        }

        self.scroll_to_cursor();
        CodeEditorAction::None
    }

    fn clamp_cursor_col(&mut self) {
        let line_len = self.lines[self.cursor_line].len();
        self.cursor_col = self.cursor_col.min(line_len);
    }

    fn scroll_to_cursor(&mut self) {
        if self.cursor_line < self.scroll_line {
            self.scroll_line = self.cursor_line;
        } else if self.cursor_line >= self.scroll_line + VISIBLE_ROWS {
            self.scroll_line = self.cursor_line + 1 - VISIBLE_ROWS;
        }
    }

    fn insert_char(&mut self, ch: char) {
        if self.cursor_col <= self.lines[self.cursor_line].len() {
            self.lines[self.cursor_line].insert(self.cursor_col, ch);
            self.cursor_col += 1;
        }
    }

    fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.lines[self.cursor_line].remove(self.cursor_col - 1);
            self.cursor_col -= 1;
        } else if self.cursor_line > 0 {
            let cur = self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_col = self.lines[self.cursor_line].len();
            self.lines[self.cursor_line].push_str(&cur);
        }
    }

    fn delete_char(&mut self) {
        let line_len = self.lines[self.cursor_line].len();
        if self.cursor_col < line_len {
            self.lines[self.cursor_line].remove(self.cursor_col);
        } else if self.cursor_line + 1 < self.lines.len() {
            let next = self.lines.remove(self.cursor_line + 1);
            self.lines[self.cursor_line].push_str(&next);
        }
    }

    fn enter(&mut self) {
        let rest = self.lines[self.cursor_line].split_off(self.cursor_col);
        let indent: usize = self.lines[self.cursor_line]
            .chars()
            .take_while(|c| *c == ' ')
            .count();
        let new_line = " ".repeat(indent) + &rest;
        self.cursor_line += 1;
        self.lines.insert(self.cursor_line, new_line);
        self.cursor_col = indent;
    }

    fn render_code_line(font: &Font, layer: &mut ScreenLayer, line: &str, x0: u32, y: u32) {
        let col_white = Color::new_rgb(210, 210, 210);
        let col_keyword = Color::new_rgb(255, 220, 60);
        let col_builtin = Color::new_rgb(60, 210, 255);
        let col_string = Color::new_rgb(80, 210, 100);
        let col_comment = Color::new_rgb(80, 80, 80);
        let col_number = Color::new_rgb(255, 150, 60);

        // Visible chars: from col 0 up to what fits in 128 - GUTTER_W pixels
        let max_chars = ((128 - x0) / CHAR_W) as usize;
        let chars: Vec<char> = line.chars().take(max_chars).collect();
        let mut i = 0;

        while i < chars.len() {
            // Comment: --
            if i + 1 < chars.len() && chars[i] == '-' && chars[i + 1] == '-' {
                for (j, ch) in chars.iter().enumerate().skip(i) {
                    let c = ch.to_ascii_uppercase();
                    draw_character(
                        font,
                        layer,
                        c,
                        Vec2::new(x0 + j as u32 * CHAR_W, y),
                        col_comment,
                    );
                }
                return;
            }
            // String literal: "..."
            if chars[i] == '"' {
                draw_character(
                    font,
                    layer,
                    '"',
                    Vec2::new(x0 + i as u32 * CHAR_W, y),
                    col_string,
                );
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    let c = chars[i].to_ascii_uppercase();
                    draw_character(
                        font,
                        layer,
                        c,
                        Vec2::new(x0 + i as u32 * CHAR_W, y),
                        col_string,
                    );
                    i += 1;
                }
                if i < chars.len() {
                    draw_character(
                        font,
                        layer,
                        '"',
                        Vec2::new(x0 + i as u32 * CHAR_W, y),
                        col_string,
                    );
                    i += 1;
                }
                continue;
            }
            // Number
            if chars[i].is_ascii_digit() {
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    draw_character(
                        font,
                        layer,
                        chars[i],
                        Vec2::new(x0 + i as u32 * CHAR_W, y),
                        col_number,
                    );
                    i += 1;
                }
                continue;
            }
            // Identifier → keyword / builtin / plain
            if chars[i].is_ascii_alphabetic() || chars[i] == '_' {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word_lower: String = chars[start..i]
                    .iter()
                    .map(|c| c.to_ascii_lowercase())
                    .collect();
                let col = if KEYWORDS.contains(&word_lower.as_str()) {
                    col_keyword
                } else if BUILTINS.contains(&word_lower.as_str()) {
                    col_builtin
                } else {
                    col_white
                };
                for (j, ch) in chars.iter().enumerate().take(i).skip(start) {
                    let c = ch.to_ascii_uppercase();
                    draw_character(font, layer, c, Vec2::new(x0 + j as u32 * CHAR_W, y), col);
                }
                continue;
            }
            // Default char
            let c = chars[i].to_ascii_uppercase();
            draw_character(
                font,
                layer,
                c,
                Vec2::new(x0 + i as u32 * CHAR_W, y),
                col_white,
            );
            i += 1;
        }
    }
}

impl Editor for CodeEditor {
    fn render(&self, layer: &mut ScreenLayer, _vm: &Vm, font: &Font, _cursor: (u32, u32)) {
        let col_bg = Color::new_rgb(8, 8, 16);
        let col_hdr_bg = Color::new_rgb(18, 18, 36);
        let col_info_bg = Color::new_rgb(12, 12, 24);
        let col_gutter_bg = Color::new_rgb(22, 22, 36);
        let col_curline = Color::new_rgb(20, 22, 52);
        let col_cursor = Color::new_rgb(160, 170, 255);
        let col_gray = Color::new_rgb(90, 90, 100);
        let col_red = Color::new_rgb(240, 60, 60);

        // Fill code area background
        clear_panel(layer, col_bg);

        // Header row background (y=0..7)
        fill_rect(layer, 0, 0, 128, 8, col_hdr_bg);

        // Filename
        let name = self
            .source_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("UNTITLED");
        let name_up: String = name
            .chars()
            .take(17)
            .map(|c| c.to_ascii_uppercase())
            .collect();
        draw_text(font, layer, &name_up, Vec2::new(1, 1), col_gray);

        // [RUN] button (x=112, y=0)
        draw_button(layer, font, 112, 0, "RUN", false);

        // Info / error bar (y=112..119)
        fill_rect(layer, 0, INFO_Y, 128, 8, col_info_bg);
        if let Some(ref err) = self.error_msg {
            let err_up: String = err
                .chars()
                .take(32)
                .map(|c| c.to_ascii_uppercase())
                .collect();
            draw_text(font, layer, &err_up, Vec2::new(1, INFO_Y + 1), col_red);
        } else {
            let info = format!("LN:{} COL:{}", self.cursor_line + 1, self.cursor_col + 1);
            draw_text(font, layer, &info, Vec2::new(1, INFO_Y + 1), col_gray);
        }

        // Code rows
        for row in 0..VISIBLE_ROWS {
            let line_idx = self.scroll_line + row;
            let y = CODE_Y + row as u32 * 8;

            // Gutter background
            fill_rect(layer, 0, y, GUTTER_W, 8, col_gutter_bg);

            if line_idx >= self.lines.len() {
                continue;
            }

            // Line number in gutter (right-aligned in 2 chars, mod 100)
            let line_num = format!("{:2}", (line_idx + 1) % 100);
            draw_text(font, layer, &line_num, Vec2::new(1, y + 2), col_gray);

            // Highlight current line background
            if line_idx == self.cursor_line {
                fill_rect(layer, GUTTER_W, y, 128 - GUTTER_W, 8, col_curline);
            }

            // Syntax-highlighted code text
            let code_x = GUTTER_W + 2;
            Self::render_code_line(font, layer, &self.lines[line_idx], code_x, y + 2);

            // Text cursor (2px wide bar)
            if line_idx == self.cursor_line {
                let cx = code_x + self.cursor_col as u32 * CHAR_W;
                if cx + 1 < 128 {
                    for dy in 1..7u32 {
                        layer.set_pixel(Vec2::new(cx, y + dy), col_cursor);
                        layer.set_pixel(Vec2::new(cx + 1, y + dy), col_cursor);
                    }
                }
            }
        }
    }

    fn handle_click(&mut self, x: u32, y: u32, _vm: &mut Vm) {
        // [RUN] button
        if button_hit(112, 0, "RUN", x, y) {
            self.pending_action = Some(CodeEditorAction::CompileAndRun);
            return;
        }
        // Click in code area → reposition cursor
        if (CODE_Y..INFO_Y).contains(&y) {
            let row = ((y - CODE_Y) / 8) as usize;
            let line_idx = self.scroll_line + row;
            if line_idx < self.lines.len() {
                self.cursor_line = line_idx;
                let code_x = GUTTER_W + 2;
                let col = if x >= code_x {
                    ((x - code_x) / CHAR_W) as usize
                } else {
                    0
                };
                self.cursor_col = col.min(self.lines[line_idx].len());
            }
        }
    }

    fn handle_scroll(&mut self, _dx: f32, dy: f32, _vm: &mut Vm) {
        if dy > 0.0 {
            self.scroll_line = self.scroll_line.saturating_sub(3);
        } else if dy < 0.0 {
            let max_scroll = self.lines.len().saturating_sub(1);
            self.scroll_line = (self.scroll_line + 3).min(max_scroll);
        }
    }
}
