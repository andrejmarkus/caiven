/// Cart layout (format version 1):
///   magic:       b"CAIVEN" (6 bytes)
///   version:     u16 LE   (= CART_FORMAT_VERSION; any other value is rejected)
///   n_sections:  u16 LE
///   header body: 64 bytes  (title[32] author[32], zero-padded UTF-8)
///   section table: n_sections × 14 bytes each:
///     kind:    u16 LE
///     offset:  u32 LE   (absolute byte offset from file start)
///     len:     u32 LE
///     crc32:   u32 LE
///   section data: packed at the offsets listed in the table
///
/// Exactly one section is `LuaSource`; the rest are assets and metadata.
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::MAX_CART_BYTES;
use crate::error::CartError;
use crate::header::CartHeader;
use crate::section::{CartSection, SectionKind};

const MAGIC: &[u8; 6] = b"CAIVEN";

/// The only cart format version this build reads and writes. Bump it when
/// old bytes would misparse under the new layout; adding a section kind
/// doesn't need a bump, since readers carry unknown kinds through as
/// `Custom(id)`.
pub(crate) const CART_FORMAT_VERSION: u16 = 1;

const HEADER_BODY_LEN: usize = 64;
// 6 (magic) + 2 (version) + 2 (n_sections) + 64 (header body)
const FIXED_HDR: usize = 74;
const SECTION_ENTRY_LEN: usize = 14; // kind[2] + offset[4] + len[4] + crc32[4]

pub struct Cart {
    pub header: CartHeader,
    /// Every section, including the one `LuaSource`.
    pub sections: Vec<CartSection>,
}

pub fn load(path: &Path) -> Result<Cart, CartError> {
    // Bound the read itself, including files that grow after being opened.
    let mut data = Vec::new();
    std::fs::File::open(path)?
        .take((MAX_CART_BYTES + 1) as u64)
        .read_to_end(&mut data)?;
    parse(&data)
}

/// Parses a cart from an in-memory byte slice (e.g. fetched over HTTP),
/// for hosts without filesystem access such as the web player.
pub fn parse(data: &[u8]) -> Result<Cart, CartError> {
    if data.len() > MAX_CART_BYTES {
        return Err(CartError::TooLarge {
            size: data.len(),
            max: MAX_CART_BYTES,
        });
    }
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
    let version = u16::from_le_bytes([data[6], data[7]]);
    if version != CART_FORMAT_VERSION {
        return Err(CartError::UnsupportedCartVersion {
            found: version,
            supported: CART_FORMAT_VERSION,
        });
    }
    let n_sections = u16::from_le_bytes([data[8], data[9]]) as usize;
    let header_buf: &[u8; HEADER_BODY_LEN] = data[10..10 + HEADER_BODY_LEN]
        .try_into()
        .map_err(|_| CartError::Truncated)?;
    let header = CartHeader::from_bytes(header_buf);

    let table_end = FIXED_HDR + n_sections * SECTION_ENTRY_LEN;
    if data.len() < table_end {
        return Err(CartError::Truncated);
    }

    // Validate the complete table before copying any payload. Overlapping
    // ranges could otherwise multiply a small input into large allocations.
    let mut entries = Vec::with_capacity(n_sections);
    let mut ranges = Vec::with_capacity(n_sections);

    for i in 0..n_sections {
        let e = FIXED_HDR + i * SECTION_ENTRY_LEN;
        let kind = SectionKind::from_u16(u16::from_le_bytes([data[e], data[e + 1]]));
        let offset = read_u32_le(data, e + 2) as usize;
        let len = read_u32_le(data, e + 6) as usize;
        let stored_crc = read_u32_le(data, e + 10);

        let end = offset.checked_add(len).ok_or(CartError::Truncated)?;
        if data.len() < end {
            return Err(CartError::Truncated);
        }
        if offset < table_end {
            return Err(CartError::InvalidLayout("section overlaps header or table"));
        }
        if len > 0 {
            ranges.push((offset, end));
        }
        entries.push((kind, offset, end, stored_crc));
    }
    check_one_lua_source(entries.iter().map(|(kind, ..)| *kind))?;
    ranges.sort_unstable();
    if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        return Err(CartError::InvalidLayout("section payloads overlap"));
    }

    let mut sections = Vec::with_capacity(entries.len());
    for (kind, offset, end, stored_crc) in entries {
        let section_data = &data[offset..end];
        let actual_crc = crc32fast::hash(section_data);
        if actual_crc != stored_crc {
            return Err(CartError::ChecksumMismatch {
                expected: stored_crc,
                actual: actual_crc,
            });
        }
        sections.push(CartSection {
            kind,
            data: section_data.to_vec(),
        });
    }

    Ok(Cart { header, sections })
}

