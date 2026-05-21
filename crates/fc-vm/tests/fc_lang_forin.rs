use fc_lang::compile;
use fc_vm::{Vm, VmConfig, default_instruction_set};
use fc_vm::input::Input;
use fc_vm::rendering::font::Font;
use std::sync::Arc;

fn make_vm() -> Vm {
    Vm::new(Arc::new(default_instruction_set()), VmConfig::default())
}

fn run_fc(src: &str) -> Vm {
    let out = compile(src).unwrap_or_else(|e| panic!("compile failed: {}", e));
    let mut vm = make_vm();
    vm.load_rom(out.program);
    let input = Input::new();
    let font = Font::empty();
    for _ in 0..200_000 {
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

// ─── for k, v in table ────────────────────────────────────────────────────────

#[test]
fn forin_sum_values() {
    // Build a table {1=10, 2=20, 3=30}, iterate summing values.
    let vm = run_fc(r#"
let result = 0
loop:
  local t = {}
  t[1] = 10
  t[2] = 20
  t[3] = 30
  local sum = 0
  for k, v in t do
    sum = sum + v
  end
  result = sum
  wait()
"#);
    assert_eq!(read_u32(&vm, G0), 60);
}

#[test]
fn forin_count_entries() {
    let vm = run_fc(r#"
let result = 0
loop:
  local t = {}
  t[1] = 1
  t[2] = 2
  local count = 0
  for k, v in t do
    count = count + 1
  end
  result = count
  wait()
"#);
    assert_eq!(read_u32(&vm, G0), 2);
}

#[test]
fn forin_empty_table() {
    // Empty table: loop body never runs, result stays 0.
    let vm = run_fc(r#"
let result = 0
loop:
  local t = {}
  for k, v in t do
    result = 99
  end
  wait()
"#);
    assert_eq!(read_u32(&vm, G0), 0);
}

#[test]
fn forin_key_and_val() {
    // Sum keys and vals separately to verify both bindings are correct.
    let vm = run_fc(r#"
let key_sum = 0
let val_sum = 0
loop:
  local t = {}
  t[10] = 100
  t[20] = 200
  local ks = 0
  local vs = 0
  for k, v in t do
    ks = ks + k
    vs = vs + v
  end
  key_sum = ks
  val_sum = vs
  wait()
"#);
    assert_eq!(read_u32(&vm, G0), 30);  // 10+20
    assert_eq!(read_u32(&vm, G1), 300); // 100+200
}
