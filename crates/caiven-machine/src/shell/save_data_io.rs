//! Persistent save-data file, one per cart under `saves/`, keyed by the
//! same `cart_id` save_state.rs uses. Delegates the actual byte format to
//! `caiven_vm::vm::SaveData::{encode, decode}` — this module only owns the
//! file path and untrusted-bytes-on-disk boundary.

use std::path::{Path, PathBuf};

/// Where save data lives: a `saves/` directory beside the binary, same as
/// `save_state::saves_dir()`.
pub fn saves_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("saves")
}

/// The save-data file for a given cart id. `id` must already be a V56-safe
/// single path component (`cart_library::cart_id` guarantees this).
pub fn save_data_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.cavdata"))
}

/// Writes via a sibling temp file and rename, so a crash mid-write cannot
/// leave a truncated save in place of the previous good one.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut temp = path.as_os_str().to_owned();
    temp.push(".tmp");
    let temp = PathBuf::from(temp);
    std::fs::write(&temp, bytes)?;
    std::fs::rename(&temp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&temp);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_data_path_joins_id_onto_dir() {
        let dir = PathBuf::from("/tmp/saves");
        assert_eq!(save_data_path(&dir, "mygame"), dir.join("mygame.cavdata"));
    }

    #[test]
    fn write_atomic_replaces_contents_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("game.cavdata");
        write_atomic(&path, b"one").expect("first write");
        write_atomic(&path, b"two").expect("second write");
        assert_eq!(std::fs::read(&path).expect("read"), b"two");
        assert_eq!(std::fs::read_dir(dir.path()).expect("dir").count(), 1);
    }
}
