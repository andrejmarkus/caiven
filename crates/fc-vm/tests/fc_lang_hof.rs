use fc_lang::compile;
use fc_vm::input::Input;
use fc_vm::rendering::font::Font;
use fc_vm::{Vm, VmConfig, default_instruction_set};
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
    for _ in 0..200_000 {
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

// ─── named function returning closure ────────────────────────────────────────

#[test]
fn named_fn_returns_closure() {
    // make_adder(n) returns a closure that adds n to its arg
    let vm = run_fc(
        r#"
fn make_adder(n)
  return fn(x) return n + x end
end
let add5 = 0
let result = 0
loop:
  add5 = make_adder(5)
  result = add5(3)
  wait()
"#,
    );
    // add5=G0=ptr, result=G1=8
    assert_eq!(read_u32(&vm, G1), 8, "make_adder(5)(3) should be 8");
}

// ─── closure returning closure ────────────────────────────────────────────────

#[test]
fn closure_returns_closure() {
    // outer is a closure that returns an inner closure
    let vm = run_fc(
        r#"
let result = 0
loop:
  local outer = fn(a) return fn(b) return a + b end end
  local inner = outer(10)
  result = inner(7)
  wait()
"#,
    );
    assert_eq!(read_u32(&vm, G0), 17, "outer(10)(7) should be 17");
}

// ─── counter: mutable upvalue across calls ────────────────────────────────────

#[test]
fn counter_mutable_upval() {
    let vm = run_fc(
        r#"
fn make_counter()
  local count = 0
  return fn()
    count = count + 1
    return count
  end
end
let c = 0
let r1 = 0
let r2 = 0
let r3 = 0
loop:
  c = make_counter()
  r1 = c()
  r2 = c()
  r3 = c()
  wait()
"#,
    );
    // c=G0, r1=G1=1, r2=G2=2, r3=G3=3
    assert_eq!(read_u32(&vm, G1), 1, "first call should return 1");
    assert_eq!(read_u32(&vm, G2), 2, "second call should return 2");
    assert_eq!(read_u32(&vm, G3), 3, "third call should return 3");
}

// ─── triple nesting ───────────────────────────────────────────────────────────

#[test]
fn triple_nested_closure() {
    // fn(a) -> fn(b) -> fn(c) -> a+b+c
    let vm = run_fc(
        r#"
let result = 0
loop:
  local f = fn(a) return fn(b) return fn(c) return a + b + c end end end
  local g = f(1)
  local h = g(2)
  result = h(3)
  wait()
"#,
    );
    assert_eq!(read_u32(&vm, G0), 6, "f(1)(2)(3) should be 6");
}

// ─── two independent closures from same factory ───────────────────────────────

#[test]
fn independent_factory_instances() {
    let vm = run_fc(
        r#"
fn make_adder(n)
  return fn(x) return n + x end
end
let add2 = 0
let add7 = 0
let r1 = 0
let r2 = 0
loop:
  add2 = make_adder(2)
  add7 = make_adder(7)
  r1 = add2(10)
  r2 = add7(10)
  wait()
"#,
    );
    // add2=G0, add7=G1, r1=G2=12, r2=G3=17
    assert_eq!(read_u32(&vm, G2), 12, "add2(10) should be 12");
    assert_eq!(read_u32(&vm, G3), 17, "add7(10) should be 17");
}

// ─── closure capturing another closure ───────────────────────────────────────

#[test]
fn closure_captures_closure() {
    // inner closure captures outer closure var (also a closure ptr)
    let vm = run_fc(
        r#"
let result = 0
loop:
  local inc = fn(x) return x + 1 end
  local apply_twice = fn(x) return inc(inc(x)) end
  result = apply_twice(5)
  wait()
"#,
    );
    assert_eq!(read_u32(&vm, G0), 7, "apply_twice(5) should be 7");
}

// ─── partial application ──────────────────────────────────────────────────────

#[test]
fn partial_application() {
    // curry(f, a) returns fn(b) return f(a, b) end
    // then partial-apply an adder
    let vm = run_fc(
        r#"
fn add(a, b) return a + b end
fn curry2(a)
  return fn(b) return add(a, b) end
end
let add10 = 0
let r1 = 0
let r2 = 0
loop:
  add10 = curry2(10)
  r1 = add10(3)
  r2 = add10(99)
  wait()
"#,
    );
    // add10=G0, r1=G1=13, r2=G2=109
    assert_eq!(read_u32(&vm, G1), 13, "add10(3) should be 13");
    assert_eq!(read_u32(&vm, G2), 109, "add10(99) should be 109");
}

// ─── upval survives after outer scope exits ───────────────────────────────────

#[test]
fn upval_outlives_outer_scope() {
    // The closure is returned before local `base` goes out of scope;
    // it should still hold the captured value after return.
    let vm = run_fc(
        r#"
fn make_fn(base)
  local extra = 100
  return fn(x) return base + extra + x end
end
let f = 0
let result = 0
loop:
  f = make_fn(5)
  result = f(1)
  wait()
"#,
    );
    // f=G0, result=G1 = 5+100+1 = 106
    assert_eq!(read_u32(&vm, G1), 106, "upval should survive outer scope");
}

// ─── deeply nested upval chain ────────────────────────────────────────────────

#[test]
fn deep_upval_chain() {
    // Each level adds 1 to accumulated sum across 4 levels
    let vm = run_fc(
        r#"
let result = 0
loop:
  local f1 = fn(a)
    return fn(b)
      return fn(c)
        return fn(d) return a + b + c + d end
      end
    end
  end
  result = f1(1)(2)(3)(4)
  wait()
"#,
    );
    assert_eq!(read_u32(&vm, G0), 10, "1+2+3+4 should be 10");
}
