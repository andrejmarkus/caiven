//! Save-state persistence: one file per cart under `saves/`, keyed by the
//! same `CartMeta::id` the library already computes (a V56-safe path
//! component, see `shell::library::cart_id`).
//!
//! A save file is untrusted the same way a `.cav` is (hand-copied onto a
//! card, edited, or truncated by a bad write) — `decode` rejects anything
//! that doesn't fit rather than trusting the lengths it read.

use std::path::{Path, PathBuf};

const MAGIC: &[u8; 4] = b"CVST";
const FORMAT_VERSION: u16 = 1;

/// Where save states live: a `saves/` directory beside the binary, the same
/// exe-relative bargain `cart_library::default_dir()` makes for `carts/`.
pub fn saves_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("saves")
}

/// The save file for a given cart id. `id` must already be a V56-safe
/// single path component (`cart_library::cart_id` guarantees this for
/// anything drawn from the library).
pub fn save_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.cavstate"))
}

/// Packs a RAM snapshot and palette bytes into a save file: magic + version
/// + length-prefixed RAM + length-prefixed palette.
pub fn encode(ram: &[u8], palette: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 2 + 4 + ram.len() + 2 + palette.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&(ram.len() as u32).to_le_bytes());
    out.extend_from_slice(ram);
    out.extend_from_slice(&(palette.len() as u16).to_le_bytes());
    out.extend_from_slice(palette);
    out
}

/// Unpacks a save file written by [`encode`]. `None` on anything that
/// doesn't parse — bad magic, unknown version, or a length header that
/// doesn't fit the bytes actually present — never a panic or OOB read.
pub fn decode(bytes: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut cursor = 0usize;

    let magic = bytes.get(cursor..cursor + 4)?;
    if magic != MAGIC {
        return None;
    }
    cursor += 4;

    let version = u16::from_le_bytes(bytes.get(cursor..cursor + 2)?.try_into().ok()?);
    if version != FORMAT_VERSION {
        return None;
    }
    cursor += 2;

    let ram_len = u32::from_le_bytes(bytes.get(cursor..cursor + 4)?.try_into().ok()?) as usize;
    cursor += 4;
    let ram = bytes.get(cursor..cursor + ram_len)?.to_vec();
    cursor += ram_len;

    let palette_len = u16::from_le_bytes(bytes.get(cursor..cursor + 2)?.try_into().ok()?) as usize;
    cursor += 2;
    let palette = bytes.get(cursor..cursor + palette_len)?.to_vec();

    Some((ram, palette))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_ram_and_palette() {
        let ram = vec![1, 2, 3, 4, 5];
        let palette = vec![10, 20, 30, 40, 50, 60];

        let bytes = encode(&ram, &palette);
        let (decoded_ram, decoded_palette) = decode(&bytes).expect("valid save file");

        assert_eq!(decoded_ram, ram);
        assert_eq!(decoded_palette, palette);
    }

    #[test]
    fn rejects_truncated_bytes() {
        let bytes = encode(&[1, 2, 3], &[4, 5, 6]);
        assert!(decode(&bytes[..bytes.len() - 2]).is_none());
        assert!(decode(&[]).is_none());
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = encode(&[1, 2, 3], &[4, 5, 6]);
        bytes[0] = b'X';
        assert!(decode(&bytes).is_none());
    }

    #[test]
    fn rejects_unknown_version() {
        let mut bytes = encode(&[1, 2, 3], &[4, 5, 6]);
        bytes[4..6].copy_from_slice(&99u16.to_le_bytes());
        assert!(decode(&bytes).is_none());
    }

    #[test]
    fn save_path_joins_id_onto_dir() {
        let dir = PathBuf::from("/tmp/saves");
        assert_eq!(save_path(&dir, "mygame"), dir.join("mygame.cavstate"));
    }
}
