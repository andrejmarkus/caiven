# Lua Sandbox Hardening + Persistent Save-Data API Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the Lua sandbox's io/os/file-loading gap, then add a `dset`/`dget` + `save_data`/`load_data` builtin API so carts can persist state across sessions.

**Architecture:** `caiven-vm` gains a restricted `mlua::StdLib` mask plus a nulled-out set of base-library file-loading globals (sandbox fix), and a new in-memory `SaveData` struct (64 numeric slots + a JSON blob, both dirty-tracked) exposed to Lua as four builtins and to host code as plain byte-encode/decode methods on `Vm` — mirroring the existing `save_state.rs` RAM-snapshot pattern exactly. `caiven-vm` never touches the filesystem itself (it must stay usable from `caiven-web`); `caiven-machine` and `caiven-studio` own reading/writing the encoded bytes to disk, the same division of responsibility `save_state.rs`/`app.rs` already use for RAM snapshots.

**Tech Stack:** Rust, `mlua` 0.10 (vendored Lua 5.4), `serde_json` (already a workspace dependency, not yet a `caiven-vm` dependency).

## Global Constraints

- No `unwrap`, `expect`, panic, or unchecked indexing on a production path (`CLAUDE.md`).
- Any public Lua API change ships with: implementation, VM-level tests in `crates/caiven-vm/tests/`, `docs/api-reference.md` update, `api_registry.rs` kept in sync with `lua_exec.rs::register_builtins`, Studio autocomplete update, an example cart, explicit error-behavior documentation, and an explicit compatibility analysis (`.claude/rules/lua-api.md`).
- Existing API behavior must never change silently.
- `cargo fmt --all -- --check` and `cargo clippy -p <crate> --all-targets -- -D warnings -A unused-imports` clean before finishing each Rust task; full `scripts/claude/pre-commit-gate.sh` before the final commit.
- Cross-crate dependency direction: `caiven-core` → `caiven-cart`/`caiven-vm` → `caiven-machine`/`caiven-studio`/`caiven-port`. `caiven-vm` must never do direct filesystem I/O for cart-facing state — that belongs to the host crates (`caiven-machine`, `caiven-studio`).
- Commit message format: `type(scope): summary` subject, blank line, flat `- ...` bullet list, no blank lines between bullets, no trailing watermark line (`CLAUDE.md`).

---

## Task 1: Sandbox hardening — restrict StdLib and remove file-loading globals

**Files:**
- Modify: `crates/caiven-vm/src/vm/lua_exec.rs:1043-1044` (the `Lua::new()` call in `load_lua_source`)
- Test: `crates/caiven-vm/tests/sandbox.rs` (new file)

**Interfaces:**
- Consumes: nothing new — this only changes how the existing `Lua` instance in `Vm::load_lua_source` is constructed.
- Produces: nothing new is exposed; this task *removes* `io`, `os`, `package`, `dofile`, `loadfile`, `load`, `require` from a loaded cart's Lua globals. Later tasks must not reintroduce a path to any of these.

- [ ] **Step 1: Write the failing test**

Create `crates/caiven-vm/tests/sandbox.rs`:

```rust
use caiven_vm::vm::config::VmConfig;
use caiven_vm::vm::Vm;
use caiven_vm::input::Input;
use caiven_vm::rendering::font::Font;

fn fresh_vm() -> (Vm, Input, Font) {
    (Vm::new(VmConfig::default()), Input::default(), Font::default())
}

#[test]
fn dangerous_globals_are_absent() {
    let (mut vm, input, font) = fresh_vm();
    vm.load_lua_source("", &input, &font).expect("empty cart loads");

    for name in ["io", "os", "package", "dofile", "loadfile", "load", "require"] {
        let src = format!("assert({name} == nil, \"{name} should be nil\")");
        vm.load_lua_source(&src, &input, &font)
            .unwrap_or_else(|e| panic!("expected {name} to be nil, got error instead: {e}"));
    }
}

#[test]
fn calling_a_removed_global_is_a_lua_error() {
    let (mut vm, input, font) = fresh_vm();
    let result = vm.load_lua_source("dofile('whatever')", &input, &font);
    assert!(result.is_err(), "dofile must not be callable from a cart");
}

#[test]
fn sanctioned_stdlib_still_works() {
    let (mut vm, input, font) = fresh_vm();
    vm.load_lua_source(
        "assert(math.floor(1.5) == 1); assert(string.upper('a') == 'A'); local t = {}; table.insert(t, 1); assert(#t == 1)",
        &input,
        &font,
    )
    .expect("math/string/table must remain available");
}
```

Adjust the exact `VmConfig`/`Input`/`Font` construction calls to whatever the existing test helpers in `crates/caiven-vm/tests/` already use (check an existing test file in that directory for the established fixture pattern before finalizing this file, since `Default` may not be implemented for all three types) — keep the three `#[test]` functions and their assertions as written.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p caiven-vm --test sandbox`
Expected: `dangerous_globals_are_absent` FAILS (currently `io`, `os`, etc. are real tables, not `nil`), other two may pass already.

- [ ] **Step 3: Implement the sandbox restriction**

In `crates/caiven-vm/src/vm/lua_exec.rs`, replace line 1044:

```rust
let lua = Lua::new();
```

with:

```rust
use mlua::StdLib;

