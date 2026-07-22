//! Cart saving: writes RAM-backed sections from the VM back to a `.cav`/`.lua` file.

use anyhow::{Context, Result};
use caiven_cart::{CartHeader, SectionKind};
use caiven_vm::Vm;
use std::path::PathBuf;

pub struct SectionLayout {
    pub kind: SectionKind,
    pub ram_base: usize,
    pub len: usize,
}

pub struct CartMeta {
    pub path: PathBuf,
    pub header: CartHeader,
    pub program: Vec<u8>,
    pub sections: Vec<SectionLayout>,
    pub lua_source: Option<String>,
}

/// Reads each tracked RAM section from the VM and writes them back to the cart file.
/// Only sections that were copied into RAM (e.g. SpriteSheet) are round-tripped.
/// Other section kinds are not currently written back.
pub(crate) fn save(vm: &Vm, meta: &CartMeta) -> Result<()> {
    let mut extra: Vec<(SectionKind, Vec<u8>)> = meta
        .sections
        .iter()
        .map(|s| {
            let bytes: Vec<u8> = (0..s.len).map(|i| vm.peek_memory(s.ram_base + i)).collect();
            (s.kind, bytes)
        })
        .collect();

    let program: &[u8] = match &meta.lua_source {
        Some(src) => {
            extra.push((SectionKind::LuaSource, src.clone().into_bytes()));
            &[]
        }
        None => &meta.program,
    };

    caiven_cart::write(&meta.path, &meta.header, program, &extra)
        .with_context(|| format!("failed to write cart to {}", meta.path.display()))
}
