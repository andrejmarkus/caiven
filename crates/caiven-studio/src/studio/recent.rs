//! Recently opened cart list, persisted next to the port auth token
//! (`%APPDATA%/caiven-studio` / `~/.config/caiven-studio`).

use std::path::{Path, PathBuf};

const MAX_RECENT: usize = 10;

fn normalized_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

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

fn save_result(list: &[PathBuf]) -> std::io::Result<()> {
    let Some(path) = recent_file_path() else {
        return Ok(());
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let content = list
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(path, content)
}

pub fn save(list: &[PathBuf]) {
    if let Err(error) = save_result(list) {
        log::warn!("failed to save recent carts: {error}");
    }
}

/// Moves `path` to the front of `list` (inserting if new), caps length, and
/// persists the result.
pub fn push(list: &mut Vec<PathBuf>, path: &Path) {
    let path = normalized_path(path);
    list.retain(|p| p != &path);
    list.insert(0, path);
    list.truncate(MAX_RECENT);
    save(list);
}

fn remove_from_list(list: &mut Vec<PathBuf>, path: &Path) -> bool {
    let path = normalized_path(path);
    let old_len = list.len();
    list.retain(|candidate| normalized_path(candidate) != path);
    list.len() != old_len
}

/// Removes one path from history without touching cart data on disk.
pub fn remove(list: &mut Vec<PathBuf>, path: &Path) -> std::io::Result<bool> {
    let removed = remove_from_list(list, path);
    if removed {
        save_result(list)?;
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::remove_from_list;
    use std::path::{Path, PathBuf};

    #[test]
    fn removing_recent_entry_only_forgets_matching_path() {
        let mut list = vec![PathBuf::from("/carts/one"), PathBuf::from("/carts/two")];

        assert!(remove_from_list(&mut list, Path::new("/carts/one")));
        assert_eq!(list, vec![PathBuf::from("/carts/two")]);
        assert!(!remove_from_list(&mut list, Path::new("/carts/missing")));
    }
}
