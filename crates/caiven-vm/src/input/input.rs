use crate::input::button::Button;

const BUTTON_COUNT: usize = Button::ALL.len();

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

    /// True only on the first frame the button reads as released (edge
    /// trigger) — mirror of `just_pressed` for the opposite edge.
    pub fn just_released(&self, button: Button) -> bool {
        !self.cur[button as usize] && self.prev[button as usize]
    }

    pub fn set_button(&mut self, button: Button, pressed: bool) {
        self.cur[button as usize] = pressed;
    }

    /// Latches current state as previous; call after each completed frame.
    pub fn end_frame(&mut self) {
        self.prev = self.cur;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::button::Button;

    #[test]
    fn just_released_true_only_on_the_frame_after_release() {
        let mut input = Input::new();

        input.set_button(Button::A, true);
        input.end_frame();
        assert!(!input.just_released(Button::A), "still held: not released");

        input.set_button(Button::A, false);
        // Not yet latched: just_released reads prev vs cur, and end_frame
        // hasn't run yet this "frame" so prev is still true, cur is false.
        assert!(
            input.just_released(Button::A),
            "cur=false, prev=true: this is the release edge"
        );

        input.end_frame();
        assert!(
            !input.just_released(Button::A),
            "prev now latched to false: no longer the release edge"
        );
    }

    #[test]
    fn just_released_false_when_never_pressed() {
        let input = Input::new();
        assert!(!input.just_released(Button::A));
    }
}
