use crate::input::button::Button;

const BUTTON_COUNT: usize = 6;

/// Current and previous-frame button state. `end_frame` must be called once
/// per VM frame so `just_pressed` (btnp) can detect edges.
#[derive(Default, Clone, Copy)]
pub struct Input {
    cur: [bool; BUTTON_COUNT],
    prev: [bool; BUTTON_COUNT],
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_pressed(&self, button: Button) -> bool {
        self.cur[button as usize]
    }

    /// True only on the first frame the button is held (edge trigger).
    pub fn just_pressed(&self, button: Button) -> bool {
        self.cur[button as usize] && !self.prev[button as usize]
    }

    pub fn set_button(&mut self, button: Button, pressed: bool) {
        self.cur[button as usize] = pressed;
    }

    /// Latches current state as previous; call after each completed frame.
    pub fn end_frame(&mut self) {
        self.prev = self.cur;
    }
}
