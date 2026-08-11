---
name: caiven-game-prototype
description: Build a tiny playable cartridge under projects/dev/ to validate an engine or Studio feature end-to-end. Use when you need to exercise a new capability through a real cart rather than only unit tests — not for building a full example game.
---

# caiven-game-prototype

## Each prototype must

- Have one focused design goal (validate one capability, not showcase many).
- Be playable quickly (seconds to load, no lengthy setup).
- Exercise the new capability directly and unambiguously.
- Expose usability problems — if a new Lua API or Studio feature is awkward
  to use in a real cart, this is where that surfaces.
- Avoid becoming a large game project — if it's growing past "tiny", stop
  and cut scope.
- Include instructions and expected behavior (what should happen when run,
  which keys/buttons matter) either as a header comment in the cart's
  `main.lua` or a short note alongside it.

## Where it lives

Existing examples (`projects/dev/audio_test/`, `catch/`, `smoke/`,
`demo_string/`, `demo_table/`, `movement/`, `sprite/`, `stdlib_demo/`,
`tiles/`) show the naming/scope convention — one project, one thing being
tested. Author as a project dir first (`caiven.toml` + `.lua`, diffable),
build to `.cav` with `caiven-studio build` (or `scripts/demo-carts/build.sh`
for the checked-in set) only if a binary artifact is actually needed (e.g.
for a `caiven-lua-api` example or Port-upload testing). Built binaries for
this set live at `carts/dev/<name>.cav`.

## Run it

```bash
cargo run -p caiven-machine -- path/to/prototype/   # project dir, hot-reloads with Ctrl+R
```
