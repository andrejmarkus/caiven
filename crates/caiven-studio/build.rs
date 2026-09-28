use std::path::Path;
use std::process::Command;
use std::time::SystemTime;

// The binary embeds `caiven-studio-ui/dist`, which cargo never rebuilds on its
// own; a stale bundle once showed a placeholder instead of the running game.
const UI_INPUTS: &[&str] = &[
    "../caiven-studio-ui/src",
    "../caiven-studio-ui/public",
    "../caiven-studio-ui/index.html",
    "../caiven-studio-ui/package.json",
    "../caiven-studio-ui/package-lock.json",
    "../caiven-studio-ui/svelte.config.js",
    "../caiven-studio-ui/vite.config.ts",
    "../caiven-ui/src",
];

fn newest(path: &Path) -> Option<SystemTime> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_dir() {
        return meta.modified().ok();
    }
    std::fs::read_dir(path)
        .ok()?
        .flatten()
        .filter_map(|entry| newest(&entry.path()))
        .max()
}

fn ensure_fresh_ui() {
    for input in UI_INPUTS {
        println!("cargo:rerun-if-changed={input}");
    }
    let built = newest(Path::new("../caiven-studio-ui/dist/index.html"));
    let sources = UI_INPUTS.iter().filter_map(|p| newest(Path::new(p))).max();
    if built.is_some_and(|built| sources.is_none_or(|sources| sources <= built)) {
        return;
    }
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let status = Command::new(npm)
        .args(["run", "build"])
        .current_dir("../caiven-studio-ui")
        .status();
    if !status.is_ok_and(|status| status.success()) {
        panic!(
            "Studio's UI bundle (crates/caiven-studio-ui/dist) is missing or older than its \
             sources, and `npm run build` failed. Run `npm --prefix crates/caiven-studio-ui ci` \
             and `npm --prefix crates/caiven-studio-ui run build`, then build again."
        );
    }
}

fn main() {
    if std::env::var_os("CARGO_FEATURE_CUSTOM_PROTOCOL").is_some() {
        ensure_fresh_ui();
    }
    tauri_build::build()
}
