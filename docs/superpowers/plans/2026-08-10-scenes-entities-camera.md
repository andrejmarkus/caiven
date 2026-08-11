# Scenes, Entities, Camera-follow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add three pure-Lua gameplay-stdlib modules — `Scenes` (stack-based
scene manager), `Entities` (flat entity list with lifecycle), `Camera`
(follow/shake wrapper over `set_camera`) — to `prelude.lua`, closing the
biggest structural gap between "can draw a game" and "can build any type of
game" per `docs/superpowers/specs/2026-08-10-scenes-entities-camera-design.md`.

**Architecture:** Same tier as existing `Particles`/`Vec2`/`Sprite`: pure Lua
in `crates/caiven-vm/src/vm/prelude.lua`, mirrored in `api_registry.rs`'s
`PRELUDE` array for editor autocomplete/hover (Studio derives its
autocomplete list live from `api_registry.rs` via a Tauri command — no
separate manual sync file exists, confirmed in `crates/caiven-studio/src/tauri_app.rs`).
No new Rust builtins; `Camera` calls the existing `set_camera(x, y)`
builtin internally.

**Tech Stack:** Lua 5.4 (via `mlua`), Rust integration tests
(`crates/caiven-vm/tests/lua_script.rs`), binary `.cav` cartridge format
(`caiven-cart`).

## Global Constraints

