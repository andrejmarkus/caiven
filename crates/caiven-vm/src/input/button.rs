/// A button a cartridge can read.
///
/// The console is a six-direction-and-two-face-buttons machine plus SELECT.
/// START is deliberately absent: it is how a player reaches the Machine's
/// pause menu, so the shell consumes it before a cart ever sees it, and an
/// index that is always false on device would be an API that lies about the
/// hardware. See [`SystemButton`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
    A = 4,
    B = 5,
    Select = 6,
}

impl Button {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Button::Up),
            1 => Some(Button::Down),
            2 => Some(Button::Left),
            3 => Some(Button::Right),
            4 => Some(Button::A),
            5 => Some(Button::B),
            6 => Some(Button::Select),
            _ => None,
        }
    }

    /// Every cart-visible button, in index order.
    pub const ALL: &'static [Button] = &[
        Button::Up,
        Button::Down,
        Button::Left,
        Button::Right,
        Button::A,
        Button::B,
        Button::Select,
    ];
}

/// A button the host reserves for itself.
///
/// These never reach cartridge Lua. START opens the pause menu, which is the
/// player's only way out of a running cart on a handheld, so a cart must not
/// be able to hold it hostage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemButton {
    Start,
}

impl SystemButton {
    pub const ALL: &'static [SystemButton] = &[SystemButton::Start];
}

#[cfg(test)]
mod tests {
    use super::{Button, SystemButton};

    #[test]
    fn indices_round_trip_in_declaration_order() {
        for (index, button) in Button::ALL.iter().enumerate() {
            assert_eq!(*button as u8, index as u8);
            assert_eq!(Button::from_u8(index as u8), Some(*button));
        }
    }

    #[test]
    fn the_original_six_keep_their_indices() {
        // Cartridges on disk hard-code these numbers.
        assert_eq!(Button::from_u8(0), Some(Button::Up));
        assert_eq!(Button::from_u8(1), Some(Button::Down));
        assert_eq!(Button::from_u8(2), Some(Button::Left));
        assert_eq!(Button::from_u8(3), Some(Button::Right));
        assert_eq!(Button::from_u8(4), Some(Button::A));
        assert_eq!(Button::from_u8(5), Some(Button::B));
    }

    #[test]
    fn out_of_range_indices_stay_unmapped() {
        assert_eq!(Button::from_u8(7), None);
        assert_eq!(Button::from_u8(255), None);
    }

    #[test]
    fn start_is_not_a_cart_button() {
        assert_eq!(SystemButton::ALL.len(), 1);
        assert!(!Button::ALL.iter().any(|b| format!("{b:?}") == "Start"));
    }
}
