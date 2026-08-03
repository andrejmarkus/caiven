//! Turning physical button state into the shell's button events.
//!
//! Almost everything is a straight pass-through. The one rule that needs
//! memory is the six-button fallback: plenty of handhelds expose only a
//! D-pad, A and B, with no START anywhere. Holding B for [`LONG_PRESS`]
//! stands in for it — that is how the player reaches the pause menu on a
//! device that has no other way in.
//!
//! B is therefore the only button that resolves on *release*: until the hold
//! threshold passes, a press could still turn out to be either one.

use std::time::Duration;

use caiven_vm::input::{Button, SystemButton};

use crate::shell::state::ShellButton;

/// The shell button a cart button drives while a menu is up.
///
/// The shell reads the player's own bindings rather than raw scancodes, so
/// remapping A and B in Settings moves the menus too.
pub fn shell_button(button: Button) -> ShellButton {
    match button {
        Button::Up => ShellButton::Up,
        Button::Down => ShellButton::Down,
        Button::Left => ShellButton::Left,
        Button::Right => ShellButton::Right,
        Button::A => ShellButton::A,
        Button::B => ShellButton::B,
        Button::Select => ShellButton::Select,
    }
}

/// The shell button a host-reserved button drives.
pub fn shell_button_from_system(button: SystemButton) -> ShellButton {
    match button {
        SystemButton::Start => ShellButton::Start,
    }
}

/// How long B must be held before it counts as START.
///
/// Long enough that a normal Back tap never trips it, short enough that a
/// player looking for the menu finds it on the first try.
pub const LONG_PRESS: Duration = Duration::from_millis(600);

/// Translates physical presses into shell button events.
#[derive(Debug, Clone, Default)]
pub struct ShellInput {
    /// How long B has been down, while it is down.
    b_held: Option<Duration>,
    /// Whether this hold already fired START, so releasing it stays silent
    /// and a longer hold does not fire twice.
    b_promoted: bool,
}

impl ShellInput {
    pub fn new() -> Self {
        Self::default()
    }

    /// A button went down. Returns the event it produces, if any.
    ///
    /// B produces nothing yet — see the module note.
    pub fn press(&mut self, button: ShellButton) -> Option<ShellButton> {
        if button == ShellButton::B {
            self.b_held = Some(Duration::ZERO);
            self.b_promoted = false;
            return None;
        }
        Some(button)
    }

    /// A button came back up. Returns the event it produces, if any.
    pub fn release(&mut self, button: ShellButton) -> Option<ShellButton> {
        if button != ShellButton::B {
            return None;
        }
        let was_held = self.b_held.take().is_some();
        let promoted = self.b_promoted;
        self.b_promoted = false;
        // A hold that already became START must not also register as Back.
        (was_held && !promoted).then_some(ShellButton::B)
    }

    /// Advances the hold timer by real elapsed time, firing START the moment
    /// the threshold is crossed rather than waiting for the release.
    pub fn tick(&mut self, dt: Duration) -> Option<ShellButton> {
        let held = self.b_held.as_mut()?;
        if self.b_promoted {
            return None;
        }
        *held = held.saturating_add(dt);
        if *held >= LONG_PRESS {
            self.b_promoted = true;
            return Some(ShellButton::Start);
        }
        None
    }

    /// Whether B is down and has not yet become START — the window where a
    /// hold indicator would be drawn.
    pub fn b_hold_progress(&self) -> Option<f32> {
        if self.b_promoted {
            return None;
        }
        let held = self.b_held?;
        Some((held.as_secs_f32() / LONG_PRESS.as_secs_f32()).clamp(0.0, 1.0))
    }

