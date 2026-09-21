//! Cart saving: writes RAM-backed sections from the VM back to disk. A
//! `.cav` path writes the binary distribution cartridge; anything else is
//! treated as a project directory (the git-friendly authoring format).

use anyhow::{Context, Result};
use caiven_cart::{
    CartHeader, CartSection, DEFAULT_BANK_NAME, SectionKind, decode_asset_bank, encode_asset_bank,
    encode_collision_types,
};
use caiven_vm::{AssetBankKind, Vm};
use std::path::{Path, PathBuf};

pub struct SectionLayout {
    pub kind: SectionKind,
    pub ram_base: usize,
    pub len: usize,
    /// Non-RAM sections such as `ModManifest` must be copied verbatim when
    /// saving; RAM-backed assets leave this as `None` and are read from the VM.
    pub preserved_data: Option<Vec<u8>>,
}

pub struct CartMeta {
    pub path: PathBuf,
    pub header: CartHeader,
    pub program: Vec<u8>,
    pub sections: Vec<SectionLayout>,
    pub lua_source: Option<String>,
}

/// Reads each tracked RAM asset section back from the VM while retaining
/// metadata sections that were never mapped into RAM. Also used by
/// `StudioCore` to (re)capture its pristine `asset_snapshot` at a moment it
/// knows VM RAM reflects only editor/compile-time state, not gameplay.
pub(crate) fn gather_sections(vm: &Vm, meta: &CartMeta) -> Vec<(SectionKind, Vec<u8>)> {
    meta.sections
        .iter()
        .map(|s| {
            if s.kind == SectionKind::CollisionTypes {
                // Cart-global metadata, not RAM-backed — always read live
                // from the VM so edits made via the type-management UI are
                // captured, unlike `preserved_data` sections which are
                // copied verbatim from what was loaded.
                return (s.kind, encode_collision_types(vm.collision_types()));
            }
            let bank = match s.kind {
                SectionKind::SpriteSheet => {
                    Some((AssetBankKind::Sprites, DEFAULT_BANK_NAME.to_string()))
                }
                SectionKind::Map => Some((AssetBankKind::Map, DEFAULT_BANK_NAME.to_string())),
                SectionKind::Collision => {
                    Some((AssetBankKind::Collision, DEFAULT_BANK_NAME.to_string()))
                }
                SectionKind::CollisionBank => s
                    .preserved_data
                    .as_deref()
                    .and_then(decode_asset_bank)
                    .map(|(name, _)| (AssetBankKind::Collision, name.to_string())),
                SectionKind::SpriteBank => s
                    .preserved_data
                    .as_deref()
                    .and_then(decode_asset_bank)
                    .map(|(name, _)| (AssetBankKind::Sprites, name.to_string())),
                SectionKind::MapBank => s
                    .preserved_data
                    .as_deref()
                    .and_then(decode_asset_bank)
                    .map(|(name, _)| (AssetBankKind::Map, name.to_string())),
                SectionKind::PaletteBank => s
                    .preserved_data
                    .as_deref()
                    .and_then(decode_asset_bank)
                    .map(|(name, _)| (AssetBankKind::Palette, name.to_string())),
                SectionKind::SfxBanks => s
                    .preserved_data
                    .as_deref()
                    .and_then(decode_asset_bank)
                    .map(|(name, _)| (AssetBankKind::Sfx, name.to_string())),
                SectionKind::MusicBanks => s
                    .preserved_data
                    .as_deref()
                    .and_then(decode_asset_bank)
                    .map(|(name, _)| (AssetBankKind::Music, name.to_string())),
                _ => None,
            };
            let bytes = if let Some((kind, name)) = bank {
                let data = vm.asset_bank_bytes(kind, &name).unwrap_or_default();
                if name == DEFAULT_BANK_NAME {
                    data
                } else {
                    encode_asset_bank(&name, &data)
                }
            } else {
                match &s.preserved_data {
                    Some(data) => data.clone(),
                    None => (0..s.len).map(|i| vm.peek_memory(s.ram_base + i)).collect(),
                }
            };
            (s.kind, bytes)
        })
        .collect()
}

