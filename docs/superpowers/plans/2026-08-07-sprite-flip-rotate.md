# Sprite flip + 90°-step rotation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the `sprite()` Lua builtin with optional `flip_x`, `flip_y`,
and `rotate` (0/90/180/270 degrees) parameters so carts can orient 8×8
sprites without duplicate mirrored art.

**Architecture:** One coordinate-transform added to the existing per-pixel
loop in `sprite()`'s closure (`crates/caiven-vm/src/vm/lua_exec.rs`) — no
new code path, no new allocation. `api_registry.rs` and
`docs/api-reference.md` are updated to match (Studio's autocomplete/hover
reads `api_registry.rs` live via a Tauri command, so no separate
Studio-side data file needs editing). A new fixture cart demonstrates the
feature.

**Tech Stack:** Rust, `mlua` (Lua 5.4), existing `caiven-vm`/`caiven-cart`
crates.

## Global Constraints

- Sprites are fixed at 8×8 (`SPRITE_SIZE = 8`,
  `crates/caiven-core/src/memory.rs:35`) — no resize/scale in scope.
- `rotate` accepts only `0, 90, 180, 270`; any other value is a Lua
  `RuntimeError`, not a silent no-op or clamp.
- Transform order is fixed: rotate first, then flip.
- All new params are optional and trail `(id, x, y)` — every existing
  3-arg `sprite()` call must produce byte-identical output to today.
- No new `unwrap`/`expect`/panic/unchecked indexing on the production path
  (`.claude/rules/rust.md`).
- `cargo fmt --all -- --check` and
  `cargo clippy -p caiven-vm --all-targets -- -D warnings -A unused-imports`
  must pass before considering Task 1 done.

---

### Task 1: `sprite()` flip + rotate transform

**Files:**
- Modify: `crates/caiven-vm/src/vm/lua_exec.rs:599-621` (the `sprite`
  closure registration)
- Test: `crates/caiven-vm/tests/lua_script.rs` (append near the existing
  `lua_pset_draws_palette_color` test, which shows the `make_vm`/
  `read_rgba` test harness this file already uses)

**Interfaces:**
- Consumes: nothing from other tasks in this plan.
- Produces: `sprite(sprite_id: u8, x: i64, y: i64, flip_x: Option<bool>,
  flip_y: Option<bool>, rotate: Option<i64>)` registered as the Lua global
  `sprite`. Later tasks (`api_registry.rs`, docs, example cart) describe
  this exact signature — keep it in sync if it changes here.

Current closure (for reference — this is what step 3 replaces):

```rust
    globals.set(
        "sprite",
        scope.create_function_mut(move |_, (sprite_id, x, y): (u8, i64, i64)| {
            let base = SPRITE_SHEET_RAM_BASE + sprite_id as usize * SPRITE_BYTES;
            let (cam_x, cam_y) = cam_offset(camera);
            let ss = sprite_size as i64;
            let mem = memory.borrow();
            let mut w = world.borrow_mut();
            for sy in 0..ss {
                for sx in 0..ss {
                    let Ok(pixel) = mem.read(base + (sy * ss + sx) as usize) else {
                        continue;
                    };
                    if pixel == 0 {
                        continue;
                    }
                    let color = palette.borrow().get_color(pixel as usize);
                    plot(&mut w, x + sx - cam_x, y + sy - cam_y, color);
                }
            }
            Ok(())
        })?,
    )?;
```

- [x] **Step 1: Write the failing tests**

Append to `crates/caiven-vm/tests/lua_script.rs`. These write a
distinctive asymmetric sprite (an "L" shape, unambiguous under every
flip/rotation combination) directly into sprite-sheet RAM via
`vm.poke_memory`, then assert on the exact set of lit pixels after each
transform.

