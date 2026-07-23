//! Editor-side intellisense support: scanning the buffer for user-defined
//! symbols, finding the token under a char position (for hover docs), and
//! detecting the enclosing function call (for signature help). Consumed by
//! `code_panel.rs`; API metadata itself lives in
//! `caiven_vm::vm::api_registry`.

use regex::Regex;
use std::sync::LazyLock;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Local,
    Function,
}

pub struct Symbol {
    pub name: String,
    pub line: usize,
    pub kind: SymbolKind,
}

static LOCAL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\s*local\s+([A-Za-z_]\w*)").unwrap());
static FUNCTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:local\s+)?function\s+([A-Za-z_][\w.:]*)\s*\(").unwrap()
});

/// Scans the whole buffer for `local x = ...` and `function name(...)` /
/// `local function name(...)` declarations, for autocomplete + hover.
pub fn scan_buffer(text: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    for caps in LOCAL_RE.captures_iter(text) {
        let m = caps.get(1).unwrap();
        symbols.push(Symbol {
            name: m.as_str().to_string(),
            line: line_at_byte(text, m.start()),
            kind: SymbolKind::Local,
        });
    }
    for caps in FUNCTION_RE.captures_iter(text) {
        let m = caps.get(1).unwrap();
        symbols.push(Symbol {
            name: m.as_str().to_string(),
            line: line_at_byte(text, m.start()),
            kind: SymbolKind::Function,
        });
    }
    symbols
}

fn line_at_byte(text: &str, byte: usize) -> usize {
    text[..byte].bytes().filter(|&b| b == b'\n').count() + 1
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Computes `(replace_start, prefix)` for the in-progress identifier at the
/// cursor — e.g. typing `sp` gives prefix `"sp"`, and typing `math.s` gives
/// prefix `"math.s"` with `replace_start` at the start of `math`, so
/// accepting a qualified candidate like `math.sin` replaces the whole
/// thing. `prefix` is `""` (with `replace_start == cursor`) when the cursor
/// isn't after an identifier char at all.
fn completion_prefix_at(text: &str, cursor: usize) -> (usize, String) {
    let chars: Vec<char> = text.chars().collect();
    let cursor = cursor.min(chars.len());

    let mut start = cursor;
    while start > 0 && is_ident_char(chars[start - 1]) {
        start -= 1;
    }
    let mut prefix: String = chars[start..cursor].iter().collect();
    let mut replace_start = start;

    if start > 0 && chars[start - 1] == '.' {
        let dot = start - 1;
        let mut ns_start = dot;
        while ns_start > 0 && is_ident_char(chars[ns_start - 1]) {
            ns_start -= 1;
        }
        if ns_start < dot {
            let ns: String = chars[ns_start..dot].iter().collect();
            prefix = format!("{ns}.{prefix}");
            replace_start = ns_start;
        }
    }

    (replace_start, prefix)
}

/// Like [`completion_prefix_at`], but returns `None` when there's nothing
/// to complete — used to gate autocomplete triggered by typing, so it never
/// pops up off a bare cursor move.
pub fn completion_context(text: &str, cursor: usize) -> Option<(usize, String)> {
    let (start, prefix) = completion_prefix_at(text, cursor);
    if prefix.is_empty() {
        None
    } else {
        Some((start, prefix))
    }
}

/// Like [`completion_context`], but never returns `None` — an empty prefix
/// just means "show everything". Used for an explicit Ctrl+Space invoke,
/// where the user asked for the list regardless of what's typed so far.
pub fn completion_context_or_empty(text: &str, cursor: usize) -> (usize, String) {
    completion_prefix_at(text, cursor)
}

/// `ch` from a mouse-position hit test (`Galley::cursor_from_pos`) is a gap
/// index — the nearest insertion point to the pointer, not "the glyph under
/// it". Hovering the right half or tail of a word commonly resolves to the
/// gap just *after* it, where `chars[ch]` is whatever comes next (`(`,
/// space, `,`) — so we also try the char just before the gap.
fn ident_span_at(chars: &[char], ch: usize) -> Option<(usize, usize)> {
    if chars.is_empty() {
        return None;
    }
    let ch = ch.min(chars.len() - 1);
    let anchor = if is_ident_char(chars[ch]) {
        ch
    } else if ch > 0 && is_ident_char(chars[ch - 1]) {
        ch - 1
    } else {
        return None;
    };
    let mut start = anchor;
    while start > 0 && is_ident_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = anchor;
    while end + 1 < chars.len() && is_ident_char(chars[end + 1]) {
        end += 1;
    }
    Some((start, end))
}

/// Returns the identifier token at or adjacent to char index `ch` (see
/// [`ident_span_at`]), for hover-doc lookups.
pub fn token_at_char(text: &str, ch: usize) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let (start, end) = ident_span_at(&chars, ch)?;
    Some(chars[start..=end].iter().collect())
}

/// Like [`token_at_char`], but if the token is preceded by `namespace.`,
/// returns the dot-qualified name (`math.sin`) instead of the bare member
/// name — stdlib entries in the API registry are keyed by their qualified
/// name, so a plain `sin` would never match.
pub fn qualified_token_at_char(text: &str, ch: usize) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let (start, end) = ident_span_at(&chars, ch)?;
    let word: String = chars[start..=end].iter().collect();

    if start > 0 && chars[start - 1] == '.' {
        let dot = start - 1;
        let mut ns_start = dot;
        while ns_start > 0 && is_ident_char(chars[ns_start - 1]) {
            ns_start -= 1;
        }
        if ns_start < dot {
            let ns: String = chars[ns_start..dot].iter().collect();
            return Some(format!("{ns}.{word}"));
        }
    }
    Some(word)
}

/// If the cursor sits inside an open `name(` call, returns the function
/// name and the (0-based) index of the parameter the cursor is currently in.
pub fn call_context_at_cursor(text: &str, cursor: usize) -> Option<(String, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let cursor = cursor.min(chars.len());
    let mut depth = 0i32;
    let mut param_index = 0usize;
    let mut i = cursor;
    while i > 0 {
        i -= 1;
        match chars[i] {
            ')' => depth += 1,
            ',' if depth == 0 => param_index += 1,
            '(' => {
                if depth == 0 {
                    let mut end = i;
                    while end > 0
                        && (chars[end - 1].is_alphanumeric()
                            || chars[end - 1] == '_'
                            || chars[end - 1] == '.')
                    {
                        end -= 1;
                    }
                    if end == i {
                        return None;
                    }
                    let name: String = chars[end..i].iter().collect();
                    return Some((name, param_index));
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
}
