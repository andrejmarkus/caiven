//! Keyboard routing for [`App`]: game input mapping, global shortcuts and
//! debugger controls.

use super::App;
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
            match code {
                KeyCode::Space if pressed && !event.repeat => {
                    self.debugger.toggle_pause();
                }
                KeyCode::KeyC if pressed && !event.repeat => {
                    self.debugger.step();
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
