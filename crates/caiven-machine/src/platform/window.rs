//! SDL2 window, renderer and the streaming texture the console draws into.

use anyhow::{Context, Result, anyhow};
use caiven_vm::rendering::screen::Screen;
use caiven_vm::{Vm, VmConfig};
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{Texture, TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;

use crate::platform::scaling::{AspectMode, DstRect, ScaleMode, dst_rect};

/// The VM composites into a plain byte slice as R, G, B, A
/// (`caiven-core/src/memory.rs` `RGBA_BYTES`). SDL names its formats by the
/// packed 32-bit integer, so on a little-endian host that byte order reads
/// back as ABGR8888. Naming it RGBA8888 here silently swaps red and blue.
pub const CONSOLE_PIXEL_FORMAT: PixelFormatEnum = PixelFormatEnum::ABGR8888;

/// Integer scale factor from console resolution to initial window size.
const WINDOW_SCALE: u32 = 4;

/// Looks up the max texture size an accelerated renderer would report,
/// without ever creating a window or renderer — the Miyoo Mini's video
/// driver only tolerates one `SDL_CreateWindow` per process, so the size
/// must be known before the one window is built, not fixed up after.
fn probe_max_texture_size(accelerated: bool) -> Option<(u32, u32)> {
    let accelerated_bit = sdl2::sys::SDL_RendererFlags::SDL_RENDERER_ACCELERATED as u32;
    sdl2::render::drivers()
        .find(|d| !accelerated || d.flags & accelerated_bit != 0)
        .filter(|d| d.max_texture_width > 0 && d.max_texture_height > 0)
        .map(|d| (d.max_texture_width, d.max_texture_height))
}

/// Builds a window and its renderer.
///
/// `accelerated` asks for a GPU-backed, vsynced renderer. `max_texture_size`
/// is a ceiling — typically from `probe_max_texture_size` — clamped against
/// the console-derived default (`config.width/height * WINDOW_SCALE`) rather
/// than used outright, since a desktop GPU renderer's ceiling (often in the
/// thousands of pixels) is not a size any window should actually request.
fn build_canvas(
    video: &sdl2::VideoSubsystem,
    config: &VmConfig,
    title: &str,
    fullscreen: bool,
    accelerated: bool,
    max_texture_size: Option<(u32, u32)>,
) -> Result<WindowCanvas> {
    let (default_w, default_h) = (config.width * WINDOW_SCALE, config.height * WINDOW_SCALE);
    let (width, height) = match max_texture_size {
        Some((max_w, max_h)) => (default_w.min(max_w), default_h.min(max_h)),
        None => (default_w, default_h),
    };
    let mut builder = video.window(title, width, height);
    // Without this, a HiDPI display (any current Mac) renders the window at
    // 1x and the OS upscales that to fit — every pixel, console art and
    // shell chrome alike, comes out visibly soft/blurry. A fixed-DPI
    // handheld panel has no such scaling to opt into, so this is a no-op
    // there.
    builder.allow_highdpi();
    builder.position_centered();
    if fullscreen {
        // Not `fullscreen_desktop()`: the Miyoo Mini's video driver never
        // populates `desktop_mode`, only its mode list, so that flag resolves
        // to a 0x0 window. Plain `SDL_WINDOW_FULLSCREEN` resolves against the
        // mode list instead and gets a real size.
        builder.fullscreen();
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
    /// Reused across frames by `composite_frame` for its `src_x` lookup
    /// table, so building that table doesn't allocate every frame.
    src_x_scratch: Vec<u32>,
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

        // `SDL_WINDOW_FULLSCREEN` can size the window larger than the Mini
        // driver's real texture ceiling; its blit path then divides by the
        // oversized window dimension and truncates to a zero-size dest rect
        // (permanently black screen, no error). Resizing or recreating the
        // window afterward both fail on this driver, so the size is probed
        // and requested exactly once, up front.
        let accelerated_size = probe_max_texture_size(true);
        let canvas = match build_canvas(video, config, title, fullscreen, true, accelerated_size) {
            Ok(canvas) => canvas,
            Err(e) => {
                log::info!("no accelerated/vsync renderer ({e}); falling back to software");
                let software_size = probe_max_texture_size(false);
                build_canvas(video, config, title, fullscreen, false, software_size)
                    .context("failed to create SDL renderer")?
            }
        };

        log::info!("render driver: {}", canvas.info().name);
        let (final_w, final_h) = canvas.window().size();
        log::info!(
            "window ready: size={final_w}x{final_h}, fullscreen_state={:?}, max_texture={}x{}",
            canvas.window().fullscreen_state(),
            canvas.info().max_texture_width,
            canvas.info().max_texture_height,
        );

        Ok(Self {
            canvas,
            src_x_scratch: Vec::new(),
        })
    }

    /// The window's current size in actual pixels — its drawable size,
    /// which on a HiDPI display is larger than `Window::size()`'s point
    /// size. Everything allocated off this (the shell surface, the console
    /// texture's destination rect) must use this, not the point size, or
    /// the rendered frame only fills a fraction of the real window.
    ///
    /// Falls back to `size()` when `drawable_size` reports `(0, 0)` — the
    /// Miyoo Mini driver's `GL_GetDrawableSize` hook does that instead of
    /// deferring, and the device has no HiDPI scaling anyway so `size()` is
    /// exact. Also clamped to `max_texture_width/height`: fullscreen can
    /// pick a window mode (e.g. 800x600) bigger than what the renderer can
    /// actually allocate a texture at (640x480 on this panel).
    pub fn window_size(&self) -> (u32, u32) {
        let (w, h) = match self.canvas.window().drawable_size() {
            (0, 0) => self.canvas.window().size(),
            size => size,
        };
        let info = self.canvas.info();
        let w = if info.max_texture_width > 0 {
            w.min(info.max_texture_width)
        } else {
            w
        };
        let h = if info.max_texture_height > 0 {
            h.min(info.max_texture_height)
        } else {
            h
        };
        (w, h)
    }

    /// Creates the texture creator the console and shell textures are
    /// allocated from. It holds its own reference to the window context, so
    /// it outlives nothing in particular and may be kept by the caller.
    pub fn texture_creator(&self) -> TextureCreator<WindowContext> {
        self.canvas.texture_creator()
    }

    /// Allocates the streaming texture the fully-composited frame (console
    /// framebuffer + shell overlay, already blended in software — see
    /// `present`'s doc comment) is uploaded into, at window resolution.
    pub fn create_frame_texture<'tc>(
        creator: &'tc TextureCreator<WindowContext>,
        width: u32,
        height: u32,
    ) -> Result<Texture<'tc>> {
        creator
            .create_texture_streaming(CONSOLE_PIXEL_FORMAT, width, height)
            .context("failed to create frame texture")
    }

    /// Composites the console framebuffer and the shell's raster overlay in
    /// software and presents the result as a single opaque blit.
    ///
    /// Software compositing instead of two hardware blits: the Miyoo Mini's
    /// `Mini_QueueCopy` hardcodes `eDFBBlendFlag = 0` and never blends, so a
    /// second blit-with-alpha just paints opaque over the console. One
    /// pre-blended image sidesteps that on every driver.
    ///
    /// `console_buffer` is scratch memory sized to `console_size` (console
    /// resolution), reused every frame so this allocates nothing per frame.
    /// `shell_rgba` is `Surface::rgba()`'s output: premultiplied alpha, in
    /// the same byte order as the console framebuffer (SPEC V46, V28).
    #[allow(clippy::too_many_arguments)]
    pub fn present(
        &mut self,
        frame_texture: &mut Texture,
        console_buffer: &mut [u8],
        console_size: (u32, u32),
        screen: &Screen,
        vm: &Vm,
        scale: ScaleMode,
        aspect: AspectMode,
        shell_rgba: &[u8],
        shell_fully_transparent: bool,
    ) -> Result<PresentTiming> {
        let t0 = std::time::Instant::now();
        screen.construct(console_buffer, vm.world_pixels(), vm.ui_pixels());
        let construct = t0.elapsed();

        let (win_w, win_h) = self.window_size();
        let dst = dst_rect((win_w, win_h), console_size, scale, aspect);

        let src_x_scratch = &mut self.src_x_scratch;
        let t1 = std::time::Instant::now();
        frame_texture
            .with_lock(None, |buffer: &mut [u8], _pitch: usize| {
                composite_frame(
                    buffer,
                    (win_w, win_h),
                    console_buffer,
                    console_size,
                    dst,
                    shell_rgba,
                    shell_fully_transparent,
                    src_x_scratch,
                );
            })
            .map_err(|e| anyhow!("failed to lock frame texture: {e}"))?;
        let composite = t1.elapsed();

        let t2 = std::time::Instant::now();
        self.canvas
            .copy(frame_texture, None, None)
            .map_err(|e| anyhow!("failed to copy frame texture: {e}"))?;
        let copy = t2.elapsed();

        let t3 = std::time::Instant::now();
        self.canvas.present();
        let sdl_present = t3.elapsed();

        Ok(PresentTiming {
            construct,
            composite,
            copy,
            sdl_present,
        })
    }
}