- Naming: descriptive, no cryptic abbreviations (README "Descriptive
  Builtin API"; `.claude/rules/lua-api.md`).
- Every new/changed Lua name needs: implementation, VM-level tests, docs,
  autocomplete entry, an example cart, explicit error-behavior, and a
  compatibility analysis (`.claude/rules/lua-api.md`).
- No per-frame allocation without a stated reason (`.claude/rules/vm-runtime.md`)
  — `Entities.update_all()` must compact its list in place, not build a new
  table each call.
- Preserve deterministic behavior: `Camera.shake()`'s jitter uses the
  existing deterministic `random_float` — no new entropy source.
- `caiven-vm` owns execution/rendering; don't reach into `caiven-studio`
  from `caiven-vm` (`.claude/rules/rust.md`).
- Additive only — no existing cart in `carts/` uses `Scenes`, `Entities`,
  or `Camera` today, so nothing here changes prior behavior.

---

## Task 1: `Scenes` module

**Files:**
- Modify: `crates/caiven-vm/src/vm/prelude.lua` (append after line 241, end
  of file — the `Particles.count()` function)
- Modify: `crates/caiven-vm/src/vm/api_registry.rs:594` (insert `ApiEntry`
  items before the closing `];` of the `PRELUDE` array)
- Modify: `crates/caiven-vm/src/vm/lua_exec.rs:110-111` (insert `"Scenes",`
  before the closing `];` of `PRELUDE_NAMES`, so the Studio debugger's
  global-state snapshot excludes it the same way it excludes `Particles`)
- Test: `crates/caiven-vm/tests/lua_script.rs` (append new `#[test]` fns
  after the last existing test, `prelude_vec2_rng_collision_sprite_work_together`)

**Interfaces:**
- Produces: `Scenes.push(scene)`, `Scenes.pop()`, `Scenes.switch(scene)`,
  `Scenes.update()`, `Scenes.draw()`, `Scenes.current()`. `scene` is any
  plain table; `enter(scene)`/`exit(scene)`/`update(scene)`/`draw(scene)`
  fields are all optional.

- [ ] **Step 1: Write the failing tests**

Append to `crates/caiven-vm/tests/lua_script.rs`:

```rust
#[test]
fn prelude_scenes_push_pop_call_enter_and_exit_in_order() {
    let got = run_and_get(
        r#"
        local log = {}
        local menu = {
          enter = function(s) table.insert(log, "menu_enter") end,
          exit = function(s) table.insert(log, "menu_exit") end,
        }
        local game = {
          enter = function(s) table.insert(log, "game_enter") end,
          exit = function(s) table.insert(log, "game_exit") end,
        }
        Scenes.push(menu)
        Scenes.push(game)
        Scenes.pop()
        Scenes.pop()
        result = table.concat(log, ",")
        "#,
        &["result"],
    );
    assert_eq!(got, vec!["menu_enter,game_enter,game_exit,menu_exit"]);
}

#[test]
fn prelude_scenes_current_reflects_top_of_stack() {
    let got = run_and_get(
        r#"
        local a, b = {}, {}
        Scenes.push(a)
        c1 = tostring(Scenes.current() == a)
        Scenes.push(b)
        c2 = tostring(Scenes.current() == b)
        "#,
        &["c1", "c2"],
    );
    assert_eq!(got, vec!["true", "true"]);
}

#[test]
fn prelude_scenes_empty_stack_update_and_draw_are_noops() {
    let got = run_and_get(
        r#"
        local ok_update = pcall(function() Scenes.update() end)
        local ok_draw = pcall(function() Scenes.draw() end)
        result = tostring(ok_update) .. "," .. tostring(ok_draw)
        "#,
        &["result"],
    );
    assert_eq!(got, vec!["true,true"]);
}

#[test]
fn prelude_scenes_empty_stack_pop_and_switch_error() {
    let got = run_and_get(
        r#"
        local ok_pop = pcall(function() Scenes.pop() end)
        local ok_switch = pcall(function() Scenes.switch({}) end)
        result = tostring(ok_pop) .. "," .. tostring(ok_switch)
        "#,
        &["result"],
    );
    assert_eq!(got, vec!["false,false"]);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p caiven-vm --test lua_script prelude_scenes --locked`
Expected: FAIL — `Scenes` is nil (attempt to index a nil value).

- [ ] **Step 3: Implement `Scenes` in `prelude.lua`**

Append to `crates/caiven-vm/src/vm/prelude.lua`:

```lua
Scenes = { stack = {} }

function Scenes.push(scene)
  if scene.enter then scene.enter(scene) end
  table.insert(Scenes.stack, scene)
end

function Scenes.pop()
  local n = #Scenes.stack
  if n == 0 then
    error("Scenes.pop() called on an empty stack", 2)
  end
  local top = Scenes.stack[n]
  if top.exit then top.exit(top) end
  table.remove(Scenes.stack)
end

function Scenes.switch(scene)
  if #Scenes.stack == 0 then
    error("Scenes.switch() called on an empty stack", 2)
  end
  Scenes.pop()
  Scenes.push(scene)
end

function Scenes.update()
  local top = Scenes.stack[#Scenes.stack]
  if top and top.update then top.update(top) end
end

function Scenes.draw()
  local top = Scenes.stack[#Scenes.stack]
  if top and top.draw then top.draw(top) end
end

function Scenes.current()
  return Scenes.stack[#Scenes.stack]
end
```

- [ ] **Step 4: Register in `api_registry.rs` and `PRELUDE_NAMES`**

In `crates/caiven-vm/src/vm/api_registry.rs`, insert before the `PRELUDE`
array's closing `];` (currently line 594):

```rust
    ApiEntry {
        name: "Scenes.push",
        params: &[param!("scene": "table")],
        returns: "nil",
        doc: "Calls scene.enter(scene) if present, then pushes scene onto the top of the stack.",
    },
    ApiEntry {
        name: "Scenes.pop",
        params: &[],
        returns: "nil",
        doc: "Calls the top scene's exit(scene) if present, then removes it. Errors if the stack is empty.",
    },
    ApiEntry {
        name: "Scenes.switch",
        params: &[param!("scene": "table")],
        returns: "nil",
        doc: "Pops the current top scene (calling its exit) and pushes scene (calling its enter) in its place. Errors if the stack is empty.",
    },
    ApiEntry {
        name: "Scenes.update",
        params: &[],
        returns: "nil",
        doc: "Calls the top scene's update(scene) if present. A no-op on an empty stack.",
    },
    ApiEntry {
        name: "Scenes.draw",
        params: &[],
        returns: "nil",
        doc: "Calls the top scene's draw(scene) if present. A no-op on an empty stack.",
    },
    ApiEntry {
        name: "Scenes.current",
        params: &[],
        returns: "table?",
        doc: "The scene table on top of the stack, or nil if the stack is empty.",
    },
```

In `crates/caiven-vm/src/vm/lua_exec.rs`, insert `"Scenes",` before the
`PRELUDE_NAMES` array's closing `];` (currently line 111).

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p caiven-vm --test lua_script prelude_scenes --locked`
Expected: PASS (4 tests).

- [ ] **Step 6: Commit**

```bash
git add crates/caiven-vm/src/vm/prelude.lua crates/caiven-vm/src/vm/api_registry.rs crates/caiven-vm/src/vm/lua_exec.rs crates/caiven-vm/tests/lua_script.rs
git commit -m "$(cat <<'EOF'
feat(lua-api): add Scenes stack-based scene manager to gameplay stdlib

- push/pop/switch/update/draw/current, stack-based so a pause menu can
  sit on top of an active game without tearing it down
- pop/switch on an empty stack raise a Lua error; update/draw are a
  no-op on an empty stack
EOF
)"
```

---

## Task 2: `Entities` module

**Files:**
- Modify: `crates/caiven-vm/src/vm/prelude.lua` (append after the `Scenes`
  block added in Task 1)
- Modify: `crates/caiven-vm/src/vm/api_registry.rs` (insert before the
  `PRELUDE` array's closing `];`)
- Modify: `crates/caiven-vm/src/vm/lua_exec.rs` (insert `"Entities",`
  before `PRELUDE_NAMES`'s closing `];`)
- Test: `crates/caiven-vm/tests/lua_script.rs` (append)

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces: `Entities.add(e)`, `Entities.update_all()`,
  `Entities.draw_all()`, `Entities.clear()`, `Entities.count()`,
  `Entities.new()` (returns an independent list with the same five
  methods). `e` is any table; `e.update(e)`/`e.draw(e)` are optional,
  `e.dead` is an optional boolean checked after `update(e)` runs.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn prelude_entities_update_all_sweeps_dead_and_preserves_order() {
    let got = run_and_get(
        r#"
        survived = ""
        local function make(name, dies)
          return {
            update = function(e) if dies then e.dead = true end end,
            draw = function(e) survived = survived .. name end,
          }
        end
        Entities.add(make("a", false))
        Entities.add(make("b", true))
        Entities.add(make("c", false))
        Entities.update_all()
        count_after = Entities.count()
        Entities.draw_all()
        "#,
        &["count_after", "survived"],
    );
    assert_eq!(got, vec!["2", "ac"]);
}

#[test]
fn prelude_entities_new_creates_an_independent_list() {
    let got = run_and_get(
        r#"
        local other = Entities.new()
        Entities.add({})
        other.add({})
        other.add({})
        default_count = Entities.count()
        other_count = other.count()
        "#,
        &["default_count", "other_count"],
    );
    assert_eq!(got, vec!["1", "2"]);
}

#[test]
fn prelude_entities_add_non_table_errors() {
    let got = run_and_get(
        r#"
        local ok = pcall(function() Entities.add(5) end)
        result = tostring(ok)
        "#,
        &["result"],
    );
    assert_eq!(got, vec!["false"]);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p caiven-vm --test lua_script prelude_entities --locked`
