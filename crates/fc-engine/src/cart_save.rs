use anyhow::{Context, Result};
use fc_rom::{RomHeader, SectionKind};
use fc_vm::vm::Vm;
use std::path::PathBuf;

pub struct SectionLayout {
    pub kind: SectionKind,
    pub ram_base: usize,
    pub len: usize,
}

pub struct CartMeta {
    pub path: PathBuf,
    pub header: RomHeader,
    pub program: Vec<u8>,
    pub sections: Vec<SectionLayout>,
}

/// Reads each tracked RAM section from the VM and writes them back to the cart file.
/// Only sections that were copied into RAM (e.g. SpriteSheet) are round-tripped.
/// Other section kinds are not currently written back.
pub fn save(vm: &Vm, meta: &CartMeta) -> Result<()> {
    let extra: Vec<(SectionKind, Vec<u8>)> = meta
        .sections
        .iter()
        .map(|s| {
            let bytes: Vec<u8> = (0..s.len).map(|i| vm.peek_memory(s.ram_base + i)).collect();
            (s.kind, bytes)
        })
        .collect();

    fc_rom::write(&meta.path, &meta.header, &meta.program, &extra)
        .with_context(|| format!("failed to write cart to {}", meta.path.display()))
}
