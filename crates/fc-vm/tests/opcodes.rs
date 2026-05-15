use fc_vm::{Vm, VmConfig, VmFault, default_instruction_set};
use fc_vm::input::Input;
use fc_vm::rendering::font::Font;
use std::sync::Arc;

fn make_vm() -> Vm {
    Vm::new(Arc::new(default_instruction_set()), VmConfig::default())
}

fn run_program(vm: &mut Vm, src: &str) {
    vm.load_program(src).expect("assemble failed");
    let input = Input::new();
    let font = Font::empty();
    for _ in 0..10000 {
        if vm.is_waiting() { break; }
        vm.step(&input, &font);
    }
}

fn reg(vm: &Vm, i: usize) -> u32 {
    vm.get_registers()[i]
}

// ─── arithmetic ───────────────────────────────────────────────────────────────

#[test]
fn mul_register() {
    let mut vm = make_vm();
    run_program(&mut vm, "MOV R0, 6\nMOV R1, 7\nMUL R0, R1\nWAIT");
    assert_eq!(reg(&vm, 0), 42);
}

#[test]
fn div_register() {
    let mut vm = make_vm();
    run_program(&mut vm, "MOV R0, 20\nMOV R1, 4\nDIV R0, R1\nWAIT");
    assert_eq!(reg(&vm, 0), 5);
}

#[test]
fn mod_register() {
    let mut vm = make_vm();
    run_program(&mut vm, "MOV R0, 17\nMOV R1, 5\nMOD R0, R1\nWAIT");
    assert_eq!(reg(&vm, 0), 2);
}

#[test]
fn div_by_zero_faults() {
    let mut vm = make_vm();
    run_program(&mut vm, "MOV R0, 10\nMOV R1, 0\nDIV R0, R1\nWAIT");
    assert_eq!(vm.get_fault(), Some(VmFault::DivisionByZero));
}

// ─── bitwise ──────────────────────────────────────────────────────────────────

#[test]
fn and_register() {
    let mut vm = make_vm();
    run_program(&mut vm, "MOV R0, 0xFF\nMOV R1, 0x0F\nAND R0, R1\nWAIT");
    assert_eq!(reg(&vm, 0), 0x0F);
}

#[test]
fn or_register() {
    let mut vm = make_vm();
    run_program(&mut vm, "MOV R0, 0xF0\nMOV R1, 0x0F\nOR R0, R1\nWAIT");
    assert_eq!(reg(&vm, 0), 0xFF);
}

#[test]
fn xor_register() {
    let mut vm = make_vm();
    run_program(&mut vm, "MOV R0, 0xFF\nMOV R1, 0x0F\nXOR R0, R1\nWAIT");
    assert_eq!(reg(&vm, 0), 0xF0);
}

#[test]
fn not_register() {
    let mut vm = make_vm();
    run_program(&mut vm, "MOV R0, 0\nNOT R0\nWAIT");
    assert_eq!(reg(&vm, 0), 0xFFFF_FFFFu32);
}

#[test]
fn shl_register() {
    let mut vm = make_vm();
    run_program(&mut vm, "MOV R0, 1\nSHL R0, 3\nWAIT");
    assert_eq!(reg(&vm, 0), 8);
}

#[test]
fn shr_register() {
    let mut vm = make_vm();
    run_program(&mut vm, "MOV R0, 16\nSHR R0, 2\nWAIT");
    assert_eq!(reg(&vm, 0), 4);
}

#[test]
fn neg_register() {
    let mut vm = make_vm();
    run_program(&mut vm, "MOV R0, 1\nNEG R0\nWAIT");
    assert_eq!(reg(&vm, 0), 0xFFFF_FFFFu32); // -1 in two's complement
}

// ─── compare ──────────────────────────────────────────────────────────────────

#[test]
fn slts_signed() {
    let mut vm = make_vm();
    // -1 (0xFFFFFFFF) < 1 → signed: yes
    run_program(&mut vm, "MOV32 R1, 0xFFFFFFFF\nMOV R2, 1\nSLTS R0, R1, R2\nWAIT");
    assert_eq!(reg(&vm, 0), 1);
}

#[test]
fn eq_equal() {
    let mut vm = make_vm();
    run_program(&mut vm, "MOV R1, 42\nMOV R2, 42\nEQ R0, R1, R2\nWAIT");
    assert_eq!(reg(&vm, 0), 1);
}

#[test]
fn eq_not_equal() {
    let mut vm = make_vm();
    run_program(&mut vm, "MOV R1, 42\nMOV R2, 43\nEQ R0, R1, R2\nWAIT");
    assert_eq!(reg(&vm, 0), 0);
}

// ─── stack ────────────────────────────────────────────────────────────────────

#[test]
fn push_pop_roundtrip() {
    let mut vm = make_vm();
    run_program(&mut vm, "MOV R0, 1234\nPUSH R0\nMOV R0, 0\nPOP R0\nWAIT");
    assert_eq!(reg(&vm, 0), 1234);
}

#[test]
fn getsp_setsp() {
    let mut vm = make_vm();
    run_program(&mut vm, "GETSP R0\nMOV R1, 100\nSETSP R1\nGETSP R2\nWAIT");
    assert_eq!(reg(&vm, 2), 100);
}

// ─── 32-bit memory ────────────────────────────────────────────────────────────

#[test]
fn ldm32_stm32() {
    let mut vm = make_vm();
    run_program(&mut vm, "MOV32 R0, 0xDEADBEEF\nSTM32 0x1000, R0\nMOV R0, 0\nLDM32 R0, 0x1000\nWAIT");
    assert_eq!(reg(&vm, 0), 0xDEADBEEFu32);
}

#[test]
fn ldm32i_stm32i() {
    let mut vm = make_vm();
    run_program(&mut vm,
        "MOV32 R0, 0xCAFEBABE\nMOV R1, 0x2000\nSTM32I R1, R0\nMOV R0, 0\nLDM32I R0, R1\nWAIT"
    );
    assert_eq!(reg(&vm, 0), 0xCAFEBABEu32);
}

// ─── mov32 immediate ──────────────────────────────────────────────────────────

#[test]
fn mov32_immediate() {
    let mut vm = make_vm();
    run_program(&mut vm, "MOV32 R0, 0x12345678\nWAIT");
    assert_eq!(reg(&vm, 0), 0x12345678u32);
}

// ─── backward compat: existing asm carts assemble ────────────────────────────

#[test]
fn existing_carts_assemble() {
    use fc_asm::Assembler;
    let asm = Assembler::new();
    for path in [
        "../../games/asm/movement.asm",
        "../../games/asm/catch.asm",
        "../../games/asm/sprite.asm",
        "../../games/asm/tiles.asm",
        "../../games/asm/audio_test.asm",
    ] {
        let p = std::path::Path::new(path);
        if p.exists() {
            asm.assemble_file(p).unwrap_or_else(|e| panic!("failed to assemble {}: {:?}", path, e));
        }
    }
}
