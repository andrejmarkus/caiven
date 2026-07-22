//! Serves the built SPA's `index.html` for any GET path that isn't an API
//! route or a static asset — the client-side router handles the rest.

use std::path::PathBuf;

use rocket::{State, fs::NamedFile, get};

use crate::PortState;

#[get("/<path..>", rank = 20)]
pub async fn fallback(path: PathBuf, state: &State<PortState>) -> Option<NamedFile> {
    if path.starts_with("api") {
        return None;
    }
    NamedFile::open(state.web_dir.join("index.html")).await.ok()
}
