//! The cart library: what the shell knows about the `.cav` files sitting in
//! the carts directory, without loading any of them into the VM.
//!
//! Everything here comes out of the cartridge header and section table —
//! there is no separate library database to keep in sync, and no metadata
//! file that can drift from the carts beside it. Plug in an SD card and the
//! library is whatever is on it.
//!
//! Cartridges are untrusted input: they arrive over the wire from Port or
//! get copied onto a card by hand. A cart that fails to parse is skipped
//! with a warning rather than taking the library down with it, and an
//! oversized file is rejected on its directory entry, before its bytes are
//! ever read into a device that may only have 128MB of RAM.

use std::path::{Path, PathBuf};

use caiven_cart::{Cart, MAX_CART_BYTES, SectionKind};
use log::warn;

/// The cartridge file extension the library scans for.
pub const CART_EXTENSION: &str = "cav";

/// Where the library lives when nothing overrides it: a `carts/` directory
/// beside the binary. Handheld firmware drops the whole console into one
/// folder on the card, so an exe-relative path is what makes the same
/// install work from any mount point — and keeps carts, settings and saves
/// together where a player can copy them off.
pub fn default_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("carts")
}

/// One cart as the library knows it: enough to draw a shelf tile and a
/// detail page, and to find the cart again when the player picks it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartMeta {
    /// Stable key for this cart, used for `saves/` and as the tie-breaker
    /// in library order. The file stem, so renaming a cart file moves its
    /// save states with the name — the same bargain every console that
    /// keys saves off a filename makes.
    pub id: String,
    pub path: PathBuf,
    /// Header title. May be empty; use [`CartMeta::display_title`].
    pub title: String,
    /// Header author. May be empty.
    pub author: String,
    /// Size of the packed cart on disk.
    pub bytes: u64,
    /// Section kinds present, deduplicated, in section-table order. Drives
    /// the detail screen's spec card ("sprites · map · music").
    pub kinds: Vec<SectionKind>,
}

impl CartMeta {
    /// What to draw. A cart built without a title still has to be pickable,
    /// so fall back to the filename rather than an empty tile.
    pub fn display_title(&self) -> &str {
        if self.title.is_empty() {
            &self.id
        } else {
            &self.title
        }
    }

    pub fn has(&self, kind: SectionKind) -> bool {
        self.kinds.contains(&kind)
    }
}

/// Scans `dir` for cartridges. Never fails: a missing directory is an empty
/// library (the empty-state screen), and an unreadable or malformed cart is
/// dropped with a warning.
///
/// Result is sorted by display title (case-insensitive), then by id, so the
/// shelf order is stable across runs and independent of directory order.
pub fn scan(dir: &Path) -> Vec<CartMeta> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            // Not an error worth surfacing: a fresh install has no carts
            // directory at all until something is downloaded into it.
            warn!("no cart library at {}: {e}", dir.display());
            return Vec::new();
        }
    };

    let mut carts = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                warn!("skipping unreadable entry in {}: {e}", dir.display());
                continue;
            }
        };
        let path = entry.path();
        if !is_cart_file(&path) {
            continue;
        }

        // Check the size from the directory entry, so a hostile or corrupt
        // multi-gigabyte ".cav" is never read into memory at all.
        let bytes = match entry.metadata() {
            Ok(metadata) if !metadata.is_file() => continue,
            Ok(metadata) => metadata.len(),
            Err(e) => {
                warn!("skipping {}: {e}", path.display());
                continue;
            }
        };
        if bytes > MAX_CART_BYTES as u64 {
            warn!(
                "skipping {}: {bytes} bytes exceeds the {MAX_CART_BYTES} byte cart limit",
                path.display()
            );
            continue;
        }

        let Some(id) = cart_id(&path) else {
            warn!(
                "skipping {}: filename is not a usable cart id",
                path.display()
            );
            continue;
        };

        match caiven_cart::load(&path) {
            Ok(cart) => carts.push(meta_from_cart(id, path, bytes, &cart)),
            Err(e) => warn!("skipping {}: {e}", path.display()),
        }
    }

    carts.sort_by(|a, b| {
        a.display_title()
            .to_lowercase()
            .cmp(&b.display_title().to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    carts
}

/// Case-insensitive `.cav` test — cards formatted on Windows hand back
/// `GAME.CAV` often enough to matter.
fn is_cart_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case(CART_EXTENSION))
}

/// Derives the save-state key from the filename. Rejects anything that is
/// not a plain, single path component: this id gets joined onto `saves/`,
/// so a stem that can escape that directory must not become an id.
fn cart_id(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    if stem.is_empty()
        || stem == "."
        || stem == ".."
        || stem.contains(['/', '\\', ':'])
        || stem.contains('\0')
    {
        return None;
    }
    Some(stem.to_string())
}

