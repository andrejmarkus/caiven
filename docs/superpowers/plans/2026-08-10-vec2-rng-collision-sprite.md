# Vec2, deterministic RNG, circle/point collision, Sprite wrapper Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the pure-Lua gameplay stdlib (`crates/caiven-vm/src/vm/prelude.lua`) with a `Vec2` value type, deterministic-by-default RNG helpers, circle/point collision tests, and a thin `Sprite` draw wrapper — closing the math/physics gap identified in the `caiven-lua-api` "any type of game" audit.

**Architecture:** Everything is pure Lua added to `prelude.lua`, with matching entries added to three places that must stay in sync: `PRELUDE_NAMES` in `lua_exec.rs` (so the debugger's global-state snapshot excludes stdlib internals), `api_registry.rs`'s `PRELUDE` table (autocomplete/hover — Studio pulls this live via a Tauri command, no separate frontend list to touch), and `docs/api-reference.md`. No Rust runtime code changes — this is additive, no existing cart in the repo uses any of the new names.

**Tech Stack:** Lua 5.4 (`mlua`), Rust test harness in `crates/caiven-vm/tests/lua_script.rs`.

## Global Constraints

- No `unwrap`/`expect`/panic/unchecked indexing on a production path (N/A here — everything is Lua, but the two Rust files touched must still pass `cargo clippy -p caiven-vm --all-targets -- -D warnings -A unused-imports`).
- Any public Lua API change ships with: implementation, VM-level tests, README/`docs/` documentation, `api_registry.rs` sync, and a compatibility analysis — per `.claude/rules/lua-api.md`.
- Naming: descriptive, no cryptic abbreviations, matching existing prelude style (see `.claude/rules/lua-api.md` and README "Descriptive Builtin API").
- Deterministic behavior (RNG seeding) must not be silently changed later without a version/compat note — per `.claude/rules/vm-runtime.md`.
- Per `.claude/rules/testing.md`: a bug fix needs a regression test; here, every new function needs a test that would fail without it. Don't weaken existing tests.
- Comments: one line max, only for non-obvious WHY (see root `CLAUDE.md`).
- Full design rationale is in `docs/superpowers/specs/2026-08-10-vec2-rng-collision-sprite-design.md` — consult it for anything this plan doesn't spell out.

---

## Task 1: Vec2 value type

**Files:**
- Modify: `crates/caiven-vm/src/vm/prelude.lua`
- Modify: `crates/caiven-vm/src/vm/lua_exec.rs:85-101` (`PRELUDE_NAMES`)
- Modify: `crates/caiven-vm/src/vm/api_registry.rs` (append to `PRELUDE` table, before line 485 `];`)
- Modify: `docs/api-reference.md:75-84` (Gameplay stdlib table)
- Test: `crates/caiven-vm/tests/lua_script.rs`

**Interfaces:**
- Produces: `Vec2.new(x, y)` → table with metatable `Vec2`, fields `.x`, `.y`. Operators `+`, `-`, `*` (Vec2*number or number*Vec2), unary `-`, `==`. Methods `v:length()`, `v:length_squared()`, `v:normalize()`, `v:dot(other)`, `v:distance(other)`. `tostring(v)` → `"(x, y)"`.
- Later tasks (Sprite, Task 4) consume `Vec2.new(x, y)` and `.x`/`.y` field access, plus `+` for position updates.

- [ ] **Step 1: Write the failing tests**

Append to `crates/caiven-vm/tests/lua_script.rs`:

```rust
#[test]
fn prelude_vec2_operators() {
    let got = run_and_get(
        r#"
        local a = Vec2.new(1, 2)
        local b = Vec2.new(3, 4)
        local sum = a + b
        local diff = b - a
        local scaled = a * 2
        local scaled2 = 2 * a
        local neg = -a
        sum_x, sum_y = sum.x, sum.y
        diff_x, diff_y = diff.x, diff.y
        scaled_x, scaled_y = scaled.x, scaled.y
        scaled2_x, scaled2_y = scaled2.x, scaled2.y
        neg_x, neg_y = neg.x, neg.y
        eq_same = Vec2.new(1, 2) == Vec2.new(1, 2)
        eq_diff = Vec2.new(1, 2) == Vec2.new(1, 3)
        str = tostring(Vec2.new(5, 6))
        "#,
        &[
            "sum_x", "sum_y", "diff_x", "diff_y", "scaled_x", "scaled_y",
            "scaled2_x", "scaled2_y", "neg_x", "neg_y", "eq_same", "eq_diff", "str",
        ],
    );
    assert_eq!(
        got,
        vec![
            "4", "6", "2", "2", "2", "4", "2", "4", "-1", "-2", "true", "false",
            "\"(5, 6)\"",
        ]
    );
}

#[test]
fn prelude_vec2_length_normalize_dot_distance() {
    let got = run_and_get(
        r#"
        local v = Vec2.new(3, 4)
        len = v:length()
        len_sq = v:length_squared()
        local n = v:normalize()
        norm_x, norm_y = n.x, n.y
        local z = Vec2.new(0, 0)
        local zn = z:normalize()
        zero_x, zero_y = zn.x, zn.y
        dotp = Vec2.new(1, 0):dot(Vec2.new(0, 1))
        dist = Vec2.new(0, 0):distance(Vec2.new(3, 4))
        "#,
        &[
            "len", "len_sq", "norm_x", "norm_y", "zero_x", "zero_y", "dotp", "dist",
        ],
    );
    assert_eq!(
        got,
        vec!["5", "25", "0.6", "0.8", "0", "0", "0", "5"]
    );
}

#[test]
fn prelude_vec2_operator_type_mismatch_errors() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        function _update()
          local ok = pcall(function() return Vec2.new(1, 2) + 5 end)
          add_ok = ok
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.run_frame(&input, &font);
    assert_eq!(vm.get_fault(), None);
    let globals = vm.lua_globals();
    let add_ok = globals
        .iter()
        .find(|(k, _)| k == "add_ok")
        .unwrap_or_else(|| panic!("missing global add_ok"))
        .1
        .clone();
    assert_eq!(add_ok, "false");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p caiven-vm --test lua_script prelude_vec2 -- --nocapture`
Expected: FAIL — `Vec2` is nil / attempt to index a nil value.

- [ ] **Step 3: Implement Vec2 in prelude.lua**

Add near the top of `crates/caiven-vm/src/vm/prelude.lua` (before the existing `lerp`/`clamp` block):

```lua
Vec2 = {}
Vec2.__index = Vec2

local function is_vec2(v)
  return type(v) == "table" and getmetatable(v) == Vec2
end

function Vec2.new(x, y)
  return setmetatable({ x = x, y = y }, Vec2)
end

function Vec2.__add(a, b)
  if not (is_vec2(a) and is_vec2(b)) then
    error("Vec2 '+' requires two Vec2 operands", 2)
  end
  return Vec2.new(a.x + b.x, a.y + b.y)
end

function Vec2.__sub(a, b)
  if not (is_vec2(a) and is_vec2(b)) then
    error("Vec2 '-' requires two Vec2 operands", 2)
  end
  return Vec2.new(a.x - b.x, a.y - b.y)
end

function Vec2.__mul(a, b)
  if is_vec2(a) and type(b) == "number" then
    return Vec2.new(a.x * b, a.y * b)
  elseif type(a) == "number" and is_vec2(b) then
    return Vec2.new(b.x * a, b.y * a)
  end
  error("Vec2 '*' requires a Vec2 and a number", 2)
end

function Vec2.__unm(v)
  return Vec2.new(-v.x, -v.y)
end

function Vec2.__eq(a, b)
  return a.x == b.x and a.y == b.y
end

function Vec2.__tostring(v)
  return "(" .. v.x .. ", " .. v.y .. ")"
end

function Vec2:length_squared()
  return self.x * self.x + self.y * self.y
end

function Vec2:length()
  return math.sqrt(self:length_squared())
end

function Vec2:normalize()
  local len = self:length()
  if len == 0 then
    return Vec2.new(0, 0)
  end
  return Vec2.new(self.x / len, self.y / len)
end

function Vec2:dot(other)
  return self.x * other.x + self.y * other.y
end

function Vec2:distance(other)
  return (self - other):length()
end
```

Add `"Vec2"` to `PRELUDE_NAMES` in `crates/caiven-vm/src/vm/lua_exec.rs:85-101` (after `"Particles"`):

```rust
const PRELUDE_NAMES: &[&str] = &[
    "lerp",
    "clamp",
    "ease_linear",
    "ease_in_quad",
    "ease_out_quad",
    "ease_in_out_quad",
    "aabb_overlap",
    "tile_solid",
    "box_touches_solid",
    "new_tween",
    "tween_update",
    "new_anim",
    "anim_update",
    "anim_sprite",
    "Particles",
    "Vec2",
];
```

Append to `PRELUDE` in `crates/caiven-vm/src/vm/api_registry.rs`, just before the closing `];` at line 485:

```rust
    ApiEntry {
        name: "Vec2.new",
        params: &[param!("x": "number"), param!("y": "number")],
        returns: "Vec2",
        doc: "Construct a 2D vector. Supports +, -, unary -, * (Vec2 * number or number * Vec2), and == (component equality). tostring(v) gives \"(x, y)\".",
    },
    ApiEntry {
        name: "Vec2:length",
        params: &[],
        returns: "number",
        doc: "Magnitude of the vector.",
    },
    ApiEntry {
        name: "Vec2:length_squared",
        params: &[],
        returns: "number",
        doc: "Squared magnitude — avoids a sqrt when only comparing magnitudes.",
    },
    ApiEntry {
        name: "Vec2:normalize",
        params: &[],
        returns: "Vec2",
        doc: "Unit-length copy of the vector. A zero-length vector returns Vec2.new(0, 0), not an error.",
    },
    ApiEntry {
        name: "Vec2:dot",
        params: &[param!("other": "Vec2")],
        returns: "number",
        doc: "Dot product with another Vec2.",
    },
    ApiEntry {
        name: "Vec2:distance",
        params: &[param!("other": "Vec2")],
        returns: "number",
        doc: "Distance to another Vec2.",
    },
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p caiven-vm --test lua_script prelude_vec2 -- --nocapture`
Expected: PASS (all three tests).

- [ ] **Step 5: Update documentation**

In `docs/api-reference.md`, add a row to the "Gameplay stdlib" table (after the `Particles.*` row, before line 84's closing):

```markdown
| `Vec2.new(x, y)`                                                                                 | 2D vector with `+`/`-`/unary `-`/`*` (scalar)/`==`; `v:length()`, `v:length_squared()`, `v:normalize()`, `v:dot(other)`, `v:distance(other)` |
```

- [ ] **Step 6: Run full targeted check**

Run: `scripts/claude/check-lua-api.sh`
Expected: passes (fmt, clippy, and VM tests for the Lua API surface).

- [ ] **Step 7: Commit**

```bash
git add crates/caiven-vm/src/vm/prelude.lua crates/caiven-vm/src/vm/lua_exec.rs crates/caiven-vm/src/vm/api_registry.rs crates/caiven-vm/tests/lua_script.rs docs/api-reference.md
git commit -m "feat(lua-api): add Vec2 value type to gameplay stdlib"
```

---

## Task 2: Deterministic RNG helpers

**Files:**
- Modify: `crates/caiven-vm/src/vm/prelude.lua`
- Modify: `crates/caiven-vm/src/vm/lua_exec.rs:85-102` (`PRELUDE_NAMES`)
- Modify: `crates/caiven-vm/src/vm/api_registry.rs` (append to `PRELUDE`)
- Modify: `docs/api-reference.md`
- Test: `crates/caiven-vm/tests/lua_script.rs`

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces: `random_range(lo, hi)` (int, inclusive), `random_float(lo, hi)` (float `[lo, hi)`), `choice(t)`, `shuffle(t)` — all pure Lua, no new Rust-side state. Global `RTK_SEEDED` (internal seeding guard, excluded from debugger snapshot).

- [ ] **Step 1: Write the failing tests**

Append to `crates/caiven-vm/tests/lua_script.rs`:

```rust
#[test]
fn prelude_rng_fresh_loads_are_deterministic() {
    let got1 = run_and_get(
        "a = random_range(1, 1000000)\nb = random_float(0, 1)",
        &["a", "b"],
    );
    let got2 = run_and_get(
        "a = random_range(1, 1000000)\nb = random_float(0, 1)",
        &["a", "b"],
    );
    assert_eq!(
        got1, got2,
        "two fresh VMs with no explicit seed should produce identical sequences"
    );
}

#[test]
fn prelude_rng_hot_reload_does_not_reset_stream() {
    // r1/r2/r3 are assigned by the initial chunk's top-level code (runs
    // once, right after prelude.lua seeds); r4 by the hot-reloaded chunk's
    // top-level code (also runs once, on the same live Lua state). Plain
    // globals rather than a table, since `Vm::lua_watch` only parses dotted
    // identifiers, not `t[i]` indexing.
    let input = Input::new();
    let font = Font::empty();
    let mut vm = make_vm();
    vm.load_lua_source(
        r#"
        r1 = random_range(1, 1000000000)
        r2 = random_range(1, 1000000000)
        r3 = random_range(1, 1000000000)
        function _update() end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.run_frame(&input, &font);

    vm.hot_reload_lua_source(
        r#"
        r4 = random_range(1, 1000000000)
        function _update() end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("hot reload failed: {e}"));
    vm.run_frame(&input, &font);

    assert_eq!(vm.get_fault(), None);
    let globals = vm.lua_globals();
    let get = |name: &str| {
        globals
            .iter()
            .find(|(k, _)| k == name)
            .unwrap_or_else(|| panic!("missing global {name}"))
            .1
            .clone()
    };
    let (r1, r4) = (get("r1"), get("r4"));
    assert_ne!(
        r1, r4,
        "hot reload re-runs prelude.lua; the seeding guard must stop it reseeding \
         — if it reseeds, r4 restarts the sequence and equals r1"
    );
}

#[test]
fn prelude_rng_choice_and_shuffle() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        function _update()
          local t = {10, 20, 30}
          picked = choice(t)
          local ok = pcall(choice, {})
          empty_ok = ok
          local s = shuffle({1, 2, 3, 4, 5})
          sum = 0
          for _, v in ipairs(s) do sum = sum + v end
          count = #s
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.run_frame(&input, &font);
    assert_eq!(vm.get_fault(), None);
    let globals = vm.lua_globals();
    let get = |name: &str| {
        globals
            .iter()
            .find(|(k, _)| k == name)
            .unwrap_or_else(|| panic!("missing global {name}"))
            .1
            .clone()
    };
    assert!(["10", "20", "30"].contains(&get("picked").as_str()));
    assert_eq!(get("empty_ok"), "false");
    assert_eq!(get("sum"), "15");
    assert_eq!(get("count"), "5");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p caiven-vm --test lua_script prelude_rng -- --nocapture`
Expected: FAIL — `random_range`/`choice`/`shuffle` not defined.

- [ ] **Step 3: Implement RNG helpers in prelude.lua**

Add near the very top of `crates/caiven-vm/src/vm/prelude.lua`, before the `Vec2` block from Task 1 (this must run first — it's the seeding guard):

```lua
-- Deterministic by default: a fresh Lua VM has RTK_SEEDED unset, so this
-- seeds once. Hot reload re-runs prelude.lua on the *same* live VM, where
-- RTK_SEEDED is already true, so a hot reload during dev doesn't reset the
-- live RNG stream mid-game. Carts opt out via their own math.randomseed(x).
if not RTK_SEEDED then
  math.randomseed(1)
  RTK_SEEDED = true
end

function random_range(lo, hi)
  return math.random(lo, hi)
end

function random_float(lo, hi)
  return lo + math.random() * (hi - lo)
end

function choice(t)
  local n = #t
  if n == 0 then
    error("choice() requires a non-empty table", 2)
  end
  return t[math.random(n)]
end

function shuffle(t)
  for i = #t, 2, -1 do
    local j = math.random(i)
    t[i], t[j] = t[j], t[i]
  end
  return t
end
```

Add to `PRELUDE_NAMES` in `lua_exec.rs` (after `"Vec2"` from Task 1):

```rust
    "Vec2",
    "RTK_SEEDED",
    "random_range",
    "random_float",
    "choice",
    "shuffle",
];
```

Append to `PRELUDE` in `api_registry.rs`:

```rust
    ApiEntry {
        name: "random_range",
        params: &[param!("lo": "number"), param!("hi": "number")],
        returns: "number",
        doc: "Random integer in [lo, hi], inclusive. Deterministic per cart run unless the cart calls math.randomseed().",
    },
    ApiEntry {
        name: "random_float",
        params: &[param!("lo": "number"), param!("hi": "number")],
        returns: "number",
        doc: "Random float in [lo, hi).",
    },
    ApiEntry {
        name: "choice",
        params: &[param!("t": "table")],
        returns: "any",
        doc: "Random element of a non-empty array-like table. Errors on an empty table.",
    },
    ApiEntry {
        name: "shuffle",
        params: &[param!("t": "table")],
        returns: "table",
        doc: "Fisher-Yates shuffle of t, in place. Returns t.",
    },
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p caiven-vm --test lua_script prelude_rng -- --nocapture`
Expected: PASS (all three tests).

- [ ] **Step 5: Update documentation**

In `docs/api-reference.md`'s "Gameplay stdlib" table, add:

```markdown
| `random_range(lo, hi)` / `random_float(lo, hi)`                                                 | Deterministic-by-default RNG (see below) — int inclusive / float `[lo, hi)`      |
| `choice(t)` / `shuffle(t)`                                                                       | Random element of a non-empty table / in-place Fisher-Yates shuffle              |
```

Also add a short paragraph under the "Gameplay stdlib" heading (after the intro sentence, before the table):

```markdown
RNG is deterministic by default — `prelude.lua` seeds `math.randomseed(1)` once per fresh cart load (not on hot reload, so live gameplay isn't disturbed by an editor save). Call `math.randomseed(os.time())` yourself for per-run variety.
```

- [ ] **Step 6: Run full targeted check**

Run: `scripts/claude/check-lua-api.sh`
Expected: passes.

- [ ] **Step 7: Commit**

```bash
git add crates/caiven-vm/src/vm/prelude.lua crates/caiven-vm/src/vm/lua_exec.rs crates/caiven-vm/src/vm/api_registry.rs crates/caiven-vm/tests/lua_script.rs docs/api-reference.md
git commit -m "feat(lua-api): add deterministic RNG helpers to gameplay stdlib"
```

---

## Task 3: Circle/point collision helpers

**Files:**
- Modify: `crates/caiven-vm/src/vm/prelude.lua`
- Modify: `crates/caiven-vm/src/vm/lua_exec.rs:85-107` (`PRELUDE_NAMES`)
- Modify: `crates/caiven-vm/src/vm/api_registry.rs` (append to `PRELUDE`)
- Modify: `docs/api-reference.md`
- Test: `crates/caiven-vm/tests/lua_script.rs`

**Interfaces:**
- Consumes: nothing from Tasks 1-2.
- Produces: `circle_overlap(x1, y1, r1, x2, y2, r2)`, `point_in_rect(px, py, x, y, w, h)`, `point_in_circle(px, py, cx, cy, r)` — all plain booleans, same style as the existing `aabb_overlap`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/caiven-vm/tests/lua_script.rs`:

```rust
#[test]
fn prelude_circle_overlap() {
    let got = run_and_get(
        r#"
        touching = circle_overlap(0, 0, 5, 8, 0, 5)
        separate = circle_overlap(0, 0, 5, 20, 0, 5)
        tangent = circle_overlap(0, 0, 5, 10, 0, 5)
        "#,
        &["touching", "separate", "tangent"],
    );
    assert_eq!(got, vec!["true", "false", "false"]);
}

#[test]
fn prelude_point_in_rect() {
    let got = run_and_get(
        r#"
        inside = point_in_rect(5, 5, 0, 0, 10, 10)
        outside = point_in_rect(15, 5, 0, 0, 10, 10)
        on_left_edge = point_in_rect(0, 5, 0, 0, 10, 10)
        just_past_right_edge = point_in_rect(10, 5, 0, 0, 10, 10)
        "#,
        &["inside", "outside", "on_left_edge", "just_past_right_edge"],
    );
    assert_eq!(got, vec!["true", "false", "true", "false"]);
}

#[test]
fn prelude_point_in_circle() {
    let got = run_and_get(
        r#"
        inside = point_in_circle(2, 0, 0, 0, 5)
        outside = point_in_circle(10, 0, 0, 0, 5)
        on_edge = point_in_circle(5, 0, 0, 0, 5)
        "#,
        &["inside", "outside", "on_edge"],
    );
    assert_eq!(got, vec!["true", "false", "true"]);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p caiven-vm --test lua_script prelude_circle_overlap prelude_point_in -- --nocapture`
Expected: FAIL — functions not defined.

- [ ] **Step 3: Implement collision helpers in prelude.lua**

Add to `crates/caiven-vm/src/vm/prelude.lua`, directly after the existing `aabb_overlap` function:

```lua
function circle_overlap(x1, y1, r1, x2, y2, r2)
  local dx = x2 - x1
  local dy = y2 - y1
  local r = r1 + r2
  return dx * dx + dy * dy < r * r
end

function point_in_rect(px, py, x, y, w, h)
  return px >= x and px < x + w and py >= y and py < y + h
end

function point_in_circle(px, py, cx, cy, r)
  local dx = px - cx
  local dy = py - cy
  return dx * dx + dy * dy <= r * r
end
```

Add to `PRELUDE_NAMES` in `lua_exec.rs` (after the RNG names from Task 2):

```rust
    "shuffle",
    "circle_overlap",
    "point_in_rect",
    "point_in_circle",
];
```

Append to `PRELUDE` in `api_registry.rs`:

```rust
    ApiEntry {
        name: "circle_overlap",
        params: &[
            param!("x1": "number"), param!("y1": "number"), param!("r1": "number"),
            param!("x2": "number"), param!("y2": "number"), param!("r2": "number"),
        ],
        returns: "bool",
        doc: "Whether two circles overlap. Exactly-tangent circles (distance == sum of radii) count as not overlapping.",
    },
    ApiEntry {
        name: "point_in_rect",
        params: &[
            param!("px": "number"), param!("py": "number"),
            param!("x": "number"), param!("y": "number"),
            param!("w": "number"), param!("h": "number"),
        ],
        returns: "bool",
        doc: "Whether (px, py) is inside the rect (x, y, w, h). The left/top edges count as inside; the right/bottom edges don't (half-open, matching aabb_overlap's convention).",
    },
    ApiEntry {
        name: "point_in_circle",
        params: &[
            param!("px": "number"), param!("py": "number"),
            param!("cx": "number"), param!("cy": "number"), param!("r": "number"),
        ],
        returns: "bool",
        doc: "Whether (px, py) is inside or exactly on the circle centered at (cx, cy) with radius r.",
    },
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p caiven-vm --test lua_script prelude_circle_overlap prelude_point_in -- --nocapture`
Expected: PASS (all three tests).

- [ ] **Step 5: Update documentation**

In `docs/api-reference.md`'s "Gameplay stdlib" table, add (after the `aabb_overlap` row):

```markdown
| `circle_overlap(x1, y1, r1, x2, y2, r2)`                                                         | Circle overlap test                                                              |
| `point_in_rect(px, py, x, y, w, h)` / `point_in_circle(px, py, cx, cy, r)`                       | Point containment tests                                                          |
```

- [ ] **Step 6: Run full targeted check**

Run: `scripts/claude/check-lua-api.sh`
Expected: passes.

- [ ] **Step 7: Commit**

```bash
git add crates/caiven-vm/src/vm/prelude.lua crates/caiven-vm/src/vm/lua_exec.rs crates/caiven-vm/src/vm/api_registry.rs crates/caiven-vm/tests/lua_script.rs docs/api-reference.md
git commit -m "feat(lua-api): add circle and point collision helpers to gameplay stdlib"
```

---

## Task 4: Sprite wrapper

**Files:**
- Modify: `crates/caiven-vm/src/vm/prelude.lua`
- Modify: `crates/caiven-vm/src/vm/lua_exec.rs:85-110` (`PRELUDE_NAMES`)
- Modify: `crates/caiven-vm/src/vm/api_registry.rs` (append to `PRELUDE`)
- Modify: `docs/api-reference.md`
- Test: `crates/caiven-vm/tests/lua_script.rs`

**Interfaces:**
- Consumes: `Vec2.new(x, y)` and `.x`/`.y` fields from Task 1; the host builtin `sprite(sprite_id, x, y, flip_x, flip_y, rotate)`.
- Produces: `Sprite.new{sprite_id, pos, flip_x, flip_y, rotate}` → plain table with a `:draw()` method (no metatable operators — matches `Particles`' plain-function style, since no arithmetic applies to a sprite).

- [ ] **Step 1: Write the failing test**

Append to `crates/caiven-vm/tests/lua_script.rs` (uses the existing `poke_l_sprite`/`lit_offsets` helpers defined above the flip/rotate tests further down the file):

```rust
#[test]
fn prelude_sprite_wrapper_draws_via_sprite_builtin() {
    let mut vm = make_vm();
    let font = Font::empty();
    poke_l_sprite(&mut vm);
    vm.load_lua_source(
        r#"
        s = Sprite.new{ sprite_id = 0, pos = Vec2.new(10, 10), flip_x = true, flip_y = false, rotate = 0 }
        function _update() end
        function _draw() s:draw() end
        "#,
        &Input::new(),
        &Font::empty(),
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.run_frame(&Input::new(), &font);

    assert_eq!(vm.get_fault(), None);
    // Same expected pixel set as the existing flip_x builtin test: left
    // column mirrors to the right column, bottom row unchanged.
    let mut expected = std::collections::BTreeSet::new();
    for sy in 0..8u32 {
        for sx in 0..8u32 {
            if sx == 7 || sy == 7 {
                expected.insert((sx, sy));
            }
        }
    }
    assert_eq!(lit_offsets(&vm, 10, 10), expected);
}

#[test]
fn prelude_sprite_wrapper_moves_via_pos_mutation() {
    let mut vm = make_vm();
    let font = Font::empty();
    poke_l_sprite(&mut vm);
    vm.load_lua_source(
        r#"
        s = Sprite.new{ sprite_id = 0, pos = Vec2.new(0, 0) }
        function _update()
          s.pos = s.pos + Vec2.new(10, 10)
        end
        function _draw() s:draw() end
        "#,
        &Input::new(),
        &Font::empty(),
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.run_frame(&Input::new(), &font);

    assert_eq!(vm.get_fault(), None);
    let mut expected = std::collections::BTreeSet::new();
    for sy in 0..8u32 {
        for sx in 0..8u32 {
            if sx == 0 || sy == 7 {
                expected.insert((sx, sy));
            }
        }
    }
    assert_eq!(lit_offsets(&vm, 10, 10), expected);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p caiven-vm --test lua_script prelude_sprite_wrapper -- --nocapture`
Expected: FAIL — `Sprite` not defined.

- [ ] **Step 3: Implement Sprite wrapper in prelude.lua**

Add to `crates/caiven-vm/src/vm/prelude.lua`, after the `Vec2` block:

```lua
Sprite = {}
Sprite.__index = Sprite

function Sprite.new(opts)
  return setmetatable({
    sprite_id = opts.sprite_id,
    pos = opts.pos,
    flip_x = opts.flip_x or false,
    flip_y = opts.flip_y or false,
    rotate = opts.rotate or 0,
  }, Sprite)
end

function Sprite:draw()
  sprite(self.sprite_id, self.pos.x, self.pos.y, self.flip_x, self.flip_y, self.rotate)
end
```

Add to `PRELUDE_NAMES` in `lua_exec.rs` (after `"point_in_circle"` from Task 3):

```rust
    "point_in_circle",
    "Sprite",
];
```

Append to `PRELUDE` in `api_registry.rs`:

```rust
    ApiEntry {
        name: "Sprite.new",
        params: &[param!("opts": "table")],
        returns: "Sprite",
        doc: "Bundle a sprite_id, Vec2 pos, and optional flip_x/flip_y/rotate (defaults false/false/0) into one drawable object. opts = { sprite_id, pos, flip_x, flip_y, rotate }.",
    },
    ApiEntry {
        name: "Sprite:draw",
        params: &[],
        returns: "nil",
        doc: "Draw the sprite at its current pos via the sprite() builtin. Move it by reassigning .pos (e.g. s.pos = s.pos + v).",
    },
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p caiven-vm --test lua_script prelude_sprite_wrapper -- --nocapture`
Expected: PASS (both tests).

- [ ] **Step 5: Update documentation**

In `docs/api-reference.md`'s "Gameplay stdlib" table, add:

```markdown
| `Sprite.new{sprite_id, pos, flip_x, flip_y, rotate}` / `s:draw()`                                | Bundles a sprite_id + Vec2 pos (+ optional orientation) into a drawable object    |
```

- [ ] **Step 6: Run full targeted check**

Run: `scripts/claude/check-lua-api.sh`
Expected: passes.

- [ ] **Step 7: Commit**

```bash
git add crates/caiven-vm/src/vm/prelude.lua crates/caiven-vm/src/vm/lua_exec.rs crates/caiven-vm/src/vm/api_registry.rs crates/caiven-vm/tests/lua_script.rs docs/api-reference.md
git commit -m "feat(lua-api): add Sprite draw wrapper to gameplay stdlib"
```

---

## Task 5: Combined worked example + compatibility note

No existing loose-source example cart project exists in this repo to extend by hand — `carts/fixtures/stdlib_demo.cav` and `crates/caiven-studio/resources/examples/stdlib_demo.cav` are both built binary `.cav` files (custom binary format, not zip, not hand-editable), and no matching `caiven.toml`/`.lua` source project is checked in for either. Rather than inventing an unverified cart-build step outside this plan's scope, this task adds a Rust integration test that exercises all four systems together in one scenario — the same pattern every other prelude feature in this codebase is demonstrated and verified by (see Task 1-4's tests, and the existing `prelude_particles_spawn_update_expire` test) — plus a compatibility callout in the design doc's already-written Compatibility section (no further action needed there, just confirming it's covered).

**Files:**
- Test: `crates/caiven-vm/tests/lua_script.rs`

**Interfaces:**
- Consumes: `Vec2`, RNG helpers, collision helpers, and `Sprite` from Tasks 1-4.
- Produces: nothing new — this is a verification-only task.

- [ ] **Step 1: Write the combined scenario test**

Append to `crates/caiven-vm/tests/lua_script.rs`:

```rust
#[test]
fn prelude_vec2_rng_collision_sprite_work_together() {
    // A minimal "spawn a sprite at a random position, then check whether
    // the player circle touches it" scenario — the kind of code this whole
    // spec exists to make possible.
    let mut vm = make_vm();
    let font = Font::empty();
    poke_l_sprite(&mut vm);
    vm.load_lua_source(
        r#"
        enemy = Sprite.new{
          sprite_id = 0,
          pos = Vec2.new(random_range(0, 50), random_range(0, 50)),
        }
        player_pos = Vec2.new(0, 0)
        player_radius = 100

        function _update()
          local dx = enemy.pos.x - player_pos.x
          local dy = enemy.pos.y - player_pos.y
          touching = circle_overlap(
            player_pos.x, player_pos.y, player_radius,
            enemy.pos.x, enemy.pos.y, 4
          )
          contained = point_in_rect(enemy.pos.x, enemy.pos.y, 0, 0, 128, 128)
        end
        function _draw()
          enemy:draw()
        end
        "#,
        &Input::new(),
        &Font::empty(),
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.run_frame(&Input::new(), &font);

    assert_eq!(vm.get_fault(), None);
    let globals = vm.lua_globals();
    let get = |name: &str| {
        globals
            .iter()
            .find(|(k, _)| k == name)
            .unwrap_or_else(|| panic!("missing global {name}"))
            .1
            .clone()
    };
    // enemy spawns within (0,0)-(50,50), well inside a radius-100 circle at
    // the origin and inside the 128x128 screen — both true by construction.
    assert_eq!(get("touching"), "true");
    assert_eq!(get("contained"), "true");
}
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p caiven-vm --test lua_script prelude_vec2_rng_collision_sprite_work_together -- --nocapture`
Expected: PASS.

- [ ] **Step 3: Run the full targeted check and the final gate**

Run: `scripts/claude/check-lua-api.sh`
Expected: passes.

Run: `scripts/claude/pre-commit-gate.sh`
Expected: passes (full workspace check — this is the last task, so run the full gate here rather than the narrower script).

- [ ] **Step 4: Commit**

```bash
git add crates/caiven-vm/tests/lua_script.rs
git commit -m "test(lua-api): add combined Vec2/RNG/collision/Sprite scenario test"
```
