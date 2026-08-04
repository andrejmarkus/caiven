//! Battery charge, for the status bar ([`crate::shell::screens::chrome::StatusInfo::battery`]).
//!
//! `sdl2` (the safe crate) exposes no battery query, only `sdl2-sys`'s raw
//! `SDL_GetPowerInfo`. It takes no subsystem handle — just two out-params —
//! so this is a small, self-contained `unsafe` call rather than a reason to
//! hand-roll more FFI than this one function needs.

use std::os::raw::c_int;

use sdl2::sys::{SDL_GetPowerInfo, SDL_PowerState};

/// Battery charge as a `0.0..=1.0` fraction, or `None` when the device has
/// no battery (desktop) or SDL can't determine one — never a fabricated
/// value.
pub fn battery_fraction() -> Option<f32> {
    let mut pct: c_int = -1;
    // SAFETY: SDL_GetPowerInfo only writes through the pointer we give it;
    // a NULL `secs` out-param is explicitly supported by the API (we don't
    // care about seconds remaining, only percentage).
    let state = unsafe { SDL_GetPowerInfo(std::ptr::null_mut(), &mut pct) };
    if pct < 0 {
        return None;
    }
    match state {
        SDL_PowerState::SDL_POWERSTATE_ON_BATTERY
        | SDL_PowerState::SDL_POWERSTATE_CHARGING
        | SDL_PowerState::SDL_POWERSTATE_CHARGED => Some((pct as f32 / 100.0).clamp(0.0, 1.0)),
        // No battery, or SDL couldn't determine a state — pct being
        // non-negative here would be surprising, but stay honest either way.
        SDL_PowerState::SDL_POWERSTATE_UNKNOWN | SDL_PowerState::SDL_POWERSTATE_NO_BATTERY => None,
    }
}

#[cfg(test)]
mod tests {
    use super::battery_fraction;

    /// Can't assert a specific value portably (CI runners have no battery),
    /// but this proves the FFI call is sound and never panics or returns an
    /// out-of-range fraction.
    #[test]
    fn battery_fraction_is_none_or_a_valid_fraction() {
        if let Some(fraction) = battery_fraction() {
            assert!((0.0..=1.0).contains(&fraction));
        }
    }
}
