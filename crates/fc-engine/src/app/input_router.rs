//! Keyboard and mouse routing for [`App`]: global shortcuts, mode switches
//! and per-editor event dispatch.

use super::{App, AppMode};
use crate::debugger::DebugMode;
use crate::editors::{CodeEditorAction, Editor};
use crate::tabs;
use log::{info, warn};
use winit::event::KeyEvent;
use winit::keyboard::{KeyCode, PhysicalKey};

impl App {
    pub(super) fn dispatch_editor_click(&mut self, x: u32, y: u32) {
        if let Some(new_mode) = tabs::hit_test(x, y) {
            self.mode = new_mode;
            return;
        }
        let vm = &mut self.core.vm;
        match self.mode {
            AppMode::Sprite => self.sprite_editor.handle_click(x, y, vm),
            AppMode::Map => self.map_editor.handle_click(x, y, vm),
            AppMode::Palette => self.palette_editor.handle_click(x, y, vm),
            AppMode::Meta => self.meta_editor.handle_click(x, y, vm),
            AppMode::Sfx => self.sfx_editor.handle_click(x, y, vm),
            AppMode::Music => self.music_editor.handle_click(x, y, vm),
            AppMode::Browser => self.browser_editor.handle_click(x, y, vm),
            AppMode::Code => self.code_editor.handle_click(x, y, vm),
            AppMode::Run => {}
        }
    }

    pub(super) fn dispatch_editor_drag(&mut self, x: u32, y: u32) {
        if tabs::hit_test(x, y).is_some() {
            return;
        }
        let vm = &mut self.core.vm;
        match self.mode {
            AppMode::Sprite => self.sprite_editor.handle_drag(x, y, vm),
            AppMode::Map => self.map_editor.handle_drag(x, y, vm),
            AppMode::Palette => self.palette_editor.handle_drag(x, y, vm),
            AppMode::Meta => self.meta_editor.handle_drag(x, y, vm),
            AppMode::Sfx => self.sfx_editor.handle_drag(x, y, vm),
            AppMode::Music => self.music_editor.handle_drag(x, y, vm),
            AppMode::Browser => self.browser_editor.handle_drag(x, y, vm),
            AppMode::Code => self.code_editor.handle_drag(x, y, vm),
            AppMode::Run => {}
        }
    }

    pub(super) fn dispatch_editor_mouse_up(&mut self, x: u32, y: u32) {
        let vm = &mut self.core.vm;
        match self.mode {
            AppMode::Sprite => self.sprite_editor.handle_mouse_up(x, y, vm),
            AppMode::Map => self.map_editor.handle_mouse_up(x, y, vm),
            AppMode::Palette => self.palette_editor.handle_mouse_up(x, y, vm),
            AppMode::Meta => self.meta_editor.handle_mouse_up(x, y, vm),
            AppMode::Sfx => self.sfx_editor.handle_mouse_up(x, y, vm),
            AppMode::Music => self.music_editor.handle_mouse_up(x, y, vm),
            AppMode::Browser => self.browser_editor.handle_mouse_up(x, y, vm),
            AppMode::Run | AppMode::Code => {}
        }
    }

    pub(super) fn dispatch_editor_right_click(&mut self, x: u32, y: u32) {
        let vm = &mut self.core.vm;
        match self.mode {
            AppMode::Sprite => self.sprite_editor.handle_right_click(x, y, vm),
            AppMode::Map => self.map_editor.handle_right_click(x, y, vm),
            _ => {}
        }
    }

    pub(super) fn dispatch_editor_right_drag(&mut self, x: u32, y: u32) {
        let vm = &mut self.core.vm;
        match self.mode {
            AppMode::Sprite => self.sprite_editor.handle_right_drag(x, y, vm),
            AppMode::Map => self.map_editor.handle_right_drag(x, y, vm),
            _ => {}
        }
    }

    pub(super) fn dispatch_editor_scroll(&mut self, dx: f32, dy: f32) {
        let vm = &mut self.core.vm;
        match self.mode {
            AppMode::Sprite => self.sprite_editor.handle_scroll(dx, dy, vm),
            AppMode::Map => self.map_editor.handle_scroll(dx, dy, vm),
            AppMode::Sfx => self.sfx_editor.handle_scroll(dx, dy, vm),
            AppMode::Music => self.music_editor.handle_scroll(dx, dy, vm),
            AppMode::Browser => self.browser_editor.handle_scroll(dx, dy, vm),
            AppMode::Code => self.code_editor.handle_scroll(dx, dy, vm),
            _ => {}
        }
    }

    pub(super) fn poll_code_editor_action(&mut self) {
        if let Some(action) = self.code_editor.pending_action.take() {
            self.apply_code_editor_action(action);
        }
    }

