//! Integration tests for the cart binary format: encode→decode roundtrip and
//! rejection of corrupted inputs (bad magic, truncation, CRC mismatch).

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use caiven_cart::{
    CartError, CartHeader, MAX_CART_BYTES, SectionKind, content_hash, load, pack, packed_len, write,
};

fn lua(source: &str) -> (SectionKind, Vec<u8>) {
    (SectionKind::LuaSource, source.as_bytes().to_vec())
}

/// Write a cart with Lua source and two asset sections, return its path.
fn write_sample(dir: &tempfile::TempDir) -> PathBuf {
    let path = dir.path().join("sample.cav");
    let header = CartHeader::new("Test Cart", "Tester");
    let sections = [
        lua("function _update() end"),
        (SectionKind::SpriteSheet, vec![9u8; 16]),
        (SectionKind::Map, vec![5u8; 8]),
    ];
    write(&path, &header, &sections).unwrap();
    path
}

#[test]
fn packed_len_matches_written_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("size.cav");
    let sections = vec![lua("x = 1"), (SectionKind::Map, vec![4; 17])];
    write(&path, &CartHeader::new("", ""), &sections).unwrap();
    assert_eq!(
        packed_len(&sections),
        std::fs::metadata(path).unwrap().len() as usize
    );
}

#[test]
fn version_1_header_layout_is_stable() {
    let bytes = pack(&CartHeader::new("T", "A"), &[lua("")]).unwrap();
    assert_eq!(&bytes[0..6], b"CAIVEN");
    assert_eq!(u16::from_le_bytes([bytes[6], bytes[7]]), 1);
    assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), 1);
    assert_eq!(bytes[10], b'T');
    assert_eq!(bytes[42], b'A');
    // Section table starts right after the 64-byte header body.
    assert_eq!(u16::from_le_bytes([bytes[74], bytes[75]]), 0x0001);
    assert_eq!(bytes.len(), 74 + 14);
}

#[test]
fn write_rejects_cart_over_shared_limit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.cav");
    let sections = [(SectionKind::LuaSource, vec![0; MAX_CART_BYTES])];
    let error = write(&path, &CartHeader::new("", ""), &sections).unwrap_err();
    assert!(matches!(
        error,
        CartError::TooLarge {
            max: MAX_CART_BYTES,
            ..
        }
    ));
    assert!(!path.exists());
}

#[test]
fn roundtrip_preserves_header_and_sections() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_sample(&dir);

    let cart = load(&path).unwrap();
    assert_eq!(cart.header.title, "Test Cart");
    assert_eq!(cart.header.author, "Tester");
    assert_eq!(cart.sections.len(), 3);
    assert_eq!(cart.sections[0].kind, SectionKind::LuaSource);
    assert_eq!(cart.sections[0].data, b"function _update() end");
    assert_eq!(cart.sections[1].kind, SectionKind::SpriteSheet);
    assert_eq!(cart.sections[1].data, vec![9u8; 16]);
    assert_eq!(cart.sections[2].kind, SectionKind::Map);
    assert_eq!(cart.sections[2].data, vec![5u8; 8]);
}

#[test]
fn long_title_is_truncated_to_32_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("long.cav");
    let title = "X".repeat(40);
    write(&path, &CartHeader::new(title.clone(), ""), &[lua("")]).unwrap();

    let cart = load(&path).unwrap();
    assert_eq!(cart.header.title, "X".repeat(32));
}

#[test]
fn long_title_is_truncated_on_a_character_boundary() {
    // Same rule as Quick Remix's writer (crates/caiven-port/web/src/lib/cav.js).
    let bytes = pack(&CartHeader::new("é".repeat(40), ""), &[lua("")]).unwrap();
    let cart = caiven_cart::parse(&bytes).unwrap();
    assert_eq!(cart.header.title, "é".repeat(16));
}

#[test]
fn bad_magic_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_sample(&dir);

    let mut bytes = std::fs::read(&path).unwrap();
    bytes[0] = b'X';
    std::fs::write(&path, &bytes).unwrap();

    assert!(matches!(load(&path), Err(CartError::BadMagic)));
}

#[test]
fn empty_file_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.cav");
    std::fs::write(&path, []).unwrap();

    assert!(matches!(load(&path), Err(CartError::BadMagic)));
}

#[test]
fn truncated_section_table_is_error_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_sample(&dir);

    // Cut into the section table: fixed header is 82 bytes, table follows.
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(&path, &bytes[..87]).unwrap();

    assert!(matches!(load(&path), Err(CartError::Truncated)));
}

#[test]
fn truncated_section_data_is_error_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_sample(&dir);

    // Keep the table intact but drop the tail of the section data.
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(&path, &bytes[..bytes.len() - 3]).unwrap();

    assert!(matches!(load(&path), Err(CartError::Truncated)));
}

#[test]
fn future_version_is_rejected_not_misparsed() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_sample(&dir);

    // Bump the version field (bytes 6..8, little-endian) past anything this
    // build knows how to read.
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[6..8].copy_from_slice(&9999u16.to_le_bytes());
    std::fs::write(&path, &bytes).unwrap();

    assert!(matches!(
        load(&path),
        Err(CartError::UnsupportedCartVersion { found: 9999, .. })
    ));
}

#[test]
fn zero_version_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_sample(&dir);

    let mut bytes = std::fs::read(&path).unwrap();
    bytes[6..8].copy_from_slice(&0u16.to_le_bytes());
    std::fs::write(&path, &bytes).unwrap();

    assert!(matches!(
        load(&path),
        Err(CartError::UnsupportedCartVersion {
            found: 0,
            supported: 1
        })
    ));
}

#[test]
fn corrupted_section_data_fails_crc_check() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_sample(&dir);

    let mut bytes = std::fs::read(&path).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xFF;
    std::fs::write(&path, &bytes).unwrap();

    assert!(matches!(
        load(&path),
        Err(CartError::ChecksumMismatch { .. })
    ));
}

#[test]
fn content_hash_ignores_header_and_section_order() {
    let a = pack(
        &CartHeader::new("One", "a"),
        &[
            lua("x = 1"),
            (SectionKind::SpriteBank, vec![2]),
            (SectionKind::SpriteBank, vec![1]),
        ],
    )
    .unwrap();
    let b = pack(
        &CartHeader::new("Two", "b"),
        &[
            (SectionKind::SpriteBank, vec![1]),
            lua("x = 1"),
            (SectionKind::SpriteBank, vec![2]),
        ],
    )
    .unwrap();
    let c = pack(&CartHeader::new("One", "a"), &[lua("x = 2")]).unwrap();
    assert_eq!(content_hash(&a).unwrap(), content_hash(&b).unwrap());
    assert_ne!(content_hash(&a).unwrap(), content_hash(&c).unwrap());
}
