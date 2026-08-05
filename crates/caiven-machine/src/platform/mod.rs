//! The SDL2 platform layer: window, rendering and input.
//!
//! `caiven-vm` owns execution, rendering primitives, input state, and (via
//! `vm::audio::sdl_audio_factory`) the SDL audio backend itself — Machine
//! just hands it the `AudioSubsystem` it opened for video. Everything else
//! platform-specific lives here. SDL2 is the choice because it is what
//! small Linux handhelds (Miyoo, TrimUI, Anbernic) ship and what lets one
//! binary cover them alongside desktop.

pub mod input;
pub mod power;
pub mod scaling;
pub mod window;
