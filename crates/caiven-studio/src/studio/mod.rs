//! Shared Studio core helpers used by Tauri commands and headless CLI.

pub(crate) mod asset_index;
pub(crate) mod cart;
pub(crate) mod recent;
pub(crate) mod templates;

use anyhow::Result;
use std::path::PathBuf;

pub struct SourceFile {
    pub path: PathBuf,
    pub text: String,
    pub dirty: bool,
}

pub fn run_studio(file: Option<PathBuf>) -> Result<()> {
    crate::tauri_app::run(file)
}
