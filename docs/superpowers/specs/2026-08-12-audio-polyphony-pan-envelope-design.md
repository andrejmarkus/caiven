# Audio: polyphonic voices, volume, pan, per-note envelope

Status: approved, pending implementation plan.

## Context

`caiven-lua-api` audit for "make the API viable for building any type of
game" flagged audio channels as a known gap (see
`2026-08-07-sprite-flip-rotate-design.md`, Out of scope). This spec covers
that gap.

Current engine (`crates/caiven-vm/src/vm/audio.rs`, `sfx.rs`,
`execution.rs`): `Sound` holds exactly one square-wave slot and one
noise-wave slot, shared globally. `SfxPlayer` (via `play_sfx`) and
`MusicPlayer` (via `play_music`) both write into that same pair of slots.
Concretely: `tick_sfx_player` and `tick_music_player` both run every frame
and both call `tick_sfx_channel` against the same `Sound`, so a `play_sfx`
call while music is active can overwrite whichever channel the music
happens to be using that frame — SFX audibly clobbers music. There is no
volume control exposed to Lua, no panning (device output is mono, the same
sample duplicated to every output channel), no polyphony (a second `play_sfx`
call while one is active just restarts the single slot), and no envelope —
notes switch on/off instantly, which is also why the "duration" field on
`SquareChannel`/`NoiseChannel` is always set to 0 by `tick_sfx_channel`
instead of coming from cart data.

