//! Regression coverage for VM-03: a cart-supplied size/coordinate/radius far
//! outside the screen or map must not turn one Lua call into a native loop
//! proportional to that size — the instruction-count watchdog only sees Lua
//! bytecode between calls, not work done inside a single native one, so an
//! unclipped per-pixel loop here would hang the process with no way for the
//! watchdog to ever step in. Each test below would hang forever without the
//! fix; the assertion just confirms the call still returns and the visible
//! result is still correct.

use caiven_vm::input::Input;
use caiven_vm::rendering::font::Font;
use caiven_vm::vm::palette::DEFAULT_COLORS;
use caiven_vm::{Vm, VmConfig};

fn fresh_vm() -> (Vm, Input, Font) {
    (Vm::new(VmConfig::default()), Input::new(), Font::empty())
}

fn slot_rgba(index: usize) -> [u8; 4] {
    let (r, g, b) = DEFAULT_COLORS[index];
    [r, g, b, 255]
}

fn read_rgba(vm: &Vm, x: u32, y: u32) -> [u8; 4] {
    let width = VmConfig::default().width;
    let i = ((y * width + x) * 4) as usize;
    let px = vm.world_pixels();
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}

fn run_one_frame(vm: &mut Vm, input: &Input, font: &Font, src: &str) {
    vm.load_lua_source(src, input, font)
        .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.run_frame(input, font);
    assert_eq!(vm.get_fault(), None, "cart must not fault on valid input");
}

#[test]
fn fill_rect_with_huge_dimensions_clips_to_the_screen() {
    let (mut vm, input, font) = fresh_vm();
    run_one_frame(
        &mut vm,
        &input,
        &font,
        "function _update() clear_screen() fill_rect(0, 0, 2000000000, 2000000000, 8) end",
    );
    assert_eq!(read_rgba(&vm, 0, 0), slot_rgba(8));
    assert_eq!(read_rgba(&vm, 191, 127), slot_rgba(8));
}

#[test]
fn draw_rect_with_huge_dimensions_does_not_hang() {
    let (mut vm, input, font) = fresh_vm();
    run_one_frame(
        &mut vm,
        &input,
        &font,
        "function _update() clear_screen() draw_rect(-1000000, -1000000, 3000000000, 3000000000, 8) end",
    );
}

#[test]
fn fill_circle_with_huge_radius_clips_to_the_screen() {
    let (mut vm, input, font) = fresh_vm();
    run_one_frame(
        &mut vm,
        &input,
        &font,
        "function _update() clear_screen() fill_circle(96, 64, 2000000000, 8) end",
    );
    // A circle whose radius dwarfs the screen just fills it solid.
    assert_eq!(read_rgba(&vm, 0, 0), slot_rgba(8));
    assert_eq!(read_rgba(&vm, 191, 127), slot_rgba(8));
}

#[test]
fn draw_circle_with_huge_radius_does_not_hang() {
    let (mut vm, input, font) = fresh_vm();
    run_one_frame(
        &mut vm,
        &input,
        &font,
        "function _update() clear_screen() draw_circle(96, 64, 5000000000, 8) end",
    );
}

#[test]
fn draw_line_to_a_faraway_endpoint_does_not_hang() {
    let (mut vm, input, font) = fresh_vm();
    run_one_frame(
        &mut vm,
        &input,
        &font,
        "function _update() clear_screen() draw_line(0, 0, 5000000000, 5000000000, 8) end",
    );
    // The visible portion of the line still starts at the origin.
    assert_eq!(read_rgba(&vm, 0, 0), slot_rgba(8));
}

#[test]
fn draw_map_with_huge_dimensions_does_not_hang() {
    let (mut vm, input, font) = fresh_vm();
    run_one_frame(
        &mut vm,
        &input,
        &font,
        "function _update() clear_screen() draw_map(0, 0, 0, 0, 2000000000, 2000000000) end",
    );
}

#[test]
fn a_normal_in_bounds_fill_rect_is_unaffected() {
    let (mut vm, input, font) = fresh_vm();
    run_one_frame(
        &mut vm,
        &input,
        &font,
        "function _update() clear_screen() fill_rect(10, 10, 4, 4, 8) end",
    );
    assert_eq!(read_rgba(&vm, 10, 10), slot_rgba(8));
    assert_eq!(read_rgba(&vm, 13, 13), slot_rgba(8));
    // `clear_screen` zeroes raw pixel bytes rather than painting palette
    // slot 0, so a spot the rect doesn't cover reads back as fully zeroed.
    assert_eq!(read_rgba(&vm, 14, 14), [0, 0, 0, 0]);
    assert_eq!(vm.get_fault(), None);
}
