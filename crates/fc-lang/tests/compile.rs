use fc_lang::compile;

fn compiles(src: &str) {
    compile(src).unwrap_or_else(|e| panic!("compile failed: {}", e));
}

#[test]
fn movement_fc() {
    compiles(r#"
const SPR0 = 0x4000
const SPEED = 2
let x = 60
let y = 60
init:
  pal(0, 10, 10, 30)
loop:
  cls()
  if key(0) then
    y -= SPEED
  end
  if key(1) then
    y += SPEED
  end
  if key(2) then
    x -= SPEED
  end
  if key(3) then
    x += SPEED
  end
  spr(x, y, SPR0)
  wait()
"#);
}

#[test]
fn while_loop() {
    compiles(r#"
let i = 0
loop:
  while i < 10 do
    i += 1
  end
  wait()
"#);
}

#[test]
fn repeat_until() {
    compiles(r#"
let i = 0
loop:
  repeat
    i += 1
  until i >= 5
  wait()
"#);
}

#[test]
fn local_vars() {
    compiles(r#"
fn add(a, b)
  local result = a + b
  return result
end
loop:
  wait()
"#);
}

#[test]
fn numeric_for() {
    compiles(r#"
fn sum10()
  local s = 0
  for i = 1, 10 do
    s += i
  end
  return s
end
loop:
  wait()
"#);
}

#[test]
fn elseif_chain() {
    compiles(r#"
let x = 5
loop:
  if x < 0 then
    x = 0
  elseif x > 10 then
    x = 10
  else
    x += 1
  end
  wait()
"#);
}

#[test]
fn logical_ops() {
    compiles(r#"
let a = 1
let b = 0
let c = 0
loop:
  c = a and b
  c = a or b
  c = not a
  wait()
"#);
}

#[test]
fn break_from_loop() {
    compiles(r#"
let i = 0
loop:
  while i < 100 do
    i += 1
    if i == 5 then
      break
    end
  end
  wait()
"#);
}

#[test]
fn string_literal_txt() {
    compiles(r#"
loop:
  txt(10, 20, "hello", 7)
  wait()
"#);
}

#[test]
fn string_concat_literals() {
    compiles(r#"
loop:
  txt(0, 0, "foo" .. "bar", 7)
  wait()
"#);
}

#[test]
fn string_len_literal() {
    compiles(r#"
let n = 0
loop:
  n = #"hello"
  wait()
"#);
}

#[test]
fn string_dedup() {
    // Same literal used twice should not error
    compiles(r#"
loop:
  txt(0, 0, "hi", 7)
  txt(0, 8, "hi", 7)
  wait()
"#);
}

#[test]
fn table_empty() {
    compiles(r#"
let t = 0
loop:
  t = {}
  wait()
"#);
}

#[test]
fn table_value_fields() {
    compiles(r#"
let t = 0
loop:
  t = {10, 20, 30}
  wait()
"#);
}

#[test]
fn table_name_fields() {
    compiles(r#"
let t = 0
loop:
  t = {x=5, y=10}
  wait()
"#);
}

#[test]
fn table_index_read() {
    compiles(r#"
let t = 0
let v = 0
loop:
  t = {100, 200}
  v = t[1]
  wait()
"#);
}

#[test]
fn table_field_read() {
    compiles(r#"
let t = 0
let v = 0
loop:
  t = {hp=42}
  v = t.hp
  wait()
"#);
}

#[test]
fn table_index_write() {
    compiles(r#"
let t = 0
loop:
  t = {0}
  t[1] = 99
  wait()
"#);
}

#[test]
fn table_field_write() {
    compiles(r#"
let t = 0
loop:
  t = {score=0}
  t.score = 1
  wait()
"#);
}

#[test]
fn table_field_compound_assign() {
    compiles(r#"
let t = 0
loop:
  t = {x=0, y=0}
  t.x += 1
  t.y -= 2
  wait()
"#);
}

#[test]
fn table_index_compound_assign() {
    compiles(r#"
let t = 0
loop:
  t = {10, 20}
  t[1] += 5
  wait()
"#);
}

#[test]
fn smoke_tables_strings() {
    let src = std::fs::read_to_string("../../games/fc/demo_smoke.fc")
        .expect("demo_smoke.fc not found");
    compiles(&src);
}
