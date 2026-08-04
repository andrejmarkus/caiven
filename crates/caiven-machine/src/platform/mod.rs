//! The SDL2 platform layer: window, rendering, audio and input.
//!
//! `caiven-vm` owns execution, rendering primitives and input state but no
//! window, so everything platform-specific lives here. SDL2 is the choice
//! because it is what small Linux handhelds (Miyoo, TrimUI, Anbernic) ship
//! and what lets one binary cover them alongside desktop.

pub mod audio;
pub mod input;
pub mod power;
pub mod scaling;
pub mod window;