```rust
use caiven_core::memory::SPRITE_SHEET_RAM_BASE;

/// Pokes an 8x8 "L" sprite (id 0, palette color 8) into sprite RAM:
/// a full left column plus a full bottom row. Asymmetric under every
/// flip/rotate combination, so each transform produces a distinct,
/// checkable pixel set.
fn poke_l_sprite(vm: &mut Vm) {
    let base = SPRITE_SHEET_RAM_BASE;
    for sy in 0..8usize {
        for sx in 0..8usize {
            let lit = sx == 0 || sy == 7;
            vm.poke_memory(base + sy * 8 + sx, if lit { 8 } else { 0 });
        }
    }
}

/// Returns the set of (x, y) offsets within an 8x8 region at (ox, oy)
/// that are lit (non-background) after drawing.
fn lit_offsets(vm: &Vm, ox: u32, oy: u32) -> std::collections::BTreeSet<(u32, u32)> {
    let mut set = std::collections::BTreeSet::new();
    for dy in 0..8u32 {
        for dx in 0..8u32 {
            if read_rgba(vm, ox + dx, oy + dy) != [0, 0, 0, 0] {
                set.insert((dx, dy));
            }
        }
    }
    set
}

#[test]
fn lua_sprite_no_optional_args_matches_current_output() {
    let mut vm = make_vm();
    let font = Font::empty();
    poke_l_sprite(&mut vm);
    vm.load_lua_source(
        "function _update() end\nfunction _draw() sprite(0, 10, 10) end",
        &Input::new(),
        &font,
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

#[test]
fn lua_sprite_flip_x_mirrors_horizontally() {
    let mut vm = make_vm();
    let font = Font::empty();
    poke_l_sprite(&mut vm);
    vm.load_lua_source(
        "function _update() end\nfunction _draw() sprite(0, 10, 10, true, false) end",
        &Input::new(),
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.run_frame(&Input::new(), &font);

    assert_eq!(vm.get_fault(), None);
    // Left column (sx==0) mirrors to the right column (sx==7); bottom row unchanged.
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
fn lua_sprite_flip_y_mirrors_vertically() {
    let mut vm = make_vm();
    let font = Font::empty();
    poke_l_sprite(&mut vm);
    vm.load_lua_source(
        "function _update() end\nfunction _draw() sprite(0, 10, 10, false, true) end",
        &Input::new(),
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.run_frame(&Input::new(), &font);

    assert_eq!(vm.get_fault(), None);
    // Bottom row (sy==7) mirrors to the top row (sy==0); left column unchanged.
    let mut expected = std::collections::BTreeSet::new();
    for sy in 0..8u32 {
        for sx in 0..8u32 {
            if sx == 0 || sy == 0 {
                expected.insert((sx, sy));
            }
        }
    }
    assert_eq!(lit_offsets(&vm, 10, 10), expected);
}

#[test]
fn lua_sprite_rotate_90_clockwise() {
    let mut vm = make_vm();
    let font = Font::empty();
    poke_l_sprite(&mut vm);
    vm.load_lua_source(
        "function _update() end\nfunction _draw() sprite(0, 10, 10, false, false, 90) end",
        &Input::new(),
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.run_frame(&Input::new(), &font);

    assert_eq!(vm.get_fault(), None);
    // 90 deg CW: source (sx, sy) -> (7 - sy, sx). Left column (sx==0) -> top row
    // (dy==0); bottom row (sy==7) -> right column (dx==7).
    let mut expected = std::collections::BTreeSet::new();
    for sy in 0..8u32 {
        for sx in 0..8u32 {
            if sx == 0 || sy == 7 {
                let (dx, dy) = (7 - sy, sx);
                expected.insert((dx, dy));
            }
        }
    }
    assert_eq!(lit_offsets(&vm, 10, 10), expected);
}

#[test]
fn lua_sprite_invalid_rotate_errors() {
    let mut vm = make_vm();
    let font = Font::empty();
    poke_l_sprite(&mut vm);
    vm.load_lua_source(
        "function _update() end\nfunction _draw() sprite(0, 10, 10, false, false, 45) end",
        &Input::new(),
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.run_frame(&Input::new(), &font);

    assert!(vm.get_fault().is_some(), "expected a fault for rotate=45");
}
```

`Vm::run_frame` (`crates/caiven-vm/src/vm/lua_exec.rs:1279`) already calls
`_draw()` itself after `_update()` when the cart defines it — confirmed by
reading `run_frame`'s body (`globals.get::<mlua::Function>("_draw")`,
called conditionally). No separate draw-phase method exists or is needed;
a single `vm.run_frame(&input, &font)` runs both phases.