/// Per-phase breakdown of [`Display::present`]'s cost. Temporary diagnostic
/// — remove once the fps report is resolved.
#[derive(Debug, Default, Clone, Copy)]
pub struct PresentTiming {
    pub construct: std::time::Duration,
    pub composite: std::time::Duration,
    pub copy: std::time::Duration,
    pub sdl_present: std::time::Duration,
}

/// Rasterizes the console framebuffer (nearest-neighbour, scaled and
/// positioned per `dst`) into `out`, then alpha-composites `shell_rgba`
/// (premultiplied, `window`-sized) on top — a software "over" blend, so the
/// result is always a fully opaque `window`-sized image regardless of what
/// the destination renderer does with texture blend modes.
fn composite_frame(
    out: &mut [u8],
    window: (u32, u32),
    console: &[u8],
    console_size: (u32, u32),
    dst: DstRect,
    shell_rgba: &[u8],
    shell_fully_transparent: bool,
    src_x_scratch: &mut Vec<u32>,
) {
    let (win_w, win_h) = window;
    let (con_w, con_h) = console_size;
    out.fill(0);

    if con_w > 0 && con_h > 0 && dst.width > 0 && dst.height > 0 {
        let x0 = dst.x.max(0) as u32;
        let y0 = dst.y.max(0) as u32;
        let x1 = (dst.x + dst.width as i32).clamp(0, win_w as i32) as u32;
        let y1 = (dst.y + dst.height as i32).clamp(0, win_h as i32) as u32;

        // `src_x` only depends on `x`, so build it once per column instead
        // of redoing the division per pixel — the Cortex-A7 has no hardware
        // 64-bit divider, so that redundant division was ~50ms/frame.
        src_x_scratch.clear();
        src_x_scratch.extend((x0..x1).map(|x| {
            let src_x = (((x as i32 - dst.x) as u64 * con_w as u64) / dst.width as u64) as u32;
            src_x.min(con_w - 1)
        }));

        for y in y0..y1 {
            let src_y = (((y as i32 - dst.y) as u64 * con_h as u64) / dst.height as u64) as u32;
            let src_y = src_y.min(con_h - 1);
            let dst_row = (y * win_w) as usize * 4;
            let src_row = (src_y * con_w) as usize * 4;
            for (i, x) in (x0..x1).enumerate() {
                let src_x = src_x_scratch[i];
                let src_i = src_row + src_x as usize * 4;
                let dst_i = dst_row + x as usize * 4;
                out[dst_i..dst_i + 4].copy_from_slice(&console[src_i..src_i + 4]);
            }
        }
    }

    // Skip the blend entirely when the shell has nothing to draw (e.g.
    // `Screen::Playing` most frames) instead of scanning every pixel for
    // `a == 0`.
    if shell_fully_transparent {
        return;
    }

    for (px_out, px_shell) in out.chunks_exact_mut(4).zip(shell_rgba.chunks_exact(4)) {
        let a = px_shell[3] as u32;
        if a == 0 {
            continue;
        }
        if a == 255 {
            px_out.copy_from_slice(px_shell);
            continue;
        }
        let inv = 255 - a;
        for c in 0..3 {
            px_out[c] = (px_shell[c] as u32 + (px_out[c] as u32 * inv) / 255).min(255) as u8;
        }
        px_out[3] = 255;
    }
}