    pub(super) fn apply_code_editor_action(&mut self, action: CodeEditorAction) {
        match action {
            CodeEditorAction::None => {}
            CodeEditorAction::Save => {
                if self.code_editor.save() {
                    info!("code editor: source saved");
                } else {
                    warn!("code editor: save failed (no path?)");
                }
            }
            CodeEditorAction::CompileAndRun => {
                let source = self.code_editor.get_source();
                match fc_lang::compile(&source) {
                    Ok(out) => {
                        self.core
                            .vm
                            .load_rom_with_source_map(out.program, out.source_map);
                        self.core.vm.set_fc_source(&source);
                        if let Some(path) = &self.code_editor.source_path {
                            let _ = std::fs::write(path, &source);
                        }
                        self.code_editor.error_msg = None;
                        self.mode = AppMode::Run;
                        info!("code editor: compiled and running");
                    }
                    Err(e) => {
                        let msg = format!("{e}");
                        warn!("code editor: compile error: {msg}");
                        self.code_editor.error_msg = Some(msg);
                    }
                }
            }
        }
    }

    pub(super) fn handle_keyboard(&mut self, event: KeyEvent) {
        let pressed = event.state.is_pressed();

        if let PhysicalKey::Code(code) = event.physical_key {
            if let Some(button) = self.core.input_map.get_button(code) {
                self.core.input.set_button(button, pressed);
            }

            let ctrl = self.modifiers.state().control_key();

            if pressed && !event.repeat {
                match code {
                    KeyCode::KeyS if ctrl => {
                        self.save_cart();
                        return;
                    }
                    // Tab-bar mode switches (F1–F7, F8=browser)
                    KeyCode::F1 => {
                        self.mode = AppMode::Run;
                        return;
                    }
                    KeyCode::F2 => {
                        self.mode = AppMode::Sprite;
                        return;
                    }
                    KeyCode::F3 => {
                        self.mode = AppMode::Map;
                        return;
                    }
                    KeyCode::F4 => {
                        self.mode = AppMode::Sfx;
                        return;
                    }
                    KeyCode::F5 => {
                        self.mode = AppMode::Music;
                        return;
                    }
                    KeyCode::F6 => {
                        self.mode = AppMode::Palette;
                        return;
                    }
                    KeyCode::F7 => {
                        self.mode = AppMode::Meta;
                        return;
                    }
                    KeyCode::F8 => {
                        self.mode = AppMode::Browser;
                        return;
                    }
                    KeyCode::F9 => {
                        self.mode = AppMode::Code;
                        return;
                    }
                    _ => {}
                }
            }

            // Code editor — handle directly with modifier state
            if self.mode == AppMode::Code && pressed {
                let shift = self.modifiers.state().shift_key();
                let action = self.code_editor.handle_key_direct(code, shift, ctrl);
                self.apply_code_editor_action(action);
                return;
            }

            // Run-mode debugger controls
            if self.mode == AppMode::Run {
                let paused = self.debugger.get_mode() == DebugMode::Paused;
                match code {
                    KeyCode::Space if pressed && !event.repeat => {
                        self.debugger.toggle_pause(self.core.vm.get_pc());
                    }
                    KeyCode::KeyC if pressed && !event.repeat => {
                        self.debugger.step();
                    }
                    KeyCode::F10 if pressed && !event.repeat && paused => {
                        self.debugger.step();
                    }
                    KeyCode::KeyB if pressed && !event.repeat && paused => {
                        self.debugger.toggle_bp_at_cursor();
                    }
                    KeyCode::ArrowUp if pressed && paused => {
                        self.debugger.cursor_up(&self.core.vm);
                    }
                    KeyCode::ArrowDown if pressed && paused => {
                        self.debugger.cursor_down(&self.core.vm);
                    }
                    KeyCode::ArrowLeft if pressed && paused => {
                        self.debugger.scrub_back();
                        if let Some(state) = self.debugger.current_scrub_snapshot() {
                            self.core.vm.restore(&state);
                        }
                    }
                    KeyCode::ArrowRight if pressed && paused => {
                        self.debugger.scrub_forward();
                        if let Some(state) = self.debugger.current_scrub_snapshot() {
                            self.core.vm.restore(&state);
                        }
                    }
                    KeyCode::KeyN if pressed && !event.repeat => {
                        self.debugger.prev_ram_page();
                    }
                    KeyCode::KeyM if pressed && !event.repeat => {
                        self.debugger.next_ram_page();
                    }
                    _ => {}
                }
            } else if pressed {
                // Delegate key to active editor
                let vm = &mut self.core.vm;
                match self.mode {
                    AppMode::Sprite => self.sprite_editor.handle_key(code, vm),
                    AppMode::Map => self.map_editor.handle_key(code, vm),
                    AppMode::Palette => self.palette_editor.handle_key(code, vm),
                    AppMode::Meta => self.meta_editor.handle_key(code, vm),
                    AppMode::Sfx => self.sfx_editor.handle_key(code, vm),
                    AppMode::Music => self.music_editor.handle_key(code, vm),
                    AppMode::Browser => self.browser_editor.handle_key(code, vm),
                    AppMode::Run | AppMode::Code => {}
                }
                self.poll_browser_load();
            }
        }
    }
}
