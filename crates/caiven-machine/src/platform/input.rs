//! Translating SDL input events into the VM's backend-agnostic identifiers.

use caiven_vm::input::{Key, PadButton};
use sdl2::controller::{Button as SdlButton, GameController};
use sdl2::keyboard::Scancode;

/// Maps an SDL physical scancode to a `controls.toml` key.
///
/// Scancodes are positional, matching the VM's `Key` — a key is identified
/// by where it sits, not by the character the current layout produces.
/// Unmapped keys return `None` and are ignored.
pub fn key_from_scancode(scancode: Scancode) -> Option<Key> {
    Some(match scancode {
        Scancode::Up => Key::ArrowUp,
        Scancode::Down => Key::ArrowDown,
        Scancode::Left => Key::ArrowLeft,
        Scancode::Right => Key::ArrowRight,
        Scancode::A => Key::KeyA,
        Scancode::B => Key::KeyB,
        Scancode::C => Key::KeyC,
        Scancode::D => Key::KeyD,
        Scancode::E => Key::KeyE,
        Scancode::F => Key::KeyF,
        Scancode::G => Key::KeyG,
        Scancode::H => Key::KeyH,
        Scancode::I => Key::KeyI,
        Scancode::J => Key::KeyJ,
        Scancode::K => Key::KeyK,
        Scancode::L => Key::KeyL,
        Scancode::M => Key::KeyM,
        Scancode::N => Key::KeyN,
        Scancode::O => Key::KeyO,
        Scancode::P => Key::KeyP,
        Scancode::Q => Key::KeyQ,
        Scancode::R => Key::KeyR,
        Scancode::S => Key::KeyS,
        Scancode::T => Key::KeyT,
        Scancode::U => Key::KeyU,
        Scancode::V => Key::KeyV,
        Scancode::W => Key::KeyW,
        Scancode::X => Key::KeyX,
        Scancode::Y => Key::KeyY,
        Scancode::Z => Key::KeyZ,
        Scancode::Num0 => Key::Digit0,
        Scancode::Num1 => Key::Digit1,
        Scancode::Num2 => Key::Digit2,
        Scancode::Num3 => Key::Digit3,
        Scancode::Num4 => Key::Digit4,
        Scancode::Num5 => Key::Digit5,
        Scancode::Num6 => Key::Digit6,
        Scancode::Num7 => Key::Digit7,
        Scancode::Num8 => Key::Digit8,
        Scancode::Num9 => Key::Digit9,
        Scancode::Space => Key::Space,
        Scancode::Return => Key::Enter,
        Scancode::Escape => Key::Escape,
        Scancode::Backspace => Key::Backspace,
        Scancode::Tab => Key::Tab,
        Scancode::LShift => Key::ShiftLeft,
        Scancode::RShift => Key::ShiftRight,
        Scancode::LCtrl => Key::ControlLeft,
        Scancode::RCtrl => Key::ControlRight,
        Scancode::LAlt => Key::AltLeft,
        Scancode::RAlt => Key::AltRight,
        _ => return None,
    })
}

/// Maps an SDL game-controller button to a `controls.toml` gamepad button.
///
/// SDL normalises every pad to this layout, which is why a handheld's
/// physical D-pad and face buttons arrive here already labelled.
pub fn pad_button_from_sdl(button: SdlButton) -> Option<PadButton> {
    Some(match button {
        SdlButton::DPadUp => PadButton::DPadUp,
        SdlButton::DPadDown => PadButton::DPadDown,
        SdlButton::DPadLeft => PadButton::DPadLeft,
        SdlButton::DPadRight => PadButton::DPadRight,
        SdlButton::A => PadButton::South,
        SdlButton::B => PadButton::East,
        SdlButton::X => PadButton::West,
        SdlButton::Y => PadButton::North,
        SdlButton::LeftShoulder => PadButton::LeftShoulder,
        SdlButton::RightShoulder => PadButton::RightShoulder,
        SdlButton::Start => PadButton::Start,
        SdlButton::Back => PadButton::Back,
        SdlButton::Guide => PadButton::Guide,
        _ => return None,
    })
}

