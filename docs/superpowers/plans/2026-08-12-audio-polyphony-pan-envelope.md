# Audio: polyphonic voices, volume, pan, per-note envelope — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the single mono square+noise `Sound` slot with an 8-voice
stereo pool driving pan, per-note attack/release envelopes, and a
polyphonic `play_sfx`/`stop_sfx` Lua API — while fixing the existing bug
where `play_sfx` clobbers concurrent music playback.

**Architecture:** `Sound` becomes `voices: [Voice; 8]` (target parameters,
behind the existing `Arc<Mutex<Sound>>`) plus three volume multipliers.
Voice 0/1 stay hard-assigned to music channels ch0/ch1 (unchanged from
today). Voice 2 stays hard-assigned to the existing single-slot
`Vm::start_sfx`/`stop_sfx`/`sfx_player()` Rust API that Studio's SFX-editor
preview already uses — untouched signature, so Studio's preview plumbing
needs no changes. Voices 3-7 (5 voices) are a new round-robin/steal pool
private to the new Lua-facing `play_sfx(id, opts)`/`stop_sfx(handle)`,
which is polyphonic. `Synth` (the real-time-thread-local waveform
generator) becomes per-voice: it detects retriggers via a per-voice
`epoch` counter written by the frame thread, ramps a linear envelope
per voice at sample rate, and mixes to stereo by pan. `tick_sfx_channel`
(shared by all four voice sources: music ch0, music ch1, legacy preview,
and the 5-slot pool) now decodes pan/attack/release from the SFX step's
previously-dead byte3 and writes directly into a specific `Voice`, instead
of writing into one shared square/noise pair — this is what structurally
fixes the clobbering bug, not a separate bugfix.

**Tech Stack:** Rust (`caiven-vm`, `caiven-core` already-fixed memory
layout, `caiven-web` wasm player), `mlua` for the Lua binding, Svelte 5 for
the Studio SFX tracker.

