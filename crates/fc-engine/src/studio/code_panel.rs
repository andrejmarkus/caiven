//! Code editor panel: syntax-highlighted TextEdit with line numbers,
//! find (Ctrl+F / Ctrl+G), and clickable compile errors that jump to
//! the offending line.

use super::app::SourceFile;
use super::cart::CompileError;
use super::theme;
use crate::debugger::Debugger;
use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};

const KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in",
    "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

/// Our own console API (see `fc-vm/src/vm/lua_exec.rs`), plus Lua's own
/// stdlib globals — `string`/`table`/etc. are highlighted as namespaces;
/// their members (`string.format`) aren't, same as most editors.
const BUILTINS: &[&str] = &[
    "clear_screen",
    "set_pixel",
    "sprite",
    "button_down",
    "button_pressed",
    "draw_text",
    "draw_number",
    "fill_screen",
    "draw_line",
    "draw_rect",
    "fill_rect",
    "draw_circle",
    "fill_circle",
    "set_camera",
    "set_palette_color",
    "draw_map",
    "get_tile",
    "set_tile",
    "get_sprite_flags",
    "set_sprite_flags",
    "play_sfx",
    "play_music",
    "stop_music",
    "_init",
    "_update",
    "assert",
    "error",
    "ipairs",
    "next",
    "pairs",
    "pcall",
    "print",
    "select",
    "setmetatable",
    "getmetatable",
    "tonumber",
    "tostring",
    "type",
    "unpack",
    "xpcall",
    "math",
    "string",
    "table",
    "os",
    "io",
    "coroutine",
];

const EDITOR_ID: &str = "fc_code_editor";

struct Goto {
    char_start: usize,
    char_end: usize,
    line: usize,
}

#[derive(Default)]
pub struct CodeState {
    pub error: Option<CompileError>,
    find_open: bool,
    find_focus: bool,
    query: String,
    goto: Option<Goto>,
}

pub fn show(
    ui: &mut egui::Ui,
    state: &mut CodeState,
    source: &mut SourceFile,
    debugger: &mut Debugger,
) {
    let editor_id = egui::Id::new(EDITOR_ID);

    if ui.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::F)) {
        state.find_open = true;
        state.find_focus = true;
    }
    if state.find_open && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        state.find_open = false;
    }
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::G)) {
        find_next(ui.ctx(), state, &source.text, editor_id);
    }

    header(ui, source);
    if state.find_open {
        find_bar(ui, state, &source.text, editor_id);
    }
    error_bar(ui, state, &source.text);
    editor(ui, state, source, editor_id, debugger);
}

fn header(ui: &mut egui::Ui, source: &SourceFile) {
    ui.horizontal(|ui| {
        let name = source
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| source.path.display().to_string());
        let title = if source.dirty {
            format!("{name} *")
        } else {
            name
        };
        ui.colored_label(theme::ACCENT, title);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.colored_label(
                theme::DIM,
                "Ctrl+R run  Ctrl+F find  Ctrl+G next  Ctrl+S save",
            );
        });
    });
}

fn find_bar(ui: &mut egui::Ui, state: &mut CodeState, text: &str, editor_id: egui::Id) {
    ui.horizontal(|ui| {
        ui.colored_label(theme::DIM, "find:");
        let resp = ui.add(
            egui::TextEdit::singleline(&mut state.query)
                .id(egui::Id::new("fc_code_find"))
                .desired_width(220.0),
        );
        if state.find_focus {
            resp.request_focus();
            state.find_focus = false;
        }
        let submitted = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let next = ui.button("next").clicked() || submitted;

        ui.colored_label(
            theme::DIM,
            format!("{} found", count_matches(text, &state.query)),
        );
        if ui.button("✕").clicked() {
            state.find_open = false;
        }
        if next {
            find_next(ui.ctx(), state, text, editor_id);
        }
    });
}