Expected: FAIL — `Entities` is nil.

- [ ] **Step 3: Implement `Entities` in `prelude.lua`**

Append to `crates/caiven-vm/src/vm/prelude.lua`:

```lua
local function make_entity_list()
  local self = { list = {} }

  function self.add(e)
    if type(e) ~= "table" then
      error("Entities.add() requires a table", 2)
    end
    table.insert(self.list, e)
  end

  -- Compacts in place (no per-frame table allocation): live entities are
  -- shifted down over dead slots, then the tail is trimmed.
  function self.update_all()
    local list = self.list
    local write = 1
    for read = 1, #list do
      local e = list[read]
      if e.update then e.update(e) end
      if not e.dead then
        list[write] = e
        write = write + 1
      end
    end
    for i = #list, write, -1 do
      list[i] = nil
    end
  end

  function self.draw_all()
    for _, e in ipairs(self.list) do
      if e.draw then e.draw(e) end
    end
  end

  function self.clear()
    self.list = {}
  end

  function self.count()
    return #self.list
  end

  return self
end

Entities = make_entity_list()
Entities.new = make_entity_list
```

- [ ] **Step 4: Register in `api_registry.rs` and `PRELUDE_NAMES`**

In `crates/caiven-vm/src/vm/api_registry.rs`, insert before the `PRELUDE`
array's closing `];`:

