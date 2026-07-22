use fc_lang::compile;
use fc_vm::input::Input;
use fc_vm::rendering::font::Font;
use fc_vm::{Vm, VmConfig, VmFault, default_instruction_set};
use std::sync::Arc;

fn make_vm() -> Vm {
    Vm::new(Arc::new(default_instruction_set()), VmConfig::default())
}

#[test]
fn heap_exhaustion_faults_cleanly_via_string_doubling() {
    // Length doubles each iteration via `..` concat, so exhausting the real
    // ~26.9KB heap/stack region only takes ~15-16 iterations (2^15 = 32768).
    let src = r#"
let s = "x"
loop:
  for i = 1, 20 do
    s = s .. s
  end
  wait()
"#;
    let out = compile(src).unwrap_or_else(|e| panic!("compile failed: {e}"));
    let mut vm = make_vm();
    vm.load_rom(out.program);
    let input = Input::new();
    let font = Font::empty();
    for _ in 0..1_000_000 {
        if vm.is_waiting() {
            break;
        }
        vm.step(&input, &font);
    }
    assert_eq!(vm.get_fault(), Some(VmFault::HeapExhausted));
}

#[test]
fn heap_exhaustion_faults_cleanly_via_closures_in_loop() {
    // A closure created once per iteration (a realistic per-frame pattern)
    // never gets reclaimed — this must fault cleanly rather than silently
    // corrupting the stack once the bump allocator collides with it.
    let src = r#"
let offset = 1
let result = 0
loop:
  for i = 1, 5000 do
    local f = fn(x) return offset + x end
    result = f(i)
  end
  wait()
"#;
    let out = compile(src).unwrap_or_else(|e| panic!("compile failed: {e}"));
    let mut vm = make_vm();
    vm.load_rom(out.program);
    let input = Input::new();
    let font = Font::empty();
    for _ in 0..1_000_000 {
        if vm.is_waiting() {
            break;
        }
        vm.step(&input, &font);
    }
    assert_eq!(vm.get_fault(), Some(VmFault::HeapExhausted));
}
