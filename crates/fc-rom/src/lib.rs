mod error;
mod format;
mod header;
mod section;
pub mod text;

pub use error::RomError;
pub use format::{Rom, load, write};
pub use header::RomHeader;
pub use section::{RomSection, SectionKind};
