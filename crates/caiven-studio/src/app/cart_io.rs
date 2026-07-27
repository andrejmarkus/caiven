//! Cart saving: writes RAM-backed sections from the VM back to disk. A
//! `.cav` path writes the binary distribution cartridge; anything else is
//! treated as a project directory (the git-friendly authoring format).

use anyhow::{Context, Result};
use caiven_cart::{CartHeader, CartSection, SectionKind};
use caiven_vm::Vm;
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
/// metadata sections that were never mapped into RAM.
fn gather_sections(vm: &Vm, meta: &CartMeta) -> Vec<(SectionKind, Vec<u8>)> {
    meta.sections
        .iter()
        .map(|s| {
            let bytes = match &s.preserved_data {
                Some(data) => data.clone(),
                None => (0..s.len).map(|i| vm.peek_memory(s.ram_base + i)).collect(),
            };
            (s.kind, bytes)
        })
        .collect()
}

/// Reads each tracked RAM section from the VM and writes them back to disk.
/// Only sections that were copied into RAM (e.g. SpriteSheet) are round-tripped.
/// `modules` are the entry buffer's sibling `.lua` files (project-relative
/// path -> live buffer text) — ignored when `meta.path` is a binary `.cav`,
/// since `write_binary` bundles them into the single `LuaSource` section
/// instead of writing separate files.
pub(crate) fn save(vm: &Vm, meta: &CartMeta, modules: &[(PathBuf, String)]) -> Result<()> {
    let extra = gather_sections(vm, meta);
    let is_binary = meta.path.extension().and_then(|e| e.to_str()) == Some("cav");

    if is_binary {
        write_binary(&extra, meta, &meta.path, modules)
    } else {
        let lua = meta.lua_source.as_deref().unwrap_or_default();
        caiven_cart::save_project(&meta.path, &meta.header, lua, modules, &extra)
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
    write_binary(&extra, meta, dest, modules)
}

fn write_binary(
    extra: &[(SectionKind, Vec<u8>)],
    meta: &CartMeta,
    dest: &Path,
    modules: &[(PathBuf, String)],
) -> Result<()> {
    let mut extra = extra.to_vec();
    let program: &[u8] = match &meta.lua_source {
        Some(entry) => {
            // A distributed .cav has no filesystem, so sibling modules
            // can't stay separate files — bundle them into one LuaSource
            // section exactly like the project loader does from disk.
            let bundle_modules: Vec<(String, String)> = modules
                .iter()
                .map(|(rel, text)| (caiven_cart::module_key(Path::new(""), rel), text.clone()))
                .collect();
            let bundled = caiven_cart::bundle_lua(entry, &bundle_modules);
            extra.push((SectionKind::LuaSource, bundled.into_bytes()));
            &[]
        }
        None => &meta.program,
    };
    // Both callers (GUI "Export Cartridge" and the publish flow's temp pack)
    // produce a distribution artifact meant for someone other than the
    // author, so strip comments/formatting from the bundled Lua here.
    let mut sections: Vec<CartSection> = extra
        .into_iter()
        .map(|(kind, data)| CartSection { kind, data })
        .collect();
    caiven_cart::minify_cart_lua(&mut sections);
    let extra: Vec<(SectionKind, Vec<u8>)> = sections
        .into_iter()
        .map(|s| (s.kind, s.data))
        .collect();
    caiven_cart::write(dest, &meta.header, program, &extra)
        .with_context(|| format!("failed to write cart to {}", dest.display()))
}