fn error_bar(ui: &mut egui::Ui, state: &mut CodeState, text: &str) {
    let Some(err) = &state.error else { return };
    let resp = ui.add(
        egui::Label::new(
            egui::RichText::new(&err.message)
                .monospace()
                .color(theme::ERROR),
        )
        .sense(egui::Sense::click()),
    );
    if resp.clicked() {
        if let Some(line) = err.line {
            state.goto = Some(goto_line(text, line));
        }
    } else if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    ui.separator();
}

fn editor(
    ui: &mut egui::Ui,
    state: &mut CodeState,
    source: &mut SourceFile,
    editor_id: egui::Id,
    debugger: &mut Debugger,
) {
    let row_h = ui.text_style_height(&egui::TextStyle::Monospace);
    let view_h = ui.available_height();

    let goto = state.goto.take();
    let mut scroll = egui::ScrollArea::both().auto_shrink([false, false]);
    if let Some(g) = &goto {
        let target = (g.line.saturating_sub(1) as f32) * row_h;
        scroll = scroll.vertical_scroll_offset((target - view_h * 0.4).max(0.0));

        let mut st = egui::TextEdit::load_state(ui.ctx(), editor_id).unwrap_or_default();
        st.cursor.set_char_range(Some(CCursorRange::two(
            CCursor::new(g.char_start),
            CCursor::new(g.char_end),
        )));
        st.store(ui.ctx(), editor_id);
    }

    let error_line = state.error.as_ref().and_then(|e| e.line);
    let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _wrap: f32| {
        let font = egui::TextStyle::Monospace.resolve(ui.style());
        let job = highlight(buf.as_str(), error_line, font);
        ui.fonts(|f| f.layout_job(job))
    };

    scroll.show(ui, |ui| {
        ui.horizontal_top(|ui| {
            gutter(ui, &source.text, error_line, debugger);
            let output = egui::TextEdit::multiline(&mut source.text)
                .id(editor_id)
                .code_editor()
                .desired_width(f32::INFINITY)
                .desired_rows(40)
                .layouter(&mut layouter)
                .show(ui);
            if output.response.changed() {
                source.dirty = true;
            }
            if goto.is_some() {
                output.response.request_focus();
            }
        });
    });
}

/// Line numbers, clickable to toggle a breakpoint (shown as a filled dot).
fn gutter(ui: &mut egui::Ui, text: &str, error_line: Option<usize>, debugger: &mut Debugger) {
    let lines = text.split('\n').count();
    let digits = lines.to_string().len();
    ui.vertical(|ui| {
        ui.add_space(2.0);
        ui.spacing_mut().item_spacing.y = 0.0;
        for line in 1..=lines {
            let is_bp = debugger.breakpoints().contains(&line);
            let is_err = Some(line) == error_line;
            let color = if is_err || is_bp {
                theme::ERROR
            } else {
                theme::DIM
            };
            let marker = if is_bp { "\u{25CF}" } else { " " };
            let label = egui::RichText::new(format!("{marker}{line:>digits$}"))
                .monospace()
                .color(color);
            let resp = ui.add(egui::Label::new(label).sense(egui::Sense::click()));
            if resp.clicked() {
                debugger.toggle_line_breakpoint(line);
            } else if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
    });
}

fn goto_line(text: &str, line: usize) -> Goto {
    let byte = line_start_byte(text, line);
    let ch = byte_to_char(text, byte);
    Goto {
        char_start: ch,
        char_end: ch,
        line,
    }
}

fn line_start_byte(text: &str, line: usize) -> usize {
    let mut current = 1;
    if line <= 1 {
        return 0;
    }
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            current += 1;
            if current == line {
                return i + 1;
            }
        }
    }
    text.len()
}

fn find_next(ctx: &egui::Context, state: &mut CodeState, text: &str, editor_id: egui::Id) {
    if state.query.is_empty() {
        return;
    }
    let from_char = egui::TextEdit::load_state(ctx, editor_id)
        .and_then(|s| s.cursor.char_range())
        .map(|r| r.primary.index.max(r.secondary.index))
        .unwrap_or(0);
    let from_byte = char_to_byte(text, from_char);
    let hit = find_ci(text, &state.query, from_byte).or_else(|| find_ci(text, &state.query, 0));
    let Some(byte) = hit else {
        return;
    };
    let line = text[..byte].bytes().filter(|&b| b == b'\n').count() + 1;
    state.goto = Some(Goto {
        char_start: byte_to_char(text, byte),
        char_end: byte_to_char(text, byte + state.query.len()),
        line,
    });
}

