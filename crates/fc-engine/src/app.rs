mod cli;
mod input_router;
pub(crate) mod rom_io;
mod run_loop;

pub use cli::run;

use crate::debugger::{DebugClickAction, DebugMode, Debugger};
use crate::editors::{
    BrowserEditor, CodeEditor, Editor, MapEditor, MetaEditor, MusicEditor, PaletteEditor,
    SfxEditor, SpriteEditor,
};
use crate::hot_reload::HotReload;
use crate::tabs;
use anyhow::Result;
use fc_vm::runtime::{ConsoleCore, WINDOW_SCALE, WindowGfx};
use rom_io::CartMeta;
use winit::event::{ElementState, Modifiers, MouseButton, MouseScrollDelta};
use winit::{application::ApplicationHandler, event::WindowEvent};

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
    core: ConsoleCore,
    gfx: WindowGfx,
    debugger: Debugger,
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
        Ok(Self {
            core: ConsoleCore::new()?,
            gfx: WindowGfx::default(),
            debugger: Debugger::new(false),
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
        let config = &self.core.config;
        let (pw, ph) = self
            .gfx
            .window
            .as_ref()
            .map(|w| {
                let s = w.inner_size();
                (s.width as f64, s.height as f64)
            })
            .unwrap_or((
                (config.width * WINDOW_SCALE) as f64,
                (config.height * WINDOW_SCALE) as f64,
            ));
        let sx =
            (self.mouse_x / pw * config.width as f64).clamp(0.0, (config.width - 1) as f64) as u32;
        let sy = (self.mouse_y / ph * config.height as f64).clamp(0.0, (config.height - 1) as f64)
            as u32;
        (sx, sy)
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.gfx.resume(event_loop, &self.core.config);
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
                self.gfx.resize(new_size);
            }
            WindowEvent::RedrawRequested => {
                self.core.screen.get_debug_layer().clear();
                let cursor = self.logical_mouse_pos();
                let font = &self.core.font;
                let vm = &self.core.vm;
                let debug_layer = self.core.screen.get_debug_layer();

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
                tabs::draw_tab_bar(
                    self.core.screen.get_debug_layer(),
                    &self.core.font,
                    self.mode,
                );

                self.gfx.present(&self.core.screen, &self.core.vm);
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
                        self.debugger.handle_click(sx, sy, &self.core.vm)
                    && let Some(state) = self.debugger.current_scrub_snapshot()
                {
                    self.core.vm.restore(&state);
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
                                let pc = self.core.vm.get_pc();
                                match self.debugger.handle_click(sx, sy, &self.core.vm) {
                                    DebugClickAction::TogglePause => self.debugger.toggle_pause(pc),
                                    DebugClickAction::Step => self.debugger.step(),
                                    DebugClickAction::RestoreScrub => {
                                        if let Some(state) = self.debugger.current_scrub_snapshot()
                                        {
                                            self.core.vm.restore(&state);
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
        self.gfx.request_redraw();
    }
}
