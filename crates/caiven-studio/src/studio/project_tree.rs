//! VS Code-style project file tree: a recursive view of every file under
//! the project dir, for opening files by click instead of only through the
//! "+" new-module popup or the OS "Open" folder picker.

use std::path::{Path, PathBuf};

/// Renders `root`'s file tree, highlighting `open`/`active` paths, and
/// returns the clicked file's path (if any) for the caller to open.
/// Re-walks the filesystem every call — project dirs are small enough that
/// this is cheaper than any cache-invalidation scheme.
pub fn show(
    ui: &mut egui::Ui,
    root: &Path,
    open: &[&Path],
    active: Option<&Path>,
) -> Option<PathBuf> {
    let mut clicked = None;
    show_dir(ui, root, open, active, &mut clicked);
    clicked
}

fn show_dir(
    ui: &mut egui::Ui,
    dir: &Path,
    open: &[&Path],
    active: Option<&Path>,
    clicked: &mut Option<PathBuf>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort_by(|a, b| {
        b.is_dir()
            .cmp(&a.is_dir())
            .then_with(|| a.file_name().cmp(&b.file_name()))
    });

    for path in paths {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if path.is_dir() {
            egui::CollapsingHeader::new(name)
                .id_salt(&path)
                .default_open(true)
                .show(ui, |ui| show_dir(ui, &path, open, active, clicked));
        } else {
            let is_active = active == Some(path.as_path());
            let is_open = open.contains(&path.as_path());
            let text = if is_open {
                egui::RichText::new(name).strong()
            } else {
                egui::RichText::new(name)
            };
            if ui.selectable_label(is_active, text).clicked() {
                *clicked = Some(path.clone());
            }
        }
    }
}
