/// ROM binary layout:
///   magic:       b"SPEAR"  (5 bytes)
///   header_len:  u16 LE   (= 80, bytes of header body that follow)
///   header body: 80 bytes  (title[32] author[32] entry[4] flags[4] size[4] crc32[4])
///   program:     `size` bytes
///
/// Total overhead: 87 bytes before program data.
use std::path::Path;

use crate::error::RomError;
use crate::header::RomHeader;

const MAGIC: &[u8; 5] = b"SPEAR";
const HEADER_BODY_LEN: u16 = 80;
const HEADER_TOTAL: usize = 5 + 2 + 80; // magic + header_len field + body

pub struct Rom {
    pub header: RomHeader,
    pub program: Vec<u8>,
}

pub fn load(path: &Path) -> Result<Rom, RomError> {
    let data = std::fs::read(path)?;

    if data.len() < HEADER_TOTAL {
        return Err(RomError::Truncated);
    }
    if &data[0..5] != MAGIC {
        return Err(RomError::BadMagic);
    }

    let header_len = u16::from_le_bytes([data[5], data[6]]) as usize;
    if data.len() < 7 + header_len {
        return Err(RomError::Truncated);
    }

    let header_buf: &[u8; 80] = data[7..7 + 80]
        .try_into()
        .map_err(|_| RomError::Truncated)?;
    let (header, program_size, stored_crc) = RomHeader::from_bytes(header_buf);
    let program_size = program_size as usize;

    let data_start = 7 + header_len;
    if data.len() < data_start + program_size {
        return Err(RomError::Truncated);
    }

    let program = data[data_start..data_start + program_size].to_vec();
    let actual_crc = crc32fast::hash(&program);
    if actual_crc != stored_crc {
        return Err(RomError::ChecksumMismatch {
            expected: stored_crc,
            actual: actual_crc,
        });
    }

    Ok(Rom { header, program })
}

pub fn write(path: &Path, header: &RomHeader, program: &[u8]) -> Result<(), RomError> {
    let crc32 = crc32fast::hash(program);
    let header_body = header.to_bytes(program.len() as u32, crc32);

    let mut out = Vec::with_capacity(HEADER_TOTAL + program.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&HEADER_BODY_LEN.to_le_bytes());
    out.extend_from_slice(&header_body);
    out.extend_from_slice(program);

    std::fs::write(path, out)?;
    Ok(())
}
