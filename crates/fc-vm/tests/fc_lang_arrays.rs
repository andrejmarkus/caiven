use fc_lang::compile;
use fc_vm::{Vm, VmConfig, default_instruction_set};
use fc_vm::input::Input;
use fc_vm::rendering::font::Font;
use std::sync::Arc;

fn make_vm() -> Vm {
    Vm::new(Arc::new(default_instruction_set()), VmConfig::default())
}

fn run_fc(src: &str) -> Vm {
    let out = compile(src).unwrap_or_else(|e| panic!("compile failed: {e}"));
    let mut vm = make_vm();
    vm.load_rom(out.program);
    let input = Input::new();
    let font = Font::empty();
    for _ in 0..500_000 {
        if vm.is_waiting() { break; }
        vm.step(&input, &font);
    }
    assert!(vm.get_fault().is_none(), "vm fault: {:?}", vm.get_fault());
    vm
}

fn read_u32(vm: &Vm, addr: usize) -> u32 {
    let b0 = vm.peek_memory(addr) as u32;
    let b1 = vm.peek_memory(addr + 1) as u32;
    let b2 = vm.peek_memory(addr + 2) as u32;
    let b3 = vm.peek_memory(addr + 3) as u32;
    b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
}

const G0: usize = 0x0000;
const G1: usize = 0x0004;
const G2: usize = 0x0008;
#[allow(dead_code)]
const G3: usize = 0x000C;

// ─── positional array literal + integer index read ───────────────────────────

#[test]
fn array_literal_index_read() {
    let vm = run_fc(r#"
let r0 = 0
let r1 = 0
let r2 = 0
loop:
  local a = {10, 20, 30}
  r0 = a[1]
  r1 = a[2]
  r2 = a[3]
  wait()
"#);
    assert_eq!(read_u32(&vm, G0), 10);
    assert_eq!(read_u32(&vm, G1), 20);
    assert_eq!(read_u32(&vm, G2), 30);
}

// ─── array index write ────────────────────────────────────────────────────────

#[test]
fn array_index_write() {
    let vm = run_fc(r#"
let result = 0
loop:
  local a = {1, 2, 3}
  a[2] = 99
  result = a[2]
  wait()
"#);
    assert_eq!(read_u32(&vm, G0), 99);
}

// ─── length operator on positional array ─────────────────────────────────────

#[test]
fn array_length() {
    let vm = run_fc(r#"
let r0 = 0
let r1 = 0
loop:
  local a = {5, 6, 7, 8}
  r0 = #a
  local b = {}
  r1 = #b
  wait()
"#);
    assert_eq!(read_u32(&vm, G0), 4, "#a should be 4");
    assert_eq!(read_u32(&vm, G1), 0, "#b should be 0");
}

// ─── named field table ────────────────────────────────────────────────────────

#[test]
fn named_field_table() {
    let vm = run_fc(r#"
let rx = 0
let ry = 0
loop:
  local pt = {x=7, y=13}
  rx = pt.x
  ry = pt.y
  wait()
"#);
    assert_eq!(read_u32(&vm, G0), 7);
    assert_eq!(read_u32(&vm, G1), 13);
}

// ─── named field write ────────────────────────────────────────────────────────

#[test]
fn named_field_write() {
    let vm = run_fc(r#"
let result = 0
loop:
  local t = {val=1}
  t.val = 42
  result = t.val
  wait()
"#);
    assert_eq!(read_u32(&vm, G0), 42);
}

// ─── array passed to function ─────────────────────────────────────────────────

#[test]
fn array_passed_to_fn() {
    let vm = run_fc(r#"
fn sum3(a)
  return a[1] + a[2] + a[3]
end
let result = 0
loop:
  local arr = {4, 5, 6}
  result = sum3(arr)
  wait()
"#);
    assert_eq!(read_u32(&vm, G0), 15);
}

// ─── array returned from function ────────────────────────────────────────────

#[test]
fn array_returned_from_fn() {
    let vm = run_fc(r#"
fn make_pair(a, b)
  return {a, b}
end
let r0 = 0
let r1 = 0
loop:
  local p = make_pair(3, 7)
  r0 = p[1]
  r1 = p[2]
  wait()
"#);
    assert_eq!(read_u32(&vm, G0), 3);
    assert_eq!(read_u32(&vm, G1), 7);
}

// ─── length of array after appending via index ───────────────────────────────

#[test]
fn array_length_after_set() {
    let vm = run_fc(r#"
let r0 = 0
let r1 = 0
loop:
  local a = {1, 2}
  r0 = #a
  a[3] = 99
  r1 = #a
  wait()
"#);
    assert_eq!(read_u32(&vm, G0), 2, "initial length should be 2");
    assert_eq!(read_u32(&vm, G1), 3, "length after set [3] should be 3");
}

// ─── for-in iterates array values ────────────────────────────────────────────

#[test]
fn forin_array_sum() {
    let vm = run_fc(r#"
let result = 0
loop:
  local a = {10, 20, 30}
  local s = 0
  for k, v in a do
    s = s + v
  end
  result = s
  wait()
"#);
    assert_eq!(read_u32(&vm, G0), 60);
}
