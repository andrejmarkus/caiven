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