let lua = Lua::new_with(
    StdLib::COROUTINE | StdLib::TABLE | StdLib::STRING | StdLib::UTF8 | StdLib::MATH,
    mlua::LuaOptions::default(),
)
.expect("restricted stdlib set is always valid");
```

(Add `StdLib` to the existing `use mlua::{...}` import list at `lua_exec.rs:29` instead of a local `use` if that fits the file's existing import style better — check the top of the file before deciding.)

Then, immediately after `register_builtins(...)?` succeeds inside the `lua.scope(|scope| { ... })` closure in the same function (right before `lua.load(PRELUDE_SOURCE)...`), add:

```rust
for name in ["dofile", "loadfile", "load", "require"] {
    globals.set(name, mlua::Nil)?;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p caiven-vm --test sandbox`
Expected: all three tests PASS.

- [ ] **Step 5: Run the full existing VM test suite to check for regressions**

Run: `cargo test -p caiven-vm`
Expected: PASS — no existing cart, prelude helper, or test uses `os.*`/`io.*`/`dofile`/`loadfile`/`load`/`require` (confirmed by grep across `prelude.lua` and `carts/` during design), so nothing else should break.

- [ ] **Step 6: Format and lint**

Run: `cargo fmt --all -- --check && cargo clippy -p caiven-vm --all-targets -- -D warnings -A unused-imports`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add crates/caiven-vm/src/vm/lua_exec.rs crates/caiven-vm/tests/sandbox.rs
git commit -m "$(cat <<'EOF'
fix(vm): close io/os/file-loading gap in the Lua sandbox

- Lua::new() loaded mlua's ALL_SAFE set, which includes io and os despite
  the sandbox rule that cart code must not reach the filesystem/process
- dofile/loadfile/load/require are base-library globals that bypass the
  StdLib mask entirely, so they're nulled out explicitly after registration
- no existing cart or prelude code touches any of these, confirmed by grep
EOF
)"
```

---

## Task 2: `SaveData` state module in `caiven-vm`

**Files:**
- Create: `crates/caiven-vm/src/vm/save_data.rs`
- Modify: `crates/caiven-vm/src/vm/mod.rs` (add `mod save_data;` + `pub use save_data::*;` near the other `mod`/`pub use` lines at the top; add a `save_data: SaveData` field to `struct Vm` and initialize it in `Vm::new`)
- Modify: `crates/caiven-vm/Cargo.toml` (add `serde_json = { workspace = true }`)
- Test: inline `#[cfg(test)]` module in `save_data.rs`

**Interfaces:**
- Consumes: nothing from other tasks.
- Produces (used by Task 3 and by host crates in Tasks 6-7):
  - `pub struct SaveData` with `slots: [f64; 64]` and `blob: serde_json::Value` (default `serde_json::Value::Object(Default::default())`) as private fields, plus a private `dirty: bool`.
  - `impl SaveData { pub fn new() -> Self; pub fn get_slot(&self, slot: u8) -> f64; pub fn set_slot(&mut self, slot: u8, value: f64) -> Result<(), SaveDataError>; pub fn blob(&self) -> &serde_json::Value; pub fn set_blob(&mut self, value: serde_json::Value) -> Result<(), SaveDataError>; pub fn is_dirty(&self) -> bool; pub fn clear_dirty(&mut self); pub fn encode(&self) -> Vec<u8>; pub fn decode(bytes: &[u8]) -> Option<Self>; }`
  - `pub enum SaveDataError { SlotOutOfRange(u8), BlobTooLarge { size: usize, max: usize } }` implementing `std::fmt::Display` (used by Task 3 to build Lua error messages).
  - `pub const SAVE_DATA_SLOT_COUNT: usize = 64;`
  - `pub const SAVE_DATA_BLOB_MAX_BYTES: usize = 4096;`
  - On `Vm`: `pub fn save_data(&self) -> &SaveData;` and `pub fn save_data_mut(&mut self) -> &mut SaveData;` (host crates use these to call `encode()`/replace-with-`decode()`/`is_dirty()`/`clear_dirty()`; the Lua builtins in Task 3 use the same accessors through the existing `RefCell`-scoped-borrow pattern the other builtins use).

- [ ] **Step 1: Write the failing tests**

Create `crates/caiven-vm/src/vm/save_data.rs` starting with just the test module (so it fails to compile, which counts as "failing" for a from-scratch module):

```rust
//! Persistent save data: 64 numeric slots plus a JSON blob, both
//! dirty-tracked so a host (`caiven-machine`, `caiven-studio`) knows when
//! to flush `encode()`'s bytes to disk. This module never touches the
//! filesystem itself — `caiven-vm` must stay usable from `caiven-web`,
//! which has no filesystem. Encoding mirrors
//! `caiven-machine/src/shell/save_state.rs`: magic + version +
//! length-prefixed sections, `decode` rejecting anything that doesn't fit
//! rather than trusting lengths it read, since a save file is untrusted
//! the same way a `.cav` is.

use std::fmt;

pub const SAVE_DATA_SLOT_COUNT: usize = 64;
pub const SAVE_DATA_BLOB_MAX_BYTES: usize = 4096;

const MAGIC: &[u8; 4] = b"CVSD";
const FORMAT_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SaveDataError {
    SlotOutOfRange(u8),
    BlobTooLarge { size: usize, max: usize },
}

impl fmt::Display for SaveDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveDataError::SlotOutOfRange(slot) => write!(
                f,
                "slot {slot} out of range (0-{})",
                SAVE_DATA_SLOT_COUNT - 1
            ),
            SaveDataError::BlobTooLarge { size, max } => {
                write!(f, "save data is {size} bytes, over the {max}-byte limit")
            }
        }
    }
}

pub struct SaveData {
    slots: [f64; SAVE_DATA_SLOT_COUNT],
    blob: serde_json::Value,
    dirty: bool,
}

impl Default for SaveData {
    fn default() -> Self {
        Self::new()
    }
}

impl SaveData {
    pub fn new() -> Self {
        Self {
            slots: [0.0; SAVE_DATA_SLOT_COUNT],
            blob: serde_json::Value::Object(Default::default()),
            dirty: false,
        }
    }

    pub fn get_slot(&self, slot: u8) -> f64 {
        self.slots
            .get(slot as usize)
            .copied()
            .unwrap_or(0.0)
    }

    pub fn set_slot(&mut self, slot: u8, value: f64) -> Result<(), SaveDataError> {
        let cell = self
            .slots
            .get_mut(slot as usize)
            .ok_or(SaveDataError::SlotOutOfRange(slot))?;
        *cell = value;
        self.dirty = true;
        Ok(())
    }

    pub fn blob(&self) -> &serde_json::Value {
        &self.blob
    }

    pub fn set_blob(&mut self, value: serde_json::Value) -> Result<(), SaveDataError> {
        let packed = serde_json::to_vec(&value).unwrap_or_default();
        if packed.len() > SAVE_DATA_BLOB_MAX_BYTES {
            return Err(SaveDataError::BlobTooLarge {
                size: packed.len(),
                max: SAVE_DATA_BLOB_MAX_BYTES,
            });
        }
        self.blob = value;
        self.dirty = true;
        Ok(())
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    pub fn encode(&self) -> Vec<u8> {
        let blob_bytes = serde_json::to_vec(&self.blob).unwrap_or_else(|_| b"{}".to_vec());
        let mut out = Vec::with_capacity(
            4 + 2 + SAVE_DATA_SLOT_COUNT * 8 + 4 + blob_bytes.len(),
        );
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        for slot in &self.slots {
            out.extend_from_slice(&slot.to_le_bytes());
        }
        out.extend_from_slice(&(blob_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&blob_bytes);
        out
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let mut cursor = 0usize;

        let magic = bytes.get(cursor..cursor + 4)?;
        if magic != MAGIC {
            return None;
        }
        cursor += 4;

        let version = u16::from_le_bytes(bytes.get(cursor..cursor + 2)?.try_into().ok()?);
        if version != FORMAT_VERSION {
            return None;
        }
        cursor += 2;

        let mut slots = [0.0f64; SAVE_DATA_SLOT_COUNT];
        for slot in &mut slots {
            let raw: [u8; 8] = bytes.get(cursor..cursor + 8)?.try_into().ok()?;
            *slot = f64::from_le_bytes(raw);
            cursor += 8;
        }

        let blob_len = u32::from_le_bytes(bytes.get(cursor..cursor + 4)?.try_into().ok()?) as usize;
        cursor += 4;
        let blob_bytes = bytes.get(cursor..cursor + blob_len)?;
        let blob: serde_json::Value = serde_json::from_slice(blob_bytes).ok()?;

        Some(Self {
            slots,
            blob,
            dirty: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_slot_is_zero() {
        let data = SaveData::new();
        assert_eq!(data.get_slot(0), 0.0);
        assert_eq!(data.get_slot(63), 0.0);
    }

    #[test]
    fn set_slot_out_of_range_errors() {
        let mut data = SaveData::new();
        assert_eq!(data.set_slot(64, 1.0), Err(SaveDataError::SlotOutOfRange(64)));
    }

    #[test]
    fn set_slot_marks_dirty() {
        let mut data = SaveData::new();
        assert!(!data.is_dirty());
        data.set_slot(0, 42.0).unwrap();
        assert!(data.is_dirty());
        data.clear_dirty();
        assert!(!data.is_dirty());
    }

    #[test]
    fn oversized_blob_is_rejected_without_mutating_state() {
        let mut data = SaveData::new();
        let huge = serde_json::json!({ "s": "x".repeat(SAVE_DATA_BLOB_MAX_BYTES) });
        let err = data.set_blob(huge).unwrap_err();
        assert!(matches!(err, SaveDataError::BlobTooLarge { .. }));
        assert_eq!(data.blob(), &serde_json::Value::Object(Default::default()));
        assert!(!data.is_dirty());
    }

    #[test]
    fn round_trips_slots_and_blob() {
        let mut data = SaveData::new();
        data.set_slot(0, 42.0).unwrap();
        data.set_slot(63, -1.5).unwrap();
        data.set_blob(serde_json::json!({ "level": 3, "name": "ok" })).unwrap();

        let bytes = data.encode();
        let decoded = SaveData::decode(&bytes).expect("valid save data");

        assert_eq!(decoded.get_slot(0), 42.0);
        assert_eq!(decoded.get_slot(63), -1.5);
        assert_eq!(decoded.blob(), &serde_json::json!({ "level": 3, "name": "ok" }));
        assert!(!decoded.is_dirty());
    }

    #[test]
    fn rejects_truncated_bytes() {
        let data = SaveData::new();
        let bytes = data.encode();
        assert!(SaveData::decode(&bytes[..bytes.len() - 2]).is_none());
        assert!(SaveData::decode(&[]).is_none());
    }

    #[test]
    fn rejects_bad_magic() {
        let data = SaveData::new();
        let mut bytes = data.encode();
        bytes[0] = b'X';
        assert!(SaveData::decode(&bytes).is_none());
    }

    #[test]
    fn rejects_unknown_version() {
        let data = SaveData::new();
        let mut bytes = data.encode();
        bytes[4..6].copy_from_slice(&99u16.to_le_bytes());
        assert!(SaveData::decode(&bytes).is_none());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p caiven-vm save_data`
Expected: fails to compile — `serde_json` isn't a `caiven-vm` dependency yet.

- [ ] **Step 3: Add the dependency and wire the module in**

In `crates/caiven-vm/Cargo.toml`, next to the existing `serde = { workspace = true }` line, add:

```toml
serde_json = { workspace = true }
```

In `crates/caiven-vm/src/vm/mod.rs`, add near the other `mod`/`pub use` declarations at the top:

```rust
mod save_data;
pub use save_data::{SaveData, SaveDataError, SAVE_DATA_BLOB_MAX_BYTES, SAVE_DATA_SLOT_COUNT};
```

Add a field to `struct Vm` (next to `asset_banks: AssetBanks,`):

```rust
save_data: SaveData,
```

Initialize it in `Vm::new` (next to `asset_banks: AssetBanks::new(),`):

```rust
save_data: SaveData::new(),
```

Add accessor methods on `impl Vm` (next to `pub fn collision_types` or another small accessor):

```rust
pub fn save_data(&self) -> &SaveData {
    &self.save_data
}

pub fn save_data_mut(&mut self) -> &mut SaveData {
    &mut self.save_data
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p caiven-vm save_data`
Expected: all 7 tests in `save_data::tests` PASS.

- [ ] **Step 5: Run the full existing VM test suite to check for regressions**

Run: `cargo test -p caiven-vm`
Expected: PASS.

- [ ] **Step 6: Format and lint**

Run: `cargo fmt --all -- --check && cargo clippy -p caiven-vm --all-targets -- -D warnings -A unused-imports`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add crates/caiven-vm/Cargo.toml crates/caiven-vm/src/vm/save_data.rs crates/caiven-vm/src/vm/mod.rs
git commit -m "$(cat <<'EOF'
feat(vm): add in-memory SaveData state (slots + JSON blob)

- 64 f64 slots plus a size-capped JSON blob, dirty-tracked so a host can
  tell when encode()'s bytes need flushing to disk
- caiven-vm does no filesystem I/O itself, same as every other RAM-backed
  state it owns — save_state.rs's RAM-snapshot pattern mirrored here
- decode() rejects anything that doesn't fit rather than trusting lengths
  it read, matching the cart-parsing untrusted-input rule
EOF
)"
```

---

## Task 3: `dset`/`dget`/`save_data`/`load_data` Lua builtins

**Files:**
- Modify: `crates/caiven-vm/src/vm/lua_exec.rs` (add to `BUILTIN_NAMES`, add registration inside `register_builtins`)
- Test: `crates/caiven-vm/tests/save_data_api.rs` (new file)

**Interfaces:**
- Consumes: `Vm::save_data_mut()` / `SaveData::{get_slot, set_slot, blob, set_blob}` / `SaveDataError` from Task 2.
- Produces: four new Lua globals available to every cart: `dset(slot, value)`, `dget(slot)`, `save_data(table)`, `load_data()`.

First, locate the existing `register_builtins` function signature in `lua_exec.rs` (it takes a `save_data: &RefCell<&mut SaveData>`-style parameter for each piece of `Vm` state it exposes — follow the exact same borrowing pattern already used for `memory`/`palette`/`camera` in that function, e.g. `let memory = RefCell::new(&mut self.memory);` at `lua_exec.rs:1052` and its corresponding parameter in `register_builtins`). Add a `save_data: &RefCell<&mut SaveData>` parameter to `register_builtins`, and in `Vm::load_lua_source` add `let save_data = RefCell::new(&mut self.save_data);` alongside the other `RefCell::new(&mut self....)` lines, then pass `&save_data` into the `register_builtins(...)` call.

- [ ] **Step 1: Write the failing test**

Create `crates/caiven-vm/tests/save_data_api.rs` (mirror whatever fixture-construction pattern Task 1's `sandbox.rs` ended up using for `Vm`/`Input`/`Font`):

```rust
use caiven_vm::vm::config::VmConfig;
use caiven_vm::vm::Vm;
use caiven_vm::input::Input;
use caiven_vm::rendering::font::Font;