/// Writes already-gathered asset sections (e.g. from [`gather_sections`] or
/// `StudioCore::asset_snapshot`) back to disk. Only sections that were
/// copied into RAM (e.g. SpriteSheet) are round-tripped. `modules` are the
/// entry buffer's sibling `.lua` files (project-relative path -> live buffer
/// text) — ignored when `meta.path` is a binary `.cav`, since `write_binary`
/// bundles them into the single `LuaSource` section instead of writing
/// separate files.
///
/// Callers pass a *pristine* snapshot rather than a live `gather_sections`
/// read: Studio's editor keeps `asset_snapshot` separate from live VM RAM
/// specifically so a mid-play gameplay mutation (e.g. `set_tile` clearing a
/// collected coin) can never leak into a save.
pub(crate) fn save_pristine(
    snapshot: &[(SectionKind, Vec<u8>)],
    meta: &CartMeta,
    modules: &[(PathBuf, String)],
    removed_banks: &[(SectionKind, String)],
) -> Result<()> {
    save_extra(snapshot, meta, modules, removed_banks)
}

fn save_extra(
    extra: &[(SectionKind, Vec<u8>)],
    meta: &CartMeta,
    modules: &[(PathBuf, String)],
    removed_banks: &[(SectionKind, String)],
) -> Result<()> {
    let is_binary = meta.path.extension().and_then(|e| e.to_str()) == Some("cav");

    if is_binary {
        // Never minify here: this is an in-place edit of the author's own
        // `.cav` (Studio only reaches this branch for a cart opened
        // directly, not unpacked to a project dir first), so Ctrl+S must
        // not silently strip the source's comments and formatting on disk.
        // Minification is for `export_binary`/`export_web`/publish, which
        // build a separate distribution artifact. See review STU-03.
        write_binary(extra, meta, &meta.path, modules, false)
    } else {
        let lua = meta.lua_source.as_deref().unwrap_or_default();
        caiven_cart::save_project(&meta.path, &meta.header, lua, modules, extra, removed_banks)
            .with_context(|| format!("failed to write project to {}", meta.path.display()))
    }
}

/// Builds a binary `.cav` cartridge at `dest` from the VM's current RAM
/// sections, regardless of where `meta.path` (the project dir) lives. Used
/// by the "Export Cartridge" action to produce a distribution artifact
/// without disturbing the project's own save location.
pub(crate) fn export_binary(
    vm: &Vm,
    meta: &CartMeta,
    dest: &Path,
    modules: &[(PathBuf, String)],
) -> Result<()> {
    let extra = gather_sections(vm, meta);
    write_binary(&extra, meta, dest, modules, true)
}

/// Packs a cart to bytes via a throwaway temp `.cav` file, read back
/// immediately after — `caiven_cart::write` has no in-memory variant, so
/// this is the only way to get packed bytes without a permanent output
/// file. Shared by `export_web` here and the CLI's `Export --web` handler
/// (`crate::app::cli`) so this sequence exists in exactly one place.
pub(crate) fn pack_to_bytes(
    header: &CartHeader,
    program: &[u8],
    extra: &[(SectionKind, Vec<u8>)],
) -> Result<Vec<u8>> {
    let temp = crate::studio::cart::temp_cav_path();
    caiven_cart::write(&temp, header, program, extra)
        .with_context(|| format!("failed to pack cart to {}", temp.display()))?;
    let packed = std::fs::read(&temp)
        .with_context(|| format!("failed to read packed cart from {}", temp.display()));
    let _ = std::fs::remove_file(&temp);
    packed
}

/// Builds a self-contained web export (single offline-playable `.html`) from
/// the VM's current RAM sections, reusing the same bundling/minify path as
/// `export_binary`.
pub(crate) fn export_web(
    vm: &Vm,
    meta: &CartMeta,
    dest: &Path,
    modules: &[(PathBuf, String)],
) -> Result<()> {
    let extra = gather_sections(vm, meta);
    let (program, extra) =
        distribution_content(&extra, meta, meta.lua_source.as_deref(), modules, true);
    let packed = pack_to_bytes(&meta.header, &program, &extra)?;

    let html = crate::app::web_export::build_web_html(&packed, &meta.header.title);
    std::fs::write(dest, html)
        .with_context(|| format!("failed to write web export to {}", dest.display()))
}

