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
const G3: usize = 0x000C;

#[test]
fn variadic_fixed_params_still_bind() {
    let vm = run_fc(
        r#"
fn foo(a, b, ...)
  return a, b
end
loop:
  local x, y = foo(100, 200, 300)
  wait()
"#,
    );
    assert_eq!(read_u32(&vm, G0), 100);
    assert_eq!(read_u32(&vm, G1), 200);
}

#[test]
fn variadic_unpack_exact_count() {
    let vm = run_fc(
        r#"
fn f(...)
  local a, b = ...
  return a, b
end
loop:
  local r1, r2 = f(10, 20)
  wait()
"#,
    );
    assert_eq!(read_u32(&vm, G0), 10);
    assert_eq!(read_u32(&vm, G1), 20);
}

#[test]
fn variadic_unpack_fewer_than_requested_nilpads() {
    let vm = run_fc(
        r#"
fn f(...)
  local a, b = ...
  return a, b
end
loop:
  local r1, r2 = f(10)
  wait()
"#,
    );
    assert_eq!(read_u32(&vm, G0), 10);
    assert_eq!(read_u32(&vm, G1), 0);
}

#[test]
fn variadic_unpack_more_than_requested_ignores_extra() {
    let vm = run_fc(
        r#"
fn f(...)
  local a, b = ...
  return a, b
end
loop:
  local r1, r2 = f(10, 20, 30, 40)
  wait()
"#,
    );
    assert_eq!(read_u32(&vm, G0), 10);
    assert_eq!(read_u32(&vm, G1), 20);
}

#[test]
fn variadic_call_from_multiple_call_sites() {
    // Same variadic function, called with different arg counts from two
    // different call sites — proves the runtime count is passed per call
    // (via R1 at each JSR), not baked into the function body.
    let vm = run_fc(
        r#"
fn f(...)
  local a, b = ...
  return a, b
end
loop:
  local r1, r2 = f(10)
  local r3, r4 = f(10, 20)
  wait()
"#,
    );
    assert_eq!(read_u32(&vm, G0), 10);
    assert_eq!(read_u32(&vm, G1), 0);
    assert_eq!(read_u32(&vm, G2), 10);
    assert_eq!(read_u32(&vm, G3), 20);
}
