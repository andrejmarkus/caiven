//! Code editor panel: syntax-highlighted TextEdit with line numbers,
//! find (Ctrl+F / Ctrl+G), and clickable compile errors that jump to
//! the offending line.

use super::app::SourceFile;
use super::cart::CompileError;
use super::intellisense;
use super::theme;
use crate::debugger::Debugger;
use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};

const KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in",
    "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

/// Extra names not in `caiven_vm::vm::api_registry` (script entry points and
/// Lua's own stdlib globals/namespaces) — `string`/`table`/etc. are
/// highlighted as namespaces; their members (`string.format`) aren't, same
/// as most editors. `Particles` is the gameplay prelude's one namespace-style
/// table (its members, e.g. `Particles.spawn`, are dotted entries in the
/// registry's `PRELUDE` and highlight via that path instead).
const EXTRA_BUILTINS: &[&str] = &[
    "_init",
    "_update",
    "_draw",
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
    "Particles",
];

/// Console builtins from the registry (single source of truth, see
/// `caiven-vm/src/vm/api_registry.rs`) plus `EXTRA_BUILTINS` above. Registry
/// names like `math.sin` include a `.` and won't match any single identifier
/// token, so they're harmless to include here — `next_token`'s word-scan
/// simply never produces them as one token.
fn is_builtin(word: &str) -> bool {
    EXTRA_BUILTINS.contains(&word) || caiven_vm::vm::api_registry::all_names().any(|n| n == word)
}

const EDITOR_ID: &str = "cav_code_editor";

struct Goto {
    char_start: usize,
    char_end: usize,
    line: usize,
}

/// Autocomplete popup state — see [`update_autocomplete`].
#[derive(Default)]
struct Autocomplete {
    open: bool,
    candidates: Vec<String>,
    selected: usize,
    replace_start: usize,
}

#[derive(Default)]
pub struct CodeState {
    pub error: Option<CompileError>,
    find_open: bool,
    find_focus: bool,
    query: String,
    goto: Option<Goto>,
    ac: Autocomplete,
}

impl CodeState {
    /// Scrolls the editor to `line` and places the cursor there — same jump
    /// used by the compile-error bar's click handler, exposed so other
    /// panels (e.g. the game view's runtime-error overlay) can trigger it.
    pub fn goto_line(&mut self, text: &str, line: usize) {
        self.goto = Some(goto_line(text, line));
    }
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
                .id(egui::Id::new("cav_code_find"))
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

    // Autocomplete keyboard handling must run BEFORE the TextEdit sees this
    // frame's input — otherwise the multiline editor consumes Enter
    // (newline) / Up / Down (cursor move) itself first, and by the time we
    // look for them they're already gone.
    let just_accepted = state.ac.open && handle_autocomplete_keys(ui, state, source, editor_id);
    let manual_trigger = ui.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Space));

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

            if !just_accepted {
                refresh_autocomplete(state, source, &output, manual_trigger);
            }
            if state.ac.open {
                // egui surrenders keyboard focus from any focused widget on
                // Escape unless that widget's focus-lock filter says
                // otherwise — a core behavior that runs before our own
                // Escape handling above ever gets a chance. Without this,
                // dismissing the popup also kicks focus out of the editor.
                ui.memory_mut(|mem| {
                    mem.set_focus_lock_filter(
                        editor_id,
                        egui::EventFilter {
                            escape: true,
                            horizontal_arrows: true,
                            vertical_arrows: true,
                            tab: false,
                        },
                    );
                });
                render_autocomplete_popup(ui, state, source, editor_id, &output);
            } else {
                let symbols = intellisense::scan_buffer(&source.text);
                show_hover_doc(ui, &output, &source.text, &symbols);
                show_signature_help(ui, &output, &source.text);
            }
        });
    });
}

/// Consumes popup-navigation keys before the `TextEdit` widget gets a
/// chance to (it would otherwise handle Enter/Up/Down itself). Returns
/// `true` if a candidate was accepted this frame, so the caller skips
/// re-triggering the popup off the resulting text change.
fn handle_autocomplete_keys(
    ui: &mut egui::Ui,
    state: &mut CodeState,
    source: &mut SourceFile,
    editor_id: egui::Id,
) -> bool {
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        state.ac.open = false;
        return false;
    }
    if state.ac.candidates.is_empty() {
        return false;
    }
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
        state.ac.selected = (state.ac.selected + 1) % state.ac.candidates.len();
    }
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
        state.ac.selected =
            (state.ac.selected + state.ac.candidates.len() - 1) % state.ac.candidates.len();
    }
    let accept = ui.input_mut(|i| {
        i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
            || i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)
    });
    if !accept {
        return false;
    }
    accept_candidate(ui.ctx(), state, source, editor_id);
    true
}