#[cfg(test)]
mod tests {
    use super::{CONSOLE_PIXEL_FORMAT, composite_frame};
    use crate::platform::scaling::DstRect;
    use caiven_vm::rendering::screen::Screen;
    use sdl2::pixels::PixelFormatEnum;

    #[test]
    fn composite_frame_leaves_transparent_shell_pixels_showing_console_beneath() {
        let mut out = vec![9u8; 8]; // 2 opaque-red console pixels, pre-filled with junk.
        let console = [255u8, 0, 0, 255, 255, 0, 0, 255];
        let shell = [0u8, 0, 0, 0, 0, 0, 0, 0]; // fully transparent overlay.
        let dst = DstRect {
            x: 0,
            y: 0,
            width: 2,
            height: 1,
        };
        composite_frame(
            &mut out,
            (2, 1),
            &console,
            (2, 1),
            dst,
            &shell,
            true,
            &mut Vec::new(),
        );
        assert_eq!(
            &out[0..4],
            &[255, 0, 0, 255],
            "console shows through untouched"
        );
    }

    #[test]
    fn composite_frame_blends_partial_alpha_shell_over_console() {
        let mut out = vec![0u8; 4];
        let console = [255u8, 0, 0, 255]; // opaque red beneath.
        // Half-alpha, already-premultiplied blue overlay: 0,0,127,127.
        let shell = [0u8, 0, 127, 127];
        let dst = DstRect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };
        composite_frame(
            &mut out,
            (1, 1),
            &console,
            (1, 1),
            dst,
            &shell,
            false,
            &mut Vec::new(),
        );
        assert_eq!(out[3], 255, "result is always fully opaque");
        assert!(
            out[0] > 100 && out[0] < 155,
            "red shows through partial alpha"
        );
        assert_eq!(out[2], 127, "premultiplied blue contributes directly");
    }

    #[test]
    fn composite_frame_letterboxes_outside_dst_rect_in_black() {
        let mut out = vec![9u8; 16]; // 4 pixels, pre-filled with junk.
        let console = [255u8, 255, 255, 255];
        let shell = [0u8; 16]; // fully transparent, window-sized.
        // Console only occupies the rightmost column of a 2x2 window.
        let dst = DstRect {
            x: 1,
            y: 0,
            width: 1,
            height: 2,
        };
        composite_frame(
            &mut out,
            (2, 2),
            &console,
            (1, 1),
            dst,
            &shell,
            true,
            &mut Vec::new(),
        );
        assert_eq!(&out[0..4], &[0, 0, 0, 0], "letterboxed pixel is black");
        assert_eq!(
            &out[4..8],
            &[255, 255, 255, 255],
            "console pixel unaffected"
        );
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
