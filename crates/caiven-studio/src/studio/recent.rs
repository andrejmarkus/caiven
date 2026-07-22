//! Recently opened cart list, persisted next to the port auth token
//! (`%APPDATA%/caiven-studio` / `~/.config/caiven-studio`).

use std::path::{Path, PathBuf};

const MAX_RECENT: usize = 10;

fn recent_file_path() -> Option<PathBuf> {
    if let Ok(appdata) = std::env::var("APPDATA") {
        return Some(
            PathBuf::from(appdata)
                .join("caiven-studio")
                .join("recent_carts"),
        );
    }
    if let Ok(home) = std::env::var("HOME") {
        return Some(
            PathBuf::from(home)
                .join(".config")
                .join("caiven-studio")
                .join("recent_carts"),
        );
    }
    None
}

/// Loads the recent list, dropping entries whose file no longer exists.
pub fn load() -> Vec<PathBuf> {
    let Some(path) = recent_file_path() else {
        return Vec::new();
    };
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    content
        .lines()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .take(MAX_RECENT)
        .collect()
}

pub fn save(list: &[PathBuf]) {
    let Some(path) = recent_file_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let content = list
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::write(path, content);
}

/// Moves `path` to the front of `list` (inserting if new), caps length, and
/// persists the result.
pub fn push(list: &mut Vec<PathBuf>, path: &Path) {
    let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    list.retain(|p| p != &path);
    list.insert(0, path);
    list.truncate(MAX_RECENT);
    save(list);
}
