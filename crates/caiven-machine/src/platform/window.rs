//! SDL2 window, renderer and the streaming texture the console draws into.

use anyhow::{Context, Result, anyhow};
use caiven_vm::rendering::screen::Screen;
use caiven_vm::{Vm, VmConfig};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{Texture, TextureCreator, WindowCanvas};
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
/// The console texture is deliberately *not* stored here: a `Texture`
/// borrows its `TextureCreator`, and keeping both in one struct would make
/// it self-referential. The caller owns the creator and passes the texture
/// in, which keeps this whole module free of unsafe.
pub struct Display {
    canvas: WindowCanvas,
    scale: ScaleMode,
    aspect: AspectMode,
}

impl Display {
    /// Creates the window and renderer.
    pub fn new(
        video: &sdl2::VideoSubsystem,
        config: &VmConfig,
        title: &str,
        fullscreen: bool,
        scale: ScaleMode,
        aspect: AspectMode,
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

        Ok(Self {
            canvas,
            scale,
            aspect,
        })
    }

    /// Creates the texture creator the console texture is allocated from.
    /// It holds its own reference to the window context, so it outlives
    /// nothing in particular and may be kept by the caller.
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

    /// Composites the screen layers over the VM's world/UI pixels and
    /// presents the frame.
    pub fn present(&mut self, texture: &mut Texture, screen: &Screen, vm: &Vm) -> Result<()> {
        texture
            .with_lock(None, |buffer: &mut [u8], _pitch: usize| {
                screen.construct(buffer, vm.world_pixels(), vm.ui_pixels());
            })
            .map_err(|e| anyhow!("failed to lock console texture: {e}"))?;

        let (win_w, win_h) = self.canvas.window().size();
        let query = texture.query();
        let dst = dst_rect(
            (win_w, win_h),
            (query.width, query.height),
            self.scale,
            self.aspect,
        );

        self.canvas.set_draw_color(Color::BLACK);
        self.canvas.clear();
        self.canvas
            .copy(
                texture,
                None,
                Some(Rect::new(dst.x, dst.y, dst.width, dst.height)),
            )
            .map_err(|e| anyhow!("failed to copy console texture: {e}"))?;
        self.canvas.present();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::CONSOLE_PIXEL_FORMAT;
    use caiven_vm::rendering::screen::Screen;
    use sdl2::pixels::PixelFormatEnum;

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