```rust
    ApiEntry {
        name: "Entities.add",
        params: &[param!("e": "table")],
        returns: "nil",
        doc: "Adds e to the entity list. e.update(e) and e.draw(e) are called if present; e.dead = true removes it on the next update_all(). Errors if e is not a table.",
    },
    ApiEntry {
        name: "Entities.update_all",
        params: &[],
        returns: "nil",
        doc: "Calls e.update(e) on every live entity (if present), then removes any entity with e.dead == true.",
    },
    ApiEntry {
        name: "Entities.draw_all",
        params: &[],
        returns: "nil",
        doc: "Calls e.draw(e) on every live entity (if present), in the order they were added.",
    },
    ApiEntry {
        name: "Entities.clear",
        params: &[],
        returns: "nil",
        doc: "Removes all entities.",
    },
    ApiEntry {
        name: "Entities.count",
        params: &[],
        returns: "number",
        doc: "Number of live entities.",
    },
    ApiEntry {
        name: "Entities.new",
        params: &[],
        returns: "table",
        doc: "Returns a fresh, independent entity list with its own add/update_all/draw_all/clear/count methods, for carts that want one list per scene instead of the shared default list.",
    },
```

In `crates/caiven-vm/src/vm/lua_exec.rs`, insert `"Entities",` before
`PRELUDE_NAMES`'s closing `];`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p caiven-vm --test lua_script prelude_entities --locked`
Expected: PASS (3 tests).

- [ ] **Step 6: Commit**

```bash
git add crates/caiven-vm/src/vm/prelude.lua crates/caiven-vm/src/vm/api_registry.rs crates/caiven-vm/src/vm/lua_exec.rs crates/caiven-vm/tests/lua_script.rs
git commit -m "$(cat <<'EOF'
feat(lua-api): add Entities list to gameplay stdlib

- add/update_all/draw_all/clear/count, matching Particles' shape;
  update_all compacts dead entries in place rather than allocating a
  fresh table each frame
- Entities.new() gives an independent list for carts that want one per
  scene instead of the shared default list
EOF
)"
```

---

## Task 3: `Camera` module

**Files:**
- Modify: `crates/caiven-vm/src/vm/prelude.lua` (append after the
  `Entities` block added in Task 2)
- Modify: `crates/caiven-vm/src/vm/api_registry.rs` (insert before the
  `PRELUDE` array's closing `];`)
- Modify: `crates/caiven-vm/src/vm/lua_exec.rs` (insert `"Camera",`
  before `PRELUDE_NAMES`'s closing `];`)
- Test: `crates/caiven-vm/tests/lua_script.rs` (append)

**Interfaces:**
- Consumes: `Vec2` (already in `prelude.lua`), `clamp` (already in
  `prelude.lua`), `random_float` (already in `prelude.lua`), the existing
  `set_camera(x: u32, y: u32)` builtin.
- Produces: `Camera.follow(entity, opts)`, `Camera.unfollow()`,
  `Camera.shake(amount, duration)`, `Camera.update()`. `entity` must have
  `.pos` (a `Vec2`) or `.x`/`.y` numeric fields, checked in that priority
  order. `opts` is an optional table with `lerp` (default `1`, meaning
  instant snap to target — no smoothing unless the cart passes a smaller
  value), `deadzone_x`/`deadzone_y` (default `0` each): the camera only
  moves along an axis once the target's distance from the current camera
  position on that axis exceeds the deadzone.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn prelude_camera_follow_converges_toward_target_by_lerp_factor() {
    let got = run_and_get(
        r#"
        local player = { pos = Vec2.new(100, 50) }
        Camera.follow(player, { lerp = 0.5 })
        Camera.update()
        x1 = Camera.x
        Camera.update()
        x2 = Camera.x
        "#,
        &["x1", "x2"],
    );
    assert_eq!(got, vec!["50", "75"]);
}

#[test]
fn prelude_camera_shake_timer_decays_to_zero_over_duration() {
    let got = run_and_get(
        r#"
        Camera.shake(10, 3)
        Camera.update()
        t1 = Camera.shake_timer
        Camera.update()
        t2 = Camera.shake_timer
        Camera.update()
        t3 = Camera.shake_timer
        "#,
        &["t1", "t2", "t3"],
    );
    assert_eq!(got, vec!["2", "1", "0"]);
}

#[test]
fn prelude_camera_follow_errors_without_pos_or_xy() {
    let got = run_and_get(
        r#"
        local ok = pcall(function() Camera.follow({}) end)
        result = tostring(ok)
        "#,
        &["result"],
    );
    assert_eq!(got, vec!["false"]);
}

#[test]
fn prelude_camera_update_clamps_negative_target_without_faulting() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        local enemy = { pos = Vec2.new(-9999, -9999) }
        Camera.follow(enemy, { lerp = 1 })
        function _update()
          Camera.update()
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));
    vm.run_frame(&input, &font);
    // Without the >= 0 clamp, set_camera's u32 params would reject a
    // negative computed position and this would fault instead.
    assert_eq!(vm.get_fault(), None);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p caiven-vm --test lua_script prelude_camera --locked`
Expected: FAIL — `Camera` is nil.

- [ ] **Step 3: Implement `Camera` in `prelude.lua`**

Append to `crates/caiven-vm/src/vm/prelude.lua`:

```lua
Camera = { target = nil, opts = nil, x = 0, y = 0, shake_amount = 0, shake_duration = 0, shake_timer = 0 }