**Deviation from the written spec:** The spec
(`docs/superpowers/specs/2026-08-12-audio-polyphony-pan-envelope-design.md`)
describes "voices 2-7, six voices" as the SFX round-robin pool. Investigating
`Vm::start_sfx`/`stop_sfx`/`sfx_player()` (already public Rust API consumed
by Studio's SFX-editor preview, `crates/caiven-studio/src/tauri_app.rs:1463-1464`)
showed it's a pre-existing single-slot player independent of the new Lua
API. Folding it into the same round-robin pool would mean touching Studio's
preview/`audio_payload` code for no spec-required benefit. Keeping it on
its own reserved voice (2) instead is a smaller, lower-risk diff; the
round-robin pool for the new Lua API is voices 3-7 (5 voices, not 6). This
does not change any Lua-facing behavior described in the spec.

## Global Constraints

- No `unwrap`/`expect`/panic/unchecked indexing on a production path
  (`.claude/rules/rust.md`, `.claude/rules/vm-runtime.md`).
- `crates/caiven-vm/src/vm/audio.rs` and `sfx.rs` run adjacent to the SDL2
  real-time audio thread — never block or allocate unpredictably there
  (`.claude/rules/vm-runtime.md`). All per-sample state lives in
  fixed-size arrays already owned by `Synth`/`Sound`; no `Vec`/`Box`
  allocation inside `Synth::next_sample` or the audio callback.
- Any public Lua API change ships with implementation + VM-level tests +
  docs (`docs/api-reference.md`) + `api_registry.rs` sync + an example cart
  + explicit compat/error-behavior notes (`.claude/rules/lua-api.md`,
  `caiven-lua-api` skill).
- Existing API behavior must never change silently — the one intentional
  behavior change (SFX no longer interrupts music) is called out in the
  spec's Compatibility section and must be called out again in
  `docs/api-reference.md`.
- Keep public interfaces narrow — don't add `pub` beyond what a later task
  in this plan actually consumes.
- `cargo fmt --all -- --check` and `cargo clippy -p caiven-vm --all-targets -- -D warnings -A unused-imports`
  (and `-p caiven-web`, `-p caiven-machine`, `-p caiven-studio` for the
  crates each task touches) before each task's commit.
- `npm run check` in `crates/caiven-studio-ui` before Task 6's commit
  (`.claude/rules/studio-ui.md`).

---

### Task 1: Voice pool and stereo synth core

**Files:**
- Modify: `crates/caiven-vm/src/vm/audio.rs`
- Modify: `crates/caiven-web/src/main.rs:126-143` (`fill_audio` — mono
  downmix of the new stereo `next_sample`)
- Test: inline `#[cfg(test)]` module in `crates/caiven-vm/src/vm/audio.rs`

**Interfaces:**
- Produces: `pub enum VoiceKind { Square, Noise }` (`Clone, Copy, Debug,
  PartialEq`); `pub struct Voice { pub kind: VoiceKind, pub gate: bool,
  pub frequency: f32, pub volume: f32, pub pan: f32, pub attack_ms: f32,
  pub release_ms: f32, pub epoch: u32 }` (`Clone, Copy, Debug`) with
  `Voice::silent() -> Voice`; `pub struct Sound { pub voices: [Voice;
  VOICE_COUNT], pub master_volume: f32, pub music_volume: f32, pub
  sfx_volume: f32 }` implementing `Default`; `pub const VOICE_COUNT: usize
  = 8`, `pub const MUSIC_VOICE_CH0: usize = 0`, `pub const
  MUSIC_VOICE_CH1: usize = 1`, `pub const LEGACY_SFX_VOICE: usize = 2`,
  `pub const SFX_POOL_START: usize = 3`, `pub const SFX_POOL_LEN: usize =
  5`; `pub const PAN_TABLE: [f32; 16]`; `pub const ENVELOPE_MS: [f32; 4]`;
  `Synth::next_sample(&mut self, sound: &Sound, sample_rate: f32) -> (f32,
  f32)` (was `-> f32`).
- Consumes: nothing from other tasks (this is the foundation task).

- [ ] **Step 1: Write the failing tests**

Add to the bottom of `crates/caiven-vm/src/vm/audio.rs`, inside a new
`#[cfg(test)] mod voice_tests`:

```rust
#[cfg(test)]
mod voice_tests {
    use super::*;

    #[test]
    fn pan_table_index_zero_is_center() {
        assert_eq!(PAN_TABLE[0], 0.0);
    }

    #[test]
    fn pan_table_alternates_left_right_growing_outward() {
        assert_eq!(PAN_TABLE[1], -0.125);
        assert_eq!(PAN_TABLE[2], 0.125);
        assert_eq!(PAN_TABLE[15], -1.0);
    }

    #[test]
    fn envelope_levels_map_to_documented_ramp_lengths() {
        assert_eq!(ENVELOPE_MS, [0.0, 15.0, 50.0, 150.0]);
    }

    #[test]
    fn instant_envelope_reaches_full_volume_within_one_sample() {
        let mut sound = Sound::default();
        sound.voices[SFX_POOL_START] = Voice {
            kind: VoiceKind::Square,
            gate: true,
            frequency: 440.0,
            volume: 1.0,
            pan: 0.0,
            attack_ms: 0.0,
            release_ms: 0.0,
            epoch: 1,
        };
        let mut synth = Synth::new();
        let (l, _r) = synth.next_sample(&sound, 44_100.0);
        assert!(l.abs() > 0.0, "expected audible output on first sample, got {l}");
    }

    #[test]
    fn center_pan_matches_todays_equal_channel_output() {
        // byte3 == 0 decodes to pan 0.0; equal-gain center must reproduce
        // the old mono-duplicated-to-both-channels behavior exactly.
        let mut sound = Sound::default();
        sound.voices[SFX_POOL_START] = Voice {
            kind: VoiceKind::Square,
            gate: true,
            frequency: 440.0,
            volume: 1.0,
            pan: 0.0,
            attack_ms: 0.0,
            release_ms: 0.0,
            epoch: 1,
        };
        let mut synth = Synth::new();
        let (l, r) = synth.next_sample(&sound, 44_100.0);
        assert_eq!(l, r);
    }

    #[test]
    fn hard_left_pan_silences_right_channel() {
        let mut sound = Sound::default();
        sound.voices[SFX_POOL_START] = Voice {
            kind: VoiceKind::Square,
            gate: true,
            frequency: 440.0,
            volume: 1.0,
            pan: -1.0,
            attack_ms: 0.0,
            release_ms: 0.0,
            epoch: 1,
        };
        let mut synth = Synth::new();
        let (_l, r) = synth.next_sample(&sound, 44_100.0);
        assert_eq!(r, 0.0);
    }

    #[test]
    fn released_voice_fades_to_silence_over_release_samples() {
        let mut sound = Sound::default();
        sound.voices[SFX_POOL_START] = Voice {
            kind: VoiceKind::Square,
            gate: false,
            frequency: 440.0,
            volume: 1.0,
            pan: 0.0,
            attack_ms: 0.0,
            release_ms: 150.0,
            epoch: 1,
        };
        let mut synth = Synth::new();
        // Force env_level to 1.0 as if the note had just been playing.
        synth.env_level[SFX_POOL_START] = 1.0;
        synth.env_epoch[SFX_POOL_START] = 1;
        let sample_rate = 44_100.0;
        let release_samples = (0.150 * sample_rate) as usize;
        for _ in 0..release_samples {
            synth.next_sample(&sound, sample_rate);
        }
        assert!(synth.env_level[SFX_POOL_START] <= 0.0);
    }

    #[test]
    fn retrigger_via_epoch_resets_envelope_even_while_gated() {
        let mut sound = Sound::default();
        sound.voices[SFX_POOL_START] = Voice {
            kind: VoiceKind::Square,
            gate: true,
            frequency: 440.0,
            volume: 1.0,
            pan: 0.0,
            attack_ms: 150.0,
            release_ms: 0.0,
            epoch: 1,
        };
        let mut synth = Synth::new();
        synth.env_level[SFX_POOL_START] = 1.0;
        synth.env_epoch[SFX_POOL_START] = 1;
        // Same epoch: envelope must NOT reset.
        synth.next_sample(&sound, 44_100.0);
        assert!(synth.env_level[SFX_POOL_START] > 0.9);
        // New epoch (retrigger/steal): envelope must reset to a fresh attack ramp.
        sound.voices[SFX_POOL_START].epoch = 2;
        synth.next_sample(&sound, 44_100.0);
        assert!(synth.env_level[SFX_POOL_START] < 0.9);
    }
}
```

`env_level`/`env_epoch` need `pub(super)` (or crate-visible) fields on
`Synth` for the test module to reach into them directly — add
`#[cfg(test)] pub(crate)` is unnecessary; since the test module is declared
inside `audio.rs` itself (`mod voice_tests` nested in the same file), plain
private fields are already visible. No visibility change needed as long as
the test module stays in this file.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p caiven-vm --lib vm::audio -- --nocapture`
Expected: compile failure (`VoiceKind`, `Voice`, `PAN_TABLE`, `ENVELOPE_MS`,
`SFX_POOL_START` don't exist yet; `next_sample` still returns `f32`).

- [ ] **Step 3: Replace `SquareChannel`/`NoiseChannel`/`Sound`/`Synth`**

Replace lines 66-121 of `crates/caiven-vm/src/vm/audio.rs` (the
`SquareChannel`/`NoiseChannel`/`Sound` structs and the `AudioFactory` type
alias, which references `Sound` and stays but now refers to the new type)
with:

```rust
pub const VOICE_COUNT: usize = 8;
/// Voice 0/1 are hard-assigned to music channels ch0/ch1, matching the
/// pre-existing `tick_music_player` assignment.
pub const MUSIC_VOICE_CH0: usize = 0;
pub const MUSIC_VOICE_CH1: usize = 1;
/// Reserved for `Vm::start_sfx`/`stop_sfx`/`sfx_player()` — the
/// pre-existing single-slot preview player Studio's SFX editor drives.
/// Kept separate from the new Lua-facing pool below so Studio's preview
/// code needs no changes.
pub const LEGACY_SFX_VOICE: usize = 2;
/// Round-robin/steal pool backing the new polyphonic `play_sfx`/`stop_sfx`
/// Lua API.
pub const SFX_POOL_START: usize = 3;
pub const SFX_POOL_LEN: usize = VOICE_COUNT - SFX_POOL_START;

/// Fixed pan positions selected by the low 4 bits of an SFX step's byte3.
/// Index 0 is deliberately center — a step that never set byte3 (every
/// cart before this change) decodes to center, matching today's output.
pub const PAN_TABLE: [f32; 16] = [
    0.0,
    -0.125, 0.125,
    -0.25, 0.25,
    -0.375, 0.375,
    -0.5, 0.5,
    -0.625, 0.625,
    -0.75, 0.75,
    -0.875, 0.875,
    -1.0,
];

/// Attack/release ramp lengths selected by byte3's 2-bit level fields.
/// Level 0 is instant, matching today's on/off behavior.
pub const ENVELOPE_MS: [f32; 4] = [0.0, 15.0, 50.0, 150.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceKind {
    Square,
    Noise,
}

/// One synth voice's target parameters, written by the frame thread and
/// read every audio sample by [`Synth::next_sample`]. `epoch` is bumped on
/// every (re)trigger so the audio thread can tell a stolen/reused voice
/// apart from one still sustaining the same note, even though both look
/// like `gate == true` from the outside.
#[derive(Debug, Clone, Copy)]
pub struct Voice {
    pub kind: VoiceKind,
    pub gate: bool,
    pub frequency: f32,
    pub volume: f32,
    pub pan: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub epoch: u32,
}

impl Voice {
    pub fn silent() -> Self {
        Self {
            kind: VoiceKind::Square,
            gate: false,
            frequency: 440.0,
            volume: 0.0,
            pan: 0.0,
            attack_ms: 0.0,
            release_ms: 0.0,
            epoch: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sound {
    pub voices: [Voice; VOICE_COUNT],
    /// Runtime-only multiplier layered on top of authored per-step
    /// volume, clamped to `[0, 1]` by whoever writes it. Not persisted to
    /// cart data.
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
}

impl Default for Sound {
    fn default() -> Self {
        Self {
            voices: [Voice::silent(); VOICE_COUNT],
            master_volume: 1.0,
            music_volume: 1.0,
            sfx_volume: 1.0,
        }
    }
}
```

Then replace the `Synth` struct and its `impl` (original lines 20-64) with:

```rust
/// Per-voice waveform + envelope synth, one output sample at a time.
/// Portable (no cpal) so both the native `Audio` stream callback and the
/// web player's `AudioWorklet` fill export can share the exact same
/// waveform math.
pub struct Synth {
    phase: [f32; VOICE_COUNT],
    lfsr: [u16; VOICE_COUNT],
    env_level: [f32; VOICE_COUNT],
    env_epoch: [u32; VOICE_COUNT],
}

impl Default for Synth {
    fn default() -> Self {
        Self::new()
    }
}

impl Synth {
    pub fn new() -> Self {
        Self {
            phase: [0.0; VOICE_COUNT],
            lfsr: [0xACE1; VOICE_COUNT],
            env_level: [0.0; VOICE_COUNT],
            env_epoch: [0; VOICE_COUNT],
        }
    }

    /// Advances every voice by one output sample and returns the mixed
    /// stereo pair, each in `[-1, 1]`.
    pub fn next_sample(&mut self, sound: &Sound, sample_rate: f32) -> (f32, f32) {
        let mut left = 0.0f32;
        let mut right = 0.0f32;

        for i in 0..VOICE_COUNT {
            let voice = &sound.voices[i];

            // A changed epoch means this voice was (re)triggered since the
            // last sample — reset phase/envelope even if `gate` still
            // reads the same as before (e.g. a stolen voice retriggered
            // mid-note).
            if voice.epoch != self.env_epoch[i] {
                self.env_epoch[i] = voice.epoch;
                self.env_level[i] = 0.0;
                self.phase[i] = 0.0;
            }

            if voice.gate {
                let step = if voice.attack_ms <= 0.0 {
                    1.0
                } else {
                    1000.0 / (voice.attack_ms * sample_rate)
                };
                self.env_level[i] = (self.env_level[i] + step).min(1.0);
            } else {
                let step = if voice.release_ms <= 0.0 {
                    1.0
                } else {
                    1000.0 / (voice.release_ms * sample_rate)
                };
                self.env_level[i] = (self.env_level[i] - step).max(0.0);
            }

            if voice.volume <= 0.0 || (self.env_level[i] <= 0.0 && !voice.gate) {
                continue;
            }

            let raw = match voice.kind {
                VoiceKind::Square => {
                    let v = if self.phase[i] < 0.5 { 1.0 } else { -1.0 };
                    self.phase[i] = (self.phase[i] + voice.frequency / sample_rate) % 1.0;
                    v
                }
                VoiceKind::Noise => {
                    self.phase[i] += voice.frequency / sample_rate;
                    if self.phase[i] >= 1.0 {
                        self.phase[i] -= 1.0;
                        let bit = (self.lfsr[i] ^ (self.lfsr[i] >> 2) ^ (self.lfsr[i] >> 3) ^ (self.lfsr[i] >> 5)) & 1;
                        self.lfsr[i] = (self.lfsr[i] >> 1) | (bit << 15);
                    }
                    if (self.lfsr[i] & 1) == 0 { 1.0 } else { -1.0 }
                }
            };

            // Voices 0/1 are the music channels; everything else (legacy
            // preview + the SFX pool) is grouped under sfx_volume.
            let group_volume = if i < 2 { sound.music_volume } else { sound.sfx_volume };
            let amp = raw * voice.volume * self.env_level[i] * CHANNEL_HEADROOM * group_volume;

            let pan = voice.pan;
            left += amp * (1.0 - pan.max(0.0));
            right += amp * (1.0 + pan.min(0.0));
        }

        (
            (left * MASTER_GAIN * sound.master_volume).clamp(-1.0, 1.0),
            (right * MASTER_GAIN * sound.master_volume).clamp(-1.0, 1.0),
        )
    }
}
```

- [ ] **Step 4: Update `ConsoleCallback` for stereo output**

Replace the `callback` body (original lines 150-165) with:

```rust
fn callback(&mut self, out: &mut [i16]) {
    let Ok(sound) = self.sound.try_lock() else {
        out.fill(0);
        return;
    };

    for frame in out.chunks_mut(self.channels) {
        let (l, r) = self.synth.next_sample(&sound, self.sample_rate);
        if self.channels <= 1 {
            let mono = to_i16((l + r) * 0.5);
            for slot in frame.iter_mut() {
                *slot = mono;
            }
        } else {
            frame[0] = to_i16(l);
            frame[1] = to_i16(r);
            for slot in frame.iter_mut().skip(2) {
                *slot = to_i16(l);
            }
        }
    }
}
```

- [ ] **Step 5: Remove `AudioPeripheral`**

`AudioPeripheral::tick` only ever decremented a frame-based `duration`
field, which every existing cart already left at `0` (the doc comment on
`tick_sfx_channel` before this change noted duration was always `0` — dead
functionality). Envelope timing is now sample-accurate inside `Synth`, so
there is nothing left for a frame-tick peripheral to do. Delete the
`AudioPeripheral` struct and its `Peripheral` impl (original lines
260-294) from `crates/caiven-vm/src/vm/audio.rs` entirely.

Then remove its registration and import at each of its 3 call sites:
- `crates/caiven-vm/src/runtime.rs:12` — drop `AudioPeripheral` from the
  `use` list; delete lines 87 and 116
  (`vm.register_peripheral(AudioPeripheral::new(vm.get_sound_shared()));`).
- `crates/caiven-web/src/main.rs:60` — delete
  `vm.register_peripheral(AudioPeripheral::new(sound.clone()));` and drop
  `AudioPeripheral` from whatever `use` line imports it in that file.

Before deleting, confirm no example/dev cart declares `audio` as a
required mod peripheral (it never has been one — `audio` is a core
built-in, not a mod): `grep -rn "audio" projects/**/caiven.toml` should
show nothing relevant.

- [ ] **Step 6: Update `caiven-web`'s mono downmix**

In `crates/caiven-web/src/main.rs`, `fill_audio` (lines 129-143) calls
`self.synth.next_sample(&s, sample_rate)` expecting an `f32`. Update the
loop body:

```rust
match self.sound.try_lock() {
    Ok(s) => {
        for sample in self.audio_buf[..num_frames].iter_mut() {
            let (l, r) = self.synth.next_sample(&s, sample_rate);
            *sample = (l + r) * 0.5;
        }
    }
    Err(_) => {
        self.audio_buf[..num_frames].fill(0.0);
    }
}
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test -p caiven-vm --lib vm::audio -- --nocapture`
Expected: PASS (7 new tests, plus the existing `sdl_audio_tests` module
still compiles — it only touches `to_i16`, untouched by this task).

Run: `cargo build -p caiven-vm -p caiven-web -p caiven-machine -p caiven-studio`
Expected: builds clean (this step alone leaves other crates referencing
`Sound.square`/`.noise` still broken — that's fixed in Task 2/3; if this
build fails on `caiven-vm`/`caiven-web`/`caiven-machine` themselves, that's
a real regression from this task and must be fixed before continuing).

- [ ] **Step 8: Format, lint, commit**

```bash
cargo fmt --all
cargo clippy -p caiven-vm -p caiven-web --all-targets -- -D warnings -A unused-imports
git add crates/caiven-vm/src/vm/audio.rs crates/caiven-vm/src/runtime.rs crates/caiven-web/src/main.rs
git commit -m "$(cat <<'EOF'
feat(vm): replace single-slot Sound with an 8-voice stereo pool

- Voice{kind,gate,frequency,volume,pan,attack_ms,release_ms,epoch}
  replaces the old SquareChannel/NoiseChannel pair; Synth mixes all 8
  voices to stereo per-sample, applying pan and a linear attack/release
  envelope
- epoch lets the audio thread detect a stolen/retriggered voice and reset
  its envelope even mid-note
- drop AudioPeripheral: its only job (frame-based duration countdown) was
  already dead since duration was always 0; envelope timing is now
  sample-accurate inside Synth instead
EOF
)"
```

---

### Task 2: byte3 decode and voice-targeted `tick_sfx_channel`

**Files:**
- Modify: `crates/caiven-vm/src/vm/sfx.rs`
- Modify: `crates/caiven-vm/src/vm/execution.rs`
- Test: inline `#[cfg(test)]` module in `crates/caiven-vm/src/vm/sfx.rs`

**Interfaces:**
- Consumes: `Voice`, `VoiceKind`, `PAN_TABLE`, `ENVELOPE_MS` from Task 1
  (`crate::vm::audio`).
- Produces: `pub fn decode_byte3(byte3: u8) -> (f32, f32, f32)` (returns
  `(pan, attack_ms, release_ms)`) in `sfx.rs`; `tick_sfx_channel(player:
  &mut SfxPlayer, memory: &Memory, voice: &mut Voice, forced_kind:
  Option<VoiceKind>, volume_scale: f32)` in `execution.rs` (signature
  change from today's `sound: &mut Sound, forced_wave: Option<u8>`) —
  Task 3 calls this once per music channel, once for the legacy preview
  voice, and once per pool slot.

- [ ] **Step 1: Write the failing test for byte3 decoding**

Add to `crates/caiven-vm/src/vm/sfx.rs`:

```rust
#[cfg(test)]
mod byte3_tests {
    use super::*;

    #[test]
    fn zero_byte_decodes_to_center_pan_and_instant_envelope() {
        assert_eq!(decode_byte3(0), (0.0, 0.0, 0.0));
    }

    #[test]
    fn pan_bits_select_the_pan_table() {
        let (pan, _, _) = decode_byte3(0b0000_0001);
        assert_eq!(pan, super::super::audio::PAN_TABLE[1]);
    }

    #[test]
    fn attack_and_release_bits_select_envelope_levels() {
        let byte3 = 0b1000_0000 | 0b0001_0000; // release=level2(bit7), attack=level1(bit4)
        let (_, attack, release) = decode_byte3(byte3);
        assert_eq!(attack, super::super::audio::ENVELOPE_MS[1]);
        assert_eq!(release, super::super::audio::ENVELOPE_MS[2]);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p caiven-vm --lib vm::sfx -- --nocapture`
Expected: compile failure, `decode_byte3` doesn't exist.

- [ ] **Step 3: Implement `decode_byte3`**

Add to `crates/caiven-vm/src/vm/sfx.rs`, near the top (after the existing
`use` line):

```rust
use super::audio::{ENVELOPE_MS, PAN_TABLE};

/// Unpacks an SFX step's byte3: bits 0-3 select a pan position, bits 4-5
/// select an attack ramp length, bits 6-7 select a release ramp length.
/// `byte3 == 0` (every step that never touched the tracker's pan/envelope
/// controls) decodes to center pan and instant attack/release.
pub fn decode_byte3(byte3: u8) -> (f32, f32, f32) {
    let pan = PAN_TABLE[(byte3 & 0x0F) as usize];
    let attack = ENVELOPE_MS[((byte3 >> 4) & 0x03) as usize];
    let release = ENVELOPE_MS[((byte3 >> 6) & 0x03) as usize];
    (pan, attack, release)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p caiven-vm --lib vm::sfx -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Rewrite `tick_sfx_channel` to target a `Voice`**

In `crates/caiven-vm/src/vm/execution.rs`, change the import line:

```rust
use crate::vm::audio::{Voice, VoiceKind};
```

(drop `Sound`, `NoiseChannel`, `SquareChannel` from that import — `Sound`
is no longer touched directly by this function).

Replace `tick_sfx_channel` (original lines 10-81) with:

```rust
fn tick_sfx_channel(
    player: &mut SfxPlayer,
    memory: &Memory,
    voice: &mut Voice,
    forced_kind: Option<VoiceKind>,
    volume_scale: f32,
) {
    if !player.active {
        return;
    }

    if player.tick_count == 0 {
        let base = SfxPlayer::sfx_bytes_base(player.sfx_id, player.step);
        let note = memory.read(base).unwrap_or(0);
        let volume = memory.read(base + 1).unwrap_or(0);
        let wave = memory.read(base + 2).unwrap_or(0);
        let byte3 = memory.read(base + 3).unwrap_or(0);
        let (pan, attack_ms, release_ms) = super::sfx::decode_byte3(byte3);

        if note == 0 {
            voice.gate = false;
        } else {
            voice.kind = forced_kind.unwrap_or(if wave == 0 {
                VoiceKind::Square
            } else {
                VoiceKind::Noise
            });
            voice.frequency = note_to_freq(note);
            voice.volume = (volume as f32 / 15.0) * volume_scale;
            voice.pan = pan;
            voice.attack_ms = attack_ms;
            voice.release_ms = release_ms;
            voice.gate = true;
            voice.epoch = voice.epoch.wrapping_add(1);
        }
    }

    player.tick_count += 1;
    if player.tick_count >= player.ticks_per_step {
        player.tick_count = 0;
        player.step += 1;
        if player.step >= 16 {
            player.active = false;
            voice.gate = false;
        }
    }
}
```

Note what changed from today and why:
- One `Voice` in, one `Voice` out — no more reaching into a shared
  `Sound.square`/`.noise` pair, so two callers targeting different voices
  can never clobber each other. This is the clobbering-bug fix.
- `forced_wave: Option<u8>` (0/1) becomes `forced_kind: Option<VoiceKind>`
  — same meaning (music ch0 forces Square, ch1 forces Noise, ignoring the
  step's own wave byte — same as today, same reason, see the comment on
  `tick_music_player` below), just typed instead of a magic `u8`.
- `note == 0` / sequence-end now set `voice.gate = false` (release ramp)
  instead of hard `enabled = false` — this is the "per-note envelope"
  behavior from the spec, not a separate change.
- New `volume_scale` parameter: the legacy preview voice and music
  channels always pass `1.0`; the new SFX pool (Task 3) passes the
  `opts.volume` a Lua caller requested.

- [ ] **Step 6: Update `tick_sfx_player`/`tick_music_player`/`trigger_music_row` call sites**

These live in the same file (`execution.rs`, `impl Vm` block, original
lines 83-150) and currently borrow `self.sound` and pass `&mut s` (the
whole `Sound`). Task 3 adds the `sfx_pool` field these methods need to
also drive, and changes what indexes into `sound.voices` they target, so
the full rewrite of this `impl Vm` block happens in Task 3 Step 3 — for
this task, leave `tick_sfx_player`/`tick_music_player`/`run_frame` as they
are (they will not compile standalone after this step, since
`tick_sfx_channel`'s signature changed) and continue directly to Task 3
without a separate commit for this step. Do not run the full test suite
between Task 2 Step 5 and Task 3 Step 3 — `cargo test -p caiven-vm --lib
vm::sfx` and `vm::audio` (the tests added so far) will still pass in
isolation since neither touches `execution.rs`'s callers.

- [ ] **Step 7: Format and lint the files touched so far**

```bash
cargo fmt --all
```

(Skip clippy/commit here — this task's change to `execution.rs` doesn't
compile as a standalone commit; it's completed by Task 3. Stage but don't
commit yet: `git add crates/caiven-vm/src/vm/sfx.rs
crates/caiven-vm/src/vm/execution.rs`.)

---

### Task 3: `Vm`-level SFX voice pool and updated stop/start methods

**Files:**
- Modify: `crates/caiven-vm/src/vm/mod.rs`
- Modify: `crates/caiven-vm/src/vm/execution.rs` (continues Task 2)
- Test: `crates/caiven-vm/tests/lua_script.rs` (added in Task 4, after the
  Lua bindings exist to drive this from script — this task's own
  correctness is exercised indirectly by Task 4's tests, since
  `play_sfx_voice`/`stop_sfx_voice` have no Lua-independent test file of
  their own today, matching how `sfx_player`/`music_player` are already
  only tested via Lua script tests)

**Interfaces:**
- Consumes: `Voice`, `VoiceKind`, `Sound`, `VOICE_COUNT`, `MUSIC_VOICE_CH0`,
  `MUSIC_VOICE_CH1`, `LEGACY_SFX_VOICE`, `SFX_POOL_START`, `SFX_POOL_LEN`
  (Task 1); `tick_sfx_channel` (Task 2, same-crate private fn, called only
  from within `execution.rs`).
- Produces: `Vm::play_sfx_voice(&mut self, id: u8, volume: f32) -> u32`;
  `Vm::stop_sfx_voice(&mut self, handle: u32)`; unchanged signatures for
  `Vm::start_sfx`, `Vm::stop_sfx`, `Vm::start_music`, `Vm::stop_music`,
  `Vm::stop_audio`, `Vm::sfx_player()`, `Vm::music_player()`. These four
  new/changed methods are what Task 4's Lua closures call.

- [ ] **Step 1: Update `Vm`'s imports and struct fields**

In `crates/caiven-vm/src/vm/mod.rs`, change line 29:

```rust
use crate::vm::audio::{Sound, Voice, SFX_POOL_LEN};
```

Add a new struct (near `Vm`, e.g. directly above `pub struct Vm {`):

```rust
/// One slot of the round-robin SFX voice pool backing the polyphonic
/// `play_sfx`/`stop_sfx` Lua API. `age` is a monotonically increasing
/// counter set on every (re)trigger — the pool steals the slot with the
/// smallest `age` when all slots are busy, i.e. the one triggered longest
/// ago.
struct PooledSfx {
    player: SfxPlayer,
    age: u64,
    volume_scale: f32,
}

impl PooledSfx {
    fn new() -> Self {
        Self {
            player: SfxPlayer::new(),
            age: 0,
            volume_scale: 1.0,
        }
    }
}

/// Packs a pool slot index and its voice's current epoch into a single
/// handle returned to Lua. `stop_sfx_voice` decodes both and only acts if
/// the epoch still matches — a handle for a voice since stolen by another
/// `play_sfx` call is a silent no-op instead of stopping the wrong sound.
fn pack_sfx_handle(slot: u32, epoch: u32) -> u32 {
    (epoch << 3) | (slot & 0x7)
}

fn unpack_sfx_handle(handle: u32) -> (u32, u32) {
    (handle & 0x7, handle >> 3)
}
```

Add to the `Vm` struct fields (near `sfx_player`/`music_player`):

```rust
    sfx_pool: [PooledSfx; SFX_POOL_LEN],
    next_sfx_age: u64,
```

Add to `Vm::new`'s field initializers (near `sfx_player: SfxPlayer::new(),`):

```rust
            sfx_pool: std::array::from_fn(|_| PooledSfx::new()),
            next_sfx_age: 0,
```

Replace the `sound: Arc::new(Mutex::new(Sound { square: SquareChannel {
... }, noise: NoiseChannel { ... } }))` literal (original lines 235-248)
with:

```rust
            sound: Arc::new(Mutex::new(Sound::default())),
```

- [ ] **Step 2: Add `play_sfx_voice`/`stop_sfx_voice` and update the
      existing start/stop methods**

Replace the block from `pub fn start_sfx` through `pub fn set_music_loop`
(original lines 643-677) with:

```rust
    pub fn start_sfx(&mut self, id: u8) {
        self.sfx_player.start(id);
    }

    pub fn stop_sfx(&mut self) {
        self.sfx_player.stop();
        if let Ok(mut s) = self.sound.try_lock() {
            let v = &mut s.voices[audio::LEGACY_SFX_VOICE];
            v.gate = false;
            v.epoch = v.epoch.wrapping_add(1);
        }
    }

    pub fn start_music(&mut self, pattern_id: u8) {
        self.music_player.start(pattern_id);
    }

    pub fn stop_music(&mut self) {
        self.music_player.stop();
        if let Ok(mut s) = self.sound.try_lock() {
            for idx in [audio::MUSIC_VOICE_CH0, audio::MUSIC_VOICE_CH1] {
                let v = &mut s.voices[idx];
                v.gate = false;
                v.epoch = v.epoch.wrapping_add(1);
            }
        }
    }

    pub fn sfx_player(&self) -> &SfxPlayer {
        &self.sfx_player
    }

    pub fn music_player(&self) -> &MusicPlayer {
        &self.music_player
    }

    pub fn set_music_loop(&mut self, on: bool) {
        self.music_player.loop_on = on;
    }

    /// Starts sound effect `id` on a free (or, if all are busy, the
    /// least-recently-triggered) pool voice. `volume` is a `[0, 1]`
    /// multiplier layered on top of each step's authored volume. Returns
    /// an opaque handle for `stop_sfx_voice`.
    pub fn play_sfx_voice(&mut self, id: u8, volume: f32) -> u32 {
        let volume = volume.clamp(0.0, 1.0);
        let slot = self
            .sfx_pool
            .iter()
            .position(|p| !p.player.active)
            .unwrap_or_else(|| {
                self.sfx_pool
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, p)| p.age)
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            });

        self.next_sfx_age = self.next_sfx_age.wrapping_add(1);
        self.sfx_pool[slot].age = self.next_sfx_age;
        self.sfx_pool[slot].volume_scale = volume;
        self.sfx_pool[slot].player.start(id);

        let epoch = if let Ok(mut s) = self.sound.try_lock() {
            let voice = &mut s.voices[audio::SFX_POOL_START + slot];
            voice.epoch = voice.epoch.wrapping_add(1);
            voice.epoch
        } else {
            0
        };
        pack_sfx_handle(slot as u32, epoch)
    }

    /// Stops the voice `handle` refers to, if it's still the current
    /// occupant of that pool slot. Silent no-op for a handle whose voice
    /// already finished or was stolen by a later `play_sfx_voice` call.
    pub fn stop_sfx_voice(&mut self, handle: u32) {
        let (slot, epoch) = unpack_sfx_handle(handle);
        let slot = slot as usize;
        if slot >= self.sfx_pool.len() {
            return;
        }

        let still_current = if let Ok(mut s) = self.sound.try_lock() {
            let voice = &mut s.voices[audio::SFX_POOL_START + slot];
            if voice.epoch == epoch {
                voice.gate = false;
                true
            } else {
                false
            }
        } else {
            false
        };

        if still_current {
            self.sfx_pool[slot].player.stop();
        }
    }
```

This file needs `use crate::vm::audio;` (module-qualified, alongside the
existing `use crate::vm::audio::{Sound, Voice, SFX_POOL_LEN};`) to reach
`audio::LEGACY_SFX_VOICE` etc. without importing every constant
individually — add it next to the existing audio import from Step 1.

- [ ] **Step 3: Update `stop_audio`**

Replace `Vm::stop_audio` (original lines 324-331):

```rust
    pub fn stop_audio(&mut self) {
        self.sfx_player.stop();
        self.music_player.stop();
        for pooled in &mut self.sfx_pool {
            pooled.player.stop();
        }
        if let Ok(mut sound) = self.sound.lock() {
            for voice in &mut sound.voices {
                voice.gate = false;
                voice.epoch = voice.epoch.wrapping_add(1);
            }
        }
    }
```

- [ ] **Step 4: Finish `execution.rs`'s `impl Vm` block (started in Task 2)**

Replace `trigger_music_row` through `run_frame` (original lines 84-163)
with:

```rust
impl Vm {
    fn trigger_music_row(&mut self) {
        let base =
            MusicPlayer::pattern_row_base(self.music_player.pattern_id, self.music_player.row);
        let ch0_ref = self.memory.read(base).unwrap_or(0);
        let ch1_ref = self.memory.read(base + 1).unwrap_or(0);
        if ch0_ref > 0 {
            self.music_player.ch0.start(ch0_ref - 1);
        } else {
            self.music_player.ch0.active = false;
        }
        if ch1_ref > 0 {
            self.music_player.ch1.start(ch1_ref - 1);
        } else {
            self.music_player.ch1.active = false;
        }
    }

    fn tick_sfx_player(&mut self) {
        if !self.sfx_player.active {
            return;
        }
        if let Ok(mut s) = self.sound.try_lock() {
            tick_sfx_channel(
                &mut self.sfx_player,
                &self.memory,
                &mut s.voices[super::audio::LEGACY_SFX_VOICE],
                None,
                1.0,
            );
        }
    }

    fn tick_music_player(&mut self) {
        if !self.music_player.active {
            return;
        }

        // First tick of a new row: load SFX references into channel players
        if self.music_player.tick_count == 0 {
            self.trigger_music_row();
        }

        // Voice 0 is hard-assigned to ch0 (forced Square) and voice 1 to
        // ch1 (forced Noise) — the per-step `wave` byte the Music tracker
        // UI lets you set is intentionally ignored here to keep both
        // channels audible at once instead of one overriding the other;
        // it only does something for single-voice SFX playback.
        if let Ok(mut s) = self.sound.try_lock() {
            let (ch0_voice, rest) = s.voices.split_first_mut().expect("voices is non-empty");
            let ch1_voice = &mut rest[0];
            tick_sfx_channel(
                &mut self.music_player.ch0,
                &self.memory,
                ch0_voice,
                Some(super::audio::VoiceKind::Square),
                1.0,
            );
            tick_sfx_channel(
                &mut self.music_player.ch1,
                &self.memory,
                ch1_voice,
                Some(super::audio::VoiceKind::Noise),
                1.0,
            );
        }

        self.music_player.tick_count += 1;
        if self.music_player.tick_count >= self.music_player.ticks_per_row {
            self.music_player.tick_count = 0;
            self.music_player.row += 1;
            if self.music_player.row >= 16 {
                if self.music_player.loop_on {
                    self.music_player.row = 0;
                } else {
                    self.music_player.active = false;
                }
            }
        }
    }

    fn tick_sfx_pool(&mut self) {
        if let Ok(mut s) = self.sound.try_lock() {
            for (i, pooled) in self.sfx_pool.iter_mut().enumerate() {
                tick_sfx_channel(
                    &mut pooled.player,
                    &self.memory,
                    &mut s.voices[super::audio::SFX_POOL_START + i],
                    None,
                    pooled.volume_scale,
                );
            }
        }
    }

    /// Advances SFX/music playback one frame without running the program —
    /// lets editors preview audio while the game is stopped or paused.
    pub fn tick_audio_players(&mut self) {
        self.tick_music_player();
        self.tick_sfx_player();
        self.tick_sfx_pool();
    }

    pub fn run_frame(&mut self, input: &Input, font: &Font) {
        self.waiting = false;
        self.tick_music_player();
        self.tick_sfx_player();
        self.tick_sfx_pool();
        self.peripherals
            .tick_all(&mut self.memory, self.frame_count);
        self.frame_count = self.frame_count.wrapping_add(1);

        self.run_frame_lua(input, font);
        self.waiting = true;
    }
}
```

`s.voices.split_first_mut()` is used for ch0/ch1 instead of two direct
mutable indexes because Rust won't allow `&mut s.voices[0]` and `&mut
s.voices[1]` as two simultaneous borrows through plain indexing;
`split_first_mut` splits the array into a disjoint `(&mut Voice, &mut
[Voice])` pair, which the borrow checker accepts. `rest[0]` is voice index
1 since `split_first_mut` removed index 0.

`crates/caiven-vm/src/vm/execution.rs` needs `use crate::vm::audio;`
instead of the narrower `Voice, VoiceKind` import from Task 2 Step 5 (both
are used here via `super::audio::VoiceKind`/`super::audio::LEGACY_SFX_VOICE`
etc.) — keep the Task 2 `use crate::vm::audio::{Voice, VoiceKind};` for
`tick_sfx_channel`'s own signature, and add `use crate::vm::audio;` for
the module-qualified constant access in `impl Vm`. Both imports coexist
fine.

- [ ] **Step 5: Run the full existing VM test suite**

Run: `cargo test -p caiven-vm`
Expected: PASS. This exercises every existing test that plays SFX/music
through `Vm` (via `crates/caiven-vm/tests/lua_script.rs`'s existing
`play_sfx`/`play_music` tests), which is the regression check that
nothing about note-triggering, sequencing, or looping broke.

- [ ] **Step 6: Format, lint, commit**

```bash
cargo fmt --all
cargo clippy -p caiven-vm --all-targets -- -D warnings -A unused-imports
git add crates/caiven-vm/src/vm/mod.rs crates/caiven-vm/src/vm/execution.rs crates/caiven-vm/src/vm/sfx.rs
git commit -m "$(cat <<'EOF'
feat(vm): drive SFX/music playback through the voice pool, fix clobbering

- tick_sfx_player/tick_music_player/new tick_sfx_pool each target their
  own Voice slot instead of a single shared Sound, which is what actually
  fixes SFX interrupting concurrent music (structural fix, not a special
  case)
- new Vm::play_sfx_voice/stop_sfx_voice back a 5-slot round-robin pool
  (voices 3-7) with oldest-steals-when-full and epoch-guarded stop, ready
  for the Lua binding in the next commit
- decode_byte3 unpacks the SFX step's previously-unread byte3 into
  pan/attack/release
EOF
)"
```

---

### Task 4: Lua API — `play_sfx(id, opts)`, `stop_sfx(handle)`, volume controls

**Files:**
- Modify: `crates/caiven-vm/src/vm/lua_exec.rs`
- Modify: `crates/caiven-vm/src/vm/api_registry.rs`
- Modify: `docs/api-reference.md`
- Test: `crates/caiven-vm/tests/lua_script.rs`

**Interfaces:**
- Consumes: `Vm::play_sfx_voice`, `Vm::stop_sfx_voice` (Task 3);
  `Sound.master_volume`/`music_volume`/`sfx_volume` (Task 1) via
  `self.sound`.
- Produces: Lua globals `play_sfx(id, opts?)` (returns integer handle),
  `stop_sfx(handle)`, `set_music_volume(v)`, `set_sfx_volume(v)`,
  `set_master_volume(v)`; unchanged `play_music(id)`, `stop_music()`.

- [ ] **Step 1: Write the failing VM-level tests**

First, read the existing `play_sfx`/`play_music` tests in
`crates/caiven-vm/tests/lua_script.rs` to match their harness style (cart
construction helper, `run_frame` driving) — reuse whatever helper those
tests already use to build a `Vm` with SFX bank data loaded, rather than
duplicating setup.

Add to `crates/caiven-vm/tests/lua_script.rs`:

```rust
#[test]
fn play_sfx_returns_a_distinct_handle_per_call() {
    let mut vm = /* existing test harness: Vm with an SFX bank whose slot 0
                    has a note on step 0 */;
    vm.load_lua_source(
        "handle_a = 0\nhandle_b = 0\nfunction _init() handle_a = play_sfx(0) handle_b = play_sfx(0) end",
        &Input::new(),
        &Font::test_default(),
    )
    .unwrap();
    vm.run_frame(&Input::new(), &Font::test_default());
    let handle_a: i64 = vm.eval_global("handle_a");
    let handle_b: i64 = vm.eval_global("handle_b");
    assert_ne!(handle_a, handle_b);
}

#[test]
fn play_sfx_is_polyphonic_two_concurrent_calls_stay_independently_audible() {
    // Two play_sfx calls on two different SFX slots while both are still
    // mid-sequence must occupy distinct voices — assert via the public
    // Vm::sound accessor (or equivalent test hook) that two voices in the
    // 3..8 pool range are simultaneously gated.
}

#[test]
fn seventh_concurrent_play_sfx_steals_the_oldest_voice() {
    // 5 pool voices (3..8); 6 concurrent play_sfx calls in one frame must
    // not error and must result in the pool having exactly 5 active
    // voices, with the first call's voice no longer holding its original
    // handle's epoch.
}

#[test]
fn play_sfx_does_not_disturb_concurrent_music_playback() {
    // play_music(...) then play_sfx(...) in the same frame: assert
    // voices[0]/voices[1] (music) are unaffected by the SFX call — this
    // is the regression test for the clobbering bug.
}

#[test]
fn stop_sfx_on_an_active_handle_releases_it() {
    // play_sfx -> stop_sfx(handle) -> assert that voice's gate is false.
}

#[test]
fn stop_sfx_on_a_stale_handle_is_a_silent_no_op() {
    // Steal a handle's voice via 6 concurrent play_sfx calls (see above),
    // then call stop_sfx on the now-stale first handle — assert no error
    // and the voice that stole the slot is untouched (still gated).
}

#[test]
fn volume_setters_clamp_to_zero_one_and_scale_output() {
    // set_master_volume(-1) then read back via whatever accessor exposes
    // Sound.master_volume for tests -> assert 0.0. set_master_volume(5) ->
    // assert 1.0.
}
```

Fill in each test body against the actual harness helpers already present
in `crates/caiven-vm/tests/lua_script.rs` (e.g. however existing
`play_sfx`/`play_music` tests construct a `Vm`, load an SFX bank via
`load_section_to_ram` or the cart-loading path, and read back state — this
plan doesn't restate that harness because Task 4's implementer must read
the existing file first, per this skill's "explore before editing" norm,
and the exact helper names weren't captured verbatim in this planning
pass). Each test needs a way to read `Sound` state back — if no such test
hook exists yet, add a `#[cfg(test)] pub fn sound_snapshot(&self) -> Sound`
to `Vm` (guarded so it doesn't widen the production public surface) for
these tests to use.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p caiven-vm --test lua_script play_sfx`
Expected: compile failure or FAIL — `play_sfx` in Lua still takes no
`opts` and returns nothing; `stop_sfx`/`set_*_volume` don't exist as Lua
globals yet.

- [ ] **Step 3: Update `register_builtins`' signature and the `play_sfx`/`stop_music` block**

In `crates/caiven-vm/src/vm/lua_exec.rs`, `register_builtins` needs access
to the SFX pool. The pool state lives on `Vm` itself (`sfx_pool`,
`next_sfx_age`), not reachable through the existing per-call `RefCell<&mut
T>` borrows
this function takes for individual fields — `play_sfx_voice`/
`stop_sfx_voice` (Task 3) are `&mut self` methods on `Vm`, but
`register_builtins` only ever receives individual field borrows, never
`&mut Vm`, to avoid re-borrowing conflicts with the `lua.scope` closure.
Match that existing pattern instead of trying to smuggle `&mut Vm`
through: add three more `'env RefCell<&'env mut T>` parameters mirroring
`sfx_player`/`music_player`, for `sound: Arc<Mutex<Sound>>` (cloned, not
borrowed — it's already share-by-clone) and the same pool-allocation logic
duplicated here as free functions operating on borrowed `&mut [PooledSfx;
SFX_POOL_LEN]` and `&mut u64`, matching what `Vm::play_sfx_voice`/
`stop_sfx_voice` do internally. To avoid duplicating that allocation logic
in two places, change `Task 3 Step 2`'s `play_sfx_voice`/`stop_sfx_voice`
bodies to be free functions taking their state as parameters, and have
`Vm::play_sfx_voice`/`stop_sfx_voice` call them:

```rust
// crates/caiven-vm/src/vm/mod.rs, module-level (not inside impl Vm):
fn allocate_sfx_voice(
    pool: &mut [PooledSfx; audio::SFX_POOL_LEN],
    next_age: &mut u64,
    sound: &Arc<Mutex<Sound>>,
    id: u8,
    volume: f32,
) -> u32 {
    let volume = volume.clamp(0.0, 1.0);
    let slot = pool
        .iter()
        .position(|p| !p.player.active)
        .unwrap_or_else(|| {
            pool.iter()
                .enumerate()
                .min_by_key(|(_, p)| p.age)
                .map(|(i, _)| i)
                .unwrap_or(0)
        });

    *next_age = next_age.wrapping_add(1);
    pool[slot].age = *next_age;
    pool[slot].volume_scale = volume;
    pool[slot].player.start(id);

    let epoch = if let Ok(mut s) = sound.try_lock() {
        let voice = &mut s.voices[audio::SFX_POOL_START + slot];
        voice.epoch = voice.epoch.wrapping_add(1);
        voice.epoch
    } else {
        0
    };
    pack_sfx_handle(slot as u32, epoch)
}

fn release_sfx_voice(
    pool: &mut [PooledSfx; audio::SFX_POOL_LEN],
    sound: &Arc<Mutex<Sound>>,
    handle: u32,
) {
    let (slot, epoch) = unpack_sfx_handle(handle);
    let slot = slot as usize;
    if slot >= pool.len() {
        return;
    }
    let still_current = if let Ok(mut s) = sound.try_lock() {
        let voice = &mut s.voices[audio::SFX_POOL_START + slot];
        if voice.epoch == epoch {
            voice.gate = false;
            true
        } else {
            false
        }
    } else {
        false
    };
    if still_current {
        pool[slot].player.stop();
    }
}
```

And simplify `Vm::play_sfx_voice`/`stop_sfx_voice` (Task 3 Step 2) to:

```rust
    pub fn play_sfx_voice(&mut self, id: u8, volume: f32) -> u32 {
        allocate_sfx_voice(&mut self.sfx_pool, &mut self.next_sfx_age, &self.sound, id, volume)
    }

    pub fn stop_sfx_voice(&mut self, handle: u32) {
        release_sfx_voice(&mut self.sfx_pool, &self.sound, handle)
    }
```

(This is a Task 3 amendment made now, in Task 4, because the need to call
the same allocation logic from both `&mut Vm` methods and from
`register_builtins`'s borrowed-field closures only becomes apparent once
Task 4's Lua binding is being wired — go back and apply this simplification
to Task 3's code before proceeding, so both call sites share one
implementation.)

Now add to `register_builtins`'s parameter list (after `music_player:
&'env RefCell<&'env mut MusicPlayer>`):

```rust
    sfx_pool: &'env RefCell<&'env mut [PooledSfx; SFX_POOL_LEN]>,
    next_sfx_age: &'env RefCell<&'env mut u64>,
    sound: Arc<Mutex<Sound>>,
```

`PooledSfx` needs `pub(crate)` visibility (currently private to `mod.rs`)
so `lua_exec.rs` can name the type — change `struct PooledSfx` to
`pub(crate) struct PooledSfx` and its `player`/`age`/`volume_scale` fields
to `pub(crate)` in `mod.rs` (Task 3 amendment). Same for `pack_sfx_handle`/
`unpack_sfx_handle`: make them `pub(crate) fn` so `lua_exec.rs` can reuse
them, or duplicate the two one-liners locally in `lua_exec.rs` — duplicating
the two 1-line functions is simpler than widening more of `mod.rs`'s
crate-visible surface; do that instead (define `pack_sfx_handle`/
`unpack_sfx_handle` as private functions in `lua_exec.rs` too, byte-for-byte
identical to `mod.rs`'s versions, and leave `mod.rs`'s versions as they
are — both are private `fn`s in different modules, no conflict).

Replace the `play_sfx`/`play_music`/`stop_music` block (original lines
1285-1307) with:

```rust
    globals.set(
        "play_sfx",
        scope.create_function_mut(move |_, (id, opts): (u8, Option<mlua::Table>)| {
            let volume = match &opts {
                Some(t) => t.get::<Option<f64>>("volume")?.unwrap_or(1.0) as f32,
                None => 1.0,
            };
            let handle = allocate_sfx_voice(
                &mut sfx_pool.borrow_mut(),
                &mut next_sfx_age.borrow_mut(),
                &sound,
                id,
                volume,
            );
            Ok(handle)
        })?,
    )?;

    globals.set(
        "stop_sfx",
        scope.create_function_mut(move |_, handle: u32| {
            release_sfx_voice(&mut sfx_pool.borrow_mut(), &sound, handle);
            Ok(())
        })?,
    )?;

    globals.set(
        "play_music",
        scope.create_function_mut(|_, id: u8| {
            music_player.borrow_mut().start(id);
            Ok(())
        })?,
    )?;

    globals.set(
        "stop_music",
        scope.create_function_mut(|_, ()| {
            music_player.borrow_mut().stop();
            Ok(())
        })?,
    )?;

    globals.set(
        "set_master_volume",
        scope.create_function_mut(move |_, v: f64| {
            if let Ok(mut s) = sound.try_lock() {
                s.master_volume = (v as f32).clamp(0.0, 1.0);
            }
            Ok(())
        })?,
    )?;

    globals.set(
        "set_music_volume",
        scope.create_function_mut(move |_, v: f64| {
            if let Ok(mut s) = sound.try_lock() {
                s.music_volume = (v as f32).clamp(0.0, 1.0);
            }
            Ok(())
        })?,
    )?;

    globals.set(
        "set_sfx_volume",
        scope.create_function_mut(move |_, v: f64| {
            if let Ok(mut s) = sound.try_lock() {
                s.sfx_volume = (v as f32).clamp(0.0, 1.0);
            }
            Ok(())
        })?,
    )?;
```

`sound` is moved into 5 separate closures above (`play_sfx`, `stop_sfx`,
and the three volume setters) — `Arc<Mutex<Sound>>` is `Clone`, so pass
`sound.clone()` into every closure but the last that uses it (or just
`.clone()` into all five for uniformity/simplicity, `Arc::clone` is cheap).

Add module-level (top of `lua_exec.rs`, near other imports):

```rust
use crate::vm::audio::{Sound, SFX_POOL_START};
use crate::vm::{PooledSfx, SFX_POOL_LEN};
use std::sync::{Arc, Mutex};

fn pack_sfx_handle(slot: u32, epoch: u32) -> u32 {
    (epoch << 3) | (slot & 0x7)
}

fn unpack_sfx_handle(handle: u32) -> (u32, u32) {
    (handle & 0x7, handle >> 3)
}

fn allocate_sfx_voice(
    pool: &mut [PooledSfx; SFX_POOL_LEN],
    next_age: &mut u64,
    sound: &Arc<Mutex<Sound>>,
    id: u8,
    volume: f32,
) -> u32 {
    // identical body to mod.rs's free function of the same name — see
    // Task 3/4 note above on why this exists in two modules.
    let volume = volume.clamp(0.0, 1.0);
    let slot = pool
        .iter()
        .position(|p| !p.player.active)
        .unwrap_or_else(|| {
            pool.iter()
                .enumerate()
                .min_by_key(|(_, p)| p.age)
                .map(|(i, _)| i)
                .unwrap_or(0)
        });
    *next_age = next_age.wrapping_add(1);
    pool[slot].age = *next_age;
    pool[slot].volume_scale = volume;
    pool[slot].player.start(id);
    let epoch = if let Ok(mut s) = sound.try_lock() {
        let voice = &mut s.voices[SFX_POOL_START + slot];
        voice.epoch = voice.epoch.wrapping_add(1);
        voice.epoch
    } else {
        0
    };
    pack_sfx_handle(slot as u32, epoch)
}

fn release_sfx_voice(pool: &mut [PooledSfx; SFX_POOL_LEN], sound: &Arc<Mutex<Sound>>, handle: u32) {
    let (slot, epoch) = unpack_sfx_handle(handle);
    let slot = slot as usize;
    if slot >= pool.len() {
        return;
    }
    let still_current = if let Ok(mut s) = sound.try_lock() {
        let voice = &mut s.voices[SFX_POOL_START + slot];
        if voice.epoch == epoch {
            voice.gate = false;
            true
        } else {
            false
        }
    } else {
        false
    };
    if still_current {
        pool[slot].player.stop();
    }
}
```

This duplicates `allocate_sfx_voice`/`release_sfx_voice`/`pack_sfx_handle`/
`unpack_sfx_handle` between `mod.rs` (for `Vm::play_sfx_voice`/
`stop_sfx_voice`, used by Studio/tests calling the Rust API directly) and
`lua_exec.rs` (for the Lua closures, which can't borrow `&mut Vm` inside
`lua.scope`). If this duplication is uncomfortable during review, the
alternative is moving `PooledSfx`, `allocate_sfx_voice`, and
`release_sfx_voice` into a new small module (`crates/caiven-vm/src/vm/sfx_pool.rs`)
that both `mod.rs` and `lua_exec.rs` import — prefer that if the reviewer
flags the duplication; either is correct, the module split is just less
plan-writing certainty about exact `pub(crate)` visibility wiring, so this
plan specifies the duplicated version as the concrete baseline.

- [ ] **Step 4: Wire the 3 new parameters at all 4 `register_builtins` call sites**

In `crates/caiven-vm/src/vm/lua_exec.rs`, at each of the 4 locations that
currently do:

```rust
        let sfx_player = RefCell::new(&mut self.sfx_player);
        let music_player = RefCell::new(&mut self.music_player);
```

(lines ~1503-1504, ~1591-1592, ~1670-1671, ~1993-1994), add immediately
after:

```rust
        let sfx_pool = RefCell::new(&mut self.sfx_pool);
        let next_sfx_age = RefCell::new(&mut self.next_sfx_age);
        let sound = self.sound.clone();
```

And at each of the corresponding `register_builtins(...)` call argument
lists (immediately after the `&music_player,` argument), add:

```rust
                &sfx_pool,
                &next_sfx_age,
                sound.clone(),
```

`self.sfx_pool`'s field type in `mod.rs` needs to be `pub(crate)` (it's
currently a private-to-`mod.rs` field via Task 3) for `lua_exec.rs`
(a sibling module inside the same `vm` module tree) to borrow it directly
as `&mut self.sfx_pool` — since `lua_exec.rs` is `mod lua_exec` inside
`mod vm`, and `Vm`'s fields are declared without any `pub` in `mod.rs`
today (private to the `vm` module, not to `mod.rs` specifically), plain
private fields are already visible to `lua_exec.rs` as a sibling submodule
of `vm`. No visibility change needed — confirm this compiles as-is; if it
doesn't (Rust's privacy is per-module, and `lua_exec` is a child of `vm`,
same as `mod.rs`'s contents, so it should just work), that's the one spot
in this task worth double-checking against a real compile rather than
this plan's field-visibility assumption.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p caiven-vm`
Expected: PASS, including Task 4 Step 1's new tests and the full existing
suite (regression check).

- [ ] **Step 6: Add `api_registry.rs` entries**

In `crates/caiven-vm/src/vm/api_registry.rs`, replace the `play_sfx`/
`play_music`/`stop_music` entries (original lines 270-287) with:

```rust
    ApiEntry {
        name: "play_sfx",
        params: &[param!("id": "u8"), param!("opts": "{volume: number}?")],
        returns: "integer",
        doc: "Start sound effect id on a free (or, if all are busy, oldest) voice. opts.volume (default 1.0) scales the step's authored volume. Returns a handle for stop_sfx. Multiple concurrent play_sfx calls are independently audible.",
    },
    ApiEntry {
        name: "stop_sfx",
        params: &[param!("handle": "integer")],
        returns: "nil",
        doc: "Stops the voice handle refers to (release ramp, not an instant cut). Silent no-op if that voice already finished or was reused by a later play_sfx call.",
    },
    ApiEntry {
        name: "play_music",
        params: &[param!("id": "u8")],
        returns: "nil",
        doc: "Start music track id, looping.",
    },
    ApiEntry {
        name: "stop_music",
        params: &[],
        returns: "nil",
        doc: "Stop the currently playing music track.",
    },
    ApiEntry {
        name: "set_master_volume",
        params: &[param!("volume": "number")],
        returns: "nil",
        doc: "Runtime-only output multiplier, clamped to [0, 1]. Not persisted to cart data.",
    },
    ApiEntry {
        name: "set_music_volume",
        params: &[param!("volume": "number")],
        returns: "nil",
        doc: "Runtime-only multiplier applied to music channels only, clamped to [0, 1]. Not persisted to cart data.",
    },
    ApiEntry {
        name: "set_sfx_volume",
        params: &[param!("volume": "number")],
        returns: "nil",
        doc: "Runtime-only multiplier applied to all SFX voices, clamped to [0, 1]. Not persisted to cart data.",
    },
```

- [ ] **Step 7: Update `docs/api-reference.md`**

Replace the 3-row audio table (`docs/api-reference.md:46-48`) with:

```markdown
| `play_sfx(id, opts)`     | Start SFX id on a free/oldest voice. `opts.volume` (0-1, default 1) is optional. Returns a handle. Polyphonic — concurrent calls get independent voices. |
| `stop_sfx(handle)`       | Stop the voice `handle` refers to. Silent no-op if it already finished or was reused. |
| `play_music(id)`         | Play a music track, looping |
| `stop_music()`           | Stop music |
| `set_master_volume(v)`   | Runtime-only output multiplier, `v` clamped to `[0, 1]` |
| `set_music_volume(v)`    | Runtime-only music-channel multiplier, `v` clamped to `[0, 1]` |
| `set_sfx_volume(v)`      | Runtime-only SFX-voice multiplier, `v` clamped to `[0, 1]` |
```

Read the surrounding context in `docs/api-reference.md` first to match its
existing table formatting exactly (column widths/alignment) before
committing this edit.

Also add a short paragraph near this table (or in whatever section already
documents the SFX bank byte layout, if one exists — search
`docs/api-reference.md` and `README.md` for "SFX bank"/"4 bytes" before
adding a new one) documenting byte3's new pan/attack/release packing, for
anyone hand-authoring an SFX bank outside Studio:

```markdown
Each SFX step is 4 bytes: `note, volume, wave, byte3`. `byte3` packs pan
(bits 0-3, index into a 16-position table, 0 = center) and attack/release
envelope levels (bits 4-5 / 6-7, each 0-3 mapping to instant/~15ms/~50ms/~150ms
ramps). `byte3 = 0` is center pan with an instant on/off envelope.
```

- [ ] **Step 8: Format, lint, commit**

```bash
cargo fmt --all
cargo clippy -p caiven-vm --all-targets -- -D warnings -A unused-imports
cargo test -p caiven-vm
git add crates/caiven-vm/src/vm/lua_exec.rs crates/caiven-vm/src/vm/mod.rs crates/caiven-vm/src/vm/api_registry.rs docs/api-reference.md crates/caiven-vm/tests/lua_script.rs
git commit -m "$(cat <<'EOF'
feat(lua-api): polyphonic play_sfx/stop_sfx, master/music/sfx volume

- play_sfx(id, opts) now returns a voice handle and accepts opts.volume;
  concurrent calls get independent voices from a 5-slot round-robin pool
  instead of restarting a single shared slot
- stop_sfx(handle) releases a specific voice; silent no-op on a handle
  whose voice already finished or was stolen, since a cart tracking a
  handle across frames shouldn't have to guard every call
- set_master_volume/set_music_volume/set_sfx_volume: runtime-only [0,1]
  multipliers, not persisted to cart data
- play_music(id)/stop_music() signatures unchanged
EOF
)"
```

---

### Task 5: Studio SFX tracker — drop the fx column, add pan/attack/release

**Files:**
- Modify: `crates/caiven-studio-ui/src/components/Workspace.svelte`

**Interfaces:**
- Consumes: nothing from earlier tasks (frontend-only; the wire format for
  byte3 was already unconditionally read/written by this file before this
  change, just under the old "fx" meaning).
- Produces: nothing consumed elsewhere in this plan.

- [ ] **Step 1: Remove the fx column**

In `crates/caiven-studio-ui/src/components/Workspace.svelte`:
- Delete the `sfxEffects` array (lines 728-733).
- Delete the `<span class="sfx-label-row">fx</span>` label (line 1460).
- Delete the entire `<div class="sfx-fx">...</div>` block (lines
  1530-1539).
- Delete the `<p class="sfx-hints subtle">The effect column is stored in
  the cart, but the VM does not apply it yet.</p>` line (1549) — no longer
  true once pan/envelope are wired up, and the column it referred to is
  gone.

- [ ] **Step 2: Add pack/unpack helpers matching Task 2's `decode_byte3` bit layout**

Add near the existing `sfxByte`/`setSfxCells` helpers (after line 756):

```ts
// byte3 packs pan (bits 0-3, 0=center) and attack/release envelope levels
// (bits 4-5 / 6-7, each 0-3). Mirrors crates/caiven-vm/src/vm/sfx.rs::decode_byte3.
const PAN_LABELS = ['C', 'L1', 'R1', 'L2', 'R2', 'L3', 'R3', 'L4', 'R4', 'L5', 'R5', 'L6', 'R6', 'L7', 'R7', 'HL'];
const ENV_LABELS = ['—', 'fast', 'med', 'slow'];

const sfxPan = (step: number) => sfxByte(step, 3) & 0x0F;
const sfxAttack = (step: number) => (sfxByte(step, 3) >> 4) & 0x03;
const sfxRelease = (step: number) => (sfxByte(step, 3) >> 6) & 0x03;

function packByte3(pan: number, attack: number, release: number) {
  return (pan & 0x0F) | ((attack & 0x03) << 4) | ((release & 0x03) << 6);
}

function setSfxPan(step: number, pan: number) {
  const current = sfxByte(step, 3);
  setSfxCells([{ step, field: 3, value: packByte3(pan, (current >> 4) & 0x03, (current >> 6) & 0x03) }]);
}

function setSfxAttack(step: number, attack: number) {
  const current = sfxByte(step, 3);
  setSfxCells([{ step, field: 3, value: packByte3(current & 0x0F, attack, (current >> 6) & 0x03) }]);
}

function setSfxRelease(step: number, release: number) {
  const current = sfxByte(step, 3);
  setSfxCells([{ step, field: 3, value: packByte3(current & 0x0F, (current >> 4) & 0x03, release) }]);
}
```

- [ ] **Step 3: Add pan/attack/release rows to the tracker markup**

In the `sfx-labels` block (after the existing `<span class="sfx-label-row">wave</span>`,
around line 1459), add:

```svelte
            <span class="sfx-label-row">pan</span>
            <span class="sfx-label-row">atk</span>
            <span class="sfx-label-row">rel</span>
```

In `sfx-columns`, after the existing `<div class="sfx-wave">...</div>`
block (after line 1528), add three step-cycling button rows following the
exact pattern the `sfx-wave` column already uses (disabled when the step
has no note, click cycles to the next value):

```svelte
            <div class="sfx-pan">
              {#each Array(16) as _, step}
                {@const empty = sfxByte(step, 0) === 0}
                {@const pan = sfxPan(step)}
                <button
                  class:empty
                  disabled={empty}
                  title={empty ? 'No note on this step' : `Pan ${PAN_LABELS[pan]}`}
                  onclick={() => setSfxPan(step, (pan + 1) % 16)}
                >{empty ? '·' : PAN_LABELS[pan]}</button>
              {/each}
            </div>

            <div class="sfx-attack">
              {#each Array(16) as _, step}
                {@const empty = sfxByte(step, 0) === 0}
                {@const attack = sfxAttack(step)}
                <button
                  class:empty
                  disabled={empty}
                  title={empty ? 'No note on this step' : `Attack ${ENV_LABELS[attack]}`}
                  onclick={() => setSfxAttack(step, (attack + 1) % 4)}
                >{empty ? '·' : ENV_LABELS[attack]}</button>
              {/each}
            </div>

            <div class="sfx-release">
              {#each Array(16) as _, step}
                {@const empty = sfxByte(step, 0) === 0}
                {@const release = sfxRelease(step)}
                <button
                  class:empty
                  disabled={empty}
                  title={empty ? 'No note on this step' : `Release ${ENV_LABELS[release]}`}
                  onclick={() => setSfxRelease(step, (release + 1) % 4)}
                >{empty ? '·' : ENV_LABELS[release]}</button>
              {/each}
            </div>
```

- [ ] **Step 4: Add CSS for the 3 new rows**

Find the existing `.sfx-wave button` / `.sfx-fx button` CSS rules (search
this file's `<style>` block for `.sfx-fx` and `.sfx-wave`) and add
`.sfx-pan`, `.sfx-attack`, `.sfx-release` rules reusing the same
grid-column/button styling as `.sfx-wave` (16-cell row, same button
sizing) — delete the now-unused `.sfx-fx` rule (its markup was removed in
Step 1) rather than leaving dead CSS behind. Match the file's existing
class-per-row grid structure exactly; read the CSS block before editing so
the new rows visually align with the existing step columns.

- [ ] **Step 5: Manual verification**

```bash
cd crates/caiven-studio-ui && npm run check
```

Then start the dev app (`npm run tauri dev` or however this project's
`scripts/claude-session.sh ui-debug` profile launches it — check
`crates/caiven-studio/CLAUDE.md` if unsure) and manually verify in the
SFX editor (F4 per `projects/dev/audio_test/main.lua`'s own comment):
draw a note, click through its pan/attack/release cells, confirm the
values persist (undo/redo still works via the existing `sfxHistory`
mechanism, untouched by this task), and confirm Play still previews sound
via the existing `onAudio('sfx', ...)` path.

- [ ] **Step 6: Commit**

```bash
cd /Users/andrejmarkus/Projects/Rust/caiven
git add crates/caiven-studio-ui/src/components/Workspace.svelte
git commit -m "$(cat <<'EOF'
feat(studio): SFX tracker pan/attack/release controls, drop dead fx column

- the "fx" column (SL/VB/DR) never drove any audible behavior in the VM;
  its bit range is reclaimed by the VM-side pan/envelope change
- new pan/atk/rel rows pack into the same byte3 the fx column used,
  mirroring crates/caiven-vm/src/vm/sfx.rs::decode_byte3's bit layout
EOF
)"
```

---

### Task 6: Example cart update

**Files:**
- Modify: `projects/dev/audio_test/main.lua`

**Interfaces:**
- Consumes: `play_sfx(id, opts)`, `stop_sfx(handle)`, `play_music(id)`
  (Task 4).
- Produces: nothing consumed elsewhere.

- [ ] **Step 1: Extend the demo script**

Replace `projects/dev/audio_test/main.lua`:

```lua
-- Audio test — press buttons to trigger SFX bank slots
-- UP: slot 0 (left pan)   DOWN: slot 1 (right pan)
-- LEFT: slot 2 (noise)    RIGHT: slot 3, held (release on button-up)
-- START: toggle background music, to show it keeps playing under SFX
-- Paint sounds into these slots in the Caiven Studio SFX tab (F4)

local held_handle = nil

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
  draw_text("START: toggle music", 4, 84, 1)

  if button_pressed(0) then play_sfx(0) end
  if button_pressed(1) then play_sfx(1) end
  if button_pressed(2) then play_sfx(2) end
  if button_pressed(3) then held_handle = play_sfx(3, {volume = 0.8}) end
  if button_released(3) and held_handle then
    stop_sfx(held_handle)
    held_handle = nil
  end
  if button_pressed(7) then
    if music_active then stop_music() else play_music(0) end
    music_active = not music_active
  end
end
```

If `button_released` doesn't exist yet (it's listed as future/out-of-scope
input work in the spec's "Out of scope" section — confirm with `grep -n
"button_released" crates/caiven-vm/src/vm/lua_exec.rs`), replace that
branch with a duration-based release approximation using
`button_pressed`/a held-frame counter instead, or drop the hold/stop_sfx
demonstration and just call `play_sfx(3, {volume = 0.8})` on press like
the other three. Verify which is true before writing this file — don't
guess.

Also verify `music_active` needs declaring as a script-global (`local
music_active = false` near the top, alongside `held_handle`) — Lua
globals default to `nil`, and `not nil` is `true`, so the toggle works
even undeclared, but declaring it `local` at file scope (outside any
function) makes it a true global in this VM's model (top-level `local` in
the main chunk is still script-global here, matching how `held_handle` is
declared) — match whatever pattern `projects/dev/stdlib_demo/main.lua` or
similar existing carts use for persistent state between `_update()` calls
before finalizing this.

- [ ] **Step 2: Manual verification**

Load this cart in `caiven-machine` or Studio, confirm: UP/DOWN sounds pan
left/right, LEFT plays noise, holding RIGHT sustains and releasing it cuts
the note via `stop_sfx`, and START starts/stops music that keeps playing
audibly under any SFX triggered while it's active (the clobbering-bug
regression check, by ear).

- [ ] **Step 3: Commit**

```bash
git add projects/dev/audio_test/main.lua
git commit -m "$(cat <<'EOF'
docs(examples): extend audio_test cart for pan, stop_sfx, and music+sfx

- demonstrates play_sfx's new opts.volume and returned handle, stop_sfx,
  and that music keeps playing under a concurrent SFX hit
EOF
)"
```

---

## Final gate

After all 6 tasks:

```bash
scripts/claude/check-rust.sh -a
cd crates/caiven-studio-ui && npm run check
```

Both must pass before considering this plan complete.
