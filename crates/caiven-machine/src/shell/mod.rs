//! The console shell: boot, library, cart detail, settings, pause and the
//! rest of the menus wrapped around a running cart.
//!
//! Everything here is CPU-rasterized. The target device has no GPU, so the
//! shell composites into an RGBA buffer and uploads it as a second SDL
//! texture beside the console framebuffer, and only redraws when state
//! changes.

// The token set is defined up front, ahead of the screens that consume it,
// so the design system lands in one reviewable piece. Drop this once the
// screens are drawn.
#[allow(dead_code)]
pub mod font;
#[allow(dead_code)]
pub mod icon;
#[allow(dead_code)]
pub mod surface;
#[allow(dead_code)]
pub mod theme;
