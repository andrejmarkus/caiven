#[allow(clippy::manual_is_multiple_of)]
pub mod asset_png;
mod bundle;
mod error;
mod format;
mod header;
mod project;
mod section;
pub mod text;

pub use bundle::{bundle_lua, list_lua_files, module_key};
pub use error::CartError;
pub use format::{Cart, load, parse, write};
pub use header::CartHeader;
pub use project::{is_project, load_project, project_lua_files, save_project};
pub use section::{CartSection, SectionKind};

use std::path::Path;

/// Opens either a project directory (or its `caiven.toml`) or a binary
/// `.cav` cartridge, dispatching on which one `path` looks like.
pub fn open(path: &Path) -> Result<Cart, CartError> {
    if project::is_project(path) {
        project::load_project(path)
    } else {
        load(path)
    }
}
