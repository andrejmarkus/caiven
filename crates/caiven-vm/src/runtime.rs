//! Shared front-end runtime: the VM + peripherals bundle and the
//! winit/pixels window plumbing used by both the editor (caiven-studio)
//! and the cart runner (caiven-machine).

use crate::input::{Input, InputMap};
use crate::rendering::font::Font;
use crate::rendering::screen::Screen;
use crate::settings::NAME;
use crate::timing::FixedTimestep;
use crate::vm::audio::{Audio, AudioPeripheral};
use crate::{Vm, VmConfig};
use anyhow::{Context, Result};
use log::{error, info};
use pixels::{Pixels, SurfaceTexture};
use std::sync::Arc;
use std::time::Instant;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

/// Glyphs available in the built-in font sheet, in sheet order.
pub const FONT_GLYPHS: &str = " 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ!?\"'()+-=.:,[]<>";
/// Path to the built-in font sheet, relative to the working directory.
pub const FONT_PATH: &str = "assets/font.png";
/// Integer scale factor from console resolution to initial window size.
pub const WINDOW_SCALE: u32 = 4;

/// Everything a console front-end needs besides a window: a VM with the
/// audio peripheral registered, screen composition buffers, input state
/// and fixed-timestep frame timing.
pub struct ConsoleCore {
    pub screen: Screen,
    pub input: Input,
    pub input_map: InputMap,
    pub vm: Vm,
    pub font: Font,
    pub config: VmConfig,
    /// Owns the audio output stream; dropping it silences the console.
    pub audio: Option<Audio>,
    pub timing: FixedTimestep,
    pub last_tick: Instant,
}

impl ConsoleCore {
    pub fn new() -> Result<Self> {
        let font =
            Font::from_image(FONT_PATH, FONT_GLYPHS, 3, 5).context("failed to initialize font")?;

        let config = VmConfig::default();
        let mut vm = Vm::new(config);

        let audio = match Audio::new(vm.get_sound_shared()) {
            Ok(a) => Some(a),
            Err(e) => {
                error!("failed to initialize audio: {e}");
                None
            }
        };

        vm.register_peripheral(AudioPeripheral::new(vm.get_sound_shared()));

        info!("fantasy console initialized");

        Ok(Self {
            screen: Screen::new(config.width, config.height),
            input: Input::new(),
            input_map: InputMap::load("controls.toml"),
            vm,
            font,
            config,
            audio,
            timing: FixedTimestep::new(60),
            last_tick: Instant::now(),
        })
    }

    /// Replaces the VM and audio device with a blank state, keeping
    /// screen/input/font/timing. Used to start editing a brand-new cart
    /// without carrying over a previously loaded cart's RAM.
    pub fn reset_vm(&mut self) {
        let mut vm = Vm::new(self.config);
        let audio = match Audio::new(vm.get_sound_shared()) {
            Ok(a) => Some(a),
            Err(e) => {
                error!("failed to initialize audio: {e}");
                None
            }
        };
        vm.register_peripheral(AudioPeripheral::new(vm.get_sound_shared()));
        self.vm = vm;
        self.audio = audio;
    }

    /// Advances the fixed-timestep clock; returns how many frames to run now.
    pub fn frame_steps(&mut self) -> u32 {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick);
        self.last_tick = now;
        self.timing.tick(dt)
    }

    /// Runs one VM frame with the current input state, then latches it so
    /// edge-triggered input (INP/INPR, `btnp`) sees per-frame transitions.
    pub fn run_frame(&mut self) {
        self.vm.run_frame(&self.input, &self.font);
        self.input.end_frame();
    }

    /// Runs one Lua-scripted frame honoring line breakpoints; input latches
    /// like `run_frame`. See [`crate::vm::Vm::run_frame_lua_bp`].
    pub fn run_frame_lua_bp(&mut self, breakpoints: &[usize]) -> crate::vm::LuaRunOutcome {
        let outcome = self
            .vm
            .run_frame_lua_bp(&self.input, &self.font, breakpoints);
        self.input.end_frame();
        outcome
    }
}

/// Winit window and pixel surface for a console front-end. Both stay `None`
/// until the first `resumed` event creates them.
#[derive(Default)]
pub struct WindowGfx {
    pub window: Option<Arc<Window>>,
    pub pixels: Option<Pixels<'static>>,
}

impl WindowGfx {
    /// Creates the window and pixel buffer on `resumed`. On failure logs
    /// the error and exits the event loop.
    pub fn resume(&mut self, event_loop: &ActiveEventLoop, config: &VmConfig) {
        let screen_w = config.width * WINDOW_SCALE;
        let screen_h = config.height * WINDOW_SCALE;
        let window_attrs = WindowAttributes::default()
            .with_title(NAME)
            .with_inner_size(LogicalSize::new(screen_w as f64, screen_h as f64))
            .with_resizable(false);

        let window = match event_loop.create_window(window_attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                error!("failed to create window: {e}");
                event_loop.exit();
                return;
            }
        };

        let size = window.inner_size();
        let surface = SurfaceTexture::new(size.width, size.height, window.clone());
        let pixels = match Pixels::new(config.width, config.height, surface) {
            Ok(p) => p,
            Err(e) => {
                error!("failed to create pixel buffer: {e}");
                event_loop.exit();
                return;
            }
        };

        self.window = Some(window);
        self.pixels = Some(pixels);
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if let Some(pixels) = self.pixels.as_mut() {
            let _ = pixels.resize_surface(new_size.width, new_size.height);
        }
    }

    /// Composites the screen layers over the VM's world/UI pixels and renders.
    pub fn present(&mut self, screen: &Screen, vm: &Vm) {
        if let Some(pixels) = self.pixels.as_mut() {
            screen.construct(pixels.frame_mut(), vm.world_pixels(), vm.ui_pixels());
            let _ = pixels.render();
        }
    }

    pub fn request_redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}
