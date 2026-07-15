//! Keyboard routing for [`App`]: game input mapping, global shortcuts and
//! debugger controls.

use super::App;
use crate::debugger::DebugMode;
use winit::event::KeyEvent;
use winit::keyboard::{KeyCode, PhysicalKey};

impl App {
    pub(super) fn handle_keyboard(&mut self, event: KeyEvent) {
        let pressed = event.state.is_pressed();

        if let PhysicalKey::Code(code) = event.physical_key {
            if let Some(button) = self.core.input_map.get_button(code) {
                self.core.input.set_button(button, pressed);
            }

            let ctrl = self.modifiers.state().control_key();

            if pressed && !event.repeat && ctrl && code == KeyCode::KeyS {
                self.save_cart();
                return;
            }

            // Debugger controls
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
        }
    }
}