fn meta_from_cart(id: String, path: PathBuf, bytes: u64, cart: &Cart) -> CartMeta {
    let mut kinds = Vec::new();
    // Program is section 0 and is not in `cart.sections`; a cart without it
    // would not have parsed.
    kinds.push(SectionKind::Program);
    for section in &cart.sections {
        if !kinds.contains(&section.kind) {
            kinds.push(section.kind);
        }
    }
    CartMeta {
        id,
        path,
        title: cart.header.title.clone(),
        author: cart.header.author.clone(),
        bytes,
        kinds,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use caiven_cart::CartHeader;

    fn write_cart(dir: &Path, name: &str, title: &str, author: &str, extras: &[SectionKind]) {
        let sections: Vec<(SectionKind, Vec<u8>)> =
            extras.iter().map(|kind| (*kind, vec![7u8; 4])).collect();
        caiven_cart::write(
            &dir.join(name),
            &CartHeader::new(title, author),
            b"print('hi')",
            &sections,
        )
        .expect("write test cart");
    }

    #[test]
    fn reads_title_author_and_sections_from_the_cart() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_cart(
            dir.path(),
            "runner.cav",
            "Runner",
            "ada",
            &[SectionKind::SpriteSheet, SectionKind::Map],
        );

        let carts = scan(dir.path());
        assert_eq!(carts.len(), 1);
        let cart = &carts[0];
        assert_eq!(cart.id, "runner");
        assert_eq!(cart.title, "Runner");
        assert_eq!(cart.author, "ada");
        assert_eq!(cart.path, dir.path().join("runner.cav"));
        assert!(cart.bytes > 0);
        assert_eq!(
            cart.kinds,
            vec![
                SectionKind::Program,
                SectionKind::SpriteSheet,
                SectionKind::Map
            ]
        );
        assert!(cart.has(SectionKind::Map));
        assert!(!cart.has(SectionKind::MusicBank));
    }

    #[test]
    fn a_missing_library_directory_is_an_empty_library() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(scan(&dir.path().join("nope")).is_empty());
    }

    #[test]
    fn sorts_by_title_case_insensitively_then_by_id() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_cart(dir.path(), "c.cav", "zebra", "", &[]);
        write_cart(dir.path(), "a.cav", "Apple", "", &[]);
        write_cart(dir.path(), "b.cav", "apple", "", &[]);

        let order: Vec<String> = scan(dir.path()).into_iter().map(|cart| cart.id).collect();
        // "Apple" before "apple" only because their ids tie-break; both
        // sort ahead of "zebra" despite the capital.
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    #[test]
    fn a_corrupt_cart_is_skipped_and_the_rest_of_the_library_survives() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_cart(dir.path(), "good.cav", "Good", "", &[]);
        std::fs::write(dir.path().join("bad.cav"), b"not a cart at all").expect("write junk");

        let carts = scan(dir.path());
        assert_eq!(carts.len(), 1);
        assert_eq!(carts[0].id, "good");
    }

    #[test]
    fn an_oversized_file_is_rejected_without_being_read() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_cart(dir.path(), "ok.cav", "Ok", "", &[]);
        std::fs::write(dir.path().join("huge.cav"), vec![0u8; MAX_CART_BYTES + 1])
            .expect("write huge");

        let carts = scan(dir.path());
        assert_eq!(carts.len(), 1);
        assert_eq!(carts[0].id, "ok");
    }

    #[test]
    fn non_cart_files_and_directories_are_ignored() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_cart(dir.path(), "keep.cav", "Keep", "", &[]);
        std::fs::write(dir.path().join("readme.txt"), b"hello").expect("write txt");
        std::fs::create_dir(dir.path().join("nested.cav")).expect("create dir");

        let carts = scan(dir.path());
        assert_eq!(carts.len(), 1);
        assert_eq!(carts[0].id, "keep");
    }

    #[test]
    fn an_uppercase_extension_still_counts_as_a_cart() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_cart(dir.path(), "SHOUT.CAV", "Shout", "", &[]);
        assert_eq!(scan(dir.path()).len(), 1);
    }

    #[test]
    fn a_titleless_cart_falls_back_to_its_filename() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_cart(dir.path(), "untitled.cav", "", "", &[]);

        let carts = scan(dir.path());
        assert_eq!(carts[0].display_title(), "untitled");
    }

    #[test]
    fn a_stem_that_could_escape_the_saves_directory_is_not_a_cart_id() {
        assert_eq!(cart_id(Path::new("game.cav")).as_deref(), Some("game"));
        assert_eq!(
            cart_id(Path::new("carts/game.cav")).as_deref(),
            Some("game")
        );
        // `..cav` stems to "." — a directory reference, not a name.
        assert!(cart_id(Path::new("..cav")).is_none());
    }
}