/// Keeps the first attached game controller open.
///
/// SDL only delivers controller events while the device is open, so the
/// handle has to be held for the lifetime of the session. Handhelds expose
/// their built-in D-pad and face buttons as a controller, so this is the
/// path that matters on device — not the keyboard one.
#[derive(Default)]
pub struct Gamepads {
    /// The open controller, if any. Only one is tracked: the console has a
    /// single player.
    controller: Option<GameController>,
}

impl Gamepads {
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens `index` if no controller is currently held.
    pub fn open(&mut self, subsystem: &sdl2::GameControllerSubsystem, index: u32) {
        if self.controller.is_some() {
            return;
        }
        match subsystem.open(index) {
            Ok(controller) => {
                log::info!("gamepad connected: {}", controller.name());
                self.controller = Some(controller);
            }
            Err(e) => log::warn!("failed to open gamepad {index}: {e}"),
        }
    }

    /// Drops the held controller when its instance id disconnects.
    pub fn close(&mut self, instance_id: u32) {
        let matches = self
            .controller
            .as_ref()
            .is_some_and(|c| c.instance_id() == instance_id);
        if matches {
            log::info!("gamepad disconnected");
            self.controller = None;
        }
    }

    /// Opens whichever controllers are already attached at startup.
    pub fn open_attached(&mut self, subsystem: &sdl2::GameControllerSubsystem) {
        let count = match subsystem.num_joysticks() {
            Ok(n) => n,
            Err(e) => {
                log::warn!("failed to enumerate joysticks: {e}");
                return;
            }
        };
        for index in 0..count {
            if subsystem.is_game_controller(index) {
                self.open(subsystem, index);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{key_from_scancode, pad_button_from_sdl};
    use caiven_vm::input::{Key, PadButton};
    use sdl2::controller::Button as SdlButton;
    use sdl2::keyboard::Scancode;

    #[test]
    fn default_bound_keys_all_translate() {
        // Every key controls.toml binds by default must survive the trip
        // from SDL, or the shipped defaults silently do nothing.
        assert_eq!(key_from_scancode(Scancode::Up), Some(Key::ArrowUp));
        assert_eq!(key_from_scancode(Scancode::Down), Some(Key::ArrowDown));
        assert_eq!(key_from_scancode(Scancode::Left), Some(Key::ArrowLeft));
        assert_eq!(key_from_scancode(Scancode::Right), Some(Key::ArrowRight));
        assert_eq!(key_from_scancode(Scancode::W), Some(Key::KeyW));
        assert_eq!(key_from_scancode(Scancode::A), Some(Key::KeyA));
        assert_eq!(key_from_scancode(Scancode::S), Some(Key::KeyS));
        assert_eq!(key_from_scancode(Scancode::D), Some(Key::KeyD));
        assert_eq!(key_from_scancode(Scancode::J), Some(Key::KeyJ));
        assert_eq!(key_from_scancode(Scancode::K), Some(Key::KeyK));
    }

    #[test]
    fn ctrl_r_reload_keys_translate() {
        assert_eq!(key_from_scancode(Scancode::LCtrl), Some(Key::ControlLeft));
        assert_eq!(key_from_scancode(Scancode::RCtrl), Some(Key::ControlRight));
        assert_eq!(key_from_scancode(Scancode::R), Some(Key::KeyR));
    }

    #[test]
    fn unmapped_keys_are_ignored() {
        assert_eq!(key_from_scancode(Scancode::F1), None);
        assert_eq!(key_from_scancode(Scancode::PrintScreen), None);
    }

    #[test]
    fn default_pad_buttons_translate_to_the_six_console_buttons() {
        assert_eq!(
            pad_button_from_sdl(SdlButton::DPadUp),
            Some(PadButton::DPadUp)
        );
        assert_eq!(pad_button_from_sdl(SdlButton::A), Some(PadButton::South));
        assert_eq!(pad_button_from_sdl(SdlButton::B), Some(PadButton::East));
    }
}
