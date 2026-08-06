//! The console shell: boot, library, cart detail, settings, pause and the
//! rest of the menus wrapped around a running cart.
//!
//! Everything here is CPU-rasterized. The target device has no GPU, so the
//! shell composites into an RGBA buffer and uploads it as a second SDL
//! texture beside the console framebuffer, and only redraws when state
//! changes.

// Some tokens and methods here are still ahead of the screens that will
// consume them (Pause T45, Settings T46, Controls T47, Port T48, save
// states T50) — drop these once every screen is drawn for real.
#[allow(dead_code)]
pub mod font;
#[allow(dead_code)]
pub mod icon;
#[allow(dead_code)]
pub mod input;
#[allow(dead_code)]
pub mod library;
#[allow(dead_code)]
pub mod save_state;
#[allow(dead_code)]
pub mod screens;
#[allow(dead_code)]
pub mod settings;
#[allow(dead_code)]
pub mod state;
#[allow(dead_code)]
pub mod surface;
#[allow(dead_code)]
pub mod theme;