- [x] **Step 2: Run tests to verify they fail**

Run: `cargo test -p caiven-vm --test lua_script lua_sprite -- --nocapture`
Expected: compile error or FAIL — `sprite` doesn't accept a 4th/5th/6th
argument yet, so `lua_sprite_flip_x_mirrors_horizontally` etc. fail (the
extra Lua args are silently ignored by the current 3-arg-only closure,
which mlua allows, so these calls won't error — they'll just draw the
unflipped sprite and the assertion will fail). Confirm each new test fails
for the expected reason before moving on.

- [x] **Step 3: Implement the transform**

Replace the `sprite` closure in `crates/caiven-vm/src/vm/lua_exec.rs`
(lines 599-621) with:

```rust
    globals.set(
        "sprite",
        scope.create_function_mut(
            move |_,
                  (sprite_id, x, y, flip_x, flip_y, rotate): (
                u8,
                i64,
                i64,
                Option<bool>,
                Option<bool>,
                Option<i64>,
            )| {
                let rotate_steps = match rotate.unwrap_or(0) {
                    0 => 0,
                    90 => 1,
                    180 => 2,
                    270 => 3,
                    other => {
                        return Err(mlua::Error::RuntimeError(format!(
                            "sprite: rotate must be 0, 90, 180, or 270 (got {other})"
                        )));
                    }
                };
                let flip_x = flip_x.unwrap_or(false);
                let flip_y = flip_y.unwrap_or(false);

                let base = SPRITE_SHEET_RAM_BASE + sprite_id as usize * SPRITE_BYTES;
                let (cam_x, cam_y) = cam_offset(camera);
                let ss = sprite_size as i64;
                let mem = memory.borrow();
                let mut w = world.borrow_mut();
                for sy in 0..ss {
                    for sx in 0..ss {
                        let Ok(pixel) = mem.read(base + (sy * ss + sx) as usize) else {
                            continue;
                        };
                        if pixel == 0 {
                            continue;
                        }
                        // Rotate (clockwise) about the sprite's own square, then flip.
                        let (mut rx, mut ry) = match rotate_steps {
                            0 => (sx, sy),
                            1 => (ss - 1 - sy, sx),
                            2 => (ss - 1 - sx, ss - 1 - sy),
                            _ => (sy, ss - 1 - sx),
                        };
                        if flip_x {
                            rx = ss - 1 - rx;
                        }
                        if flip_y {
                            ry = ss - 1 - ry;
                        }
                        let color = palette.borrow().get_color(pixel as usize);
                        plot(&mut w, x + rx - cam_x, y + ry - cam_y, color);
                    }
                }
                Ok(())
            },
        )?,
    )?;
```

- [x] **Step 4: Run tests to verify they pass**

Run: `cargo test -p caiven-vm --test lua_script lua_sprite -- --nocapture`
Expected: PASS for all five new tests, and
`lua_pset_draws_palette_color`/other pre-existing tests in the file still
PASS (regression check for the "byte-identical for 3-arg calls" claim).

- [x] **Step 5: Full crate check**

Run: `scripts/claude/check-rust.sh` (or, narrower:
`cargo fmt --all -- --check && cargo clippy -p caiven-vm --all-targets -- -D warnings -A unused-imports && cargo test -p caiven-vm`)
Expected: clean.

- [x] **Step 6: Commit**

```bash
git add crates/caiven-vm/src/vm/lua_exec.rs crates/caiven-vm/tests/lua_script.rs
git commit -m "$(cat <<'EOF'
feat(vm): add flip and 90-degree rotation to sprite()

- adds optional flip_x/flip_y/rotate params, trailing the existing
  (id, x, y) signature so every current call keeps working unchanged
- rotate accepts only 0/90/180/270; anything else is a Lua error
EOF
)"
```

---

### Task 2: Sync `api_registry.rs`

**Files:**
- Modify: `crates/caiven-vm/src/vm/api_registry.rs:49-58` (the `sprite`
  `ApiEntry`)

**Interfaces:**
- Consumes: the finalized `sprite(id, x, y, flip_x, flip_y, rotate)`
  signature from Task 1.