local function camera_entity_position(entity)
  if entity.pos then
    return entity.pos.x, entity.pos.y
  elseif entity.x and entity.y then
    return entity.x, entity.y
  end
  error("Camera.follow() requires an entity with .pos or .x/.y", 2)
end

function Camera.follow(entity, opts)
  camera_entity_position(entity) -- validate eagerly, fail at the call site
  Camera.target = entity
  Camera.opts = opts or {}
end

function Camera.unfollow()
  Camera.target = nil
  Camera.opts = nil
end

function Camera.shake(amount, duration)
  Camera.shake_amount = amount
  Camera.shake_duration = duration
  Camera.shake_timer = duration
end

function Camera.update()
  if Camera.target then
    local tx, ty = camera_entity_position(Camera.target)
    local lerp_t = (Camera.opts and Camera.opts.lerp) or 1
    local deadzone_x = (Camera.opts and Camera.opts.deadzone_x) or 0
    local deadzone_y = (Camera.opts and Camera.opts.deadzone_y) or 0
    local dx = tx - Camera.x
    local dy = ty - Camera.y
    if math.abs(dx) > deadzone_x then
      Camera.x = Camera.x + dx * lerp_t
    end
    if math.abs(dy) > deadzone_y then
      Camera.y = Camera.y + dy * lerp_t
    end
  end

  local shake_x, shake_y = 0, 0
  if Camera.shake_timer > 0 then
    local strength = Camera.shake_amount * (Camera.shake_timer / Camera.shake_duration)
    shake_x = random_float(-strength, strength)
    shake_y = random_float(-strength, strength)
    Camera.shake_timer = Camera.shake_timer - 1
  end

  local final_x = math.floor(clamp(Camera.x + shake_x, 0, math.huge))
  local final_y = math.floor(clamp(Camera.y + shake_y, 0, math.huge))
  set_camera(final_x, final_y)
end
```

- [ ] **Step 4: Register in `api_registry.rs` and `PRELUDE_NAMES`**

In `crates/caiven-vm/src/vm/api_registry.rs`, insert before the `PRELUDE`
array's closing `];`:

```rust
    ApiEntry {
        name: "Camera.follow",
        params: &[param!("entity": "table"), param!("opts": "table?")],
        returns: "nil",
        doc: "Tracks entity's position (entity.pos, a Vec2, or entity.x/entity.y) on every Camera.update() call. opts = { lerp, deadzone_x, deadzone_y }, all optional: lerp defaults to 1 (instant snap), deadzone_x/deadzone_y default to 0 (camera moves on any target movement). Errors immediately if entity has neither .pos nor .x/.y.",
    },
    ApiEntry {
        name: "Camera.unfollow",
        params: &[],
        returns: "nil",
        doc: "Stops following the current target. Camera.update() then holds its last position.",
    },
    ApiEntry {
        name: "Camera.shake",
        params: &[param!("amount": "number"), param!("duration": "number")],
        returns: "nil",
        doc: "Adds random jitter (up to +/- amount per axis) on top of the followed position for duration frames, linearly decaying to 0.",
    },
    ApiEntry {
        name: "Camera.update",
        params: &[],
        returns: "nil",
        doc: "Advances follow smoothing and shake decay by one frame, then calls set_camera() with the result. A no-op position-wise if Camera.follow() was never called. The computed position is clamped to >= 0 before calling set_camera (which takes unsigned coordinates).",
    },
