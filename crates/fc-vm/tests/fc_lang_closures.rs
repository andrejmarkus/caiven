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
    // run init + one loop iteration
    for _ in 0..100_000 {
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

// Globals are allocated at 0x0000, 4 bytes each in declaration order.
const G0: usize = 0x0000;
const G1: usize = 0x0004;
const G2: usize = 0x0008;

// ─── closure creation ─────────────────────────────────────────────────────────

#[test]
fn closure_ptr_nonzero() {
    // A closure expression should produce a non-zero heap pointer.
    let vm = run_fc(r#"
let f = 0
loop:
  f = fn(x) return x end
  wait()
"#);
    assert_ne!(read_u32(&vm, G0), 0, "closure ptr should be nonzero");
}

// ─── identity closure (no upvals) ─────────────────────────────────────────────

#[test]
fn closure_call_identity() {
    let vm = run_fc(r#"
let f = 0
let result = 0
loop:
  f = fn(x) return x end
  result = f(42)
  wait()
"#);
    assert_eq!(read_u32(&vm, G1), 42);
}

// ─── upvalue capture ──────────────────────────────────────────────────────────

#[test]
fn closure_captures_global() {
    let vm = run_fc(r#"
let base = 10
let f = 0
let result = 0
loop:
  f = fn(x) return base + x end
  result = f(5)
  wait()
"#);
    // base=G0=10, f=G1=ptr, result=G2=15
    assert_eq!(read_u32(&vm, G2), 15);
}

#[test]
fn closure_captures_local() {
    let vm = run_fc(r#"
let result = 0
loop:
  local offset = 7
  local f = fn(x) return offset + x end
  result = f(3)
  wait()
"#);
    assert_eq!(read_u32(&vm, G0), 10);
}

#[test]
fn closure_captures_multiple_upvals() {
    let vm = run_fc(r#"
let a = 2
let b = 3
let f = 0
let result = 0
loop:
  f = fn(x) return a + b + x end
  result = f(10)
  wait()
"#);
    // result = 2+3+10 = 15
    assert_eq!(read_u32(&vm, G3), 15);
}

const G3: usize = 0x000C;

// ─── closure called multiple times ────────────────────────────────────────────

#[test]
fn closure_called_twice() {
    let vm = run_fc(r#"
let base = 100
let f = 0
let r1 = 0
let r2 = 0
loop:
  f = fn(x) return base + x end
  r1 = f(1)
  r2 = f(2)
  wait()
"#);
    // base=G0=100, f=G1=ptr, r1=G2=101, r2=G3=102
    assert_eq!(read_u32(&vm, G2), 101);
    assert_eq!(read_u32(&vm, G3), 102);
}

// ─── closure as value passed to fn ────────────────────────────────────────────

#[test]
fn closure_passed_as_arg() {
    // Named fn receives closure ptr in param[0] (direct call, no env_ptr),
    // then calls it dynamically.
    let vm = run_fc(r#"
fn apply(cb, val)
  return cb(val)
end
let offset = 20
let f = 0
let result = 0
loop:
  f = fn(x) return offset + x end
  result = apply(f, 5)
  wait()
"#);
    // offset=G0=20, f=G1=ptr, result=G2=25
    assert_eq!(read_u32(&vm, G2), 25);
}
