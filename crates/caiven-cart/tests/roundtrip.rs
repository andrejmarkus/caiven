//! Integration tests for the cart binary format: encode→decode roundtrip and
//! rejection of corrupted inputs (bad magic, truncation, CRC mismatch).

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use caiven_cart::{CartError, CartHeader, SectionKind, load, write};

/// Write a cart with one program and two asset sections, return its path.
fn write_sample(dir: &tempfile::TempDir) -> PathBuf {
    let path = dir.path().join("sample.cav");
    let mut header = CartHeader::new("Test Cart", "Tester");
    header.entry_point = 0x1234;
    header.flags = 7;
    let program = vec![0x01, 0x02, 0x03, 0x04];
    let extra = [
        (SectionKind::SpriteSheet, vec![9u8; 16]),
        (SectionKind::Map, vec![5u8; 8]),
    ];
    write(&path, &header, &program, &extra).unwrap();
    path
}

#[test]
fn roundtrip_preserves_header_program_and_sections() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_sample(&dir);

    let cart = load(&path).unwrap();
    assert_eq!(cart.header.title, "Test Cart");
    assert_eq!(cart.header.author, "Tester");
    assert_eq!(cart.header.entry_point, 0x1234);
    assert_eq!(cart.header.flags, 7);
    assert_eq!(cart.program, vec![0x01, 0x02, 0x03, 0x04]);
    assert_eq!(cart.sections.len(), 2);
    assert_eq!(cart.sections[0].kind, SectionKind::SpriteSheet);
    assert_eq!(cart.sections[0].data, vec![9u8; 16]);
    assert_eq!(cart.sections[1].kind, SectionKind::Map);
    assert_eq!(cart.sections[1].data, vec![5u8; 8]);
}

#[test]
fn long_title_is_truncated_to_32_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("long.cav");
    let title = "X".repeat(40);
    write(&path, &CartHeader::new(title.clone(), ""), &[0u8], &[]).unwrap();

    let cart = load(&path).unwrap();
    assert_eq!(cart.header.title, "X".repeat(32));
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
