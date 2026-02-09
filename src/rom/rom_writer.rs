use crate::rom::rom_error::RomError;
use std::fs;
use std::path::Path;

pub fn write_rom(path: &Path, program: &[u8]) -> Result<(), RomError> {
    let mut out = Vec::new();

    out.extend(b"SPEAR"); // Magic number
    out.push(1); // Version
    out.extend(&(program.len() as u32).to_le_bytes()); // Program size
    out.extend(program); // Program data

    fs::write(path, out).map_err(|e| RomError::IoError(e))
}