- Produces: nothing consumed by later tasks — this only feeds Studio's
  autocomplete/hover UI via the existing `ApiEntryPayload` bridge in
  `crates/caiven-studio/src/tauri_app.rs`.

- [x] **Step 1: Update the entry**

Replace the `sprite` `ApiEntry` (`crates/caiven-vm/src/vm/api_registry.rs:49-58`):

```rust
    ApiEntry {
        name: "sprite",
        params: &[
            param!("sprite_id": "u8"),
            param!("x": "number"),
            param!("y": "number"),
            param!("flip_x": "bool?"),
            param!("flip_y": "bool?"),
            param!("rotate": "number?"),
        ],
        returns: "nil",
        doc: "Draw sprite sprite_id with its top-left at (x, y), camera-relative. flip_x/flip_y mirror the sprite (default false); rotate is 0/90/180/270 degrees clockwise (default 0, any other value is a Lua error). Rotation is applied before flipping.",
    },
```

- [x] **Step 2: Verify it compiles and the registry test (if any) passes**

Run: `cargo test -p caiven-vm api_registry`
Expected: PASS (there may be no dedicated test — a clean
`cargo build -p caiven-vm` is the acceptance bar if not).

- [x] **Step 3: Commit**

```bash
git add crates/caiven-vm/src/vm/api_registry.rs
git commit -m "docs(api-registry): document sprite() flip/rotate params"
```

---

### Task 3: Update `docs/api-reference.md`

**Files:**
- Modify: `docs/api-reference.md:24` (the `sprite(id, x, y)` row under
  Sprites & Map)

**Interfaces:**
- Consumes: the finalized signature and error semantics from Task 1.
- Produces: nothing — pure documentation.

- [x] **Step 1: Update the row**

Replace line 24 of `docs/api-reference.md`:

```
| `sprite(id, x, y)`                        | Draw 8×8 sprite (camera-aware)                                 |
```

with:

```
| `sprite(id, x, y, flip_x, flip_y, rotate)` | Draw 8×8 sprite (camera-aware); `flip_x`/`flip_y` mirror it (default `false`), `rotate` is `0`/`90`/`180`/`270` degrees clockwise (default `0`, applied before flipping — any other value is a Lua error) |
```

Adjust the surrounding `|` column padding to keep the table's existing
alignment style if the file uses aligned pipes (check the current file
before saving — some rows in this table already have uneven padding, so
match whatever the nearest rows do rather than reformatting the whole
table).

- [x] **Step 2: Commit**

```bash
git add docs/api-reference.md
git commit -m "docs(api): document sprite() flip/rotate parameters"
```

---

### Task 4: Example cart demonstrating flip/rotate

**Files:**
- Create (temporary, deleted at the end of this task):
  `crates/caiven-cart/examples/gen_sprite_flip_rotate_cart.rs`
- Create (checked in): `carts/fixtures/sprite_flip_rotate.cav`

