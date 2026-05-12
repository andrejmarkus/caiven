use std::collections::HashMap;

use log::warn;
use serde::Deserialize;
use winit::keyboard::KeyCode;

use crate::input::Button;

#[derive(Deserialize)]
struct ControlsFile {
    #[serde(default)]
    controls: ControlsSection,
}

#[derive(Deserialize)]
struct ControlsSection {
    #[serde(default = "default_up")]
    up: Vec<String>,
    #[serde(default = "default_down")]
    down: Vec<String>,
    #[serde(default = "default_left")]
    left: Vec<String>,
    #[serde(default = "default_right")]
    right: Vec<String>,
    #[serde(default = "default_a")]
    a: Vec<String>,
    #[serde(default = "default_b")]
    b: Vec<String>,
}

fn default_up() -> Vec<String> {
    vec!["ArrowUp".into(), "KeyW".into()]
}
fn default_down() -> Vec<String> {
    vec!["ArrowDown".into(), "KeyS".into()]
}
fn default_left() -> Vec<String> {
    vec!["ArrowLeft".into(), "KeyA".into()]
}
fn default_right() -> Vec<String> {
    vec!["ArrowRight".into(), "KeyD".into()]
}
fn default_a() -> Vec<String> {
    vec!["KeyJ".into()]
}
fn default_b() -> Vec<String> {
    vec!["KeyK".into()]
}

impl Default for ControlsSection {
    fn default() -> Self {
        Self {
            up: default_up(),
            down: default_down(),
            left: default_left(),
            right: default_right(),
            a: default_a(),
            b: default_b(),
        }
    }
}

pub struct InputMap {
    map: HashMap<KeyCode, Button>,
}

impl Default for InputMap {
    fn default() -> Self {
        Self::from_controls(ControlsSection::default())
    }
}

impl InputMap {
    pub fn load(path: &str) -> Self {
        let content = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => return Self::default(),
        };
        let file: ControlsFile = match toml::from_str(&content) {
            Ok(f) => f,
            Err(e) => {
                warn!("failed to parse {path}: {e}");
                return Self::default();
            }
        };
        Self::from_controls(file.controls)
    }

    pub fn get_button(&self, key: KeyCode) -> Option<Button> {
        self.map.get(&key).copied()
    }

    fn from_controls(controls: ControlsSection) -> Self {
        let mut map: HashMap<KeyCode, Button> = HashMap::new();
        let bindings = [
            (&controls.up, Button::Up),
            (&controls.down, Button::Down),
            (&controls.left, Button::Left),
            (&controls.right, Button::Right),
            (&controls.a, Button::A),
            (&controls.b, Button::B),
        ];
        for (keys, button) in bindings {
            for name in keys {
                if let Some(kc) = parse_keycode(name) {
                    map.insert(kc, button);
                } else {
                    warn!("unknown key name in controls: {name}");
                }
            }
        }
        Self { map }
    }
}

fn parse_keycode(name: &str) -> Option<KeyCode> {
    match name {
        "ArrowUp" => Some(KeyCode::ArrowUp),
        "ArrowDown" => Some(KeyCode::ArrowDown),
        "ArrowLeft" => Some(KeyCode::ArrowLeft),
        "ArrowRight" => Some(KeyCode::ArrowRight),
        "KeyA" => Some(KeyCode::KeyA),
        "KeyB" => Some(KeyCode::KeyB),
        "KeyC" => Some(KeyCode::KeyC),
        "KeyD" => Some(KeyCode::KeyD),
        "KeyE" => Some(KeyCode::KeyE),
        "KeyF" => Some(KeyCode::KeyF),
        "KeyG" => Some(KeyCode::KeyG),
        "KeyH" => Some(KeyCode::KeyH),
        "KeyI" => Some(KeyCode::KeyI),
        "KeyJ" => Some(KeyCode::KeyJ),
        "KeyK" => Some(KeyCode::KeyK),
        "KeyL" => Some(KeyCode::KeyL),
        "KeyM" => Some(KeyCode::KeyM),
        "KeyN" => Some(KeyCode::KeyN),
        "KeyO" => Some(KeyCode::KeyO),
        "KeyP" => Some(KeyCode::KeyP),
        "KeyQ" => Some(KeyCode::KeyQ),
        "KeyR" => Some(KeyCode::KeyR),
        "KeyS" => Some(KeyCode::KeyS),
        "KeyT" => Some(KeyCode::KeyT),
        "KeyU" => Some(KeyCode::KeyU),
        "KeyV" => Some(KeyCode::KeyV),
        "KeyW" => Some(KeyCode::KeyW),
        "KeyX" => Some(KeyCode::KeyX),
        "KeyY" => Some(KeyCode::KeyY),
        "KeyZ" => Some(KeyCode::KeyZ),
        "Digit0" => Some(KeyCode::Digit0),
        "Digit1" => Some(KeyCode::Digit1),
        "Digit2" => Some(KeyCode::Digit2),
        "Digit3" => Some(KeyCode::Digit3),
        "Digit4" => Some(KeyCode::Digit4),
        "Digit5" => Some(KeyCode::Digit5),
        "Digit6" => Some(KeyCode::Digit6),
        "Digit7" => Some(KeyCode::Digit7),
        "Digit8" => Some(KeyCode::Digit8),
        "Digit9" => Some(KeyCode::Digit9),
        "Space" => Some(KeyCode::Space),
        "Enter" => Some(KeyCode::Enter),
        "Escape" => Some(KeyCode::Escape),
        "Backspace" => Some(KeyCode::Backspace),
        "Tab" => Some(KeyCode::Tab),
        "ShiftLeft" => Some(KeyCode::ShiftLeft),
        "ShiftRight" => Some(KeyCode::ShiftRight),
        "ControlLeft" => Some(KeyCode::ControlLeft),
        "ControlRight" => Some(KeyCode::ControlRight),
        "AltLeft" => Some(KeyCode::AltLeft),
        "AltRight" => Some(KeyCode::AltRight),
        _ => None,
    }
}
