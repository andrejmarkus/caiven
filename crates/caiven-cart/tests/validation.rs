//! Untrusted cartridge tables must not amplify allocations or hide the program.
#![allow(clippy::unwrap_used)]

use caiven_cart::{CartError, CartHeader, MAX_CART_BYTES, SectionKind, load, parse, write};

// Byte offsets in a v1 cart: 74-byte fixed header, then 14-byte table entries.
const TABLE: usize = 74;
const ENTRY: usize = 14;

fn cart_bytes() -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.cav");
    write(
        &path,
        &CartHeader::new("Test", ""),
        &[
            (SectionKind::LuaSource, b"return 1".to_vec()),
            (SectionKind::Meta, b"{}".to_vec()),
        ],
    )
    .unwrap();
    std::fs::read(path).unwrap()
}

#[test]
fn oversized_input_rejected_by_parse_and_load() {
    let mut bytes = cart_bytes();
    bytes.resize(MAX_CART_BYTES + 1, 0);
    assert!(matches!(parse(&bytes), Err(CartError::TooLarge { .. })));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.cav");
    std::fs::write(&path, bytes).unwrap();
    assert!(matches!(load(&path), Err(CartError::TooLarge { .. })));
}

#[test]
fn sections_cannot_overlap_even_with_valid_checksums() {
    let mut bytes = cart_bytes();
    // Point the second entry at the first entry's payload (offset, len, crc).
    let first = bytes[TABLE + 2..TABLE + ENTRY].to_vec();
    bytes[TABLE + ENTRY + 2..TABLE + 2 * ENTRY].copy_from_slice(&first);
    assert!(parse(&bytes).is_err());
}

#[test]
fn sections_cannot_point_into_header() {
    let mut bytes = cart_bytes();
    bytes[TABLE + 2..TABLE + 6].copy_from_slice(&0u32.to_le_bytes());
    let len = u32::from_le_bytes(bytes[TABLE + 6..TABLE + 10].try_into().unwrap()) as usize;
    let crc = crc32fast::hash(&bytes[..len]);
    bytes[TABLE + 10..TABLE + 14].copy_from_slice(&crc.to_le_bytes());
    assert!(parse(&bytes).is_err());
}

#[test]
fn duplicate_and_missing_lua_source_rejected() {
    let mut bytes = cart_bytes();
    let lua = bytes[TABLE..TABLE + 2].to_vec();
    let meta = bytes[TABLE + ENTRY..TABLE + ENTRY + 2].to_vec();
    bytes[TABLE + ENTRY..TABLE + ENTRY + 2].copy_from_slice(&lua);
    assert!(matches!(parse(&bytes), Err(CartError::InvalidLayout(_))));
    bytes[TABLE..TABLE + 2].copy_from_slice(&meta);
    bytes[TABLE + ENTRY..TABLE + ENTRY + 2].copy_from_slice(&meta);
    assert!(matches!(parse(&bytes), Err(CartError::InvalidLayout(_))));
}

#[test]
fn writer_rejects_missing_or_duplicate_lua_source_without_touching_destination() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("existing.cav");
    std::fs::write(&path, b"keep").unwrap();
    let lua = (SectionKind::LuaSource, b"x".to_vec());
    for sections in [vec![], vec![lua.clone(), lua]] {
        assert!(write(&path, &CartHeader::new("", ""), &sections).is_err());
    }
    assert_eq!(std::fs::read(path).unwrap(), b"keep");
}

#[test]
fn writer_rejects_custom_kind_aliasing_lua_source() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("alias.cav");
    assert!(
        write(
            &path,
            &CartHeader::new("", ""),
            &[
                (SectionKind::LuaSource, b"x".to_vec()),
                (SectionKind::Custom(SectionKind::LuaSource.to_u16()), vec![]),
            ]
        )
        .is_err()
    );
    assert!(!path.exists());
}

#[test]
fn unordered_table_and_unknown_sections_still_supported() {
    let mut bytes = cart_bytes();
    let first = bytes[TABLE..TABLE + ENTRY].to_vec();
    let second = bytes[TABLE + ENTRY..TABLE + 2 * ENTRY].to_vec();
    bytes[TABLE..TABLE + ENTRY].copy_from_slice(&second);
    bytes[TABLE + ENTRY..TABLE + 2 * ENTRY].copy_from_slice(&first);
    let cart = parse(&bytes).unwrap();
    assert_eq!(cart.sections[1].kind, SectionKind::LuaSource);
    assert_eq!(cart.sections[1].data, b"return 1");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.cav");
    write(
        &path,
        &CartHeader::new("", ""),
        &[
            (SectionKind::LuaSource, Vec::new()),
            (SectionKind::Custom(500), vec![]),
            (SectionKind::Custom(500), b"opaque".to_vec()),
        ],
    )
    .unwrap();
    assert_eq!(load(&path).unwrap().sections.len(), 3);
}

#[test]
fn every_truncated_prefix_rejected_without_panicking() {
    let bytes = cart_bytes();
    for end in 0..bytes.len() {
        assert!(parse(&bytes[..end]).is_err(), "accepted prefix {end}");
    }
}