fn fresh_vm() -> (Vm, Input, Font) {
    (Vm::new(VmConfig::default()), Input::default(), Font::default())
}

#[test]
fn dset_dget_round_trip_and_default_zero() {
    let (mut vm, input, font) = fresh_vm();
    vm.load_lua_source(
        "dset(0, 42); assert(dget(0) == 42); assert(dget(1) == 0)",
        &input,
        &font,
    )
    .expect("dset/dget round trip");
}

#[test]
fn dset_out_of_range_slot_is_a_lua_error() {
    let (mut vm, input, font) = fresh_vm();
    let result = vm.load_lua_source("dset(64, 1)", &input, &font);
    assert!(result.is_err(), "slot 64 is out of the 0-63 range");
}

#[test]
fn dget_out_of_range_slot_is_a_lua_error() {
    let (mut vm, input, font) = fresh_vm();
    let result = vm.load_lua_source("dget(64)", &input, &font);
    assert!(result.is_err(), "slot 64 is out of the 0-63 range");
}

#[test]
fn load_data_with_no_prior_save_returns_empty_table() {
    let (mut vm, input, font) = fresh_vm();
    vm.load_lua_source(
        "local t = load_data(); local count = 0; for _ in pairs(t) do count = count + 1 end; assert(count == 0)",
        &input,
        &font,
    )
    .expect("load_data with nothing saved yet returns {}");
}

