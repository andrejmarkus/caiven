use std::sync::Arc;

use crate::input::Input;
use crate::isa::default_instruction_set;
use crate::rendering::font::Font;
use crate::vm::Vm;

fn make_vm() -> Vm {
    Vm::new(Arc::new(default_instruction_set()))
}

fn run(vm: &mut Vm, src: &str) {
    vm.load_program(src).expect("assemble failed");
    let font = Font::empty();
    let input = Input::new();
    vm.run_frame(&input, &font);
}

// ── arithmetic ───────────────────────────────────────────────────────────────

#[test]
fn mov_loads_immediate_into_register() {
    let mut vm = make_vm();
    run(&mut vm, "MOV R0, 42\nWAIT");
    assert_eq!(vm.get_registers()[0], 42);
}

#[test]
fn add_accumulates_into_register() {
    let mut vm = make_vm();
    run(&mut vm, "MOV R0, 10\nADD R0, 5\nWAIT");
    assert_eq!(vm.get_registers()[0], 15);
}

#[test]
fn sub_decrements_register() {
    let mut vm = make_vm();
    run(&mut vm, "MOV R0, 10\nSUB R0, 3\nWAIT");
    assert_eq!(vm.get_registers()[0], 7);
}

#[test]
fn dec_decrements_by_one() {
    let mut vm = make_vm();
    run(&mut vm, "MOV R0, 5\nDEC R0\nWAIT");
    assert_eq!(vm.get_registers()[0], 4);
}

#[test]
fn movr_copies_between_registers() {
    let mut vm = make_vm();
    run(&mut vm, "MOV R0, 99\nMOVR R1, R0\nWAIT");
    assert_eq!(vm.get_registers()[1], 99);
}

#[test]
fn addr_adds_registers() {
    let mut vm = make_vm();
    run(&mut vm, "MOV R0, 7\nMOV R1, 3\nADDR R0, R1\nWAIT");
    assert_eq!(vm.get_registers()[0], 10);
}

#[test]
fn subr_subtracts_registers() {
    let mut vm = make_vm();
    run(&mut vm, "MOV R0, 10\nMOV R1, 4\nSUBR R0, R1\nWAIT");
    assert_eq!(vm.get_registers()[0], 6);
}

#[test]
fn slt_sets_one_when_less_than() {
    let mut vm = make_vm();
    run(&mut vm, "MOV R0, 3\nMOV R1, 5\nSLT R2, R0, R1\nWAIT");
    assert_eq!(vm.get_registers()[2], 1);
}

#[test]
fn slt_sets_zero_when_not_less_than() {
    let mut vm = make_vm();
    run(&mut vm, "MOV R0, 5\nMOV R1, 3\nSLT R2, R0, R1\nWAIT");
    assert_eq!(vm.get_registers()[2], 0);
}

// ── control flow ─────────────────────────────────────────────────────────────

#[test]
fn jmp_transfers_control() {
    let mut vm = make_vm();
    // JMP skips the MOV R0, 99 — R0 stays 0
    run(&mut vm, "JMP skip\nMOV R0, 99\nskip:\nWAIT");
    assert_eq!(vm.get_registers()[0], 0);
}

#[test]
fn jnz_jumps_when_nonzero() {
    let mut vm = make_vm();
    run(&mut vm, "MOV R0, 1\nJNZ R0, skip\nMOV R1, 99\nskip:\nWAIT");
    assert_eq!(vm.get_registers()[1], 0);
}

#[test]
fn jnz_falls_through_when_zero() {
    let mut vm = make_vm();
    run(&mut vm, "MOV R0, 0\nJNZ R0, skip\nMOV R1, 42\nskip:\nWAIT");
    assert_eq!(vm.get_registers()[1], 42);
}

#[test]
fn jz_jumps_when_zero() {
    let mut vm = make_vm();
    run(&mut vm, "MOV R0, 0\nJZ R0, skip\nMOV R1, 99\nskip:\nWAIT");
    assert_eq!(vm.get_registers()[1], 0);
}

#[test]
fn jz_falls_through_when_nonzero() {
    let mut vm = make_vm();
    run(&mut vm, "MOV R0, 1\nJZ R0, skip\nMOV R1, 42\nskip:\nWAIT");
    assert_eq!(vm.get_registers()[1], 42);
}

#[test]
fn jsr_and_ret_returns_to_caller() {
    let mut vm = make_vm();
    run(
        &mut vm,
        "JSR subroutine\nMOV R1, 42\nWAIT\nsubroutine:\nMOV R0, 7\nRET",
    );
    assert_eq!(vm.get_registers()[0], 7);
    assert_eq!(vm.get_registers()[1], 42);
}

// ── memory ───────────────────────────────────────────────────────────────────

#[test]
fn stm_and_ldm_round_trip() {
    let mut vm = make_vm();
    run(&mut vm, "MOV R0, 123\nSTM 0x1000, R0\nLDM R1, 0x1000\nWAIT");
    assert_eq!(vm.get_registers()[1], 123);
}

#[test]
fn stmw_and_ldmw_round_trip_word() {
    let mut vm = make_vm();
    run(&mut vm, "MOV R0, 1000\nSTMW 0x1000, R0\nLDMW R1, 0x1000\nWAIT");
    assert_eq!(vm.get_registers()[1], 1000);
}

#[test]
fn stmi_and_ldmi_indirect_round_trip() {
    let mut vm = make_vm();
    // R0 = address, R1 = value to store
    run(
        &mut vm,
        "MOV R0, 0x1000\nMOV R1, 77\nSTMI R0, R1\nLDMI R2, R0\nWAIT",
    );
    assert_eq!(vm.get_registers()[2], 77);
}
