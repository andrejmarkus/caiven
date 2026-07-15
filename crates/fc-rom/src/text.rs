//! Text-embedded asset sections for `.fc` cart source files.
//!
//! A `.fc` file is fc-lang code optionally followed by hex data blocks,
//! one per asset kind:
//!
//! ```text
//! -- game code ...
//!
//! __gfx__
//! 0011aa...
//! __pal__
//! 0a0a1e...
//! ```
//!
//! Markers: `__gfx__` (sprite sheet), `__map__`, `__flags__` (sprite
//! flags), `__pal__` (palette), `__sfx__`, `__music__`. Everything before
//! the first marker is code; each block is raw hex, two chars per byte,
//! whitespace between lines ignored.

use crate::SectionKind;

const MARKERS: [(&str, SectionKind); 6] = [
    ("__gfx__", SectionKind::SpriteSheet),
    ("__map__", SectionKind::Map),
    ("__flags__", SectionKind::SpriteFlags),
    ("__pal__", SectionKind::Palette),
    ("__sfx__", SectionKind::SfxBank),
    ("__music__", SectionKind::MusicBank),
];

fn marker_for(line: &str) -> Option<SectionKind> {
    MARKERS
        .iter()
        .find(|(m, _)| line.trim() == *m)
        .map(|(_, k)| *k)
}

fn marker_name(kind: SectionKind) -> Option<&'static str> {
    MARKERS.iter().find(|(_, k)| *k == kind).map(|(m, _)| *m)
}

/// A parsed asset section: its kind and raw decoded bytes.
pub type Section = (SectionKind, Vec<u8>);

/// Splits `.fc` source into the code part and any embedded asset sections.
/// The code part keeps its trailing newline structure so compile-error line
/// numbers match the file on disk.
pub fn split_source(src: &str) -> Result<(String, Vec<Section>), String> {
    let mut code_end: Option<usize> = None;
    let mut sections: Vec<Section> = Vec::new();
    let mut current: Option<usize> = None;

    let mut offset = 0;
    for line in src.split_inclusive('\n') {
        let stripped = line.strip_suffix('\n').unwrap_or(line);
        if let Some(kind) = marker_for(stripped) {
            if code_end.is_none() {
                code_end = Some(offset);
            }
            sections.push((kind, Vec::new()));
            current = Some(sections.len() - 1);
        } else if let Some(idx) = current {
            let hex = stripped.trim();
            if !hex.is_empty() {
                decode_hex_line(hex, &mut sections[idx].1).map_err(|e| {
                    format!("bad hex in {} block: {e}", marker_label(sections[idx].0))
                })?;
            }
        }
        offset += line.len();
    }

    let code = src[..code_end.unwrap_or(src.len())].to_string();
    Ok((code, sections))
}

fn marker_label(kind: SectionKind) -> &'static str {
    marker_name(kind).unwrap_or("data")
}

fn decode_hex_line(hex: &str, out: &mut Vec<u8>) -> Result<(), String> {
    let bytes = hex.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return Err(format!("odd number of hex digits in line '{hex}'"));
    }
    for pair in bytes.chunks_exact(2) {
        let hi =
            hex_val(pair[0]).ok_or_else(|| format!("invalid hex digit '{}'", pair[0] as char))?;
        let lo =
            hex_val(pair[1]).ok_or_else(|| format!("invalid hex digit '{}'", pair[1] as char))?;
        out.push((hi << 4) | lo);
    }
    Ok(())
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Serializes code plus asset sections back into a single `.fc` text.
/// Sections with no data are skipped; trailing zero bytes are trimmed
/// (parsers must treat missing tail bytes as zero).
pub fn join_source(code: &str, sections: &[(SectionKind, Vec<u8>)]) -> String {
    let mut out = String::from(code.trim_end_matches(['\n', '\r']));
    out.push('\n');

    for (kind, data) in sections {
        let trimmed = trim_trailing_zeros(data);
        if trimmed.is_empty() {
            continue;
        }
        let Some(marker) = marker_name(*kind) else {
            continue;
        };
        out.push('\n');
        out.push_str(marker);
        out.push('\n');
        for chunk in trimmed.chunks(64) {
            for b in chunk {
                out.push_str(&format!("{b:02x}"));
            }
            out.push('\n');
        }
    }
    out
}

fn trim_trailing_zeros(data: &[u8]) -> &[u8] {
    let end = data.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    &data[..end]
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn split_plain_code_has_no_sections() {
        let (code, sections) = split_source("loop:\n  cls()\n").unwrap();
        assert_eq!(code, "loop:\n  cls()\n");
        assert!(sections.is_empty());
    }

    #[test]
    fn roundtrip_preserves_code_and_data() {
        let code = "loop:\n  spr(0, 10, 10)\n  wait()\n";
        let gfx: Vec<u8> = (0..130).map(|i| (i % 16) as u8).collect();
        let pal = vec![10u8, 20, 30];
        let sections = vec![
            (SectionKind::SpriteSheet, gfx.clone()),
            (SectionKind::Palette, pal.clone()),
        ];
        let text = join_source(code, &sections);
        let (code2, sections2) = split_source(&text).unwrap();
        assert_eq!(code2.trim_end(), code.trim_end());
        assert_eq!(sections2.len(), 2);
        assert_eq!(sections2[0], (SectionKind::SpriteSheet, gfx));
        assert_eq!(sections2[1], (SectionKind::Palette, pal));
    }

    #[test]
    fn join_trims_trailing_zeros_and_skips_empty() {
        let sections = vec![
            (SectionKind::SpriteSheet, vec![1, 2, 0, 0]),
            (SectionKind::Map, vec![0, 0, 0]),
        ];
        let text = join_source("code\n", &sections);
        assert!(text.contains("__gfx__\n0102\n"));
        assert!(!text.contains("__map__"));
    }

    #[test]
    fn split_rejects_bad_hex() {
        let err = split_source("code\n__pal__\nzz\n").unwrap_err();
        assert!(err.contains("__pal__"));
    }

    #[test]
    fn marker_must_be_alone_on_line() {
        let (code, sections) = split_source("let x = 1 -- __gfx__ in comment\n").unwrap();
        assert!(sections.is_empty());
        assert!(code.contains("__gfx__ in comment"));
    }
}
