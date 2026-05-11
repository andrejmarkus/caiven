use crate::input::button::Button;

#[derive(Default, Clone, Copy)]
pub struct Input {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub a: bool,
    pub b: bool,
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_pressed(&self, button: Button) -> bool {
        match button {
            Button::Up => self.up,
            Button::Down => self.down,
            Button::Left => self.left,
            Button::Right => self.right,
            Button::A => self.a,
            Button::B => self.b,
        }
    }
}