#[test]
fn save_data_load_data_round_trip() {
    let (mut vm, input, font) = fresh_vm();
    vm.load_lua_source(
        "save_data({ level = 3, name = 'ok' }); local t = load_data(); assert(t.level == 3); assert(t.name == 'ok')",
        &input,
        &font,
    )
    .expect("save_data/load_data round trip");
}

#[test]
fn save_data_over_size_cap_is_a_lua_error() {
    let (mut vm, input, font) = fresh_vm();
    let src = "save_data({ s = string.rep('x', 5000) })";
    let result = vm.load_lua_source(src, &input, &font);
    assert!(result.is_err(), "5000+ bytes must exceed the 4096-byte cap");
}

#[test]
fn dset_marks_vm_save_data_dirty() {
    let (mut vm, input, font) = fresh_vm();
    assert!(!vm.save_data().is_dirty());
    vm.load_lua_source("dset(0, 1)", &input, &font).expect("dset succeeds");
    assert!(vm.save_data().is_dirty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p caiven-vm --test save_data_api`
Expected: fails to compile — `dset`/`dget`/`save_data`/`load_data` aren't registered yet.

- [ ] **Step 3: Implement the builtins**

In `crates/caiven-vm/src/vm/lua_exec.rs`, add to `BUILTIN_NAMES` (`lua_exec.rs:36-72`):

```rust
"dset",
"dget",
"save_data",
"load_data",
```

Inside `register_builtins`, following the exact pattern of an existing simple builtin like `set_pixel` (borrow the `RefCell`, return a Lua error via `mlua::Error::RuntimeError` on bad input — check how an existing bounds-checked builtin such as `set_palette_color` in `api_registry.rs`/`lua_exec.rs` raises its error, and match that exact error-construction style), add:

```rust
{
    let save_data = save_data;
    globals.set(
        "dset",
        scope.create_function(move |_, (slot, value): (i64, f64)| {
            let slot: u8 = slot.try_into().map_err(|_| {
                mlua::Error::RuntimeError(SaveDataError::SlotOutOfRange(slot as u8).to_string())
            })?;
            save_data
                .borrow_mut()
                .set_slot(slot, value)
                .map_err(|e| mlua::Error::RuntimeError(e.to_string()))
        })?,
    )?;
}

globals.set(
    "dget",
    scope.create_function(move |_, slot: i64| {
        let slot: u8 = slot.try_into().unwrap_or(u8::MAX);
        if slot as usize >= crate::vm::SAVE_DATA_SLOT_COUNT {
            return Err(mlua::Error::RuntimeError(
                SaveDataError::SlotOutOfRange(slot).to_string(),
            ));
        }
        Ok(save_data.borrow().get_slot(slot))
    })?,
)?;

globals.set(
    "save_data",
    scope.create_function(move |lua, table: mlua::Table| {
        let value: serde_json::Value = lua.from_value(mlua::Value::Table(table))?;
        save_data
            .borrow_mut()
            .set_blob(value)
            .map_err(|e| mlua::Error::RuntimeError(e.to_string()))
    })?,
)?;

globals.set(
    "load_data",
    scope.create_function(move |lua, ()| {
        lua.to_value(save_data.borrow().blob())
    })?,
)?;
```

`dset`'s bounds check goes through `SaveData::set_slot`'s own `Result`, and `dget`'s is the explicit `if slot as usize >= SAVE_DATA_SLOT_COUNT` guard shown above — both end up raising the same `SaveDataError::SlotOutOfRange(slot).to_string()` message, so the two builtins report an out-of-range slot identically.

`lua.from_value`/`lua.to_value` require the `mlua` `serialize` feature — check `crates/caiven-vm/Cargo.toml`'s existing `mlua` dependency line; if it doesn't already list `"serialize"` in `features`, add it (it comes from the workspace `mlua = { version = "0.10", features = ["lua54", "vendored"] }` — override in `caiven-vm/Cargo.toml` with the full feature list including `"serialize"` if the crate doesn't already do a per-crate override).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p caiven-vm --test save_data_api`
Expected: all 7 tests PASS.

- [ ] **Step 5: Run the full existing VM test suite to check for regressions**

Run: `cargo test -p caiven-vm`
Expected: PASS.

- [ ] **Step 6: Format and lint**

Run: `cargo fmt --all -- --check && cargo clippy -p caiven-vm --all-targets -- -D warnings -A unused-imports`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add crates/caiven-vm/src/vm/lua_exec.rs crates/caiven-vm/Cargo.toml crates/caiven-vm/tests/save_data_api.rs
git commit -m "$(cat <<'EOF'
feat(vm): expose dset/dget/save_data/load_data to cart Lua

- dset/dget: 64 numeric slots, out-of-range slot is a Lua error, unset
  slot reads back 0
- save_data/load_data: JSON blob capped at 4KiB packed, oversized or
  unserializable input is a Lua error naming the problem; load_data with
  nothing saved yet returns {} rather than erroring
- both write through Vm's in-memory SaveData only; no filesystem access
  happens inside caiven-vm
EOF
)"
```

---

## Task 4: `api_registry.rs` entries

**Files:**
- Modify: `crates/caiven-vm/src/vm/api_registry.rs`

**Interfaces:**
- Consumes: the four builtin names/behaviors from Task 3.
- Produces: `ApiEntry` records consumed by Studio's autocomplete/hover and the syntax highlighter's builtin list (`all_names`, referenced in the file's module doc comment).

- [ ] **Step 1: Add entries**

In `crates/caiven-vm/src/vm/api_registry.rs`, in the `BUILTINS` array (after the `load_music_bank`/`play_sfx`/etc. entries, before the `math.*`/`string.*`/`table.*` entries), add:

```rust
ApiEntry {
    name: "dset",
    params: &[param!("slot": "u8"), param!("value": "number")],
    returns: "nil",
    doc: "Write value into save slot 0-63. Errors if slot is out of range. Persists across sessions once the host flushes it to disk.",
},
ApiEntry {
    name: "dget",
    params: &[param!("slot": "u8")],
    returns: "number",
    doc: "Read save slot 0-63; 0 if never set. Errors if slot is out of range.",
},
ApiEntry {
    name: "save_data",
    params: &[param!("data": "table")],
    returns: "nil",
    doc: "Replace the persisted save blob with data (string/number/bool/nested-table keys and values only). Errors if the packed size exceeds 4KiB or a value can't be serialized.",
},
ApiEntry {
    name: "load_data",
    params: &[],
    returns: "table",
    doc: "Return the persisted save blob, or {} if save_data has never been called.",
},
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p caiven-vm`
Expected: succeeds.

- [ ] **Step 3: Format and lint**

Run: `cargo fmt --all -- --check && cargo clippy -p caiven-vm --all-targets -- -D warnings -A unused-imports`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add crates/caiven-vm/src/vm/api_registry.rs
git commit -m "$(cat <<'EOF'
docs(vm): add dset/dget/save_data/load_data to the api_registry

Keeps api_registry.rs in sync with lua_exec.rs::register_builtins per the
lua-api rule — Studio's autocomplete/hover and the syntax highlighter's
builtin list both derive from this file.
EOF
)"
```

---

## Task 5: `docs/api-reference.md` update

**Files:**
- Modify: `docs/api-reference.md`

- [ ] **Step 1: Add a "Persistent Data" section**

In `docs/api-reference.md`, after the `## Audio` section and before `## System`, add:

```markdown
## Persistent Data

| Function                | Description                                                                                          |
| :------------------------| :----------------------------------------------------------------------------------------------------|
| `dset(slot, value)`     | Write `value` into save slot `0-63`; errors if `slot` is out of range                                |
| `dget(slot)`            | Read save slot `0-63`; `0` if never set; errors if `slot` is out of range                             |
| `save_data(table)`      | Replace the persisted save blob (string/number/bool/nested-table only); errors over 4KiB packed or on an unserializable value |
| `load_data()`           | Return the persisted save blob, or `{}` if `save_data` has never been called                          |

Save data is per cart (keyed the same way save states already are — see
System Specifications below) and is written to disk by the host (Machine
or Studio), not by the Lua sandbox directly.
```

- [ ] **Step 2: Commit**

```bash
git add docs/api-reference.md
git commit -m "$(cat <<'EOF'
docs(api): document the persistent save-data API
EOF
)"
```

---

## Task 6: `caiven-machine` disk wiring

**Files:**
- Create: `crates/caiven-machine/src/shell/save_data_io.rs`
- Modify: `crates/caiven-machine/src/shell/mod.rs` (or wherever `save_state` is declared as a module — add `pub(crate) mod save_data_io;` next to it)
- Modify: `crates/caiven-machine/src/app.rs` (load after cart load, flush when dirty after the per-frame `run_frame` loop)

**Interfaces:**
- Consumes: `Vm::save_data()`/`save_data_mut()`, `SaveData::{encode, decode, is_dirty, clear_dirty}` from Task 2; `cart_library::cart_id` (already `pub(crate)`, `library.rs:156-167`).
- Produces: `saves_dir() -> PathBuf`, `save_data_path(dir: &Path, id: &str) -> PathBuf`, used only within `caiven-machine`.

First, find exactly how `save_state` is declared as a module (check the top of `crates/caiven-machine/src/shell/mod.rs` for `pub(crate) mod save_state;` or similar) and mirror that declaration for the new module.

- [ ] **Step 1: Write the failing test**

Create `crates/caiven-machine/src/shell/save_data_io.rs` with its test module written first (following `save_state.rs`'s own test style at `crates/caiven-machine/src/shell/save_state.rs:74-116` as the template):

```rust
//! Persistent save-data file, one per cart under `saves/`, keyed by the
//! same `cart_id` save_state.rs uses. Delegates the actual byte format to
//! `caiven_vm::vm::SaveData::{encode, decode}` — this module only owns the
//! file path and untrusted-bytes-on-disk boundary.

use std::path::{Path, PathBuf};

/// Where save data lives: a `saves/` directory beside the binary, same as
/// `save_state::saves_dir()`.
pub fn saves_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("saves")
}

/// The save-data file for a given cart id. `id` must already be a V56-safe
/// single path component (`cart_library::cart_id` guarantees this).
pub fn save_data_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.cavdata"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_data_path_joins_id_onto_dir() {
        let dir = PathBuf::from("/tmp/saves");
        assert_eq!(
            save_data_path(&dir, "mygame"),
            dir.join("mygame.cavdata")
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p caiven-machine save_data_path`
Expected: fails to compile until the module is declared in `shell/mod.rs`.

- [ ] **Step 3: Declare the module and wire load/flush into `app.rs`**

In `crates/caiven-machine/src/shell/mod.rs`, next to the existing `save_state` module declaration, add:

```rust
pub(crate) mod save_data_io;
```

In `crates/caiven-machine/src/app.rs`, add the import next to the existing `use crate::shell::save_state;` line:

```rust
use crate::shell::save_data_io;
```

Immediately after line 118 (`self.cart_id = cart_library::cart_id(path);`), add a load step:

```rust
if let Some(id) = &self.cart_id {
    let path = save_data_io::save_data_path(&save_data_io::saves_dir(), id);
    if let Ok(bytes) = std::fs::read(&path) {
        if let Some(data) = caiven_vm::vm::SaveData::decode(&bytes) {
            *self.core.vm.save_data_mut() = data;
        }
    }
}
```

(A missing file or a file that fails to decode is a normal "nothing saved yet" state, matching `load_state`'s existing `ErrorKind::NotFound` handling above it in the same file — don't surface either as an error here.)

After the per-frame loop in the main run function (right after the `for _ in 0..steps { app.core.run_frame(); }` block, around `app.rs:864-866`), add a flush step:

```rust
if app.core.vm.save_data().is_dirty() {
    let dir = save_data_io::saves_dir();
    if let Some(id) = &app.cart_id {
        let _ = std::fs::create_dir_all(&dir);
        let path = save_data_io::save_data_path(&dir, id);
        if std::fs::write(&path, app.core.vm.save_data().encode()).is_ok() {
            app.core.vm.save_data_mut().clear_dirty();
        }
    }
}
```

(A failed write is left dirty so the next frame retries — matches the "prefer rejecting/retrying over silently losing data" spirit of the security rules without introducing a new error-reporting path; this is not expected to be a hot per-frame cost since `dset`/`save_data` calls are rare relative to draw calls, but if profiling later shows otherwise, throttling the flush is a follow-up, not part of this task.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p caiven-machine save_data_path`
Expected: PASS.

- [ ] **Step 5: Add an integration test mirroring the existing save-state round-trip test**

In `crates/caiven-machine/src/app.rs`'s existing `#[cfg(test)] mod tests` block (the one containing `save_state_round_trips_ram_and_palette` around line 1024), add:

```rust
#[test]
fn save_data_persists_across_reload_via_disk() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut app = test_app();
    app.cart_id = Some("mygame".to_string());

    app.core.vm.save_data_mut().set_slot(0, 7.0).expect("slot 0 is in range");
    let path = crate::shell::save_data_io::save_data_path(dir.path(), "mygame");
    std::fs::create_dir_all(dir.path()).unwrap();
    std::fs::write(&path, app.core.vm.save_data().encode()).unwrap();

    let mut app2 = test_app();
    app2.cart_id = Some("mygame".to_string());
    let bytes = std::fs::read(&path).unwrap();
    let data = caiven_vm::vm::SaveData::decode(&bytes).expect("valid save data");
    *app2.core.vm.save_data_mut() = data;

    assert_eq!(app2.core.vm.save_data().get_slot(0), 7.0);
}
```

(Match this test's exact helper calls — `test_app()`, `tempfile::tempdir()` — to whatever `save_state_round_trips_ram_and_palette` already uses in the same file, since those helpers already exist there.)

Run: `cargo test -p caiven-machine save_data`
Expected: PASS.

- [ ] **Step 6: Run the full existing machine test suite to check for regressions**

Run: `cargo test -p caiven-machine`
Expected: PASS.

- [ ] **Step 7: Format and lint**

Run: `cargo fmt --all -- --check && cargo clippy -p caiven-machine --all-targets -- -D warnings -A unused-imports`
Expected: clean.

- [ ] **Step 8: Commit**

```bash
git add crates/caiven-machine/src/shell/save_data_io.rs crates/caiven-machine/src/shell/mod.rs crates/caiven-machine/src/app.rs
git commit -m "$(cat <<'EOF'
feat(machine): persist cart save data to saves/<cart_id>.cavdata

- loaded once right after a cart's Lua source loads, same cart_id as
  save states already use
- flushed automatically whenever Vm::save_data() reports dirty after a
  frame, so a cart never needs a manual "save" hotkey for this
- a failed write leaves the dirty flag set so the next frame retries
  rather than silently losing the write
EOF
)"
```

---

## Task 7: `caiven-studio` disk wiring

**Files:**
- Modify: `crates/caiven-studio/src/studio/cart.rs` (or wherever the project-open and per-frame-tick functions Task-0 research found at `studio/cart.rs:73,187,205` live — confirm exact function names before editing)

**Interfaces:**
- Consumes: the same `caiven_vm::vm::SaveData`/`Vm::save_data()`/`save_data_mut()` API as Task 6, and the same `crate::shell::save_data_io`-equivalent path helpers (Studio has no existing `saves/`-style directory convention — reuse `caiven_machine`'s `save_data_io` module if `caiven-studio` can depend on `caiven-machine`'s public items, otherwise duplicate the two small path-helper functions locally under a new `crates/caiven-studio/src/app/save_data_io.rs` following the exact same shape as Task 6's file, keyed on the open project's directory name instead of a `.cav` filename stem).

- [ ] **Step 1: Confirm the integration points**

Read `crates/caiven-studio/src/studio/cart.rs` around the line numbers already identified (`73`, `187`, `205`) to find: (a) the function that runs a cart's Lua source for the first time in a Studio session (analogous to `caiven-machine`'s `load_lua_source` call site), and (b) the function that steps the VM each frame (analogous to `run_frame`). Note their exact names and signatures — this step has no code changes, just confirms where Steps 2-3 attach.

- [ ] **Step 2: Load save data when a project starts running**

At the point identified in Step 1(a), after the VM's Lua source is loaded successfully, add the same read-and-decode logic as Task 6's Step 3 load block, keyed on a stable identifier for the open project (use the project directory's folder name, sanitized the same way `cart_library::cart_id` sanitizes a `.cav` filename stem — do not introduce a second, differently-behaved sanitizer; if `cart_id`'s sanitization function can be shared/exposed for reuse here, prefer that over duplicating the rules).

- [ ] **Step 3: Flush save data when dirty after each frame**

At the point identified in Step 1(b), after each frame step, add the same dirty-check-then-write logic as Task 6's Step 3 flush block, writing to `<project-dir>/.caiven/saves/<id>.cavdata` (or wherever Studio already keeps generated/derived per-project files — check for an existing convention like a `.caiven/` directory before inventing a new location).

- [ ] **Step 4: Add a test**

Add a test in the same file (or its existing test module) mirroring Task 6 Step 5's round-trip test, adapted to however Studio constructs a test `Vm`/project fixture in its existing tests (check for an existing test helper before writing a new one).

Run: `cargo test -p caiven-studio save_data`
Expected: PASS.

- [ ] **Step 5: Run the full existing studio test suite to check for regressions**

Run: `cargo test -p caiven-studio`
Expected: PASS.

- [ ] **Step 6: Format and lint**

Run: `cargo fmt --all -- --check && cargo clippy -p caiven-studio --all-targets -- -D warnings -A unused-imports`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add -A crates/caiven-studio
git commit -m "$(cat <<'EOF'
feat(studio): persist cart save data alongside the open project

Mirrors caiven-machine's saves/<cart_id>.cavdata convention so a cart's
save data behaves identically whether it's run from Studio or Machine.
EOF
)"
```

---

## Task 8: Studio autocomplete update

**Files:**
- Modify: the CodeMirror Lua definitions file(s) under `crates/caiven-studio-ui` that currently list builtin names for autocomplete/hover (search for wherever `sprite`, `button_down`, etc. are already listed as completions — likely a `.ts`/`.js` file generated from or mirroring `api_registry.rs`'s `BUILTINS`).

- [ ] **Step 1: Locate the existing builtin completion list**

Run: `grep -rn "button_pressed" crates/caiven-studio-ui/src` to find the file(s) that need the four new entries.

- [ ] **Step 2: Add the four entries**

Add `dset`, `dget`, `save_data`, `load_data` to that list using the exact same shape (name, params, doc string) as a neighboring entry like `play_sfx`, copying the parameter/return/doc text from Task 4's `api_registry.rs` entries verbatim so the two stay worded identically.

- [ ] **Step 3: Verify Studio UI type-checks**

Run: `scripts/claude/check-studio-ui.sh`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add -A crates/caiven-studio-ui
git commit -m "$(cat <<'EOF'
feat(studio-ui): add dset/dget/save_data/load_data to Lua autocomplete
EOF
)"
```

---

## Task 9: Example cart

**Files:**
- Modify: `games/carts/stdlib_demo.cav`'s source project (find the loose-source project directory it's built from — check `carts/` for a matching `.lua`/`caiven.toml` pair, per `.claude/rules/cart-format.md`'s description of the human-diffable project format), or create a new small example under `carts/` if `stdlib_demo` doesn't have easy room for this without disrupting its existing easing/particle/tween demo.

- [ ] **Step 1: Add a persisted high-score counter**

Add a few lines to the example's `_init()`/`_update()` that call `dget(0)` on startup to read a stored high score, compare it against a running score each frame, and call `dset(0, score)` when the running score exceeds it — following whatever coordinate/scoring convention the existing demo already uses.

- [ ] **Step 2: Run the demo and confirm it behaves**

Run: `cargo run -p caiven-machine -- games/carts/stdlib_demo.cav` (or the equivalent path for a new example cart) and confirm no Lua errors appear in the log and the high-score value persists after quitting and relaunching.

- [ ] **Step 3: Commit**

```bash
git add -A carts/ games/carts/
git commit -m "$(cat <<'EOF'
docs(carts): demonstrate persistent save data in the stdlib example
EOF
)"
```

---

## Task 10: Final full pass

- [ ] **Step 1: Run the full pre-commit gate**

Run: `scripts/claude/pre-commit-gate.sh`
Expected: clean across the whole workspace.

- [ ] **Step 2: Re-read the design doc against what shipped**

Open `docs/superpowers/specs/2026-08-07-lua-sandbox-and-save-data-design.md` and confirm every "Tests" / "Documentation & tooling" bullet in both Part 1 and Part 2 has a corresponding completed task above. Note any gap in a follow-up task rather than leaving it silently unaddressed.
