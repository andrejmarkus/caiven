use std::collections::HashMap;

use log::warn;
use serde::Deserialize;

use crate::input::{Button, Key, PadButton};

#[derive(Deserialize)]
struct ControlsFile {
    #[serde(default)]
    controls: ControlsSection,
    #[serde(default)]
    gamepad: GamepadSection,
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

/// Optional `[gamepad]` table. Absent in every `controls.toml` written before
/// gamepad support existed, so each field falls back to the fixed mapping.
#[derive(Deserialize)]
struct GamepadSection {
    #[serde(default = "default_pad_up")]
    up: Vec<String>,
    #[serde(default = "default_pad_down")]
    down: Vec<String>,
    #[serde(default = "default_pad_left")]
    left: Vec<String>,
    #[serde(default = "default_pad_right")]
    right: Vec<String>,
    #[serde(default = "default_pad_a")]
    a: Vec<String>,
    #[serde(default = "default_pad_b")]
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

fn default_pad_up() -> Vec<String> {
    vec!["DPadUp".into()]
}
fn default_pad_down() -> Vec<String> {
    vec!["DPadDown".into()]
}
fn default_pad_left() -> Vec<String> {
    vec!["DPadLeft".into()]
}
fn default_pad_right() -> Vec<String> {
    vec!["DPadRight".into()]
}
fn default_pad_a() -> Vec<String> {
    vec!["South".into()]
}
fn default_pad_b() -> Vec<String> {
    vec!["East".into()]
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

impl Default for GamepadSection {
    fn default() -> Self {
        Self {
            up: default_pad_up(),
            down: default_pad_down(),
            left: default_pad_left(),
            right: default_pad_right(),
            a: default_pad_a(),
            b: default_pad_b(),
        }
    }
}

pub struct InputMap {
    map: HashMap<Key, Button>,
    pad: HashMap<PadButton, Button>,
}

impl Default for InputMap {
    fn default() -> Self {
        Self::from_controls(ControlsSection::default(), GamepadSection::default())
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
        Self::from_controls(file.controls, file.gamepad)
    }

    pub fn get_button(&self, key: Key) -> Option<Button> {
        self.map.get(&key).copied()
    }

    pub fn get_pad_button(&self, button: PadButton) -> Option<Button> {
        self.pad.get(&button).copied()
    }

    fn from_controls(controls: ControlsSection, gamepad: GamepadSection) -> Self {
        let mut map: HashMap<Key, Button> = HashMap::new();
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
                if let Some(key) = Key::from_name(name) {
                    map.insert(key, button);
                } else {
                    warn!("unknown key name in controls: {name}");
                }
            }
        }

        let mut pad: HashMap<PadButton, Button> = HashMap::new();
        let pad_bindings = [
            (&gamepad.up, Button::Up),
            (&gamepad.down, Button::Down),
            (&gamepad.left, Button::Left),
            (&gamepad.right, Button::Right),
            (&gamepad.a, Button::A),
            (&gamepad.b, Button::B),
        ];
        for (names, button) in pad_bindings {
            for name in names {
                if let Some(pad_button) = PadButton::from_name(name) {
                    pad.insert(pad_button, button);
                } else {
                    warn!("unknown gamepad button in controls: {name}");
                }
            }
        }

        Self { map, pad }
    }

    #[cfg(test)]
    fn parse_str(content: &str) -> Self {
        let file: ControlsFile = toml::from_str(content).expect("test controls should parse");
        Self::from_controls(file.controls, file.gamepad)
    }
}

#[cfg(test)]
mod tests {
    use super::InputMap;
    use crate::input::{Button, Key, PadButton};

    #[test]
    fn defaults_bind_the_documented_keys() {
        let map = InputMap::default();
        assert_eq!(map.get_button(Key::ArrowUp), Some(Button::Up));
        assert_eq!(map.get_button(Key::KeyW), Some(Button::Up));
        assert_eq!(map.get_button(Key::KeyJ), Some(Button::A));
        assert_eq!(map.get_button(Key::KeyK), Some(Button::B));
        assert_eq!(map.get_button(Key::Escape), None);
    }

    #[test]
    fn defaults_bind_the_fixed_gamepad_mapping() {
        let map = InputMap::default();
        assert_eq!(map.get_pad_button(PadButton::DPadUp), Some(Button::Up));
        assert_eq!(map.get_pad_button(PadButton::South), Some(Button::A));
        assert_eq!(map.get_pad_button(PadButton::East), Some(Button::B));
        assert_eq!(map.get_pad_button(PadButton::North), None);
    }

    #[test]
    fn explicit_controls_override_defaults() {
        let map = InputMap::parse_str(
            r#"
            [controls]
            up = ["KeyE"]
            a = ["Space"]
            "#,
        );
        assert_eq!(map.get_button(Key::KeyE), Some(Button::Up));
        assert_eq!(map.get_button(Key::Space), Some(Button::A));
        // Unlisted entries keep their documented defaults.
        assert_eq!(map.get_button(Key::KeyK), Some(Button::B));
        assert_eq!(map.get_button(Key::ArrowUp), None);
    }

    #[test]
    fn controls_file_without_gamepad_table_keeps_pad_defaults() {
        let map = InputMap::parse_str(
            r#"
            [controls]
            up = ["KeyE"]
            "#,
        );
        assert_eq!(map.get_pad_button(PadButton::DPadUp), Some(Button::Up));
        assert_eq!(map.get_pad_button(PadButton::South), Some(Button::A));
    }

    #[test]
    fn gamepad_table_overrides_pad_defaults() {
        let map = InputMap::parse_str(
            r#"
            [gamepad]
            a = ["East"]
            b = ["South"]
            "#,
        );
        assert_eq!(map.get_pad_button(PadButton::East), Some(Button::A));
        assert_eq!(map.get_pad_button(PadButton::South), Some(Button::B));
    }

    #[test]
    fn unknown_names_are_dropped_without_breaking_the_rest() {
        let map = InputMap::parse_str(
            r#"
            [controls]
            up = ["NotAKey", "KeyE"]
            "#,
        );
        assert_eq!(map.get_button(Key::KeyE), Some(Button::Up));
    }

    #[test]
    fn missing_file_falls_back_to_defaults() {
        let map = InputMap::load("definitely-not-a-real-controls-file.toml");
        assert_eq!(map.get_button(Key::ArrowUp), Some(Button::Up));
    }
}
