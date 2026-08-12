# Input/Audio Query Completeness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `button_released(button_index)`, `is_sfx_playing(handle)`, and `is_music_playing()` as new Lua builtins, and document `math.randomseed` in the API registry — three small, additive query functions closing real gaps in the console's Lua API.

**Architecture:** Each new builtin mirrors an existing sibling exactly (`button_released` mirrors `button_pressed`; `is_sfx_playing`/`is_music_playing` read existing `active`/epoch state already tracked by the SFX voice pool and `MusicPlayer`). No new runtime state is introduced anywhere — every addition is a pure read of state that already exists.

**Tech Stack:** Rust (`caiven-vm` crate, `mlua` Lua bindings), integration tests in `crates/caiven-vm/tests/lua_script.rs`.

## Global Constraints

- Naming: descriptive, no abbreviations (per README "Descriptive Builtin API").
- Any new global registered in `lua_exec.rs::register_builtins` MUST also be added to the `BUILTIN_NAMES` const near the top of `lua_exec.rs` — omitting it causes a `SIGABRT` on the *next* hot-reload of a cart using that name (`is_reload_join_candidate` treats it as a script closure and calls `lua_upvaluejoin` on a native fn). See `.claude/rules/lua-api.md`.
- `crates/caiven-vm/src/vm/api_registry.rs`'s `BUILTINS`/`STDLIB` consts must stay in sync with `lua_exec.rs::register_builtins` (it's the single source of truth for Studio autocomplete — `caiven-studio/src/tauri_app.rs::api_payload` derives Studio's autocomplete from these consts directly, so no separate Studio-side codemirror edit is needed).
- Error semantics: out-of-range `button_index` on `button_released` returns `false`, not a Lua error (matches `button_down`/`button_pressed`). Stale/invalid `is_sfx_playing` handle returns `false`, not a Lua error (matches `stop_sfx_voice`'s existing silent-no-op behavior).
- No `unwrap`/`expect`/panic/unchecked indexing on a production path (`.claude/rules/rust.md`).
- Spec: `docs/superpowers/specs/2026-08-12-input-audio-query-completeness-design.md`.

---

## Task 1: `Input::just_released`

**Files:**
- Modify: `crates/caiven-vm/src/input/input.rs`
- Test: `crates/caiven-vm/src/input/input.rs` (inline `#[cfg(test)]`, new module — no existing tests in this file)

**Interfaces:**
- Consumes: `Input::cur`/`Input::prev` fields (private, already exist), `Button` enum.
- Produces: `pub fn just_released(&self, button: Button) -> bool` — later tasks (Task 2) call this directly.

- [ ] **Step 1: Write the failing test**

Add to the bottom of `crates/caiven-vm/src/input/input.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::button::Button;

    #[test]
    fn just_released_true_only_on_the_frame_after_release() {
        let mut input = Input::new();

        input.set_button(Button::A, true);
        input.end_frame();
        assert!(!input.just_released(Button::A), "still held: not released");

        input.set_button(Button::A, false);
        // Not yet latched: just_released reads prev vs cur, and end_frame
        // hasn't run yet this "frame" so prev is still true, cur is false.
        assert!(
            input.just_released(Button::A),
            "cur=false, prev=true: this is the release edge"
        );

        input.end_frame();
        assert!(
            !input.just_released(Button::A),
            "prev now latched to false: no longer the release edge"
        );
    }

    #[test]
    fn just_released_false_when_never_pressed() {
        let input = Input::new();
        assert!(!input.just_released(Button::A));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p caiven-vm --lib input::input::tests -- --nocapture`
Expected: FAIL with `no method named 'just_released' found`.

- [ ] **Step 3: Write minimal implementation**

In `crates/caiven-vm/src/input/input.rs`, add right after `just_pressed`:

```rust
    /// True only on the first frame the button reads as released (edge
    /// trigger) — mirror of `just_pressed` for the opposite edge.
    pub fn just_released(&self, button: Button) -> bool {
        !self.cur[button as usize] && self.prev[button as usize]
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p caiven-vm --lib input::input::tests -- --nocapture`
Expected: PASS (both tests).

- [ ] **Step 5: Commit**

```bash
git add crates/caiven-vm/src/input/input.rs
git commit -m "feat(input): add Input::just_released edge-trigger query"
```

---

## Task 2: `button_released` Lua builtin

**Files:**
- Modify: `crates/caiven-vm/src/vm/lua_exec.rs` (register the builtin, add to `BUILTIN_NAMES`)
- Modify: `crates/caiven-vm/src/vm/api_registry.rs` (add `ApiEntry`)
- Test: `crates/caiven-vm/tests/lua_script.rs`

**Interfaces:**
- Consumes: `Input::just_released(button: Button) -> bool` (Task 1), `Button::from_u8(u8) -> Option<Button>` (already exists, same pattern as `button_pressed`'s registration at `lua_exec.rs:935-942`).
- Produces: Lua global `button_released(button_index: u8) -> bool`.

- [ ] **Step 1: Write the failing test**

Add to `crates/caiven-vm/tests/lua_script.rs`, near the other `button_*` tests (after `lua_reads_select_at_index_six_and_nothing_beyond_it`, around line 118):

```rust
#[test]
fn lua_button_released_fires_on_the_frame_after_release_only() {
    let mut vm = make_vm();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        frame = 0
        released_on = {}
        function _update()
          frame = frame + 1
          if button_released(4) then
            released_on[#released_on + 1] = frame
          end
        end
        "#,
        &Input::new(),
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    // Frame 1: pressed. Frame 2: still held. Frame 3: released. Frame 4: up.
    let mut input = Input::new();
    input.set_button(caiven_vm::input::Button::A, true);
    vm.run_frame(&input, &font);
    input.end_frame();

    vm.run_frame(&input, &font);
    input.end_frame();

    input.set_button(caiven_vm::input::Button::A, false);
    vm.run_frame(&input, &font);
    input.end_frame();

    vm.run_frame(&input, &font);
    input.end_frame();

    assert_eq!(vm.get_fault(), None);
    let released_on = vm
        .lua_watch("released_on")
        .unwrap_or_else(|e| panic!("lua_watch failed: {e}"))
        .text;
    assert_eq!(released_on, "{3}", "should fire exactly once, on frame 3");
}

#[test]
fn lua_button_released_out_of_range_index_is_false() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        function _update()
          if button_released(99) then
            set_pixel(0, 0, 1)
          else
            set_pixel(0, 0, 2)
          end
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    vm.run_frame(&input, &font);

    assert_eq!(vm.get_fault(), None);
    // color index 2 confirms the false branch ran (no panic, no Lua error).
    assert_eq!(read_rgba(&vm, 0, 0), read_rgba_for_index(2));
}
```

`DEFAULT_COLORS[2]` (`crates/caiven-vm/src/vm/palette.rs:9`) is `(94, 44, 92)`, so:

```rust
    assert_eq!(vm.get_fault(), None);
    // color index 2 = dark purple (94, 44, 92) confirms the false branch ran.
    assert_eq!(read_rgba(&vm, 0, 0), [94, 44, 92, 255]);
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p caiven-vm --test lua_script lua_button_released -- --nocapture`
Expected: FAIL — `button_released` is a nil global (Lua error: `attempt to call a nil value`), surfacing as a fault or load error.

- [ ] **Step 3: Write minimal implementation**

In `crates/caiven-vm/src/vm/lua_exec.rs`, immediately after the `button_pressed` registration (currently lines 935-942):

```rust
    globals.set(
        "button_released",
        scope.create_function(|_, button_index: u8| {
            Ok(Button::from_u8(button_index)
                .map(|b| input.just_released(b))
                .unwrap_or(false))
        })?,
    )?;
```

Add `"button_released",` to `BUILTIN_NAMES` (near line 46, right after `"button_pressed",`):

```rust
    "button_down",
    "button_pressed",
    "button_released",
```

In `crates/caiven-vm/src/vm/api_registry.rs`, immediately after the `button_pressed` `ApiEntry` (currently lines 68-73):

```rust
    ApiEntry {
        name: "button_released",
        params: &[param!("button_index": "u8")],
        returns: "bool",
        doc: "True on the single frame button_index was released. Same indices as button_down; an out-of-range index is always false.",
    },
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p caiven-vm --test lua_script lua_button_released -- --nocapture`
Expected: PASS (both tests).

- [ ] **Step 5: Commit**

```bash
git add crates/caiven-vm/src/vm/lua_exec.rs crates/caiven-vm/src/vm/api_registry.rs crates/caiven-vm/tests/lua_script.rs
git commit -m "feat(lua-api): add button_released builtin"
```

---

## Task 3: `is_sfx_playing(handle)` / `is_music_playing()` Lua builtins

**Files:**
- Modify: `crates/caiven-vm/src/vm/lua_exec.rs` (register both builtins, add both to `BUILTIN_NAMES`)
- Modify: `crates/caiven-vm/src/vm/api_registry.rs` (add both `ApiEntry`s)
- Test: `crates/caiven-vm/tests/lua_script.rs`

**Interfaces:**
- Consumes: `unpack_sfx_handle(handle: u32) -> (u32, u32)` (already exists, `mod.rs:264-266`), `sfx_pool: &'env RefCell<&'env mut [PooledSfx; SFX_POOL_LEN]>` (already in scope in `register_builtins`, same variable `play_sfx`/`stop_sfx` use), `PooledSfx { player: SfxPlayer, epoch: u32, .. }` where `SfxPlayer.active: bool` is `pub` (`sfx.rs:36`), `music_player: &'env RefCell<&'env mut MusicPlayer>` (already in scope), `MusicPlayer.active: bool` is `pub` (`sfx.rs:78`).
- Produces: Lua globals `is_sfx_playing(handle: u32) -> bool`, `is_music_playing() -> bool`.

- [ ] **Step 1: Write the failing test**

Add to `crates/caiven-vm/tests/lua_script.rs`, near the other audio tests (after `play_sfx_does_not_disturb_concurrent_music_playback`, around line 720):

```rust
#[test]
fn is_sfx_playing_true_while_active_false_after_stop() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_section_to_ram(SFX_RAM_BASE, &[49, 12, 0, 0]);
    vm.load_lua_source(
        r#"
        handle = 0
        before_stop = false
        after_stop = true
        function _init()
          handle = play_sfx(0)
        end
        function _update()
          before_stop = is_sfx_playing(handle)
          stop_sfx(handle)
          after_stop = is_sfx_playing(handle)
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    vm.run_frame(&input, &font);

    assert_eq!(vm.get_fault(), None);
    let before = vm
        .lua_watch("before_stop")
        .unwrap_or_else(|e| panic!("lua_watch failed: {e}"))
        .text;
    let after = vm
        .lua_watch("after_stop")
        .unwrap_or_else(|e| panic!("lua_watch failed: {e}"))
        .text;
    assert_eq!(before, "true");
    assert_eq!(after, "false");
}

#[test]
fn is_sfx_playing_false_for_stale_handle_after_voice_stolen() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_section_to_ram(SFX_RAM_BASE, &[49, 12, 0, 0]);
    // Fill the pool, then trigger one more so the oldest voice (whose
    // handle we captured first) gets stolen and its epoch bumped.
    let fill_calls: String = (0..SFX_POOL_LEN)
        .map(|_| "play_sfx(0)\n".to_string())
        .collect();
    vm.load_lua_source(
        &format!(
            r#"
            first_handle = 0
            stale_result = true
            function _init()
              first_handle = play_sfx(0)
              {fill_calls}
              -- one more call than the pool has slots: steals the oldest,
              -- which is first_handle's voice.
              play_sfx(0)
              stale_result = is_sfx_playing(first_handle)
            end
            "#
        ),
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    vm.run_frame(&input, &font);

    assert_eq!(vm.get_fault(), None);
    let stale_result = vm
        .lua_watch("stale_result")
        .unwrap_or_else(|e| panic!("lua_watch failed: {e}"))
        .text;
    assert_eq!(stale_result, "false");
}

#[test]
fn is_music_playing_true_after_play_false_after_stop() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_section_to_ram(SFX_RAM_BASE, &[49, 12, 0, 0]);
    vm.load_section_to_ram(MUSIC_RAM_BASE, &[1, 0]);
    vm.load_lua_source(
        r#"
        before_stop = false
        after_stop = true
        function _init()
          play_music(0)
        end
        function _update()
          before_stop = is_music_playing()
          stop_music()
          after_stop = is_music_playing()
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    vm.run_frame(&input, &font);

    assert_eq!(vm.get_fault(), None);
    let before = vm
        .lua_watch("before_stop")
        .unwrap_or_else(|e| panic!("lua_watch failed: {e}"))
        .text;
    let after = vm
        .lua_watch("after_stop")
        .unwrap_or_else(|e| panic!("lua_watch failed: {e}"))
        .text;
    assert_eq!(before, "true");
    assert_eq!(after, "false");
}

#[test]
fn is_music_playing_false_when_never_started() {
    let mut vm = make_vm();
    let input = Input::new();
    let font = Font::empty();
    vm.load_lua_source(
        r#"
        result = true
        function _update()
          result = is_music_playing()
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    vm.run_frame(&input, &font);

    assert_eq!(vm.get_fault(), None);
    let result = vm
        .lua_watch("result")
        .unwrap_or_else(|e| panic!("lua_watch failed: {e}"))
        .text;
    assert_eq!(result, "false");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p caiven-vm --test lua_script is_sfx_playing is_music_playing -- --nocapture`
Expected: FAIL — both globals are nil, Lua error surfaces as a fault.

- [ ] **Step 3: Write minimal implementation**

In `crates/caiven-vm/src/vm/lua_exec.rs`, immediately after the `stop_sfx` registration (currently lines 1340-1347):

```rust
    globals.set(
        "is_sfx_playing",
        scope.create_function(move |_, handle: u32| {
            let (slot, epoch) = unpack_sfx_handle(handle);
            let slot = slot as usize;
            let pool = sfx_pool.borrow();
            Ok(slot < pool.len() && pool[slot].epoch == epoch && pool[slot].player.active)
        })?,
    )?;
```

`unpack_sfx_handle` is a private `fn` in `mod.rs` (line 264), already reachable from `lua_exec.rs` the same way `allocate_sfx_voice`/`release_sfx_voice` are (private items are visible to the defining module's descendants, and `lua_exec` is a submodule of `vm`). Add it to the existing `use super::{...}` block at the top of the file (currently lines 21-24):

```rust
use super::{
    AssetBankKind, AssetBanks, Camera, PooledSfx, Vm, VmFault, allocate_sfx_voice,
    release_sfx_voice, unpack_sfx_handle,
};
```

Immediately after the `stop_music` registration (currently lines 1357-1363):

```rust
    globals.set(
        "is_music_playing",
        scope.create_function(move |_, ()| Ok(music_player.borrow().active))?,
    )?;
```

Add `"is_sfx_playing",` and `"is_music_playing",` to `BUILTIN_NAMES`, near the other audio builtin names (find `"play_sfx"`, `"stop_sfx"`, `"play_music"`, `"stop_music"` in the list and add the two new names adjacent to them):

```rust
    "play_sfx",
    "stop_sfx",
    "is_sfx_playing",
    "play_music",
    "stop_music",
    "is_music_playing",
```

In `crates/caiven-vm/src/vm/api_registry.rs`, add both entries after the existing `stop_music` entry (find it near the `play_sfx`/`stop_sfx`/`play_music`/`stop_music` `ApiEntry`s):

```rust
    ApiEntry {
        name: "is_sfx_playing",
        params: &[param!("handle": "number")],
        returns: "bool",
        doc: "True if handle refers to a voice that is still actively playing. A stale handle (finished naturally, or its voice reused by a later play_sfx call) returns false, not an error.",
    },
    ApiEntry {
        name: "is_music_playing",
        params: &[],
        returns: "bool",
        doc: "True while a music track is playing.",
    },
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p caiven-vm --test lua_script is_sfx_playing is_music_playing -- --nocapture`
Expected: PASS (all four tests).

- [ ] **Step 5: Run the full VM test suite to catch any registration regressions**

Run: `cargo test -p caiven-vm`
Expected: PASS — this also exercises hot-reload tests, which are the ones that would catch a missing `BUILTIN_NAMES` entry via `SIGABRT` (per `.claude/rules/lua-api.md`).

- [ ] **Step 6: Commit**

```bash
git add crates/caiven-vm/src/vm/lua_exec.rs crates/caiven-vm/src/vm/api_registry.rs crates/caiven-vm/tests/lua_script.rs
git commit -m "feat(lua-api): add is_sfx_playing/is_music_playing builtins"
```

---

## Task 4: `math.randomseed` autocomplete entry + README cross-check

**Files:**
- Modify: `crates/caiven-vm/src/vm/api_registry.rs` (add `STDLIB` entry)
- Verify only (no edit expected): `README.md`, `docs/api-reference.md`

**Interfaces:**
- Consumes: nothing new — `math.randomseed` is real Lua 5.4 stdlib, already reachable (`StdLib::MATH` is enabled in `lua_exec.rs:1524-1530`); this task only adds documentation/autocomplete metadata.
- Produces: an `ApiEntry` in `STDLIB` so Studio's autocomplete/hover shows `math.randomseed`.

- [ ] **Step 1: Add the STDLIB entry**

In `crates/caiven-vm/src/vm/api_registry.rs`, in the `STDLIB` const, immediately after the `math.random` entry (currently lines 810-815, right before `math.huge`):

```rust
    ApiEntry {
        name: "math.randomseed",
        params: &[param!("x": "number")],
        returns: "nil",
        doc: "Set the RNG seed. The console seeds to 1 by default at cart load, so runs are deterministic unless a cart calls this itself (e.g. math.randomseed(os.time()) for per-run variety).",
    },
```

- [ ] **Step 2: Verify README/docs already cover it (no placeholder gap)**

`docs/api-reference.md:123` already documents this behavior in prose under the `core` prelude module section:
> "RNG is deterministic by default — the prelude core seeds `math.randomseed(1)` once per fresh cart load... Call `math.randomseed(os.time())` yourself for per-run variety."

Read that file's `## Input`, `## Audio`, and `### core` sections (lines 33-60, 121-130) to confirm the existing prose still reads correctly once `button_released`/`is_sfx_playing`/`is_music_playing` are added in Task 5 — no edit needed here for `math.randomseed` specifically since the prose already exists.

- [ ] **Step 3: Run the existing autocomplete-consistency test suite**

Run: `cargo test -p caiven-vm --lib prelude_consistency_tests`
Expected: PASS. (This suite checks `PRELUDE` entries specifically, not `STDLIB` — there is no existing automated check that `STDLIB` entries match real Lua globals, since they're hand-authored wrappers around real stdlib functions the sandbox already exposes via `StdLib::MATH`. No new test is needed for this task: `math.randomseed` requires no Rust registration to work, so there's no registration/entry mismatch to regress-test the way `BUILTIN_NAMES` needs.)

- [ ] **Step 4: Commit**

```bash
git add crates/caiven-vm/src/vm/api_registry.rs
git commit -m "docs(lua-api): add math.randomseed to STDLIB autocomplete entries"
```

---

## Task 5: README/docs API reference updates for the three new builtins

**Files:**
- Modify: `docs/api-reference.md`

**Interfaces:**
- Consumes: nothing (pure documentation).
- Produces: nothing consumed by later tasks — terminal documentation task.

- [ ] **Step 1: Update the Input table**

In `docs/api-reference.md`, the `## Input` section (currently lines 33-43) reads:

```markdown
## Input

| Function             | Description                                               |
| :--------------------| :----------------------------------------------------------|
| `button_down(id)`    | Button held (0=Up 1=Down 2=Left 3=Right 4=A 5=B 6=Select) |
| `button_pressed(id)` | Button pressed this frame                                  |

START is reserved by the console. It opens the pause menu, which on a
handheld is the player's only way out of a running cart, so it never reaches
cartridge code — there is no index for it. Any index outside the table above
returns `false` rather than erroring.
```

Change the table to add a row after `button_pressed`:

```markdown
| Function              | Description                                               |
| :---------------------| :----------------------------------------------------------|
| `button_down(id)`     | Button held (0=Up 1=Down 2=Left 3=Right 4=A 5=B 6=Select) |
| `button_pressed(id)`  | Button pressed this frame                                  |
| `button_released(id)` | Button released this frame                                 |
```

- [ ] **Step 2: Update the Audio table**

The `## Audio` section (currently lines 45-56) has a table ending at `stop_music()`/`set_master_volume`/etc. Add two rows, `is_sfx_playing` right after `stop_sfx`, and `is_music_playing` right after `stop_music`:

```markdown
| `play_sfx(id, opts)`     | Start SFX `id` on a free (or, if all are busy, oldest) voice. `opts.volume` (0-1, default 1) is optional. Returns a handle. Polyphonic — concurrent calls get independent voices. |
| `stop_sfx(handle)`       | Stop the voice `handle` refers to. Silent no-op if it already finished or was reused.                                                                   |
| `is_sfx_playing(handle)` | True if `handle` refers to a voice still actively playing. Stale handle returns `false`, not an error.                                                  |
| `play_music(id)`         | Play a music track, looping                                                                                                                              |
| `stop_music()`           | Stop music                                                                                                                                                |
| `is_music_playing()`     | True while a music track is playing.                                                                                                                     |
```

- [ ] **Step 3: Read the rendered result to confirm table alignment**

Markdown tables don't require column alignment to render correctly, but check the file renders sanely — run:

Run: `grep -n "button_released\|is_sfx_playing\|is_music_playing" docs/api-reference.md`
Expected: 3 matches, all inside their respective tables.

- [ ] **Step 4: Commit**

```bash
git add docs/api-reference.md
git commit -m "docs(api-reference): document button_released/is_sfx_playing/is_music_playing"
```

---

## Task 6: Example cart snippet — extend `projects/dev/audio_test/main.lua`

**Files:**
- Modify: `projects/dev/audio_test/main.lua`

This cart already demonstrates `play_sfx`/`stop_sfx`/`play_music`/`stop_music`
and already has a hand-rolled "release" check (`held_down and not
button_down(3)`) that `button_released` directly replaces with something
simpler — the ideal spot for all three new builtins in one place.

**Interfaces:**
- Consumes: `button_released`, `is_sfx_playing`, `is_music_playing` (Tasks 2-3).
- Produces: nothing consumed by later tasks — terminal example task.

- [ ] **Step 1: Replace the hand-rolled release check with `button_released`**

Current content (full file, for reference):

```lua
-- Audio test — press buttons to trigger SFX bank slots
-- UP: slot 0 (left pan)   DOWN: slot 1 (right pan)
-- LEFT: slot 2 (noise)    RIGHT: slot 3, held (release on button-up)
-- SELECT: toggle background music, to show it keeps playing under SFX
-- Paint sounds into these slots in the Caiven Studio SFX tab (F4)

held_handle = nil
held_down = false
music_active = false

function _init()
  set_palette_color(0, 10, 10, 20)
  set_palette_color(1, 255, 255, 255)
end

function _update()
  clear_screen()

  draw_text("UP: LEFT PAN", 4, 20, 1)
  draw_text("DOWN: RIGHT PAN", 4, 36, 1)
  draw_text("LEFT: NOISE", 4, 52, 1)
  draw_text("RIGHT (hold): stop_sfx on release", 4, 68, 1)
  draw_text("SELECT: toggle music", 4, 84, 1)

  if button_pressed(0) then play_sfx(0) end
  if button_pressed(1) then play_sfx(1) end
  if button_pressed(2) then play_sfx(2) end

  if button_pressed(3) then
    held_handle = play_sfx(3, {volume = 0.8})
    held_down = true
  elseif held_down and not button_down(3) then
    stop_sfx(held_handle)
    held_handle = nil
    held_down = false
  end

  if button_pressed(6) then
    if music_active then stop_music() else play_music(0) end
    music_active = not music_active
  end
end
```

Replace it with:

```lua
-- Audio test — press buttons to trigger SFX bank slots
-- UP: slot 0 (left pan)   DOWN: slot 1 (right pan)
-- LEFT: slot 2 (noise)    RIGHT: slot 3, held (stop on button_released)
-- A: slot 0 again, but only if it isn't already playing (is_sfx_playing)
-- SELECT: toggle background music, to show it keeps playing under SFX
-- Paint sounds into these slots in the Caiven Studio SFX tab (F4)

held_handle = nil

function _init()
  set_palette_color(0, 10, 10, 20)
  set_palette_color(1, 255, 255, 255)
end

function _update()
  clear_screen()

  draw_text("UP: LEFT PAN", 4, 20, 1)
  draw_text("DOWN: RIGHT PAN", 4, 36, 1)
  draw_text("LEFT: NOISE", 4, 52, 1)
  draw_text("RIGHT (hold): stop_sfx on button_released", 4, 68, 1)
  draw_text("A: replay slot 0, skipped if already playing", 4, 84, 1)
  draw_text("SELECT: toggle music", 4, 100, 1)
  draw_text(is_music_playing() and "MUSIC: ON" or "MUSIC: OFF", 4, 116, 1)

  if button_pressed(0) then play_sfx(0) end
  if button_pressed(1) then play_sfx(1) end
  if button_pressed(2) then play_sfx(2) end

  if button_pressed(3) then
    held_handle = play_sfx(3, {volume = 0.8})
  elseif button_released(3) then
    stop_sfx(held_handle)
  end

  -- Don't restart slot 0 if a previous A-press's voice is still playing.
  if button_pressed(4) and not is_sfx_playing(held_handle or 0) then
    play_sfx(0)
  end

  if button_pressed(6) then
    if is_music_playing() then stop_music() else play_music(0) end
  end
end
```

Note this drops the `music_active` local entirely — `is_music_playing()`
is now the single source of truth instead of a hand-tracked boolean that
could drift from the real player state, which is exactly the kind of gap
this spec closes.

- [ ] **Step 2: Manually load the cart and confirm no Lua fault**

This is a scripted cart change, not a Rust change, so there's no `cargo
test` for it.

Run: `cargo run -p caiven-machine -- projects/dev/audio_test`
Expected: cart loads and runs without a Lua fault printed to stdout/stderr.
Manually exercise: hold and release RIGHT (should stop cleanly on release,
not one frame late or early), press A twice quickly (second press should
not restart the sound while the first is still playing), toggle SELECT
twice (music text should flip between ON/OFF).

- [ ] **Step 3: Commit**

```bash
git add projects/dev/audio_test/main.lua
git commit -m "docs(example): demonstrate button_released and is_sfx_playing/is_music_playing"
```

---

## Final check

- [ ] Run the full targeted check script: `scripts/claude/check-lua-api.sh`
- [ ] Run: `cargo test -p caiven-vm`
- [ ] Confirm `git status` is clean and all 6 tasks are committed as separate commits.