fn find_ci(hay: &str, needle: &str, from: usize) -> Option<usize> {
    let h = hay.as_bytes();
    let n = needle.as_bytes();
    if n.is_empty() || h.len() < n.len() || from + n.len() > h.len() {
        return None;
    }
    (from..=h.len() - n.len()).find(|&i| h[i..i + n.len()].eq_ignore_ascii_case(n))
}

fn count_matches(text: &str, query: &str) -> usize {
    if query.is_empty() {
        return 0;
    }
    let mut count = 0;
    let mut at = 0;
    while let Some(i) = find_ci(text, query, at) {
        count += 1;
        at = i + query.len();
    }
    count
}

fn char_to_byte(s: &str, ci: usize) -> usize {
    s.char_indices().nth(ci).map(|(b, _)| b).unwrap_or(s.len())
}

fn byte_to_char(s: &str, bi: usize) -> usize {
    s[..bi].chars().count()
}

#[derive(Clone, Copy, PartialEq)]
enum TokenClass {
    Plain,
    Keyword,
    Builtin,
    String,
    Number,
    Comment,
}

fn highlight(text: &str, error_line: Option<usize>, font: egui::FontId) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;

    let mut line = 1usize;
    let mut i = 0;
    while i < text.len() {
        let (len, class) = next_token(&text[i..]);
        let token = &text[i..i + len];
        // Split at newlines so the error-line background stays per-line.
        for chunk in token.split_inclusive('\n') {
            let color = match class {
                TokenClass::Plain => theme::TEXT,
                TokenClass::Keyword => theme::KEYWORD,
                TokenClass::Builtin => theme::BUILTIN,
                TokenClass::String => theme::STRING,
                TokenClass::Number => theme::NUMBER,
                TokenClass::Comment => theme::COMMENT,
            };
            let background = if Some(line) == error_line {
                theme::ERROR_BG
            } else {
                egui::Color32::TRANSPARENT
            };
            job.append(
                chunk,
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color,
                    background,
                    ..Default::default()
                },
            );
            if chunk.ends_with('\n') {
                line += 1;
            }
        }
        i += len;
    }
    job
}

fn next_token(rest: &str) -> (usize, TokenClass) {
    let bytes = rest.as_bytes();
    let first = bytes[0];

    if rest.starts_with("--[[") {
        let len = rest.find("]]").map(|i| i + 2).unwrap_or(rest.len());
        return (len, TokenClass::Comment);
    }
    if rest.starts_with("--") {
        let len = rest.find('\n').unwrap_or(rest.len());
        return (len, TokenClass::Comment);
    }
    if first == b'"' || first == b'\'' {
        let quote = first as char;
        let mut len = 1;
        for (i, c) in rest.char_indices().skip(1) {
            len = i + c.len_utf8();
            if c == quote || c == '\n' {
                break;
            }
        }
        return (len, TokenClass::String);
    }
    if first.is_ascii_digit() {
        let len = rest
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '_')
            .unwrap_or(rest.len());
        return (len.max(1), TokenClass::Number);
    }
    if first.is_ascii_alphabetic() || first == b'_' {
        let len = rest
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .unwrap_or(rest.len());
        let word = &rest[..len];
        let class = if KEYWORDS.contains(&word) {
            TokenClass::Keyword
        } else if BUILTINS.contains(&word) {
            TokenClass::Builtin
        } else {
            TokenClass::Plain
        };
        return (len, class);
    }
    if first.is_ascii_whitespace() {
        let len = rest
            .find(|c: char| !c.is_ascii_whitespace())
            .unwrap_or(rest.len());
        return (len, TokenClass::Plain);
    }
    (
        rest.chars().next().map(char::len_utf8).unwrap_or(1),
        TokenClass::Plain,
    )
}
