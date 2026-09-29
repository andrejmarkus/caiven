//! Studio's per-user settings folder: recent carts, Port login, publish history.

use std::path::PathBuf;

#[cfg(test)]
thread_local! {
    pub static TEST_CONFIG_DIR: std::cell::RefCell<Option<PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

/// `%APPDATA%\caiven-studio` on Windows, `~/Library/Application Support/caiven-studio`
/// on macOS, `$XDG_CONFIG_HOME/caiven-studio` (default `~/.config`) elsewhere.
pub fn config_dir() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(dir) = TEST_CONFIG_DIR.with(|dir| dir.borrow().clone()) {
        return Some(dir);
    }
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
        absolute("XDG_CONFIG_HOME").or_else(|| absolute("HOME").map(|home| home.join(".config")))
    };
    base.map(|base| base.join("caiven-studio"))
}