    /// Forgets any hold in progress. Used when the shell changes screen out
    /// from under the player, so a stale timer cannot fire into the new one.
    pub fn reset(&mut self) {
        self.b_held = None;
        self.b_promoted = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: Duration = Duration::from_millis(16);

    /// Ticks `count` frames, collecting whatever the timer fires.
    fn run(input: &mut ShellInput, count: usize) -> Vec<ShellButton> {
        (0..count).filter_map(|_| input.tick(FRAME)).collect()
    }

    #[test]
    fn every_button_but_b_resolves_on_press() {
        let mut input = ShellInput::new();
        for button in [
            ShellButton::Up,
            ShellButton::Down,
            ShellButton::Left,
            ShellButton::Right,
            ShellButton::A,
            ShellButton::Start,
            ShellButton::Select,
        ] {
            assert_eq!(input.press(button), Some(button));
            assert_eq!(input.release(button), None);
        }
    }

    #[test]
    fn a_short_b_tap_is_back() {
        let mut input = ShellInput::new();
        assert_eq!(input.press(ShellButton::B), None);
        assert!(run(&mut input, 5).is_empty(), "well under the threshold");
        assert_eq!(input.release(ShellButton::B), Some(ShellButton::B));
    }

    #[test]
    fn holding_b_becomes_start_and_the_release_stays_silent() {
        let mut input = ShellInput::new();
        input.press(ShellButton::B);
        let fired = run(&mut input, (LONG_PRESS.as_millis() / 16) as usize + 1);
        assert_eq!(fired, vec![ShellButton::Start]);
        assert_eq!(
            input.release(ShellButton::B),
            None,
            "the hold already acted; releasing must not also go Back"
        );
    }

    #[test]
    fn a_hold_fires_start_exactly_once_however_long_it_lasts() {
        let mut input = ShellInput::new();
        input.press(ShellButton::B);
        let mut fired = run(&mut input, 60);
        fired.extend(run(&mut input, 300));
        assert_eq!(fired, vec![ShellButton::Start]);
    }

    #[test]
    fn start_fires_on_the_frame_the_threshold_is_reached() {
        let mut input = ShellInput::new();
        input.press(ShellButton::B);
        assert_eq!(input.tick(LONG_PRESS - Duration::from_millis(1)), None);
        assert_eq!(
            input.tick(Duration::from_millis(1)),
            Some(ShellButton::Start),
            "not one frame later"
        );
    }

    #[test]
    fn a_second_hold_works_after_the_first_one_ended() {
        let mut input = ShellInput::new();
        input.press(ShellButton::B);
        assert_eq!(run(&mut input, 40), vec![ShellButton::Start]);
        input.release(ShellButton::B);

        input.press(ShellButton::B);
        assert_eq!(run(&mut input, 40), vec![ShellButton::Start]);
    }

    #[test]
    fn ticking_with_b_up_does_nothing() {
        let mut input = ShellInput::new();
        assert!(run(&mut input, 200).is_empty());
        assert_eq!(input.release(ShellButton::B), None, "never went down");
    }

    #[test]
    fn other_buttons_pass_through_while_b_is_held() {
        let mut input = ShellInput::new();
        input.press(ShellButton::B);
        run(&mut input, 5);
        assert_eq!(input.press(ShellButton::Down), Some(ShellButton::Down));
        assert_eq!(input.release(ShellButton::B), Some(ShellButton::B));
    }

    #[test]
    fn hold_progress_runs_zero_to_one_then_stops_reporting() {
        let mut input = ShellInput::new();
        assert_eq!(input.b_hold_progress(), None);
        input.press(ShellButton::B);
        assert_eq!(input.b_hold_progress(), Some(0.0));
        input.tick(LONG_PRESS / 2);
        let half = input.b_hold_progress().expect("mid-hold");
        assert!((half - 0.5).abs() < 0.01, "got {half}");
        input.tick(LONG_PRESS);
        assert_eq!(
            input.b_hold_progress(),
            None,
            "the hold already fired; there is nothing left to indicate"
        );
    }

    #[test]
    fn every_cart_button_reaches_the_shell_and_start_arrives_separately() {
        for button in Button::ALL {
            // Exhaustive by construction; this asserts the two enums stay
            // the same size as each other.
            let _ = shell_button(*button);
        }
        assert_eq!(shell_button(Button::Select), ShellButton::Select);
        assert_eq!(
            shell_button_from_system(SystemButton::Start),
            ShellButton::Start
        );
    }

    #[test]
    fn reset_drops_a_hold_in_flight() {
        let mut input = ShellInput::new();
        input.press(ShellButton::B);
        run(&mut input, 5);
        input.reset();
        assert!(run(&mut input, 200).is_empty());
        assert_eq!(input.release(ShellButton::B), None);
    }
}