/// Frames to run headlessly before capturing a screenshot — matches the
/// publish flow's cover-capture default (`app/cli.rs` `Publish::frames`).
const SCREENSHOT_FRAMES: u32 = 30;

/// Captures a PNG of the VM's current RAM sections run headlessly from
/// `_init()`, reusing the same pack-to-temp-file step as `export_web` and
/// the screenshot primitive already shared by publish (`port_client.rs`).
pub(crate) fn export_screenshot(
    vm: &Vm,
    meta: &CartMeta,
    dest: &Path,
    modules: &[(PathBuf, String)],
) -> Result<()> {
    let extra = gather_sections(vm, meta);
    let (program, extra) =
        distribution_content(&extra, meta, meta.lua_source.as_deref(), modules, true);

    let temp = crate::studio::cart::temp_cav_path();
    caiven_cart::write(&temp, &meta.header, &program, &extra)
        .with_context(|| format!("failed to pack cart to {}", temp.display()))?;
    let cart = caiven_cart::load(&temp)
        .with_context(|| format!("failed to reload packed cart from {}", temp.display()));
    let _ = std::fs::remove_file(&temp);
    let cart = cart?;

    let png_bytes = crate::port_client::capture_screenshot(
        &cart,
        caiven_vm::VmConfig::default(),
        SCREENSHOT_FRAMES,
    )
    .context("failed to capture screenshot")?;
    std::fs::write(dest, png_bytes)
        .with_context(|| format!("failed to write screenshot to {}", dest.display()))
}

/// Packages a project-dir cart's `caiven.toml` + Lua source + assets as a
/// zip at `dest`. Writes current live buffers to a throwaway temp project
/// dir first (same shape as `save()`'s project branch) rather than zipping
/// the live project dir in place, so unsaved edits are included and no
/// stray non-project files leak in. Binary `.cav` carts have no source tree
/// to export.
pub(crate) fn export_source_zip(
    vm: &Vm,
    meta: &CartMeta,
    dest: &Path,
    modules: &[(PathBuf, String)],
) -> Result<()> {
    if meta.path.extension().and_then(|e| e.to_str()) == Some("cav") {
        anyhow::bail!("Export Source is only available for project-directory carts");
    }

    let extra = gather_sections(vm, meta);
    let lua = meta.lua_source.as_deref().unwrap_or_default();
    let temp_dir = crate::studio::cart::temp_project_dir_path();
    let write_result =
        caiven_cart::save_project(&temp_dir, &meta.header, lua, modules, &extra, &[])
            .with_context(|| format!("failed to write project to {}", temp_dir.display()));
    let zip_result = write_result.and_then(|()| zip_dir(&temp_dir, dest));
    let _ = std::fs::remove_dir_all(&temp_dir);
    zip_result
}

fn zip_dir(dir: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::create(dest)
        .with_context(|| format!("failed to create zip at {}", dest.display()))?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("failed to read project dir {}", dir.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        writer
            .start_file(name, options)
            .with_context(|| format!("failed to add {name} to zip"))?;
        let bytes =
            std::fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
        std::io::Write::write_all(&mut writer, &bytes)
            .with_context(|| format!("failed to write {name} to zip"))?;
    }
    writer.finish().context("failed to finalize zip")?;
    Ok(())
}

fn write_binary(
    extra: &[(SectionKind, Vec<u8>)],
    meta: &CartMeta,
    dest: &Path,
    modules: &[(PathBuf, String)],
    minify: bool,
) -> Result<()> {
    let (program, extra) =
        distribution_content(extra, meta, meta.lua_source.as_deref(), modules, minify);
    caiven_cart::write(dest, &meta.header, &program, &extra)
        .with_context(|| format!("failed to write cart to {}", dest.display()))
}