The SFX bank step format (`crates/caiven-vm/src/vm/sfx.rs::sfx_bytes_base`)
is 4 bytes/step: `note, volume, wave, byte3`. The VM reads nowhere past
byte 2 today — `tick_sfx_channel` never touches byte3. Studio's SFX tracker
UI (`Workspace.svelte`, `sfx-fx` column) does write byte3, as an "effect"
selector (0=none, 1=slide, 2=vibrato, 3=drop) that has never been wired up
to any audible behavior (see the tracker's own hint: "stored in the cart,
but the VM does not apply it yet"). So byte3 is VM-dead but not
guaranteed-zero in every stored cart — a cart authored with a non-default
fx pick has non-zero byte3 today. Repurposing it is still the right call:
nothing audible has ever depended on its value, so there's no runtime
behavior to preserve, only stored bytes to reinterpret (see Compatibility).
That gives room to add per-instrument pan and envelope without growing
`SFX_BANK_LEN` (`crates/caiven-core/src/memory.rs`), so no memory-map
address shift and no cart version bump.

## Byte3 encoding

```
bit:   7 6 5 4 3 2 1 0
       [release][attack][ pan  ]
```

- `pan` (bits 0-3): index 0-15 into a fixed zigzag table where index 0 =
  center (`0.0`), remaining indices alternate left/right out to ±1.0.
  Index 0 is deliberately the "unset" value so a cart that never wrote
  byte3 gets centered pan, not hard-left.
- `attack` (bits 4-5): level 0-3 → ramp length `{0, ~15ms, ~50ms, ~150ms}`.
  Level 0 = instant (today's behavior).
- `release` (bits 6-7): same four levels, applied when the note ends
  (duration expiry or explicit stop).
- `byte3 == 0` decodes to center pan, instant attack, instant release —
  the common case (a cart that never touched the fx column) is unaffected.

Music rows reference SFX ids (`MusicPlayer::pattern_row_base`), so a note
played from a music pattern inherits that instrument's byte3 automatically.
No music bank format change needed.

## Voice pool

`Sound` changes from `{square, noise}` to a fixed pool of 8 `Voice`s
(`kind: Square|Noise, frequency, volume, pan, envelope state, duration`).
Voices 0-1 are reserved for music (`ch0` always voice 0, `ch1` always voice
1) exactly as today's hardcoded assignment — this preserves current music
behavior and guarantees SFX can never steal a music voice, which is also
what fixes the clobbering bug. Voices 2-7 are a round-robin pool for SFX:
`play_sfx` takes the next free voice, or steals the oldest active one if all
six are busy.

`Synth::next_sample` becomes stereo: mix all active voices into `(left,
right)` using each voice's pan, apply each voice's attack/release ramp.
`AudioCallback::callback` writes per output-channel-pair instead of
duplicating one mono sample (still degrades to a stereo pair even on a
mono device — an unused second channel is harmless).

## Lua API

- `play_sfx(id, opts)` — `opts` optional table, `{volume = 1.0}`. Returns
  an integer voice handle.
- `stop_sfx(handle)` — new. Stops the given voice immediately (applies
  release ramp, not a hard cut, so it doesn't click). No-op if the handle
  is no longer active (already finished or stolen) — silent no-op, not an
  error, since a cart tracking a handle across frames shouldn't have to
  guard every call.
- `set_music_volume(v)`, `set_sfx_volume(v)`, `set_master_volume(v)` — new.
  `v` clamped to `[0, 1]`. Runtime-only multipliers layered on top of
  authored per-step volume; not persisted to cart data (these are
  player-facing settings, analogous to a system volume slider, not
  authored content).
- `play_sfx(id)` (no opts), `play_music(id)`, `stop_music()` — signatures
  unchanged.

## Studio SFX editor

Remove the `sfx-fx` column (`Workspace.svelte`, `sfxEffects` array and its
click handler) — it never drove audible behavior and its bit range is being
reclaimed. Add pan and attack/release controls to the SFX tracker's
per-step row in its place, alongside the existing note/volume/wave columns,
using pack/unpack helpers for byte3's three sub-fields (not a raw 0-255
value like the other columns). Tracker UI change only — no asset-bank
encode/decode change, since the byte was already part of the wire format.

## Testing

VM-level tests in `crates/caiven-vm/tests/`:

- Byte3 all-zero → identical `Sound`/`Voice` state to current behavior
  (regression guard for the compat claim).
- Pan table: index 0 → 0.0 center; a couple of non-zero indices → expected
  sign/magnitude.
- Attack/release levels 0-3 map to the documented ramp lengths.
- Polyphony: two `play_sfx` calls while both active occupy distinct voices
  and both remain audible (non-zero volume) simultaneously.
- Voice stealing: 7th concurrent `play_sfx` call reuses the oldest SFX
  voice rather than being dropped or erroring.
- Music/SFX independence: `play_sfx` while `play_music` is active does not
  change `ch0`/`ch1` state (the specific bug this spec fixes).
- `stop_sfx` on an active handle silences it (post-release-ramp); on an
  already-finished/stolen handle is a no-op, not an error.
- `set_master_volume`/`set_music_volume`/`set_sfx_volume` clamp to `[0,1]`
  and scale output as expected.

## Documentation

- `docs/api-reference.md` (or README API reference) — `play_sfx`,
  `stop_sfx`, `set_music_volume`, `set_sfx_volume`, `set_master_volume`
  entries; note the byte3 pan/envelope format for anyone hand-authoring SFX
  banks.
- `crates/caiven-vm/src/vm/api_registry.rs` — kept in sync (autocomplete).
- `crates/caiven-studio-ui` codemirror Lua definitions — updated.
- Example: extend an existing audio-demo cart (or add one) under
  `projects/dev/` showing two overlapping SFX with different pan, and music
  continuing to play under an SFX hit.

## Compatibility

Additive for runtime behavior: `play_sfx(id)`/`play_music(id)`/
`stop_music()` signatures are unchanged; `opts` on `play_sfx` is optional.
The clobbering-bug fix (SFX no longer interrupts music) is an intentional
observable behavior change, called out explicitly.

Byte3 reinterpretation is a one-time stored-data compat note, not a code
compat break: the VM has never read byte3, so no cart's *audible* behavior
depended on its value. A cart authored with the old Studio fx column set to
non-default (SL/VB/DR) on some steps will, the first time it's loaded after
this ships, get whatever pan/attack/release that old value happens to
decode to under the new bit layout — silent reinterpretation of unused
data, not a regression of anything that used to work. `byte3 == 0` (never
touched the fx column, the common case) decodes to center pan/instant
envelope, matching today exactly.

## Out of scope (future specs)

- Input completeness (`button_released`, diagonal helpers).
- Collision & many-sprite helpers (circle/point tests, pooling).
- Rendering (sprite layering/z-order, particle control) and text/UI
  (wrapping, dialogue boxes) — flagged in the same audit, separate specs.
