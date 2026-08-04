use std::collections::HashMap;
use std::path::Path;

use log::warn;
use serde::{Deserialize, Serialize};

use crate::input::{Button, Key, PadButton, SystemButton};

/// The whole `controls.toml` document: what's on disk, and what the remap
/// screen edits in memory before writing it back (SPEC V40).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ControlsFile {
    #[serde(default)]
    pub controls: ControlsSection,
    #[serde(default)]
    pub gamepad: GamepadSection,
}

impl ControlsFile {
    /// Reads `controls.toml`, falling back to the documented defaults on a
    /// missing or corrupt file — the same tolerance `InputMap::load` has
    /// always had.
    pub fn load(path: &Path) -> Self {
        let Ok(content) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        match toml::from_str(&content) {
            Ok(file) => file,
            Err(e) => {
                warn!("failed to parse {}: {e}", path.display());
                Self::default()
            }
        }
    }

    /// Writes this document back to `path` in the same `[controls]`/
    /// `[gamepad]` shape it was read in (SPEC V30, V40).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, content)
    }

    /// Builds the runtime lookup table this document resolves to.
    pub fn to_input_map(&self) -> InputMap {
        InputMap::from_controls(self.controls.clone(), self.gamepad.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlsSection {
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
    /// Added after the original six. Absent in files written before SELECT
    /// existed, so it falls back like everything else.
    #[serde(default = "default_select")]
    select: Vec<String>,
    /// START never reaches a cartridge — it opens the pause menu.
    #[serde(default = "default_start")]
    start: Vec<String>,
}

/// Optional `[gamepad]` table. Absent in every `controls.toml` written before
/// gamepad support existed, so each field falls back to the fixed mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamepadSection {
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
    #[serde(default = "default_pad_select")]
    select: Vec<String>,
    #[serde(default = "default_pad_start")]
    start: Vec<String>,
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
fn default_select() -> Vec<String> {
    vec!["ShiftLeft".into(), "ShiftRight".into()]
}
fn default_start() -> Vec<String> {
    vec!["Enter".into()]
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
fn default_pad_select() -> Vec<String> {
    vec!["Back".into()]
}
fn default_pad_start() -> Vec<String> {
    vec!["Start".into()]
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
            select: default_select(),
            start: default_start(),
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
            select: default_pad_select(),
            start: default_pad_start(),
        }
    }
}

impl ControlsSection {
    /// The key names currently bound to `button`. The remap screen displays
    /// these; `Start` has no `Button` binding here (it lives in its own
    /// `start` field) so it always reads empty.
    pub fn names(&self, button: Button) -> &[String] {
        match button {
            Button::Up => &self.up,
            Button::Down => &self.down,
            Button::Left => &self.left,
            Button::Right => &self.right,
            Button::A => &self.a,
            Button::B => &self.b,
            Button::Select => &self.select,
        }
    }

    /// Replaces the binding for `button` with exactly `names` — a remap
    /// capture always sets a single fresh binding rather than appending to
    /// the old one.
    pub fn set(&mut self, button: Button, names: Vec<String>) {
        *match button {
            Button::Up => &mut self.up,
            Button::Down => &mut self.down,
            Button::Left => &mut self.left,
            Button::Right => &mut self.right,
            Button::A => &mut self.a,
            Button::B => &mut self.b,
            Button::Select => &mut self.select,
        } = names;
    }
}

impl GamepadSection {
    /// The gamepad button names currently bound to `button`.
    pub fn names(&self, button: Button) -> &[String] {
        match button {
            Button::Up => &self.up,
            Button::Down => &self.down,
            Button::Left => &self.left,
            Button::Right => &self.right,
            Button::A => &self.a,
            Button::B => &self.b,
            Button::Select => &self.select,
        }
    }

    /// Replaces the gamepad binding for `button` with exactly `names`.
    pub fn set(&mut self, button: Button, names: Vec<String>) {
        *match button {
            Button::Up => &mut self.up,
            Button::Down => &mut self.down,
            Button::Left => &mut self.left,
            Button::Right => &mut self.right,
            Button::A => &mut self.a,
            Button::B => &mut self.b,
            Button::Select => &mut self.select,
        } = names;
    }
}

pub struct InputMap {
    map: HashMap<Key, Button>,
    pad: HashMap<PadButton, Button>,
    system: HashMap<Key, SystemButton>,
    pad_system: HashMap<PadButton, SystemButton>,
}

impl Default for InputMap {
    fn default() -> Self {
        Self::from_controls(ControlsSection::default(), GamepadSection::default())
    }
}

impl InputMap {
    pub fn load(path: &str) -> Self {
        ControlsFile::load(Path::new(path)).to_input_map()
    }

    pub fn get_button(&self, key: Key) -> Option<Button> {
        self.map.get(&key).copied()
    }

    pub fn get_pad_button(&self, button: PadButton) -> Option<Button> {
        self.pad.get(&button).copied()
    }

    /// The host-reserved button this key drives, if any. Never a cart button.
    pub fn get_system_button(&self, key: Key) -> Option<SystemButton> {
        self.system.get(&key).copied()
    }

    /// The host-reserved button this gamepad button drives, if any.
    pub fn get_pad_system_button(&self, button: PadButton) -> Option<SystemButton> {
        self.pad_system.get(&button).copied()
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
            (&controls.select, Button::Select),
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

        let mut system: HashMap<Key, SystemButton> = HashMap::new();
        for name in &controls.start {
            match Key::from_name(name) {
                Some(key) => {
                    system.insert(key, SystemButton::Start);
                }
                None => warn!("unknown key name in controls: {name}"),
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
            (&gamepad.select, Button::Select),
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

        let mut pad_system: HashMap<PadButton, SystemButton> = HashMap::new();
        for name in &gamepad.start {
            match PadButton::from_name(name) {
                Some(pad_button) => {
                    pad_system.insert(pad_button, SystemButton::Start);
                }
                None => warn!("unknown gamepad button in controls: {name}"),
            }
        }

        // A binding listed under both a cart button and START belongs to
        // START. Otherwise a cart could hold the pause menu hostage by
        // shipping a controls file that claims the key.
        for key in system.keys() {
            if map.remove(key).is_some() {
                warn!(
                    "{} is bound to START; the cart binding is ignored",
                    key.name()
                );
            }
        }
        for pad_button in pad_system.keys() {
            if pad.remove(pad_button).is_some() {
                warn!(
                    "{} is bound to START; the cart binding is ignored",
                    pad_button.name()
                );
            }
        }

        Self {
            map,
            pad,
            system,
            pad_system,
        }
    }

    #[cfg(test)]
    fn parse_str(content: &str) -> Self {
        let file: ControlsFile = toml::from_str(content).expect("test controls should parse");
        Self::from_controls(file.controls, file.gamepad)
    }
}

#[cfg(test)]
mod tests {
    use super::{ControlsFile, InputMap};
    use crate::input::{Button, Key, PadButton, SystemButton};

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
    fn defaults_bind_select_and_start() {
        let map = InputMap::default();
        assert_eq!(map.get_button(Key::ShiftLeft), Some(Button::Select));
        assert_eq!(map.get_button(Key::ShiftRight), Some(Button::Select));
        assert_eq!(map.get_pad_button(PadButton::Back), Some(Button::Select));

        assert_eq!(map.get_system_button(Key::Enter), Some(SystemButton::Start));
        assert_eq!(
            map.get_pad_system_button(PadButton::Start),
            Some(SystemButton::Start)
        );
        // START is host-only: it must never arrive as a cart button.
        assert_eq!(map.get_button(Key::Enter), None);
        assert_eq!(map.get_pad_button(PadButton::Start), None);
    }

    #[test]
    fn a_controls_file_from_before_select_existed_still_gets_both() {
        // Every file on a user's disk predates these two fields.
        let map = InputMap::parse_str(
            r#"
            [controls]
            up    = ["ArrowUp", "KeyW"]
            down  = ["ArrowDown", "KeyS"]
            left  = ["ArrowLeft", "KeyA"]
            right = ["ArrowRight", "KeyD"]
            a     = ["KeyJ"]
            b     = ["KeyK"]
            "#,
        );
        assert_eq!(map.get_button(Key::KeyJ), Some(Button::A));
        assert_eq!(map.get_button(Key::ShiftLeft), Some(Button::Select));
        assert_eq!(map.get_system_button(Key::Enter), Some(SystemButton::Start));
    }

    #[test]
    fn start_wins_when_a_binding_claims_the_same_input_twice() {
        let map = InputMap::parse_str(
            r#"
            [controls]
            a     = ["Enter"]
            start = ["Enter"]

            [gamepad]
            a     = ["Start"]
            start = ["Start"]
            "#,
        );
        assert_eq!(map.get_system_button(Key::Enter), Some(SystemButton::Start));
        assert_eq!(map.get_button(Key::Enter), None);
        assert_eq!(
            map.get_pad_system_button(PadButton::Start),
            Some(SystemButton::Start)
        );
        assert_eq!(map.get_pad_button(PadButton::Start), None);
    }

    #[test]
    fn start_can_be_rebound_off_its_default() {
        let map = InputMap::parse_str(
            r#"
            [controls]
            start = ["Escape"]
            "#,
        );
        assert_eq!(
            map.get_system_button(Key::Escape),
            Some(SystemButton::Start)
        );
        assert_eq!(map.get_system_button(Key::Enter), None);
    }

    #[test]
    fn missing_file_falls_back_to_defaults() {
        let map = InputMap::load("definitely-not-a-real-controls-file.toml");
        assert_eq!(map.get_button(Key::ArrowUp), Some(Button::Up));
    }

    #[test]
    fn a_remap_round_trips_through_disk_unchanged() {
        // SPEC V40/V30: what the remap screen writes must read back
        // identically, key names rather than raw codes, and every other
        // binding untouched.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("controls.toml");

        let mut file = ControlsFile::load(&path); // no file yet: defaults
        assert_eq!(file.controls.names(Button::Up), ["ArrowUp", "KeyW"]);
        file.controls.set(Button::Up, vec!["KeyE".to_string()]);
        file.gamepad.set(Button::A, vec!["West".to_string()]);
        file.save(&path).expect("save controls.toml");

        let reloaded = ControlsFile::load(&path);
        assert_eq!(reloaded.controls.names(Button::Up), ["KeyE"]);
        assert_eq!(reloaded.gamepad.names(Button::A), ["West"]);
        // Untouched bindings kept their values, not just their defaults.
        assert_eq!(reloaded.controls.names(Button::B), ["KeyK"]);
        assert_eq!(reloaded.gamepad.names(Button::B), ["East"]);

        let map = reloaded.to_input_map();
        assert_eq!(map.get_button(Key::KeyE), Some(Button::Up));
        assert_eq!(
            map.get_button(Key::ArrowUp),
            None,
            "the old binding is gone"
        );
        assert_eq!(map.get_pad_button(PadButton::West), Some(Button::A));
    }

    #[test]
    fn set_replaces_rather_than_appends() {
        let mut section = super::ControlsSection::default();
        section.set(Button::A, vec!["Space".to_string()]);
        assert_eq!(section.names(Button::A), ["Space"]);
    }
}
