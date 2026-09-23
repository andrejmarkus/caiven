//! Shared caiven-port client helpers used by both the `publish` CLI command
//! (`app/cli.rs`) and the Studio browser panel's publish dialog
//! (`port_api.rs`): multipart body building and the headless
//! screenshot capture used to illustrate a published cart.

use anyhow::{Context, Result};
use caiven_cart::SectionKind;
use caiven_vm::input::Input;
use caiven_vm::rendering::font::Font;
use caiven_vm::{Vm, VmConfig};

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
    cart: &caiven_cart::Cart,
    config: VmConfig,
    frames: u32,
) -> Result<Vec<u8>> {
    let mut vm = Vm::new(config);

    if let Some(section) = cart
        .sections
        .iter()
        .find(|s| s.kind == SectionKind::PreludeModules)
    {
        let manifest = String::from_utf8_lossy(&section.data);
        let modules: Vec<&str> = manifest
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        vm.set_prelude_modules(&modules)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("cart declares an invalid stdlib module")?;
    }

    // Asset RAM must be in place before the Lua load, since it runs
    // `_init()` immediately.
    let lua_source = vm
        .load_cart_sections(&cart.sections)
        .context("cart has no Lua source section (bytecode carts are no longer supported)")?;

    let font = Font::builtin()?;
    let input = Input::new();

    vm.load_lua_source(&lua_source, &input, &font)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("failed to load Lua cart for screenshot")?;

    for _ in 0..frames {
        vm.run_frame(&input, &font);
        if let Some(fault) = vm.get_fault() {
            anyhow::bail!("cart failed during screenshot capture: {fault:?}");
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cart(source: &str) -> caiven_cart::Cart {
        caiven_cart::Cart {
            header: caiven_cart::CartHeader::new("Screenshot test", "Caiven"),
            program: vec![],
            sections: vec![caiven_cart::CartSection {
                kind: SectionKind::LuaSource,
                data: source.as_bytes().to_vec(),
            }],
        }
    }

    #[test]
    fn screenshot_renders_builtin_text() {
        let png = capture_screenshot(
            &cart("function _update() clear_screen() draw_text('A', 0, 0, 1) end"),
            VmConfig::default(),
            1,
        )
        .unwrap();
        let image = image::load_from_memory(&png).unwrap().to_rgba8();
        let background = *image.get_pixel(191, 127);
        assert!(image.pixels().any(|pixel| *pixel != background));
    }

    // Remix starters are published remixable, so each must survive real
    // play (A presses, left/right sweeps) and offer Quick Remix's constant
    // chips (`local NAME = number`).
    #[test]
    fn remix_starters_play_cleanly_and_expose_constants() {
        use caiven_vm::input::Button;

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../projects/remix");
        let font = Font::builtin().unwrap();
        let mut seen = 0;
        for entry in std::fs::read_dir(&root).unwrap() {
            let dir = entry.unwrap().path();
            let cart = caiven_cart::load_project(&dir).unwrap();
            let source = Vm::new(VmConfig::default())
                .load_cart_sections(&cart.sections)
                .unwrap();
            // The `-- try N` hints are the first remix most people make, so
            // run with every one of them applied too.
            let wild: String = source
                .lines()
                .map(|line| match line.split_once(" -- try ") {
                    Some((code, value)) => match code.split_once(" = ") {
                        Some((name, _)) => format!("{name} = {}\n", value.trim()),
                        None => format!("{line}\n"),
                    },
                    None => format!("{line}\n"),
                })
                .collect();
            for (label, lua, frames) in [
                ("as shipped", &source, 900u32),
                ("with try values", &wild, 600),
            ] {
                let mut vm = Vm::new(VmConfig::default());
                vm.load_cart_sections(&cart.sections).unwrap();
                let mut input = Input::new();
                vm.load_lua_source(lua, &input, &font)
                    .unwrap_or_else(|e| panic!("{} {label}: {e}", dir.display()));
                for frame in 0..frames {
                    input.set_button(Button::A, frame % 25 == 0);
                    input.set_button(Button::Left, frame % 120 < 60);
                    input.set_button(Button::Right, frame % 120 >= 60);
                    vm.run_frame(&input, &font);
                    input.end_frame();
                    assert!(
                        vm.get_fault().is_none(),
                        "{} {label} faulted at frame {frame}: {:?}",
                        dir.display(),
                        vm.get_fault()
                    );
                }
            }

            let lua = std::fs::read_to_string(dir.join("main.lua")).unwrap();
            let constants = lua
                .lines()
                .filter(|line| {
                    line.strip_prefix("local ")
                        .and_then(|rest| rest.split_once(" = "))
                        .is_some_and(|(name, value)| {
                            let value = value.split("--").next().unwrap_or("").trim();
                            name.chars().all(|c| c.is_ascii_uppercase() || c == '_')
                                && value.parse::<f64>().is_ok()
                        })
                })
                .count();
            assert!(
                constants >= 6,
                "{} has {constants} constants",
                dir.display()
            );
            seen += 1;
        }
        assert!(seen >= 4, "expected the remix starters, found {seen}");
    }

    #[test]
    fn screenshot_reports_runtime_failure() {
        let error = capture_screenshot(
            &cart("function _update() error('broken cart') end"),
            VmConfig::default(),
            1,
        )
        .unwrap_err();
        assert!(error.to_string().contains("screenshot capture"));
    }
}
