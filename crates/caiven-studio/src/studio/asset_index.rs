use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetRef {
    pub path: String,
    pub line: usize,
    pub col: usize,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetEntry {
    pub kind: String,
    pub id: usize,
    pub used: bool,
    pub nonzero: bool,
    pub bytes: usize,
    pub refs: Vec<AssetRef>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndex {
    pub entries: Vec<AssetEntry>,
    pub computed_refs: usize,
}

struct CallKind {
    names: &'static [&'static str],
    kind: &'static str,
    max: usize,
}

const CALL_KINDS: &[CallKind] = &[
    CallKind {
        names: &["sprite"],
        kind: "sprite",
        max: 256,
    },
    CallKind {
        names: &["play_sfx"],
        kind: "sfx",
        max: 16,
    },
    CallKind {
        names: &["play_music"],
        kind: "music",
        max: 8,
    },
    CallKind {
        names: &["set_palette_color"],
        kind: "color",
        max: 16,
    },
];

pub fn build(
    sources: &[(String, String)],
    sprite_sheet: &[u8],
    map: &[u8],
    sfx: &[u8],
    music: &[u8],
    palette: &[u8],
) -> AssetIndex {
    let mut refs: BTreeMap<(String, usize), Vec<AssetRef>> = BTreeMap::new();
    let mut computed_refs = 0;

    for (path, text) in sources {
        for (line_index, line) in text.lines().enumerate() {
            let code = line.split("--").next().unwrap_or_default();
            for call_kind in CALL_KINDS {
                for name in call_kind.names {
                    let mut from = 0;
                    while let Some(relative) = code[from..].find(name) {
                        let at = from + relative;
                        if at > 0 && code.as_bytes()[at - 1].is_ascii_alphanumeric()
                            || at > 0 && code.as_bytes()[at - 1] == b'_'
                        {
                            from = at + name.len();
                            continue;
                        }
                        let after_name = &code[at + name.len()..];
                        let Some(open_relative) = after_name.find('(') else {
                            break;
                        };
                        if !after_name[..open_relative].trim().is_empty() {
                            from = at + name.len();
                            continue;
                        }
                        let arg = after_name[open_relative + 1..].trim_start();
                        let digits = arg.chars().take_while(char::is_ascii_digit).count();
                        if digits == 0 {
                            computed_refs += 1;
                        } else if let Ok(id) = arg[..digits].parse::<usize>()
                            && id < call_kind.max
                        {
                            refs.entry((call_kind.kind.to_string(), id))
                                .or_default()
                                .push(AssetRef {
                                    path: path.clone(),
                                    line: line_index + 1,
                                    col: at + 1,
                                    label: format!("{path}:{}", line_index + 1),
                                });
                        }
                        from = at + name.len();
                    }
                }
            }
        }
    }

    let mut map_sprites = BTreeSet::new();
    for &tile in map {
        map_sprites.insert(tile as usize);
    }
    for sprite in map_sprites {
        refs.entry(("sprite".to_string(), sprite))
            .or_default()
            .push(AssetRef {
                path: "map.png".to_string(),
                line: 0,
                col: 0,
                label: "map".to_string(),
            });
    }

    for (pattern, bytes) in music.chunks(32).enumerate() {
        for &value in bytes {
            if value > 0 && value <= 16 {
                refs.entry(("sfx".to_string(), (value - 1) as usize))
                    .or_default()
                    .push(AssetRef {
                        path: "music.hex".to_string(),
                        line: pattern + 1,
                        col: 0,
                        label: format!("music {pattern:02}"),
                    });
            }
        }
    }

    let mut color_counts = [0usize; 16];
    for &color in sprite_sheet {
        if let Some(count) = color_counts.get_mut(color as usize) {
            *count += 1;
        }
    }
    for (color, count) in color_counts.into_iter().enumerate() {
        if count > 0 {
            refs.entry(("color".to_string(), color))
                .or_default()
                .push(AssetRef {
                    path: "sprites.png".to_string(),
                    line: 0,
                    col: 0,
                    label: format!("sprite sheet · {count} px"),
                });
        }
    }

    let mut entries = Vec::with_capacity(296);
    add_entries(&mut entries, &mut refs, "sprite", 256, 64, sprite_sheet);
    add_entries(&mut entries, &mut refs, "sfx", 16, 64, sfx);
    add_entries(&mut entries, &mut refs, "music", 8, 32, music);
    add_entries(&mut entries, &mut refs, "color", 16, 3, palette);

    AssetIndex {
        entries,
        computed_refs,
    }
}

fn add_entries(
    entries: &mut Vec<AssetEntry>,
    refs: &mut BTreeMap<(String, usize), Vec<AssetRef>>,
    kind: &str,
    count: usize,
    bytes_per: usize,
    data: &[u8],
) {
    for id in 0..count {
        let start = id * bytes_per;
        let end = (start + bytes_per).min(data.len());
        let nonzero = start < data.len() && data[start..end].iter().any(|&byte| byte != 0);
        let refs = refs.remove(&(kind.to_string(), id)).unwrap_or_default();
        entries.push(AssetEntry {
            kind: kind.to_string(),
            id,
            used: !refs.is_empty(),
            nonzero,
            bytes: bytes_per,
            refs,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::build;

    #[test]
    fn indexes_literal_and_structural_references() {
        let sources = vec![(
            "main.lua".to_string(),
            "sprite(7, x, y)\nplay_sfx(dynamic_id)\nplay_music(2)\n".to_string(),
        )];
        let mut sprites = vec![0; 256 * 64];
        sprites[7 * 64] = 1;
        let map = vec![7, 9];
        let sfx = vec![0; 16 * 64];
        let mut music = vec![0; 8 * 32];
        music[2] = 4;
        let palette = vec![0; 48];

        let index = build(&sources, &sprites, &map, &sfx, &music, &palette);
        let sprite = index
            .entries
            .iter()
            .find(|entry| entry.kind == "sprite" && entry.id == 7)
            .unwrap();
        assert!(sprite.used);
        assert!(sprite.nonzero);
        assert_eq!(sprite.refs.len(), 2);
        assert_eq!(index.computed_refs, 1);
        assert!(index.entries.iter().any(|entry| {
            entry.kind == "sfx"
                && entry.id == 3
                && entry
                    .refs
                    .iter()
                    .any(|reference| reference.label == "music 00")
        }));
    }
}
