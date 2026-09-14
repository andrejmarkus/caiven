//! Untrusted cartridge tables must not amplify allocations or hide programs.
#![allow(clippy::unwrap_used)]

use caiven_cart::{CartError, CartHeader, MAX_CART_BYTES, SectionKind, load, parse, write};

fn cart_bytes() -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.cav");
    write(
        &path,
        &CartHeader::new("Test", ""),
        b"code",
        &[(SectionKind::LuaSource, b"return 1".to_vec())],
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
    let first = bytes[84..96].to_vec();
    bytes[98..110].copy_from_slice(&first);
    assert!(parse(&bytes).is_err());
}

#[test]
fn sections_cannot_point_into_header() {
    let mut bytes = cart_bytes();
    bytes[84..88].copy_from_slice(&0u32.to_le_bytes());
    let crc = crc32fast::hash(&bytes[..4]);
    bytes[92..96].copy_from_slice(&crc.to_le_bytes());
    assert!(parse(&bytes).is_err());
}

#[test]
fn duplicate_and_missing_program_rejected() {
    let mut bytes = cart_bytes();
    let program = bytes[82..84].to_vec();
    let lua = bytes[96..98].to_vec();
    bytes[96..98].copy_from_slice(&program);
    assert!(parse(&bytes).is_err());
    bytes[82..84].copy_from_slice(&lua);
    bytes[96..98].copy_from_slice(&lua);
    assert!(parse(&bytes).is_err());
}

#[test]
fn writer_rejects_duplicate_program_without_touching_destination() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("existing.cav");
    std::fs::write(&path, b"keep").unwrap();
    assert!(
        write(
            &path,
            &CartHeader::new("", ""),
            &[],
            &[(SectionKind::Program, b"extra".to_vec()),]
        )
        .is_err()
    );
    assert_eq!(std::fs::read(path).unwrap(), b"keep");
}

#[test]
fn writer_rejects_custom_kind_aliasing_program() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("alias.cav");
    assert!(
        write(
            &path,
            &CartHeader::new("", ""),
            &[],
            &[(SectionKind::Custom(SectionKind::Program.to_u16()), vec![]),]
        )
        .is_err()
    );
    assert!(!path.exists());
}

#[test]
fn unordered_table_and_empty_program_still_supported() {
    let mut bytes = cart_bytes();
    let first = bytes[82..96].to_vec();
    let second = bytes[96..110].to_vec();
    bytes[82..96].copy_from_slice(&second);
    bytes[96..110].copy_from_slice(&first);
    assert_eq!(parse(&bytes).unwrap().program, b"code");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.cav");
    write(
        &path,
        &CartHeader::new("", ""),
        &[],
        &[
            (SectionKind::LuaSource, b"return 1".to_vec()),
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