```

In `crates/caiven-vm/src/vm/lua_exec.rs`, insert `"Camera",` before
`PRELUDE_NAMES`'s closing `];`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p caiven-vm --test lua_script prelude_camera --locked`
Expected: PASS (4 tests).

- [ ] **Step 6: Commit**

```bash
git add crates/caiven-vm/src/vm/prelude.lua crates/caiven-vm/src/vm/api_registry.rs crates/caiven-vm/src/vm/lua_exec.rs crates/caiven-vm/tests/lua_script.rs
git commit -m "$(cat <<'EOF'
feat(lua-api): add Camera follow/shake wrapper to gameplay stdlib

- follow(entity, opts)/unfollow()/shake(amount, duration)/update(),
  wraps the existing set_camera(x, y) builtin
- computed position clamps to >= 0 before calling set_camera, since
  that builtin takes unsigned coordinates
EOF
)"
```

---

## Task 4: Documentation

**Files:**
- Modify: `docs/api-reference.md:77-92` (the "Gameplay stdlib" table)

**Interfaces:**
- Consumes: the final function signatures from Tasks 1-3.

- [ ] **Step 1: Add table rows**

In `docs/api-reference.md`, insert three new rows into the "Gameplay
stdlib" table (after the `Sprite.new{...}` row, currently line 92):

```markdown
| `Scenes.push(scene)` / `.pop()` / `.switch(scene)` / `.update()` / `.draw()` / `.current()` | Stack-based scene manager; scene = table with optional enter/exit/update/draw |
| `Entities.add(e)` / `.update_all()` / `.draw_all()` / `.clear()` / `.count()` / `.new()`     | Entity list with lifecycle (e.dead removes on next update_all); .new() gives an independent list |
| `Camera.follow(entity, opts)` / `.unfollow()` / `.shake(amount, duration)` / `.update()`     | Wraps set_camera() with smoothed follow (opts.lerp, default 1) and decaying shake |
```

- [ ] **Step 2: Verify rendering**

Run: `grep -c '^|' docs/api-reference.md` and confirm the count increased
by exactly 3 versus `git show HEAD:docs/api-reference.md | grep -c '^|'`.

- [ ] **Step 3: Commit**

```bash
git add docs/api-reference.md
git commit -m "$(cat <<'EOF'
docs(lua-api): document Scenes, Entities, Camera in API reference

- adds the three new gameplay stdlib modules to the Gameplay stdlib
  table alongside Particles/Vec2/Sprite
EOF
)"
```

---

## Task 5: Example cart

**Files:**
- Create (throwaway, deleted at the end of this task):
  `crates/caiven-cart/examples/pack_scenes_demo.rs`
- Create (checked in): `carts/fixtures/scenes_demo.cav`
- Create (checked in): `crates/caiven-studio/resources/examples/scenes_demo.cav`
- Modify: `docs/api-reference.md:73` (Gameplay stdlib intro paragraph,
  mentioning `stdlib_demo.cav`)

**Interfaces:**
- Consumes: `caiven_cart::{CartHeader, write}` (`crates/caiven-cart/src/format.rs`);
  `Scenes`/`Entities`/`Camera`/`Vec2` from Tasks 1-3.

- [ ] **Step 1: Write the throwaway packer example**

Create `crates/caiven-cart/examples/pack_scenes_demo.rs`:

