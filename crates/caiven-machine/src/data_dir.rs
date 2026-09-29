//! Where carts, saves, settings and controls live.

use std::path::{Path, PathBuf};

/// Beside the binary when a `carts/` folder is there (a handheld card or a
/// portable zip, so everything copies off together); otherwise the per-user
/// data folder, because the binary's own folder may be read-only
/// (Program Files, a macOS app bundle).
pub fn data_dir() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    pick(exe_dir, user_data_dir())
}

fn pick(exe_dir: Option<PathBuf>, user_dir: Option<PathBuf>) -> PathBuf {
    match exe_dir {
        Some(dir) if dir.join("carts").is_dir() => dir,
        exe_dir => user_dir.or(exe_dir).unwrap_or_else(|| PathBuf::from(".")),
    }
}

fn user_data_dir() -> Option<PathBuf> {
    let absolute = |name: &str| {
        std::env::var_os(name)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    };
    let base = if cfg!(windows) {
        absolute("APPDATA")
    } else if cfg!(target_os = "macos") {
        absolute("HOME").map(|home| home.join("Library/Application Support"))
    } else {
        absolute("XDG_DATA_HOME").or_else(|| absolute("HOME").map(|home| home.join(".local/share")))
    };
    base.map(|base| base.join("caiven-machine"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_carts_folder_beside_the_binary_keeps_everything_portable() {
        let exe = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(exe.path().join("carts")).expect("carts");
        let user = PathBuf::from("/home/me/.local/share/caiven-machine");
        assert_eq!(pick(Some(exe.path().to_path_buf()), Some(user)), exe.path());
    }

    #[test]
    fn without_one_the_per_user_folder_is_used() {
        let exe = tempfile::tempdir().expect("tempdir");
        let user = PathBuf::from("/home/me/.local/share/caiven-machine");
        assert_eq!(
            pick(Some(exe.path().to_path_buf()), Some(user.clone())),
            user
        );
        assert_eq!(pick(Some(exe.path().to_path_buf()), None), exe.path());
    }
}
