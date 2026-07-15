mod cli;
mod input_router;
pub(crate) mod rom_io;
mod run_loop;

pub use cli::run;

use crate::debugger::{DebugClickAction, DebugMode, Debugger};
use crate::hot_reload::HotReload;
use anyhow::Result;
use fc_vm::runtime::{ConsoleCore, WINDOW_SCALE, WindowGfx};
use rom_io::CartMeta;
use winit::event::{ElementState, Modifiers, MouseButton};
use winit::{application::ApplicationHandler, event::WindowEvent};

pub struct App {
    core: ConsoleCore,
    gfx: WindowGfx,
    debugger: Debugger,
    hot_reload: HotReload,
    cart_meta: Option<CartMeta>,
    mouse_x: f64,
    mouse_y: f64,
    mouse_left: bool,
    modifiers: Modifiers,
}

impl App {
    fn new() -> Result<Self> {
        Ok(Self {
            core: ConsoleCore::new()?,
            gfx: WindowGfx::default(),
            debugger: Debugger::new(false),
            hot_reload: HotReload::new(),
            cart_meta: None,
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_left: false,
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
                let font = &self.core.font;
                let vm = &self.core.vm;
                let debug_layer = self.core.screen.get_debug_layer();

                match self.debugger.get_mode() {
                    DebugMode::Paused | DebugMode::Step => {
                        self.debugger.draw_overlay(debug_layer, vm, font);
                    }
                    DebugMode::Running => {
                        self.debugger.draw_status_bar(debug_layer, vm, font);
                    }
                }

                self.gfx.present(&self.core.screen, &self.core.vm);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_x = position.x;
                self.mouse_y = position.y;
                let (sx, sy) = self.logical_mouse_pos();
                // Debugger timeline drag
                if self.mouse_left
                    && self.debugger.is_enabled()
                    && let DebugClickAction::RestoreScrub =
                        self.debugger.handle_click(sx, sy, &self.core.vm)
                    && let Some(state) = self.debugger.current_scrub_snapshot()
                {
                    self.core.vm.restore(&state);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    let pressed = state == ElementState::Pressed;
                    self.mouse_left = pressed;
                    if pressed && self.debugger.is_enabled() {
                        let (sx, sy) = self.logical_mouse_pos();
                        let pc = self.core.vm.get_pc();
                        match self.debugger.handle_click(sx, sy, &self.core.vm) {
                            DebugClickAction::TogglePause => self.debugger.toggle_pause(pc),
                            DebugClickAction::Step => self.debugger.step(),
                            DebugClickAction::RestoreScrub => {
                                if let Some(state) = self.debugger.current_scrub_snapshot() {
                                    self.core.vm.restore(&state);
                                }
                            }
                            DebugClickAction::None => {}
                        }
                    }
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
