//! Headless smoke tests for the shipped .asm demos: every demo must
//! assemble and run a few frames without faulting, and the tile maze must
//! populate map RAM and collide against sprite flags.

use fc_core::memory::{MAP_RAM_BASE, MAP_W, SPRITE_FLAGS_RAM_BASE, SPRITE_SHEET_RAM_BASE};
use fc_vm::input::{Button, Input};
use fc_vm::rendering::font::Font;
use fc_vm::{Vm, VmConfig, default_instruction_set};
use std::sync::Arc;

fn load_demo(name: &str) -> Vm {
    let path = format!("../../games/asm/{name}.asm");
    let source = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let out =
        fc_asm::assemble_with_sections(&source).unwrap_or_else(|e| panic!("assemble {path}: {e}"));
    let mut vm = Vm::new(Arc::new(default_instruction_set()), VmConfig::default());
    vm.load_rom(out.program);
    for (_, data) in &out.extra_sections {
        vm.load_section_to_ram(SPRITE_SHEET_RAM_BASE, data);
    }
    vm
}

fn run_frames(vm: &mut Vm, input: &mut Input, frames: usize) {
    let font = Font::empty();
    for _ in 0..frames {
        vm.run_frame(input, &font);
        input.end_frame();
        assert!(vm.get_fault().is_none(), "vm fault: {:?}", vm.get_fault());
    }
}

#[test]
fn all_asm_demos_run_without_fault() {
    for name in ["movement", "sprite", "tiles", "catch", "audio_test"] {
        let mut vm = load_demo(name);
        let mut input = Input::new();
        run_frames(&mut vm, &mut input, 5);
    }
}

#[test]
fn tiles_maze_loads_map_and_flags() {
    let mut vm = load_demo("tiles");
    let mut input = Input::new();
    run_frames(&mut vm, &mut input, 2);

    // border cell (0,0) = wall sprite 2, inner cell (1,1) = floor sprite 1
    assert_eq!(vm.peek_memory(MAP_RAM_BASE), 2);
    assert_eq!(vm.peek_memory(MAP_RAM_BASE + MAP_W + 1), 1);
    // wall sprite is marked solid
    assert_eq!(vm.peek_memory(SPRITE_FLAGS_RAM_BASE + 2), 1);
    assert_eq!(vm.peek_memory(SPRITE_FLAGS_RAM_BASE + 1), 0);
}

#[test]
fn tiles_maze_collision_blocks_walls() {
    const PLAYER_X: usize = 5;

    let mut vm = load_demo("tiles");
    let mut input = Input::new();
    run_frames(&mut vm, &mut input, 1);
    assert_eq!(vm.peek_memory(PLAYER_X), 8);

    // border wall on the left blocks movement
    input.set_button(Button::Left, true);
    run_frames(&mut vm, &mut input, 30);
    assert_eq!(vm.peek_memory(PLAYER_X), 8);

    // the corridor to the right is open
    input.set_button(Button::Left, false);
    input.set_button(Button::Right, true);
    run_frames(&mut vm, &mut input, 30);
    assert_eq!(vm.peek_memory(PLAYER_X), 38);
}