/// Compares wire ids so a `Custom` kind can't alias `LuaSource` past the check.
fn check_one_lua_source(kinds: impl Iterator<Item = SectionKind>) -> Result<(), CartError> {
    let lua = SectionKind::LuaSource.to_u16();
    if kinds.filter(|kind| kind.to_u16() == lua).count() == 1 {
        Ok(())
    } else {
        Err(CartError::InvalidLayout(
            "expected exactly one LuaSource section",
        ))
    }
}

/// Packs a cart in memory. `sections` must contain exactly one `LuaSource`;
/// sections are written in the given order.
pub fn pack(
    header: &CartHeader,
    sections: &[(SectionKind, Vec<u8>)],
) -> Result<Vec<u8>, CartError> {
    check_one_lua_source(sections.iter().map(|(kind, _)| *kind))?;
    let packed_len = packed_len(sections);
    if packed_len > MAX_CART_BYTES {
        return Err(CartError::TooLarge {
            size: packed_len,
            max: MAX_CART_BYTES,
        });
    }

    let mut out = Vec::with_capacity(packed_len);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&CART_FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&(sections.len() as u16).to_le_bytes());
    out.extend_from_slice(&header.to_bytes());

    let mut offset = FIXED_HDR + sections.len() * SECTION_ENTRY_LEN;
    for (kind, data) in sections {
        out.extend_from_slice(&kind.to_u16().to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&crc32fast::hash(data).to_le_bytes());
        offset += data.len();
    }
    for (_, data) in sections {
        out.extend_from_slice(data);
    }
    Ok(out)
}

/// [`pack`]s a cart and writes it to `path`. Nothing is written on error.
pub fn write(
    path: &Path,
    header: &CartHeader,
    sections: &[(SectionKind, Vec<u8>)],
) -> Result<(), CartError> {
    std::fs::write(path, pack(header, sections)?)?;
    Ok(())
}

/// Exact byte length produced by [`pack`] for this section set.
pub fn packed_len(sections: &[(SectionKind, Vec<u8>)]) -> usize {
    FIXED_HDR
        + sections.len() * SECTION_ENTRY_LEN
        + sections.iter().map(|(_, data)| data.len()).sum::<usize>()
}

/// Content-identity hash used for theft/dedup detection: covers only the
/// sections, deliberately excluding the header (title, author) so a
/// cosmetic rename before re-upload can't evade detection. Section order
/// doesn't affect the hash.
pub fn content_hash(data: &[u8]) -> Result<String, CartError> {
    let cart = parse(data)?;
    let mut hasher = Sha256::new();
    let mut sections: Vec<&CartSection> = cart.sections.iter().collect();
    sections.sort_by(|a, b| {
        a.kind
            .to_u16()
            .cmp(&b.kind.to_u16())
            .then_with(|| a.data.cmp(&b.data))
    });
    for s in sections {
        hasher.update(s.kind.to_u16().to_le_bytes());
        hasher.update(&s.data);
    }
    Ok(hex_encode(&hasher.finalize()))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}
