#[repr(u8)]
pub enum Button {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
    A = 4,
    B = 5,
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
            _ => None,
        }
    }
}
