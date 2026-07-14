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
    for _ in 0..500_000 {
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

fn read_cstr(vm: &Vm, addr: u32) -> String {
    let mut s = String::new();
    let mut a = addr as usize;
    loop {
        let b = vm.peek_memory(a);
        if b == 0 {
            break;
        }
        s.push(b as char);
        a += 1;
    }
    s
}

const G0: usize = 0x0000;
const G1: usize = 0x0004;
const G2: usize = 0x0008;
#[allow(dead_code)]
const G3: usize = 0x000C;

// ─── strlen of literal ────────────────────────────────────────────────────────

#[test]
fn strlen_literal() {
    let vm = run_fc(
        r#"
let r0 = 0
let r1 = 0
loop:
  r0 = strlen("hello")
  r1 = strlen("")
  wait()
"#,
    );
    assert_eq!(read_u32(&vm, G0), 5, "strlen(\"hello\") should be 5");
    assert_eq!(read_u32(&vm, G1), 0, "strlen(\"\") should be 0");
}

// ─── strlen of concat ─────────────────────────────────────────────────────────

#[test]
fn strlen_concat() {
    let vm = run_fc(
        r#"
let r0 = 0
let r1 = 0
loop:
  r0 = strlen("ab" .. "cd")
  r1 = strlen("hello" .. " world")
  wait()
"#,
    );
    assert_eq!(read_u32(&vm, G0), 4, "strlen(\"ab\"..\"cd\") should be 4");
    assert_eq!(
        read_u32(&vm, G1),
        11,
        "strlen(\"hello\" .. \" world\") should be 11"
    );
}

// ─── dynamic concat content ───────────────────────────────────────────────────

#[test]
fn dynamic_concat_content() {
    let vm = run_fc(
        r#"
let ptr = 0
loop:
  local a = "foo"
  local b = "bar"
  ptr = a .. b
  wait()
"#,
    );
    let ptr = read_u32(&vm, G0);
    assert_eq!(
        read_cstr(&vm, ptr),
        "foobar",
        "\"foo\"..\"bar\" should be \"foobar\""
    );
}

// ─── concat with variable and literal ────────────────────────────────────────

#[test]
fn concat_var_and_literal() {
    let vm = run_fc(
        r#"
let ptr = 0
loop:
  local prefix = "hello"
  ptr = prefix .. " world"
  wait()
"#,
    );
    let ptr = read_u32(&vm, G0);
    assert_eq!(read_cstr(&vm, ptr), "hello world");
}

// ─── tostring positive ────────────────────────────────────────────────────────

#[test]
fn tostring_positive() {
    let vm = run_fc(
        r#"
let p0 = 0
let p1 = 0
let p2 = 0
loop:
  p0 = tostring(0)
  p1 = tostring(42)
  p2 = tostring(100)
  wait()
"#,
    );
    assert_eq!(
        read_cstr(&vm, read_u32(&vm, G0)),
        "0",
        "tostring(0) should be \"0\""
    );
    assert_eq!(
        read_cstr(&vm, read_u32(&vm, G1)),
        "42",
        "tostring(42) should be \"42\""
    );
    assert_eq!(
        read_cstr(&vm, read_u32(&vm, G2)),
        "100",
        "tostring(100) should be \"100\""
    );
}

// ─── tostring negative ────────────────────────────────────────────────────────

#[test]
fn tostring_negative() {
    let vm = run_fc(
        r#"
let p0 = 0
let p1 = 0
loop:
  p0 = tostring(-1)
  p1 = tostring(-99)
  wait()
"#,
    );
    assert_eq!(read_cstr(&vm, read_u32(&vm, G0)), "-1");
    assert_eq!(read_cstr(&vm, read_u32(&vm, G1)), "-99");
}

// ─── tostring then strlen ─────────────────────────────────────────────────────

#[test]
fn tostring_strlen() {
    let vm = run_fc(
        r#"
let r0 = 0
let r1 = 0
loop:
  r0 = strlen(tostring(12345))
  r1 = strlen(tostring(-7))
  wait()
"#,
    );
    assert_eq!(read_u32(&vm, G0), 5, "strlen(tostring(12345)) should be 5");
    assert_eq!(read_u32(&vm, G1), 2, "strlen(tostring(-7)) should be 2");
}

// ─── concat three strings via chaining ───────────────────────────────────────

#[test]
fn concat_three_strings() {
    let vm = run_fc(
        r#"
let ptr = 0
loop:
  ptr = ("a" .. "b") .. "c"
  wait()
"#,
    );
    let ptr = read_u32(&vm, G0);
    assert_eq!(read_cstr(&vm, ptr), "abc");
}

// ─── txt with dynamic string (tostring) ──────────────────────────────────────

#[test]
fn txt_dynamic_tostring_no_fault() {
    // txt(tostring(n), x, y, color) — TXTZ opcode path, no compile-time length
    let vm = run_fc(
        r#"
let n = 42
loop:
  txt(tostring(n), 0, 0, 7)
  wait()
"#,
    );
    // just verify no vm fault — screen pixels not testable in unit test
    let _ = vm;
}

// ─── txt with concat expression ───────────────────────────────────────────────

#[test]
fn txt_concat_no_fault() {
    let vm = run_fc(
        r#"
loop:
  local label = "score: " .. tostring(99)
  txt(label, 0, 0, 7)
  wait()
"#,
    );
    let _ = vm;
}

// ─── txt literal still works (TXT path unchanged) ────────────────────────────

#[test]
fn txt_literal_no_fault() {
    let vm = run_fc(
        r#"
loop:
  txt("hello", 0, 0, 7)
  wait()
"#,
    );
    let _ = vm;
}

// ─── sub() substring builtin ─────────────────────────────────────────────────

#[test]
fn sub_extracts_substring() {
    let vm = run_fc(
        r#"
let r0 = 0
let r1 = 0
loop:
  local s = "HELLO"
  r0 = strlen(sub(s, 2, 4))
  r1 = strlen(sub(s, 4, 99))
  wait()
"#,
    );
    assert_eq!(read_u32(&vm, G0), 3, "sub(s,2,4) is ELL (len 3)");
    assert_eq!(read_u32(&vm, G1), 2, "sub clamps j to len: LO (len 2)");
}
