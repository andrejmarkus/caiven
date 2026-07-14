mod cli;
mod input_router;
mod rom_io;
mod run_loop;

pub use cli::run;

use crate::debugger::{DebugClickAction, DebugMode, Debugger};
use crate::editors::{
    BrowserEditor, CodeEditor, Editor, MapEditor, MetaEditor, MusicEditor, PaletteEditor,
    SfxEditor, SpriteEditor,
};
use crate::hot_reload::HotReload;
use crate::tabs;
use anyhow::{Context, Result};
use fc_vm::default_instruction_set;
use fc_vm::input::{Input, InputMap};
use fc_vm::rendering::font::Font;
use fc_vm::rendering::screen::Screen;
use fc_vm::settings::NAME;
use fc_vm::timing::FixedTimestep;
use fc_vm::vm::audio::{Audio, AudioPeripheral};
use fc_vm::{Vm, VmConfig};
use log::{error, info};
use pixels::{Pixels, SurfaceTexture};
use rom_io::CartMeta;
use std::sync::Arc;
use std::time::Instant;
use winit::event::{ElementState, Modifiers, MouseButton, MouseScrollDelta};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    window::{Window, WindowAttributes},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Browser,
    Code,
    Run,
    Sprite,
    Map,
    Sfx,
    Music,
    Palette,
    Meta,
}

pub struct App {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    screen: Screen,
    input: Input,
    input_map: InputMap,
    vm: Vm,
    font: Font,
    config: VmConfig,
    #[allow(dead_code)]
    audio: Option<Audio>,
    debugger: Debugger,
    timing: FixedTimestep,
    last_tick: Instant,
    hot_reload: HotReload,
    mode: AppMode,
    sprite_editor: SpriteEditor,
    map_editor: MapEditor,
    palette_editor: PaletteEditor,
    meta_editor: MetaEditor,
    sfx_editor: SfxEditor,
    music_editor: MusicEditor,
    browser_editor: BrowserEditor,
    code_editor: CodeEditor,
    cart_meta: Option<CartMeta>,
    mouse_x: f64,
    mouse_y: f64,
    mouse_left: bool,
    mouse_right: bool,
    modifiers: Modifiers,
}

impl App {
    fn new() -> Result<Self> {
        let font = Font::from_image(
            "assets/font.png",
            " 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ!?\"'()+-=.:,[]<>",
            3,
            5,
        )
        .context("failed to initialize font")?;

        let config = VmConfig::default();
        let instruction_set = Arc::new(default_instruction_set());
        let mut vm = Vm::new(instruction_set, config);

        let audio = match Audio::new(vm.get_sound_shared()) {
            Ok(a) => Some(a),
            Err(e) => {
                error!("failed to initialize audio: {e}");
                None
            }
        };

        vm.register_peripheral(AudioPeripheral::new(vm.get_sound_shared()));

        info!("fantasy console engine initialized");

        Ok(Self {
            window: None,
            pixels: None,
            screen: Screen::new(config.width, config.height),
            input: Input::new(),
            input_map: InputMap::load("controls.toml"),
            vm,
            font,
            config,
            audio,
            debugger: Debugger::new(false),
            timing: FixedTimestep::new(60),
            last_tick: Instant::now(),
            hot_reload: HotReload::new(),
            mode: AppMode::Run,
            sprite_editor: SpriteEditor::new(),
            map_editor: MapEditor::new(),
            palette_editor: PaletteEditor::new(),
            meta_editor: MetaEditor::new(),
            sfx_editor: SfxEditor::new(),
            music_editor: MusicEditor::new(),
            browser_editor: BrowserEditor::new(),
            code_editor: CodeEditor::new(),
            cart_meta: None,
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_left: false,
            mouse_right: false,
            modifiers: Modifiers::default(),
        })
    }

    fn set_debug_enabled(&mut self, enabled: bool) {
        self.debugger.set_enabled(enabled);
    }

