use fc_lang::compile;
use fc_vm::input::Input;
use fc_vm::rendering::font::Font;
use fc_vm::{Vm, VmConfig, default_instruction_set};
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
    for _ in 0..100_000 {
        if vm.is_waiting() {
            break;
        }
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

#[test]
fn return_single_value() {
    let vm = run_fc(
        r#"
fn simple()
  return 42
end
let result = 0
loop:
  result = simple()
  wait()
"#,
    );
    assert_eq!(read_u32(&vm, G0), 42);
}

#[test]
fn return_three_unpack_all() {
    let vm = run_fc(
        r#"
fn multi()
  return 10, 20, 30
end
loop:
  local a, b, c = multi()
  wait()
"#,
    );
    assert_eq!(read_u32(&vm, G0), 10);
    assert_eq!(read_u32(&vm, G1), 20);
    assert_eq!(read_u32(&vm, G2), 30);
}

#[test]
fn return_three_unpack_partial() {
    let vm = run_fc(
        r#"
fn multi()
  return 100, 200, 300
end
loop:
  local x, y = multi()
  wait()
"#,
    );
    assert_eq!(read_u32(&vm, G0), 100);
    assert_eq!(read_u32(&vm, G1), 200);
}

#[test]
fn return_one_unpack_three_nilpads() {
    let vm = run_fc(
        r#"
fn single()
  return 555
end
loop:
  local x, y, z = single()
  wait()
"#,
    );
    assert_eq!(read_u32(&vm, G0), 555);
    assert_eq!(read_u32(&vm, G1), 0);
    assert_eq!(read_u32(&vm, G2), 0);
}

#[test]
fn return_multi_used_in_expr_truncates() {
    let vm = run_fc(
        r#"
fn multi()
  return 10, 20, 30
end
let result = 0
loop:
  result = multi() + 5
  wait()
"#,
    );
    assert_eq!(read_u32(&vm, G0), 15);
}

#[test]
fn return_recursive_nested_multireturn() {
    // outer() itself unpacks inner()'s multi-return before doing its own
    // multi-return — proves the return buffer isn't stomped by the nested
    // call before the caller gets to consume it.
    let vm = run_fc(
        r#"
fn inner()
  return 1, 2
end
fn outer()
  local a, b = inner()
  return a + 10, b + 20
end
loop:
  local x, y = outer()
  wait()
"#,
    );
    assert_eq!(read_u32(&vm, G0), 11);
    assert_eq!(read_u32(&vm, G1), 22);
}
