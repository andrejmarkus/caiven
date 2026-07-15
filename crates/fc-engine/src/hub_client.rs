//! Shared fc-hub client helpers used by both the `publish` CLI command
//! (`app/cli.rs`) and the Studio browser panel's publish dialog
//! (`studio/browser_panel.rs`): multipart body building and the headless
//! screenshot capture used to illustrate a published cart.

use anyhow::{Context, Result};
use fc_core::memory::{
    MAP_RAM_BASE, MUSIC_RAM_BASE, PALETTE_RAM_BASE, SFX_RAM_BASE, SPRITE_SHEET_RAM_BASE,
};
use fc_rom::SectionKind;
use fc_vm::default_instruction_set;
use fc_vm::input::Input;
use fc_vm::rendering::font::Font;
use fc_vm::{Vm, VmConfig};
use std::sync::Arc;

pub(crate) fn build_multipart(
    boundary: &str,
    parts: &[(&str, Option<&str>, &str, &[u8])],
) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, filename, content_type, data) in parts {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        let cd = match filename {
            Some(fname) => {
                format!("Content-Disposition: form-data; name=\"{name}\"; filename=\"{fname}\"\r\n")
            }
            None => format!("Content-Disposition: form-data; name=\"{name}\"\r\n"),
        };
        body.extend_from_slice(cd.as_bytes());
        body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
        body.extend_from_slice(data);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

pub(crate) fn capture_screenshot(
    rom: &fc_rom::Rom,
    config: VmConfig,
    frames: u32,
) -> Result<Vec<u8>> {
    let instruction_set = Arc::new(default_instruction_set());
    let mut vm = Vm::new(instruction_set, config);

    vm.load_rom(rom.program.clone());
    for section in &rom.sections {
        match section.kind {
            SectionKind::SpriteSheet => {
                vm.load_section_to_ram(SPRITE_SHEET_RAM_BASE, &section.data)
            }
            SectionKind::Map => vm.load_section_to_ram(MAP_RAM_BASE, &section.data),
            SectionKind::Palette => {
                vm.load_section_to_ram(PALETTE_RAM_BASE, &section.data);
                vm.set_palette_from_bytes(&section.data);
            }
            SectionKind::SfxBank => vm.load_section_to_ram(SFX_RAM_BASE, &section.data),
            SectionKind::MusicBank => vm.load_section_to_ram(MUSIC_RAM_BASE, &section.data),
            _ => {}
        }
    }

    let font = Font::empty();
    let input = Input::new();
    for _ in 0..frames {
        vm.run_frame(&input, &font);
    }

    let world = vm.world_pixels();
    let ui = vm.ui_pixels();
    let pixel_count = (config.width * config.height) as usize;
    let mut rgba = vec![0u8; pixel_count * 4];
    for i in 0..pixel_count {
        let base = i * 4;
        if ui[base + 3] > 0 {
            rgba[base..base + 4].copy_from_slice(&ui[base..base + 4]);
        } else {
            rgba[base..base + 4].copy_from_slice(&world[base..base + 4]);
        }
    }

    let img = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(config.width, config.height, rgba)
        .context("failed to create image buffer")?;
    let mut png_bytes = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    )
    .context("failed to encode screenshot PNG")?;
    Ok(png_bytes)
}
