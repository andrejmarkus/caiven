//! Drift guard: the RAM memory map is single-sourced in `src/memory.rs`
//! (`MemRegion`), but it is also hand-copied into two frontend files and the
//! README for humans to read. Nothing stops those copies from going stale
//! when a region in `MemRegion` is resized, so this test parses each copy
//! and asserts every address it finds still matches `MemRegion`.
//!
//! On failure, the assertion message names the file, the region, the value
//! found in that file, and the value `MemRegion` actually expects — that's
//! the fix.

use caiven_core::memory::MemRegion;
use regex::Regex;

const IPC_TS: &str = include_str!("../../caiven-studio-ui/src/lib/ipc.ts");
const DRAWER_MATH_TS: &str = include_str!("../../caiven-studio-ui/src/lib/drawerMath.ts");
const README: &str = include_str!("../../../README.md");

fn parse_hex(s: &str) -> usize {
    usize::from_str_radix(s, 16).expect("regex only captures hex digits")
}

#[test]
fn ipc_ts_memory_object_matches_layout() {
    let re = Regex::new(r"(\w+):\s*0x([0-9A-Fa-f]+)").expect("valid regex");
    let cases: [(&str, MemRegion); 6] = [
        ("sprites", MemRegion::SpriteSheet),
        ("map", MemRegion::Map),
        ("palette", MemRegion::Palette),
        ("sfx", MemRegion::Sfx),
        ("music", MemRegion::Music),
        ("collision", MemRegion::Collision),
    ];

    // Scope to the `MEMORY` object literal so we don't match unrelated `key: 0x…` pairs.
    let start = IPC_TS
        .find("export const MEMORY")
        .expect("ipc.ts: MEMORY object not found");
    let end = IPC_TS[start..]
        .find("as const")
        .expect("ipc.ts: MEMORY object not closed")
        + start;
    let block = &IPC_TS[start..end];

    for (key, region) in cases {
        let found = re
            .captures_iter(block)
            .find(|c| &c[1] == key)
            .unwrap_or_else(|| panic!("ipc.ts: MEMORY.{key} not found"));
        let got = parse_hex(&found[2]);
        let want = region.base();
        assert_eq!(
            got, want,
            "ipc.ts MEMORY.{key}: found 0x{got:04X}, expected 0x{want:04X} from MemRegion::{region:?}"
        );
    }
}

#[test]
fn drawer_math_ts_regions_match_layout() {
    let re = Regex::new(r"label:\s*'(\w+)',\s*address:\s*0x([0-9A-Fa-f]+)").expect("valid regex");
    let cases: [(&str, MemRegion); 6] = [
        ("WORK", MemRegion::Work),
        ("SPRITES", MemRegion::SpriteSheet),
        ("MAP", MemRegion::Map),
        ("PALETTE", MemRegion::Palette),
        ("SFX", MemRegion::Sfx),
        ("MUSIC", MemRegion::Music),
    ];

    for (label, region) in cases {
        let found = re
            .captures_iter(DRAWER_MATH_TS)
            .find(|c| &c[1] == label)
            .unwrap_or_else(|| panic!("drawerMath.ts: MEMORY_REGIONS entry '{label}' not found"));
        let got = parse_hex(&found[2]);
        let want = region.base();
        assert_eq!(
            got, want,
            "drawerMath.ts MEMORY_REGIONS '{label}': found 0x{got:04X}, expected 0x{want:04X} from MemRegion::{region:?}"
        );
    }
}

#[test]
fn readme_memory_map_table_matches_layout() {
    let re =
        Regex::new(r"`0x([0-9A-Fa-f]+).0x([0-9A-Fa-f]+)`\s*\|\s*([^|]+)\|").expect("valid regex");
    // (keyword to find in the row's description, region it names)
    let cases: [(&str, MemRegion); 8] = [
        ("Sprite sheet", MemRegion::SpriteSheet),
        ("Tilemap", MemRegion::Map),
        ("Palette", MemRegion::Palette),
        ("SFX bank", MemRegion::Sfx),
        ("Music bank", MemRegion::Music),
        ("RTC", MemRegion::Rtc),
        ("Collision", MemRegion::Collision),
        ("Reserved", MemRegion::Heap),
    ];

    for (keyword, region) in cases {
        let row = re
            .captures_iter(README)
            .find(|c| c[3].contains(keyword))
            .unwrap_or_else(|| panic!("README.md: memory-map row for '{keyword}' not found"));
        let got_base = parse_hex(&row[1]);
        let got_end = parse_hex(&row[2]);
        let want_base = region.base();
        let want_end = region.base() + region.span() - 1;
        assert_eq!(
            got_base, want_base,
            "README.md '{keyword}' row: base found 0x{got_base:04X}, expected 0x{want_base:04X} from MemRegion::{region:?}"
        );
        assert_eq!(
            got_end, want_end,
            "README.md '{keyword}' row: end found 0x{got_end:04X}, expected 0x{want_end:04X} from MemRegion::{region:?}"
        );
    }
}
