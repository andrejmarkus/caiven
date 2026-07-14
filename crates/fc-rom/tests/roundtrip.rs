//! Integration tests for the ROM binary format: encode→decode roundtrip and
//! rejection of corrupted inputs (bad magic, truncation, CRC mismatch).

use std::path::PathBuf;

use fc_rom::{RomError, RomHeader, SectionKind, load, write};

/// Write a ROM with one program and two asset sections, return its path.
fn write_sample(dir: &tempfile::TempDir) -> PathBuf {
    let path = dir.path().join("sample.rom");
    let mut header = RomHeader::new("Test Cart", "Tester");
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

    let rom = load(&path).unwrap();
    assert_eq!(rom.header.title, "Test Cart");
    assert_eq!(rom.header.author, "Tester");
    assert_eq!(rom.header.entry_point, 0x1234);
    assert_eq!(rom.header.flags, 7);
    assert_eq!(rom.program, vec![0x01, 0x02, 0x03, 0x04]);
    assert_eq!(rom.sections.len(), 2);
    assert_eq!(rom.sections[0].kind, SectionKind::SpriteSheet);
    assert_eq!(rom.sections[0].data, vec![9u8; 16]);
    assert_eq!(rom.sections[1].kind, SectionKind::Map);
    assert_eq!(rom.sections[1].data, vec![5u8; 8]);
}

#[test]
fn long_title_is_truncated_to_32_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("long.rom");
    let title = "X".repeat(40);
    write(&path, &RomHeader::new(title.clone(), ""), &[0u8], &[]).unwrap();

    let rom = load(&path).unwrap();
    assert_eq!(rom.header.title, "X".repeat(32));
}

#[test]
fn bad_magic_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_sample(&dir);

    let mut bytes = std::fs::read(&path).unwrap();
    bytes[0] = b'X';
    std::fs::write(&path, &bytes).unwrap();

    assert!(matches!(load(&path), Err(RomError::BadMagic)));
}

#[test]
fn empty_file_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.rom");
    std::fs::write(&path, []).unwrap();

    assert!(matches!(load(&path), Err(RomError::BadMagic)));
}

#[test]
fn truncated_section_table_is_error_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_sample(&dir);

    // Cut into the section table: fixed header is 82 bytes, table follows.
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(&path, &bytes[..87]).unwrap();

    assert!(matches!(load(&path), Err(RomError::Truncated)));
}

#[test]
fn truncated_section_data_is_error_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_sample(&dir);

    // Keep the table intact but drop the tail of the section data.
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(&path, &bytes[..bytes.len() - 3]).unwrap();

    assert!(matches!(load(&path), Err(RomError::Truncated)));
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
        Err(RomError::ChecksumMismatch { .. })
    ));
}