/// Exact size of the distribution cartridge built from current live buffers
/// and VM-backed assets. Uses the same bundling and minification path as export.
pub(crate) fn packed_size(
    vm: &Vm,
    meta: &CartMeta,
    entry: Option<&str>,
    modules: &[(PathBuf, String)],
) -> usize {
    let extra = gather_sections(vm, meta);
    // The 128 KiB cap applies to the distributed cartridge regardless of
    // authoring format, so the size indicator always estimates the
    // minified/bundled artifact — unlike a save-in-place `.cav`, which no
    // longer minifies (see review STU-03).
    let (program, extra) = distribution_content(&extra, meta, entry, modules, true);
    caiven_cart::packed_len(&program, &extra)
}

fn distribution_content(
    extra: &[(SectionKind, Vec<u8>)],
    meta: &CartMeta,
    entry: Option<&str>,
    modules: &[(PathBuf, String)],
    minify: bool,
) -> (Vec<u8>, Vec<(SectionKind, Vec<u8>)>) {
    let mut extra = extra.to_vec();
    let program = match entry {
        Some(entry) => {
            // A distributed .cav has no filesystem, so sibling modules
            // can't stay separate files — bundle them into one LuaSource
            // section exactly like the project loader does from disk.
            // Minify each source before bundling: minifying the bundle would
            // copy every module's long-string body through untouched.
            let prep = |text: &str| {
                if minify {
                    caiven_cart::minify_lua(text)
                } else {
                    text.to_string()
                }
            };
            let bundle_modules: Vec<(String, String)> = modules
                .iter()
                .map(|(rel, text)| (caiven_cart::module_key(Path::new(""), rel), prep(text)))
                .collect();
            let bundled = caiven_cart::bundle_lua(&prep(entry), &bundle_modules);
            extra.push((SectionKind::LuaSource, bundled.into_bytes()));
            Vec::new()
        }
        None => meta.program.clone(),
    };
    let mut sections: Vec<CartSection> = extra
        .into_iter()
        .map(|(kind, data)| CartSection { kind, data })
        .collect();
    if minify && entry.is_none() {
        // Legacy carts have no per-file sources to minify ahead of time.
        caiven_cart::minify_cart_lua(&mut sections);
    }
    let extra: Vec<(SectionKind, Vec<u8>)> =
        sections.into_iter().map(|s| (s.kind, s.data)).collect();
    (program, extra)
}

#[cfg(test)]
mod tests {
    use super::*;
    use caiven_vm::VmConfig;

    fn temp_path(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "caiven-cart-io-{label}-{}-{unique}.cav",
            std::process::id()
        ))
    }

    #[test]
    fn saving_a_cav_in_place_preserves_comments_but_export_still_minifies() {
        // Review STU-03: Ctrl+S on a cart opened directly as a `.cav` used
        // to go through the same minifying path as "Export Cartridge",
        // silently stripping the author's own comments/formatting from the
        // file they're editing. Save-in-place and export must diverge here.
        let lua = "-- a helpful comment\nfunction _init() end\nfunction _update() end\n";
        let cav_path = temp_path("save-in-place");
        let meta = CartMeta {
            path: cav_path.clone(),
            header: CartHeader::new("Test", ""),
            program: Vec::new(),
            sections: Vec::new(),
            lua_source: Some(lua.to_string()),
        };

        save_pristine(&[], &meta, &[], &[]).expect("save in place");
        let saved = caiven_cart::load(&cav_path).expect("reload saved cav");
        let saved_lua = saved
            .sections
            .iter()
            .find(|s| s.kind == SectionKind::LuaSource)
            .map(|s| String::from_utf8_lossy(&s.data).into_owned())
            .unwrap();
        assert!(
            saved_lua.contains("a helpful comment"),
            "save-in-place must not minify: {saved_lua}"
        );

        let export_path = temp_path("export");
        let vm = Vm::new(VmConfig::default());
        export_binary(&vm, &meta, &export_path, &[]).expect("export");
        let exported = caiven_cart::load(&export_path).expect("reload exported cav");
        let exported_lua = exported
            .sections
            .iter()
            .find(|s| s.kind == SectionKind::LuaSource)
            .map(|s| String::from_utf8_lossy(&s.data).into_owned())
            .unwrap();
        assert!(
            !exported_lua.contains("a helpful comment"),
            "export must still minify: {exported_lua}"
        );

        std::fs::remove_file(&cav_path).ok();
        std::fs::remove_file(&export_path).ok();
    }
}
