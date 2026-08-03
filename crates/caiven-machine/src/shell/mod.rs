//! The console shell: boot, library, cart detail, settings, pause and the
//! rest of the menus wrapped around a running cart.
//!
//! Everything here is CPU-rasterized. The target device has no GPU, so the
//! shell composites into an RGBA buffer and uploads it as a second SDL
//! texture beside the console framebuffer, and only redraws when state
//! changes.

// The token set and the navigation graph land ahead of the screens that
// consume them, so each piece is reviewable on its own. Drop these once the
// screens are drawn and the app loop drives the state machine.
#[allow(dead_code)]
pub mod font;
#[allow(dead_code)]
pub mod icon;
#[allow(dead_code)]
pub mod input;
#[allow(dead_code)]
pub mod settings;
#[allow(dead_code)]
pub mod state;
#[allow(dead_code)]
pub mod surface;
#[allow(dead_code)]
pub mod theme;