    fn logical_mouse_pos(&self) -> (u32, u32) {
        let (pw, ph) = self
            .window
            .as_ref()
            .map(|w| {
                let s = w.inner_size();
                (s.width as f64, s.height as f64)
            })
            .unwrap_or((
                (self.config.width * 4) as f64,
                (self.config.height * 4) as f64,
            ));
        let sx = (self.mouse_x / pw * self.config.width as f64)
            .clamp(0.0, (self.config.width - 1) as f64) as u32;
        let sy = (self.mouse_y / ph * self.config.height as f64)
            .clamp(0.0, (self.config.height - 1) as f64) as u32;
        (sx, sy)
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let screen_w = self.config.width * 4;
        let screen_h = self.config.height * 4;
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
        let pixels = match Pixels::new(self.config.width, self.config.height, surface) {
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

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if let Some(pixels) = self.pixels.as_mut() {
                    let _ = pixels.resize_surface(new_size.width, new_size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                self.screen.get_debug_layer().clear();
                let cursor = self.logical_mouse_pos();
                let font = &self.font;
                let vm = &self.vm;
                let debug_layer = self.screen.get_debug_layer();

                match self.mode {
                    AppMode::Sprite => {
                        self.sprite_editor.render(debug_layer, vm, font, cursor);
                    }
                    AppMode::Map => {
                        self.map_editor.render(debug_layer, vm, font, cursor);
                    }
                    AppMode::Palette => {
                        self.palette_editor.render(debug_layer, vm, font, cursor);
                    }
                    AppMode::Meta => {
                        self.meta_editor.render(debug_layer, vm, font, cursor);
                    }
                    AppMode::Sfx => {
                        self.sfx_editor.render(debug_layer, vm, font, cursor);
                    }
                    AppMode::Music => {
                        self.music_editor.render(debug_layer, vm, font, cursor);
                    }
                    AppMode::Code => {
                        self.code_editor.render(debug_layer, vm, font, cursor);
                    }
                    AppMode::Run => match self.debugger.get_mode() {
                        DebugMode::Paused | DebugMode::Step => {
                            self.debugger.draw_overlay(debug_layer, vm, font);
                        }
                        DebugMode::Running => {
                            self.debugger.draw_status_bar(debug_layer, vm, font);
                        }
                    },
                    AppMode::Browser => {
                        self.browser_editor.render(debug_layer, vm, font, cursor);
                    }
                }

                // Tab bar always visible
                tabs::draw_tab_bar(self.screen.get_debug_layer(), &self.font, self.mode);

                if let Some(pixels) = self.pixels.as_mut() {
                    self.screen.construct(
                        pixels.frame_mut(),
                        self.vm.world_pixels(),
                        self.vm.ui_pixels(),
                    );
                    let _ = pixels.render();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_x = position.x;
                self.mouse_y = position.y;
                let (sx, sy) = self.logical_mouse_pos();
                if self.mouse_left && self.mode != AppMode::Run {
                    self.dispatch_editor_drag(sx, sy);
                }
                if self.mouse_right && self.mode != AppMode::Run {
                    self.dispatch_editor_right_drag(sx, sy);
                }
                // Debugger timeline drag (Run mode)
                if self.mouse_left
                    && self.mode == AppMode::Run
                    && self.debugger.is_enabled()
                    && let DebugClickAction::RestoreScrub =
                        self.debugger.handle_click(sx, sy, &self.vm)
                    && let Some(state) = self.debugger.current_scrub_snapshot()
                {
                    self.vm.restore(&state);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if self.mode != AppMode::Run {
                    let (dx, dy) = match delta {
                        MouseScrollDelta::LineDelta(x, y) => (x, y),
                        MouseScrollDelta::PixelDelta(pos) => {
                            (pos.x as f32 / 20.0, pos.y as f32 / 20.0)
                        }
                    };
                    self.dispatch_editor_scroll(dx, dy);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                let (sx, sy) = self.logical_mouse_pos();
                match button {
                    MouseButton::Left => {
                        self.mouse_left = pressed;
                        if pressed {
                            // always dispatch so tab bar is clickable in Run mode
                            self.dispatch_editor_click(sx, sy);
                            self.poll_browser_load();
                            self.poll_code_editor_action();
                            // debugger overlay click (Run mode only)
                            if self.mode == AppMode::Run && self.debugger.is_enabled() {
                                let pc = self.vm.get_pc();
                                match self.debugger.handle_click(sx, sy, &self.vm) {
                                    DebugClickAction::TogglePause => self.debugger.toggle_pause(pc),
                                    DebugClickAction::Step => self.debugger.step(),
                                    DebugClickAction::RestoreScrub => {
                                        if let Some(state) = self.debugger.current_scrub_snapshot()
                                        {
                                            self.vm.restore(&state);
                                        }
                                    }
                                    DebugClickAction::None => {}
                                }
                            }
                        } else if !pressed && self.mode != AppMode::Run {
                            self.dispatch_editor_mouse_up(sx, sy);
                        }
                    }
                    MouseButton::Right => {
                        self.mouse_right = pressed;
                        if pressed && self.mode != AppMode::Run {
                            self.dispatch_editor_right_click(sx, sy);
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods;
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard(event);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.update();

        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}
