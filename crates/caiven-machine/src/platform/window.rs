//! SDL2 window, renderer and the streaming texture the console draws into.

use anyhow::{Context, Result, anyhow};
use caiven_vm::rendering::screen::Screen;
use caiven_vm::{Vm, VmConfig};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Texture, TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;

use crate::platform::scaling::{AspectMode, ScaleMode, dst_rect};

/// The VM composites into a plain byte slice as R, G, B, A
/// (`caiven-core/src/memory.rs` `RGBA_BYTES`). SDL names its formats by the
/// packed 32-bit integer, so on a little-endian host that byte order reads
/// back as ABGR8888. Naming it RGBA8888 here silently swaps red and blue.
pub const CONSOLE_PIXEL_FORMAT: PixelFormatEnum = PixelFormatEnum::ABGR8888;

/// Integer scale factor from console resolution to initial window size.
const WINDOW_SCALE: u32 = 4;

/// Builds a window and its renderer.
///
/// `accelerated` asks for a GPU-backed, vsynced renderer. The window has to
/// be rebuilt for a retry because `into_canvas` consumes it — cheap, and it
/// only ever happens once at startup.
fn build_canvas(
    video: &sdl2::VideoSubsystem,
    config: &VmConfig,
    title: &str,
    fullscreen: bool,
    accelerated: bool,
) -> Result<WindowCanvas> {
    let mut builder = video.window(
        title,
        config.width * WINDOW_SCALE,
        config.height * WINDOW_SCALE,
    );
    // Without this, a HiDPI display (any current Mac) renders the window at
    // 1x and the OS upscales that to fit — every pixel, console art and
    // shell chrome alike, comes out visibly soft/blurry. A fixed-DPI
    // handheld panel has no such scaling to opt into, so this is a no-op
    // there.
    builder.allow_highdpi();
    builder.position_centered();
    if fullscreen {
        builder.fullscreen_desktop();
    }
    let window = builder.build().context("failed to create SDL window")?;

    let mut canvas_builder = window.into_canvas();
    if accelerated {
        canvas_builder = canvas_builder.accelerated().present_vsync();
    }
    canvas_builder
        .build()
        .context("failed to create SDL renderer")
}

/// The window and renderer.
///
/// The console and shell textures are deliberately *not* stored here: a
/// `Texture` borrows its `TextureCreator`, and keeping it in this struct
/// would make it self-referential. The caller owns the creator and passes
/// the textures in every frame, which keeps this whole module free of
/// unsafe.
pub struct Display {
    canvas: WindowCanvas,
}

impl Display {
    /// Creates the window and renderer.
    pub fn new(
        video: &sdl2::VideoSubsystem,
        config: &VmConfig,
        title: &str,
        fullscreen: bool,
    ) -> Result<Self> {
        // Nearest-neighbour. A fantasy console that smooth-scales its pixels
        // is not a fantasy console.
        sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "0");

        // SDL only picks a render driver that supports every requested flag,
        // so asking for acceleration or vsync on a device that has neither
        // fails outright rather than degrading. A GPU-less handheld is
        // exactly that device, so fall back to whatever SDL can give us.
        let canvas = match build_canvas(video, config, title, fullscreen, true) {
            Ok(canvas) => canvas,
            Err(e) => {
                log::info!("no accelerated/vsync renderer ({e}); falling back to software");
                build_canvas(video, config, title, fullscreen, false)
                    .context("failed to create SDL renderer")?
            }
        };

        log::info!("render driver: {}", canvas.info().name);

