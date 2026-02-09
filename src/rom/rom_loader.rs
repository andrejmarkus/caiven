use std::fs;
use std::path::Path;

use crate::rom::rom_error::RomError;

pub fn load_rom(path: &Path) -> Result<Vec<u8>, RomError> {
    let data = fs::read(path).map_err(|e| RomError::IoError(e))?;

    if data.len() < 9 || &data[0..5] != b"SPEAR" {
        return Err(RomError::InvalidFormat);
    }

    let version = data[5];
    if version != 1 {
        return Err(RomError::InvalidFormat);
    }

    let program_size = u32::from_le_bytes([data[6], data[7], data[8], data[9]]) as usize;
    if data.len() < 10 + program_size {
        return Err(RomError::InvalidFormat);
    }

    Ok(data[10..10 + program_size].to_vec())
}