/// Splices the selected candidate into `source.text` in place of the
/// in-progress prefix, and moves the cursor to just after it.
fn accept_candidate(
    ctx: &egui::Context,
    state: &mut CodeState,
    source: &mut SourceFile,
    editor_id: egui::Id,
) {
    let candidate = state.ac.candidates[state.ac.selected].clone();
    let cursor = egui::TextEdit::load_state(ctx, editor_id)
        .and_then(|s| s.cursor.char_range())
        .map(|r| r.primary.index)
        .unwrap_or(state.ac.replace_start);

    let chars: Vec<char> = source.text.chars().collect();
    let start = state.ac.replace_start.min(chars.len());
    let end = cursor.min(chars.len()).max(start);
    let before: String = chars[..start].iter().collect();
    let after: String = chars[end..].iter().collect();
    source.text = format!("{before}{candidate}{after}");

    let new_cursor = start + candidate.chars().count();
    let mut st = egui::TextEdit::load_state(ctx, editor_id).unwrap_or_default();
    st.cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(new_cursor))));
    st.store(ctx, editor_id);

    state.ac.open = false;
}

/// Splices `text` in at the editor's last known cursor position (or at the
/// end of the buffer if the editor has never been focused this session) —
/// used by the command palette and API reference panel's "insert" actions,
/// which act on the code editor from outside its own `show()` call.
pub fn insert_at_cursor(ctx: &egui::Context, source: &mut SourceFile, text: &str) {
    let editor_id = egui::Id::new(EDITOR_ID);
    let cursor = egui::TextEdit::load_state(ctx, editor_id)
        .and_then(|s| s.cursor.char_range())
        .map(|r| r.primary.index)
        .unwrap_or_else(|| source.text.chars().count());

    let chars: Vec<char> = source.text.chars().collect();
    let at = cursor.min(chars.len());
    let before: String = chars[..at].iter().collect();
    let after: String = chars[at..].iter().collect();
    source.text = format!("{before}{text}{after}");
    source.dirty = true;

    let new_cursor = at + text.chars().count();
    let mut st = egui::TextEdit::load_state(ctx, editor_id).unwrap_or_default();
    st.cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(new_cursor))));
    st.store(ctx, editor_id);
}

/// Recomputes the in-progress identifier at the cursor and opens/updates/
/// closes the popup accordingly. Only *opens* a closed popup when the text
/// actually changed this frame (typing) or `manual` is set (an explicit
/// Ctrl+Space) — merely moving the cursor around (arrow keys, clicking)
/// must never pop it up on its own. Once open, it stays synced to the
/// cursor every frame until dismissed or the context no longer applies.
fn refresh_autocomplete(
    state: &mut CodeState,
    source: &mut SourceFile,
    output: &egui::text_edit::TextEditOutput,
    manual: bool,
) {
    if !output.response.has_focus() {
        state.ac.open = false;
        return;
    }
    if !manual && !output.response.changed() && !state.ac.open {
        return;
    }

    let Some(range) = output.state.cursor.char_range() else {
        state.ac.open = false;
        return;
    };
    if range.primary.index != range.secondary.index {
        state.ac.open = false;
        return;
    }
    let cursor = range.primary.index;

    let (replace_start, prefix) = if manual {
        intellisense::completion_context_or_empty(&source.text, cursor)
    } else {
        let Some(ctx) = intellisense::completion_context(&source.text, cursor) else {
            state.ac.open = false;
            return;
        };
        ctx
    };
    let in_code = !matches!(
        token_class_at_char(&source.text, cursor.saturating_sub(1)),
        TokenClass::String | TokenClass::Comment
    );
    if !in_code {
        state.ac.open = false;
        return;
    }

    let symbols = intellisense::scan_buffer(&source.text);
    let mut candidates: Vec<String> = caiven_vm::vm::api_registry::all_names()
        .map(str::to_string)
        .chain(symbols.into_iter().map(|s| s.name))
        .filter(|n| n.starts_with(&prefix) && n != &prefix)
        .collect();
    candidates.sort();
    candidates.dedup();
    candidates.truncate(50);

    if candidates.is_empty() {
        state.ac.open = false;
        return;
    }

    let previous = state.ac.candidates.get(state.ac.selected).cloned();
    state.ac.selected = previous
        .and_then(|p| candidates.iter().position(|c| *c == p))
        .unwrap_or(0);
    state.ac.candidates = candidates;
    state.ac.replace_start = replace_start;
    state.ac.open = true;
}