```rust
//! Throwaway: packs the Scenes/Entities/Camera example cart, then this
//! file is deleted. The two .cav files it writes are the checked-in
//! artifacts, not this generator.
use caiven_cart::{CartHeader, write};
use std::path::Path;

const SOURCE: &str = r#"
PLAYER_SPEED = 1

local function make_player()
  return {
    pos = Vec2.new(64, 64),
    update = function(e)
      local dx, dy = 0, 0
      if button_down(2) then dx = dx - 1 end
      if button_down(3) then dx = dx + 1 end
      if button_down(0) then dy = dy - 1 end
      if button_down(1) then dy = dy + 1 end
      e.pos = e.pos + Vec2.new(dx, dy) * PLAYER_SPEED
    end,
    draw = function(e)
      fill_rect(math.floor(e.pos.x), math.floor(e.pos.y), 8, 8, 11)
    end,
  }
end

local function make_enemy(x, y)
  return {
    pos = Vec2.new(x, y),
    update = function(e)
      e.pos = e.pos + Vec2.new(0, 1)
      if e.pos.y > 200 then e.dead = true end
    end,
    draw = function(e)
      fill_rect(math.floor(e.pos.x), math.floor(e.pos.y), 8, 8, 9)
    end,
  }
end

title_scene = {
  update = function(s)
    if button_pressed(4) then
      Scenes.switch(play_scene)
    end
  end,
  draw = function(s)
    clear_screen()
    draw_text("SCENES DEMO", 20, 50, 7)
    draw_text("PRESS A TO PLAY", 15, 70, 6)
  end,
}

play_scene = {
  enter = function(s)
    Entities.clear()
    player = make_player()
    Entities.add(player)
    Entities.add(make_enemy(20, 0))
    Entities.add(make_enemy(80, 0))
    Camera.follow(player, { lerp = 0.2 })
    score = 0
  end,
  update = function(s)
    Entities.update_all()
    Camera.update()
    score = score + 1
    if score > 300 then
      Scenes.switch(gameover_scene)
    end
  end,
  draw = function(s)
    clear_screen()
    Entities.draw_all()
    draw_text("SURVIVE", 2, 2, 7)
  end,
}

gameover_scene = {
  update = function(s)
    if button_pressed(4) then
      Scenes.switch(title_scene)
    end
  end,
  draw = function(s)
    clear_screen()
    draw_text("GAME OVER", 25, 50, 8)
    draw_text("PRESS A FOR TITLE", 10, 70, 6)
  end,
}

function _init()
  Scenes.push(title_scene)
end

function _update()
  Scenes.update()
end

function _draw()
  Scenes.draw()
end
"#;

fn main() {
    let mut header = CartHeader::new("Scenes Demo", "Caiven");
    header.entry_point = 0;
    let program = SOURCE.as_bytes().to_vec();
    for path in [
        "carts/fixtures/scenes_demo.cav",
        "crates/caiven-studio/resources/examples/scenes_demo.cav",
    ] {
        write(Path::new(path), &header, &program, &[])
            .unwrap_or_else(|e| panic!("failed to pack {path}: {e}"));
    }
}
```

- [ ] **Step 2: Run the packer**

Run: `cargo run -p caiven-cart --example pack_scenes_demo`
Expected: exits 0, creates `carts/fixtures/scenes_demo.cav` and
`crates/caiven-studio/resources/examples/scenes_demo.cav`.

- [ ] **Step 3: Manually verify the cart plays**

Run: `cargo run -p caiven-machine -- carts/fixtures/scenes_demo.cav`
Expected: title screen with "SCENES DEMO" / "PRESS A TO PLAY" text.
Press A: switches to the play scene — a blue player square (arrow keys /
WASD-mapped buttons move it), two red enemy squares drifting downward, the
camera panning smoothly to follow the player. Wait ~5 seconds (300 frames):
switches to a "GAME OVER" screen. Press A again: returns to the title
screen. Close the window when done.

- [ ] **Step 4: Delete the throwaway packer**

Run: `rm crates/caiven-cart/examples/pack_scenes_demo.rs`

- [ ] **Step 5: Mention the new example in the docs**

In `docs/api-reference.md`, the "Gameplay stdlib" intro paragraph
(currently line 73) mentions `stdlib_demo.cav`. Append a sentence:

```markdown
The `Scenes`/`Entities`/`Camera` trio has its own example: `carts/fixtures/scenes_demo.cav` (`cargo run -p caiven-machine -- carts/fixtures/scenes_demo.cav`) — a title screen, a play scene with a camera-followed player and two entities, and a game-over screen.
```

- [ ] **Step 6: Commit**

```bash
git add carts/fixtures/scenes_demo.cav crates/caiven-studio/resources/examples/scenes_demo.cav docs/api-reference.md
git commit -m "$(cat <<'EOF'
feat(lua-api): add Scenes/Entities/Camera example cart

- title -> play -> game-over state machine demonstrating all three
  new gameplay stdlib modules together
- packed via a throwaway caiven-cart example (deleted after running,
  matching how sprite_flip_rotate.cav was produced)
EOF
)"
```

---

## Final gate

- [ ] Run: `scripts/claude/check-lua-api.sh` — confirms `api_registry.rs`
      and `lua_exec.rs` changed together, then runs the full `caiven-vm`
      test suite.
- [ ] Run: `scripts/claude/pre-commit-gate.sh` for the full workspace pass
      before considering this feature complete.
