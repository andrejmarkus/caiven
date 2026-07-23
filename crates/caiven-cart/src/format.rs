/// Cart layout:
///   magic:       b"CAIVEN" (6 bytes)
///   version:     u16 LE   (= 3, unchecked by any reader — bump is informational)
///   n_sections:  u16 LE
///   header body: 72 bytes  (title[32] author[32] entry[4] flags[4])
///   section table: n_sections × 14 bytes each:
///     kind:    u16 LE
///     offset:  u32 LE   (absolute byte offset from file start)
///     len:     u32 LE
///     crc32:   u32 LE
///   section data: packed at the offsets listed in the table
use std::path::Path;

use crate::error::CartError;
use crate::header::CartHeader;
use crate::section::{CartSection, SectionKind};

const MAGIC: &[u8; 6] = b"CAIVEN";

const HEADER_BODY_LEN: usize = 72;
// 6 (magic) + 2 (version) + 2 (n_sections) + 72 (header body)
const FIXED_HDR: usize = 82;
const SECTION_ENTRY_LEN: usize = 14; // kind[2] + offset[4] + len[4] + crc32[4]

pub struct Cart {
    pub header: CartHeader,
    pub program: Vec<u8>,
    /// Non-Program sections (SpriteSheet, Map, etc.)
    pub sections: Vec<CartSection>,
}

pub fn load(path: &Path) -> Result<Cart, CartError> {
    let data = std::fs::read(path)?;
    parse(&data)
}

/// Parses a cart from an in-memory byte slice (e.g. fetched over HTTP),
/// for hosts without filesystem access such as the web player.
pub fn parse(data: &[u8]) -> Result<Cart, CartError> {
    if data.len() < MAGIC.len() || &data[0..MAGIC.len()] != MAGIC {
        return Err(CartError::BadMagic);
    }
    load_bytes(data)
}

/// Read a little-endian u32 at `pos`. Caller must ensure `pos + 4 <= data.len()`.
fn read_u32_le(data: &[u8], pos: usize) -> u32 {
    u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
}

fn load_bytes(data: &[u8]) -> Result<Cart, CartError> {
    if data.len() < FIXED_HDR {
        return Err(CartError::Truncated);
    }
    // data[6..8] = version u16 (ignored for now)
    let n_sections = u16::from_le_bytes([data[8], data[9]]) as usize;
    let header_buf: &[u8; HEADER_BODY_LEN] = data[10..10 + HEADER_BODY_LEN]
        .try_into()
        .map_err(|_| CartError::Truncated)?;
    let header = CartHeader::from_bytes(header_buf);

    let table_end = FIXED_HDR + n_sections * SECTION_ENTRY_LEN;
    if data.len() < table_end {
        return Err(CartError::Truncated);
    }

    let mut program = Vec::new();
    let mut sections = Vec::new();

    for i in 0..n_sections {
        let e = FIXED_HDR + i * SECTION_ENTRY_LEN;
        let kind_id = u16::from_le_bytes([data[e], data[e + 1]]);
        let offset = read_u32_le(data, e + 2) as usize;
        let len = read_u32_le(data, e + 6) as usize;
        let stored_crc = read_u32_le(data, e + 10);

        let end = offset.checked_add(len).ok_or(CartError::Truncated)?;
        if data.len() < end {
            return Err(CartError::Truncated);
        }
        let section_data = data[offset..end].to_vec();
        let actual_crc = crc32fast::hash(&section_data);
        if actual_crc != stored_crc {
            return Err(CartError::ChecksumMismatch {
                expected: stored_crc,
                actual: actual_crc,
            });
        }

        let kind = SectionKind::from_u16(kind_id);
        if kind == SectionKind::Program {
            program = section_data;
        } else {
            sections.push(CartSection {
                kind,
                data: section_data,
            });
        }
    }

    Ok(Cart {
        header,
        program,
        sections,
    })
}

/// Write a cart with optional extra asset sections.
/// Program is always written as section 0; extra_sections follow in order.
pub fn write(
    path: &Path,
    header: &CartHeader,
    program: &[u8],
    extra_sections: &[(SectionKind, Vec<u8>)],
) -> Result<(), CartError> {
    let n = 1 + extra_sections.len();
    let header_body = header.to_bytes();
    let table_len = n * SECTION_ENTRY_LEN;
    let data_start = FIXED_HDR + table_len;

    let mut offsets = Vec::with_capacity(n);
    let mut cur = data_start;
    offsets.push(cur);
    cur += program.len();
    for (_, d) in extra_sections {
        offsets.push(cur);
        cur += d.len();
    }

    let mut out = Vec::with_capacity(cur);

    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&3u16.to_le_bytes()); // version
    out.extend_from_slice(&(n as u16).to_le_bytes());
    out.extend_from_slice(&header_body);

    append_section_entry(&mut out, SectionKind::Program, offsets[0], program);
    for (i, (kind, d)) in extra_sections.iter().enumerate() {
        append_section_entry(&mut out, *kind, offsets[i + 1], d);
    }

    out.extend_from_slice(program);
    for (_, d) in extra_sections {
        out.extend_from_slice(d);
    }

    std::fs::write(path, out)?;
    Ok(())
}

fn append_section_entry(out: &mut Vec<u8>, kind: SectionKind, offset: usize, data: &[u8]) {
    let crc = crc32fast::hash(data);
    out.extend_from_slice(&kind.to_u16().to_le_bytes());
    out.extend_from_slice(&(offset as u32).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&crc.to_le_bytes());
}