/// Renders the floating candidate list at the cursor and applies a click.
fn render_autocomplete_popup(
    ui: &mut egui::Ui,
    state: &mut CodeState,
    source: &mut SourceFile,
    editor_id: egui::Id,
    output: &egui::text_edit::TextEditOutput,
) {
    let Some(range) = output.state.cursor.char_range() else {
        return;
    };
    let rect = output
        .galley
        .pos_from_cursor(CCursor::new(range.primary.index));
    let pos = output.galley_pos + rect.left_bottom().to_vec2();

    let mut clicked = None;
    egui::Area::new(egui::Id::new("cav_autocomplete_popup"))
        .fixed_pos(pos)
        .movable(false)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                for (i, cand) in state.ac.candidates.iter().enumerate() {
                    let resp = ui.selectable_label(
                        i == state.ac.selected,
                        egui::RichText::new(cand).monospace(),
                    );
                    if resp.clicked() {
                        clicked = Some(i);
                    }
                }
            });
        });

    if let Some(i) = clicked {
        state.ac.selected = i;
        accept_candidate(ui.ctx(), state, source, editor_id);
    }
}

/// Tooltip with signature + one-line doc when hovering a builtin, stdlib
/// member, or a symbol declared elsewhere in this buffer.
fn show_hover_doc(
    ui: &mut egui::Ui,
    output: &egui::text_edit::TextEditOutput,
    text: &str,
    symbols: &[intellisense::Symbol],
) {
    if !output.response.hovered() {
        return;
    }
    let Some(pointer) = ui.ctx().pointer_hover_pos() else {
        return;
    };
    let local = pointer - output.galley_pos;
    let cursor = output.galley.cursor_from_pos(local);
    let Some(word) = intellisense::token_at_char(text, cursor.index) else {
        return;
    };
    if matches!(
        token_class_at_char(text, cursor.index),
        TokenClass::String | TokenClass::Comment
    ) {
        return;
    }

    let qualified =
        intellisense::qualified_token_at_char(text, cursor.index).unwrap_or_else(|| word.clone());
    let entry = caiven_vm::vm::api_registry::lookup(&qualified);
    let symbol = symbols.iter().find(|s| s.name == word);
    if entry.is_none() && symbol.is_none() {
        return;
    }

    egui::Tooltip::always_open(
        ui.ctx().clone(),
        output.response.layer_id,
        egui::Id::new("cav_hover_doc"),
        egui::PopupAnchor::Pointer,
    )
    .at_pointer()
    .show(|ui| {
        if let Some(entry) = entry {
            let params = entry
                .params
                .iter()
                .map(|p| format!("{}: {}", p.name, p.ty))
                .collect::<Vec<_>>()
                .join(", ");
            ui.monospace(format!("{}({}) -> {}", entry.name, params, entry.returns));
            ui.label(entry.doc);
        } else if let Some(sym) = symbol {
            let kind = match sym.kind {
                intellisense::SymbolKind::Local => "local",
                intellisense::SymbolKind::Function => "function",
            };
            ui.label(format!(
                "{kind} {} — declared at line {}",
                sym.name, sym.line
            ));
        }
    });
}

/// Overlay showing a builtin/stdlib function's full signature, with the
/// parameter under the cursor bolded, while typing inside a call's parens.
fn show_signature_help(ui: &mut egui::Ui, output: &egui::text_edit::TextEditOutput, text: &str) {
    let Some(range) = output.state.cursor.char_range() else {
        return;
    };
    let cursor = range.primary.index;
    let Some((name, active)) = intellisense::call_context_at_cursor(text, cursor) else {
        return;
    };
    let Some(entry) = caiven_vm::vm::api_registry::lookup(&name) else {
        return;
    };
    if entry.params.is_empty() {
        return;
    }

    let rect = output.galley.pos_from_cursor(CCursor::new(cursor));
    let pos = output.galley_pos + rect.left_top().to_vec2() - egui::vec2(0.0, 4.0);

    egui::Area::new(egui::Id::new("cav_signature_help"))
        .fixed_pos(pos)
        .pivot(egui::Align2::LEFT_BOTTOM)
        .movable(false)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.monospace(format!("{}(", entry.name));
                    for (i, p) in entry.params.iter().enumerate() {
                        if i > 0 {
                            ui.monospace(", ");
                        }
                        let text = egui::RichText::new(format!("{}: {}", p.name, p.ty)).monospace();
                        let text = if i == active {
                            text.color(theme::ACCENT)
                        } else {
                            text
                        };
                        ui.label(text);
                    }
                    ui.monospace(format!(") -> {}", entry.returns));
                });
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

/// Classifies the token containing char index `ch`, so callers (autocomplete,
/// hover docs) can suppress themselves inside string literals and comments —
/// `next_token`'s word-scan can't otherwise tell "hello" the identifier from
/// "hello" the string content.
fn token_class_at_char(text: &str, ch: usize) -> TokenClass {
    if text.is_empty() {
        return TokenClass::Plain;
    }
    let target = char_to_byte(text, ch).min(text.len() - 1);
    let mut i = 0;
    while i < text.len() {
        let (len, class) = next_token(&text[i..]);
        let len = len.max(1);
        if target < i + len {
            return class;
        }
        i += len;
    }
    TokenClass::Plain
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
        } else if is_builtin(word) {
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
