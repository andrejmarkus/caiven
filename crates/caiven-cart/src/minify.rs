use crate::section::{CartSection, SectionKind};

/// Strips comments and collapses whitespace/indentation in Lua source, so a
/// distributed cart doesn't hand out commented, formatted source to anyone
/// who opens the `.cav`. Deliberately conservative: string/long-string
/// contents are copied verbatim, and every line break collapses to exactly
/// one `\n` (never dropped entirely) to avoid Lua's statement-adjacency
/// ambiguity, e.g. `a = b` followed by `(f)()` on the next line must not
/// become `a = b(f)()`.
pub fn minify_lua(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    let n = bytes.len();

    while i < n {
        let c = bytes[i];

        // Short string literal.
        if c == b'"' || c == b'\'' {
            let quote = c;
            let start = i;
            i += 1;
            while i < n {
                if bytes[i] == b'\\' && i + 1 < n {
                    i += 2;
                    continue;
                }
                if bytes[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            out.push_str(&src[start..i]);
            continue;
        }

        // Long bracket: `[=*[ ... ]=*]`, either a raw long string or a
        // `--[=*[ ... ]=*]` long comment (handled by the caller below).
        if c == b'[' {
            if let Some(level) = long_bracket_level(bytes, i) {
                let start = i;
                let end = skip_long_bracket(bytes, i, level);
                out.push_str(&src[start..end]);
                i = end;
                continue;
            }
        }

        // Comment: `--` optionally followed by a long bracket.
        if c == b'-' && i + 1 < n && bytes[i + 1] == b'-' {
            let after_dashes = i + 2;
            if after_dashes < n && bytes[after_dashes] == b'[' {
                if let Some(level) = long_bracket_level(bytes, after_dashes) {
                    i = skip_long_bracket(bytes, after_dashes, level);
                    continue;
                }
            }
            // Line comment: skip to end of line (newline handled next loop).
            i = after_dashes;
            while i < n && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Whitespace run: collapse to one '\n' if it contains a newline,
        // else a single space.
        if c == b' ' || c == b'\t' || c == b'\r' || c == b'\n' {
            let mut has_newline = false;
            while i < n && matches!(bytes[i], b' ' | b'\t' | b'\r' | b'\n') {
                if bytes[i] == b'\n' {
                    has_newline = true;
                }
                i += 1;
            }
            out.push(if has_newline { '\n' } else { ' ' });
            continue;
        }

        // Everything else (identifiers, operators, numbers, punctuation).
        // Decode a full char so non-ASCII bytes (e.g. UTF-8 identifiers)
        // round-trip correctly instead of being reinterpreted byte-by-byte.
        let ch = src[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }

    out
}

/// If `bytes[pos..]` starts a long-bracket opener `[=*[`, returns its level
/// (number of `=` signs). Otherwise `None`.
fn long_bracket_level(bytes: &[u8], pos: usize) -> Option<usize> {
    if bytes.get(pos) != Some(&b'[') {
        return None;
    }
    let mut j = pos + 1;
    let mut level = 0;
    while bytes.get(j) == Some(&b'=') {
        level += 1;
        j += 1;
    }
    if bytes.get(j) == Some(&b'[') {
        Some(level)
    } else {
        None
    }
}

/// Given a confirmed long-bracket opener at `pos` with `level` `=` signs,
/// returns the index just past the matching closer `]=*]` (or end of input
/// if unterminated).
fn skip_long_bracket(bytes: &[u8], pos: usize, level: usize) -> usize {
    let n = bytes.len();
    let mut i = pos + 2 + level; // past `[=*[`
    while i < n {
        if bytes[i] == b']' {
            let mut j = i + 1;
            let mut lvl = 0;
            while bytes.get(j) == Some(&b'=') {
                lvl += 1;
                j += 1;
            }
            if lvl == level && bytes.get(j) == Some(&b']') {
                return j + 1;
            }
        }
        i += 1;
    }
    n
}

/// Minifies the `LuaSource` section in place, if present. No-op otherwise.
pub fn minify_cart_lua(sections: &mut [CartSection]) {
    for section in sections.iter_mut() {
        if section.kind == SectionKind::LuaSource {
            let text = String::from_utf8_lossy(&section.data).into_owned();
            section.data = minify_lua(&text).into_bytes();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_line_comments() {
        let src = "-- header comment\nlocal x = 1 -- trailing\nprint(x)\n";
        let out = minify_lua(src);
        assert!(!out.contains("comment"));
        assert!(!out.contains("trailing"));
        assert!(out.contains("local x = 1"));
        assert!(out.contains("print(x)"));
    }

    #[test]
    fn strips_block_comments() {
        let src = "--[[\nmulti\nline\ncomment\n]]\nlocal y = 2\n";
        let out = minify_lua(src);
        assert!(!out.contains("multi"));
        assert!(out.contains("local y = 2"));
    }

    #[test]
    fn strips_leveled_block_comments() {
        let src = "--[==[ has ]] inside ]==]\nlocal z = 3\n";
        let out = minify_lua(src);
        assert!(!out.contains("has"));
        assert!(out.contains("local z = 3"));
    }

    #[test]
    fn preserves_string_with_dashes() {
        let src = r#"local s = "--not a comment""#;
        let out = minify_lua(src);
        assert!(out.contains("--not a comment"));
    }

    #[test]
    fn preserves_string_with_bracket() {
        let src = r#"local s = "[[not a long string]]""#;
        let out = minify_lua(src);
        assert!(out.contains("[[not a long string]]"));
    }

    #[test]
    fn preserves_long_string_contents() {
        let src = "local s = [==[ raw -- text [[ here ]==]\nprint(s)\n";
        let out = minify_lua(src);
        assert!(out.contains("raw -- text [[ here"));
        assert!(out.contains("print(s)"));
    }

    #[test]
    fn line_comment_with_no_trailing_newline() {
        let src = "print(1) -- done";
        let out = minify_lua(src);
        assert!(out.trim() == "print(1)");
    }

    #[test]
    fn newline_ambiguity_preserved() {
        // `a = b` then a call on the next line must NOT become `a = b(f)()`.
        let src = "a = b\n(f)()\n";
        let out = minify_lua(src);
        assert_eq!(out, "a = b\n(f)()\n");
    }

    #[test]
    fn collapses_indentation_and_blank_lines() {
        let src = "function f()\n\n    local x = 1\n\n    return x\nend\n";
        let out = minify_lua(src);
        assert!(!out.contains("    "));
        assert!(out.contains("function f()"));
        assert!(out.contains("local x = 1"));
        assert!(out.contains("return x"));
        assert!(out.contains("end"));
    }

    #[test]
    fn escaped_quote_inside_string() {
        let src = r#"local s = "a \" -- b"
print(s)"#;
        let out = minify_lua(src);
        assert!(out.contains(r#""a \" -- b""#));
        assert!(out.contains("print(s)"));
    }
}
