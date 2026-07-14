use fc_lang::compile;

fn compiles(src: &str) {
    compile(src).unwrap_or_else(|e| panic!("compile failed: {}", e));
}

#[test]
fn movement_fc() {
    compiles(
        r#"
const SPEED = 2
let x = 60
let y = 60
init:
  pal(0, 10, 10, 30)
loop:
  cls()
  if btn(0) then
    y -= SPEED
  end
  if btn(1) then
    y += SPEED
  end
  if btn(2) then
    x -= SPEED
  end
  if btn(3) then
    x += SPEED
  end
  spr(0, x, y)
  wait()
"#,
    );
}

#[test]
fn while_loop() {
    compiles(
        r#"
let i = 0
loop:
  while i < 10 do
    i += 1
  end
  wait()
"#,
    );
}

#[test]
fn repeat_until() {
    compiles(
        r#"
let i = 0
loop:
  repeat
    i += 1
  until i >= 5
  wait()
"#,
    );
}

#[test]
fn local_vars() {
    compiles(
        r#"
fn add(a, b)
  local result = a + b
  return result
end
loop:
  wait()
"#,
    );
}

#[test]
fn numeric_for() {
    compiles(
        r#"
fn sum10()
  local s = 0
  for i = 1, 10 do
    s += i
  end
  return s
end
loop:
  wait()
"#,
    );
}

#[test]
fn elseif_chain() {
    compiles(
        r#"
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
"#,
    );
}

#[test]
fn logical_ops() {
    compiles(
        r#"
let a = 1
let b = 0
let c = 0
loop:
  c = a and b
  c = a or b
  c = not a
  wait()
"#,
    );
}

#[test]
fn break_from_loop() {
    compiles(
        r#"
let i = 0
loop:
  while i < 100 do
    i += 1
    if i == 5 then
      break
    end
  end
  wait()
"#,
    );
}

#[test]
fn string_literal_txt() {
    compiles(
        r#"
loop:
  txt("hello", 10, 20, 7)
  wait()
"#,
    );
}

#[test]
fn string_concat_literals() {
    compiles(
        r#"
loop:
  txt("foo" .. "bar", 0, 0, 7)
  wait()
"#,
    );
}

#[test]
fn string_len_literal() {
    compiles(
        r#"
let n = 0
loop:
  n = #"hello"
  wait()
"#,
    );
}

#[test]
fn string_dedup() {
    // Same literal used twice should not error
    compiles(
        r#"
loop:
  txt("hi", 0, 0, 7)
  txt("hi", 0, 8, 7)
  wait()
"#,
    );
}

#[test]
fn table_empty() {
    compiles(
        r#"
let t = 0
loop:
  t = {}
  wait()
"#,
    );
}

#[test]
fn table_value_fields() {
    compiles(
        r#"
let t = 0
loop:
  t = {10, 20, 30}
  wait()
"#,
    );
}

#[test]
fn table_name_fields() {
    compiles(
        r#"
let t = 0
loop:
  t = {x=5, y=10}
  wait()
"#,
    );
}

#[test]
fn table_index_read() {
    compiles(
        r#"
let t = 0
let v = 0
loop:
  t = {100, 200}
  v = t[1]
  wait()
"#,
    );
}

#[test]
fn table_field_read() {
    compiles(
        r#"
let t = 0
let v = 0
loop:
  t = {hp=42}
  v = t.hp
  wait()
"#,
    );
}

#[test]
fn table_index_write() {
    compiles(
        r#"
let t = 0
loop:
  t = {0}
  t[1] = 99
  wait()
"#,
    );
}

#[test]
fn table_field_write() {
    compiles(
        r#"
let t = 0
loop:
  t = {score=0}
  t.score = 1
  wait()
"#,
    );
}

#[test]
fn table_field_compound_assign() {
    compiles(
        r#"
let t = 0
loop:
  t = {x=0, y=0}
  t.x += 1
  t.y -= 2
  wait()
"#,
    );
}

#[test]
fn table_index_compound_assign() {
    compiles(
        r#"
let t = 0
loop:
  t = {10, 20}
  t[1] += 5
  wait()
"#,
    );
}

#[test]
fn all_fc_demos_compile() {
    let dir = std::path::Path::new("../../games/fc");
    let mut count = 0;
    for entry in std::fs::read_dir(dir).expect("games/fc not found") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) == Some("fc") {
            let src = std::fs::read_to_string(&path).expect("read .fc source");
            compile(&src)
                .unwrap_or_else(|e| panic!("compile failed for {}: {}", path.display(), e));
            count += 1;
        }
    }
    assert!(count >= 4, "expected at least 4 .fc demos, found {count}");
}

#[test]
fn closure_no_upvals() {
    compiles(
        r#"
let f = 0
loop:
  f = fn(x) return x end
  wait()
"#,
    );
}

#[test]
fn closure_with_upval() {
    compiles(
        r#"
let f = 0
let base = 10
loop:
  f = fn(x) return base + x end
  wait()
"#,
    );
}

#[test]
fn closure_call_via_var() {
    compiles(
        r#"
let f = 0
let result = 0
let base = 5
loop:
  f = fn(x) return base + x end
  result = f(3)
  wait()
"#,
    );
}

#[test]
fn closure_as_fn_arg() {
    compiles(
        r#"
fn apply(cb, val)
  return cb(val)
end
let f = 0
let result = 0
let offset = 7
loop:
  f = fn(x) return offset + x end
  result = apply(f, 2)
  wait()
"#,
    );
}