        Ok(Self { canvas })
    }

    /// The window's current size in actual pixels — its drawable size,
    /// which on a HiDPI display is larger than `Window::size()`'s point
    /// size. Everything allocated off this (the shell surface, the console
    /// texture's destination rect) must use this, not the point size, or
    /// the rendered frame only fills a fraction of the real window.
    pub fn window_size(&self) -> (u32, u32) {
        self.canvas.window().drawable_size()
    }

    /// Creates the texture creator the console and shell textures are
    /// allocated from. It holds its own reference to the window context, so
    /// it outlives nothing in particular and may be kept by the caller.
    pub fn texture_creator(&self) -> TextureCreator<WindowContext> {
        self.canvas.texture_creator()
    }

    /// Allocates the streaming texture the console framebuffer is uploaded
    /// into, at the console's own resolution.
    pub fn create_console_texture<'tc>(
        creator: &'tc TextureCreator<WindowContext>,
        config: &VmConfig,
    ) -> Result<Texture<'tc>> {
        creator
            .create_texture_streaming(CONSOLE_PIXEL_FORMAT, config.width, config.height)
            .context("failed to create console texture")
    }

    /// Allocates the streaming texture the shell's CPU-rasterized overlay
    /// (`shell::surface::Surface`) is uploaded into, at the shell layout's
    /// own resolution (`shell::theme::Metrics::width/height`).
    pub fn create_shell_texture<'tc>(
        creator: &'tc TextureCreator<WindowContext>,
        width: u32,
        height: u32,
    ) -> Result<Texture<'tc>> {
        let mut texture = creator
            .create_texture_streaming(CONSOLE_PIXEL_FORMAT, width, height)
            .context("failed to create shell texture")?;
        texture.set_blend_mode(BlendMode::Blend);
        Ok(texture)
    }

    /// Composites the console framebuffer and the shell's raster overlay
    /// and presents the frame.
    ///
    /// `shell_rgba` is `Surface::rgba()`'s output: premultiplied alpha, in
    /// the same byte order as the console framebuffer (SPEC V46, V28). SDL's
    /// `BlendMode::Blend` assumes straight alpha, and the `sdl2` crate
    /// exposes no way to hand a `Texture` a custom premultiplied compose
    /// (`SDL_ComposeCustomBlendMode` needs a raw texture handle nothing here
    /// gets to touch) — so the bytes are un-premultiplied on the way in
    /// instead. Same result (no dark edge fringing on the overlay), no
    /// unsafe FFI.
    #[allow(clippy::too_many_arguments)]
    pub fn present(
        &mut self,
        console_texture: &mut Texture,
        shell_texture: &mut Texture,
        screen: &Screen,
        vm: &Vm,
        scale: ScaleMode,
        aspect: AspectMode,
        shell_rgba: &[u8],
    ) -> Result<()> {
        console_texture
            .with_lock(None, |buffer: &mut [u8], _pitch: usize| {
                screen.construct(buffer, vm.world_pixels(), vm.ui_pixels());
            })
            .map_err(|e| anyhow!("failed to lock console texture: {e}"))?;

        let (win_w, win_h) = self.canvas.window().drawable_size();
        let query = console_texture.query();
        let dst = dst_rect((win_w, win_h), (query.width, query.height), scale, aspect);

        self.canvas.set_draw_color(Color::BLACK);
        self.canvas.clear();
        self.canvas
            .copy(
                console_texture,
                None,
                Some(Rect::new(dst.x, dst.y, dst.width, dst.height)),
            )
            .map_err(|e| anyhow!("failed to copy console texture: {e}"))?;

        let straight = unpremultiply(shell_rgba);
        shell_texture
            .with_lock(None, |buffer: &mut [u8], _pitch: usize| {
                buffer.copy_from_slice(&straight);
            })
            .map_err(|e| anyhow!("failed to lock shell texture: {e}"))?;
        self.canvas
            .copy(shell_texture, None, None)
            .map_err(|e| anyhow!("failed to copy shell texture: {e}"))?;

        self.canvas.present();
        Ok(())
    }
}

/// Converts premultiplied-alpha RGBA bytes to straight alpha, per pixel,
/// leaving the byte order untouched (channel identity doesn't matter to the
/// arithmetic — only which byte is alpha does).
fn unpremultiply(rgba: &[u8]) -> Vec<u8> {
    let mut out = rgba.to_vec();
    for px in out.chunks_exact_mut(4) {
        let a = px[3] as u32;
        if a == 0 {
            px[0] = 0;
            px[1] = 0;
            px[2] = 0;
        } else if a < 255 {
            px[0] = ((px[0] as u32 * 255) / a).min(255) as u8;
            px[1] = ((px[1] as u32 * 255) / a).min(255) as u8;
            px[2] = ((px[2] as u32 * 255) / a).min(255) as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{CONSOLE_PIXEL_FORMAT, unpremultiply};
    use caiven_vm::rendering::screen::Screen;
    use sdl2::pixels::PixelFormatEnum;

    #[test]
    fn unpremultiply_leaves_opaque_and_fully_transparent_pixels_alone() {
        let rgba = [10, 20, 30, 255, 99, 99, 99, 0];
        let out = unpremultiply(&rgba);
        assert_eq!(&out[0..4], &[10, 20, 30, 255], "opaque pixel unchanged");
        assert_eq!(
            &out[4..8],
            &[0, 0, 0, 0],
            "fully transparent pixel zeroed, not left with premultiplied junk color"
        );
    }

    #[test]
    fn unpremultiply_scales_partial_alpha_channels_back_up() {
        // Half-alpha ember (0xFEB05D premultiplied by ~0.5) should scale
        // back close to the original straight-alpha color.
        let half_alpha = 127u8;
        let premultiplied = [127, 88, 46, half_alpha];
        let out = unpremultiply(&premultiplied);
        assert_eq!(out[3], half_alpha, "alpha itself is untouched");
        assert!(out[0] >= 250, "red channel scales back up toward 255");
    }

    /// The VM writes R,G,B,A byte order; SDL names formats by packed u32.
    /// If these ever disagree, red and blue swap on screen — a bug that
    /// looks like an art problem rather than a rendering one.
    #[test]
    fn console_pixel_format_matches_vm_byte_order() {
        assert_eq!(CONSOLE_PIXEL_FORMAT, PixelFormatEnum::ABGR8888);

        let width = 2u32;
        let height = 1u32;
        let screen = Screen::new(width, height);

        // Opaque red then opaque blue, in the VM's R,G,B,A byte order.
        let world: Vec<u8> = vec![255, 0, 0, 255, 0, 0, 255, 255];
        let ui: Vec<u8> = vec![0; (width * height * 4) as usize];
        let mut out = vec![0u8; (width * height * 4) as usize];
        screen.construct(&mut out, &world, &ui);

        // Byte 0 is red, byte 2 is blue — which ABGR8888 reads as the packed
        // little-endian u32 0xFF0000FF (A=FF, B=00, G=00, R=FF).
        assert_eq!(&out[0..4], &[255, 0, 0, 255], "first pixel should be red");
        assert_eq!(&out[4..8], &[0, 0, 255, 255], "second pixel should be blue");
    }
}
