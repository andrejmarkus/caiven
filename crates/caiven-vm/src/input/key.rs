//! Backend-agnostic keyboard and gamepad button identifiers.
//!
//! `controls.toml` names keys as strings; front-ends (SDL2 today, anything
//! else later) translate their own scancodes into these at the edge, so the
//! VM never depends on a windowing library. The string names are the ones
//! `controls.toml` has always used and are part of the documented format —
//! see the controls section of the repository README.

/// Declares the key enum together with its `controls.toml` spelling, so the
/// name table and the variant list cannot drift apart.
macro_rules! keys {
    ($($variant:ident => $name:literal),+ $(,)?) => {
        /// A physical keyboard key, identified by position rather than by the
        /// character it produces on the current layout.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Key {
            $($variant),+
        }

        impl Key {
            /// Parses a `controls.toml` key name. Unknown names return `None`.
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $($name => Some(Key::$variant),)+
                    _ => None,
                }
            }

            /// The `controls.toml` spelling of this key.
            pub fn name(self) -> &'static str {
                match self {
                    $(Key::$variant => $name,)+
                }
            }

            /// Every key the controls format accepts, in declaration order.
            pub const ALL: &'static [Key] = &[$(Key::$variant),+];
        }
    };
}

keys! {
    ArrowUp => "ArrowUp",
    ArrowDown => "ArrowDown",
    ArrowLeft => "ArrowLeft",
    ArrowRight => "ArrowRight",
    KeyA => "KeyA",
    KeyB => "KeyB",
    KeyC => "KeyC",
    KeyD => "KeyD",
    KeyE => "KeyE",
    KeyF => "KeyF",
    KeyG => "KeyG",
    KeyH => "KeyH",
    KeyI => "KeyI",
    KeyJ => "KeyJ",
    KeyK => "KeyK",
    KeyL => "KeyL",
    KeyM => "KeyM",
    KeyN => "KeyN",
    KeyO => "KeyO",
    KeyP => "KeyP",
    KeyQ => "KeyQ",
    KeyR => "KeyR",
    KeyS => "KeyS",
    KeyT => "KeyT",
    KeyU => "KeyU",
    KeyV => "KeyV",
    KeyW => "KeyW",
    KeyX => "KeyX",
    KeyY => "KeyY",
    KeyZ => "KeyZ",
    Digit0 => "Digit0",
    Digit1 => "Digit1",
    Digit2 => "Digit2",
    Digit3 => "Digit3",
    Digit4 => "Digit4",
    Digit5 => "Digit5",
    Digit6 => "Digit6",
    Digit7 => "Digit7",
    Digit8 => "Digit8",
    Digit9 => "Digit9",
    Space => "Space",
    Enter => "Enter",
    Escape => "Escape",
    Backspace => "Backspace",
    Tab => "Tab",
    ShiftLeft => "ShiftLeft",
    ShiftRight => "ShiftRight",
    ControlLeft => "ControlLeft",
    ControlRight => "ControlRight",
    AltLeft => "AltLeft",
    AltRight => "AltRight",
}

/// Declares the gamepad button enum alongside its `controls.toml` spelling.
/// Names follow SDL's game-controller vocabulary so a device's face buttons
/// mean the same thing on every handheld.
macro_rules! pad_buttons {
    ($($variant:ident => $name:literal),+ $(,)?) => {
        /// A button on an abstract game controller.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum PadButton {
            $($variant),+
        }

        impl PadButton {
            /// Parses a `controls.toml` gamepad button name.
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $($name => Some(PadButton::$variant),)+
                    _ => None,
                }
            }

            /// The `controls.toml` spelling of this button.
            pub fn name(self) -> &'static str {
                match self {
                    $(PadButton::$variant => $name,)+
                }
            }

            /// Every gamepad button the controls format accepts.
            pub const ALL: &'static [PadButton] = &[$(PadButton::$variant),+];
        }
    };
}

pad_buttons! {
    DPadUp => "DPadUp",
    DPadDown => "DPadDown",
    DPadLeft => "DPadLeft",
    DPadRight => "DPadRight",
    South => "South",
    East => "East",
    West => "West",
    North => "North",
    LeftShoulder => "LeftShoulder",
    RightShoulder => "RightShoulder",
    Start => "Start",
    Back => "Back",
    Guide => "Guide",
}

#[cfg(test)]
mod tests {
    use super::{Key, PadButton};

    #[test]
    fn every_key_name_round_trips() {
        for key in Key::ALL {
            assert_eq!(Key::from_name(key.name()), Some(*key));
        }
    }

    #[test]
    fn every_pad_button_name_round_trips() {
        for button in PadButton::ALL {
            assert_eq!(PadButton::from_name(button.name()), Some(*button));
        }
    }

    #[test]
    fn documented_default_key_names_still_parse() {
        // The defaults shipped in controls.toml. Users have these on disk;
        // dropping any of them would silently unbind their controls.
        for name in [
            "ArrowUp",
            "ArrowDown",
            "ArrowLeft",
            "ArrowRight",
            "KeyW",
            "KeyA",
            "KeyS",
            "KeyD",
            "KeyJ",
            "KeyK",
        ] {
            assert!(Key::from_name(name).is_some(), "{name} should parse");
        }
    }

    #[test]
    fn unknown_names_are_rejected() {
        assert_eq!(Key::from_name("KeyÄ"), None);
        assert_eq!(Key::from_name(""), None);
        assert_eq!(PadButton::from_name("Triangle"), None);
    }
}