**Interfaces:**
- Consumes: the finalized `sprite()` signature from Task 1 (the generated
  cart's Lua source calls it with the new params).
- Produces: nothing — this is a standalone fixture cart, not code other
  tasks depend on.

Cart binaries in this repo (`carts/fixtures/*.cav`) are packed via
`caiven_cart::format::write` and aren't hand-editable text, so this task
generates the fixture with a throwaway `cargo run --example`, then removes
the generator — the same pattern already used for the crate's own
round-trip tests in `crates/caiven-cart/src/project.rs`.

- [x] **Step 1: Write the generator**

Create `crates/caiven-cart/examples/gen_sprite_flip_rotate_cart.rs`:

```rust
//! One-shot generator for carts/fixtures/sprite_flip_rotate.cav.
//! Run with: cargo run -p caiven-cart --example gen_sprite_flip_rotate_cart
//! Deleted after use — see docs/superpowers/plans/2026-08-07-sprite-flip-rotate.md.

use std::path::Path;

use caiven_cart::format;
use caiven_cart::header::CartHeader;
use caiven_cart::section::SectionKind;
use caiven_core::memory::SPRITE_SHEET_LEN;

const LUA_SOURCE: &str = r#"
function _update() end

function _draw()
  clear_screen()
  fill_screen(1)

  draw_text("flip_x", 4, 4, 7)
  sprite(0, 4, 14)
  sprite(0, 20, 14, true, false)

  draw_text("flip_y", 4, 34, 7)
  sprite(0, 4, 44)
  sprite(0, 20, 44, false, true)

  draw_text("rotate", 4, 64, 7)
  sprite(0, 4, 74, false, false, 0)
  sprite(0, 20, 74, false, false, 90)
  sprite(0, 36, 74, false, false, 180)
  sprite(0, 52, 74, false, false, 270)
end
"#;

fn main() {
    // An asymmetric "flag" sprite (color index 8) so every flip/rotate
    // combination looks visibly distinct: a pole down the left column,
    // a pennant along the top-left triangle, a base row along the bottom.
    #[rustfmt::skip]
    let flag: [u8; 64] = [
        8, 8, 8, 8, 8, 0, 0, 0,
        8, 8, 8, 8, 0, 0, 0, 0,
        8, 8, 8, 0, 0, 0, 0, 0,
        8, 8, 0, 0, 0, 0, 0, 0,
        8, 0, 0, 0, 0, 0, 0, 0,
        8, 0, 0, 0, 0, 0, 0, 0,
        8, 0, 0, 0, 0, 0, 0, 0,
        8, 8, 8, 8, 8, 8, 8, 8,
    ];
    let mut sprite_sheet = vec![0u8; SPRITE_SHEET_LEN];
    sprite_sheet[0..64].copy_from_slice(&flag);

    let header = CartHeader::default_for("sprite_flip_rotate");
    let path = Path::new("carts/fixtures/sprite_flip_rotate.cav");
    format::write(
        path,
        &header,
        LUA_SOURCE.as_bytes(),
        &[(SectionKind::SpriteSheet, sprite_sheet)],
    )
    .expect("writing carts/fixtures/sprite_flip_rotate.cav failed");
    println!("wrote {}", path.display());
}
```

- [x] **Step 2: Run the generator from the repo root**

Run: `cargo run -p caiven-cart --example gen_sprite_flip_rotate_cart`
Expected: prints `wrote carts/fixtures/sprite_flip_rotate.cav` and the
file appears on disk.

- [x] **Step 3: Verify the cart actually runs and looks right**

Run: `cargo run -p caiven-machine -- carts/fixtures/sprite_flip_rotate.cav`
Expected: the window shows three labeled rows — "flip_x" with two flag
sprites (one mirrored), "flip_y" with two flag sprites (one mirrored
vertically), "rotate" with four flag sprites each rotated 90° further.
Close the window when confirmed (Esc or window close).

- [x] **Step 4: Delete the generator, keep the generated cart**

```bash
git rm crates/caiven-cart/examples/gen_sprite_flip_rotate_cart.rs
git add carts/fixtures/sprite_flip_rotate.cav
git status
```

Confirm `git status` shows the new `.cav` fixture staged and the example
generator removed (not staged as an addition).

- [x] **Step 5: Commit**

```bash
git commit -m "$(cat <<'EOF'
docs(carts): add sprite_flip_rotate fixture demonstrating sprite() orientation

- shows flip_x, flip_y, and each 90-degree rotation step side by side
- generated once via a throwaway caiven-cart example, then removed;
  the packed .cav is the checked-in artifact
EOF
)"
```

---

## Self-Review Notes

- **Spec coverage:** Signature (Task 1), error semantics (Task 1, Step 3 +
  test), non-breaking claim (Task 1 regression test), Rust implementation
  location (Task 1), tests (Task 1), docs (Tasks 2 + 3), example cart
  (Task 4), transform order fixed rotate-then-flip (Task 1 doc comment +
  api_registry doc string) — all spec sections have a task.
- **Placeholder scan:** no TBD/TODO; every step has literal code or an
  exact command.
- **Type consistency:** `sprite(id, x, y, flip_x, flip_y, rotate)` used
  identically across Task 1's Rust signature, Task 2's `ApiEntry`, Task
  3's doc row, and Task 4's example Lua calls.
- **Verified against source, not assumed:** confirmed `run_frame` already
  invokes `_draw()` internally by reading `lua_exec.rs:1279` directly, so
  the test helper in Task 1 uses a single `run_frame` call rather than a
  guessed second method name.
