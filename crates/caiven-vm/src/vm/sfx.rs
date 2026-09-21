use super::audio::{ENVELOPE_MS, MUSIC_VOICE_COUNT, PAN_TABLE};
use super::memory::Memory;
use caiven_core::memory::{
    MUSIC_LOOP_POINT_OFFSET, MUSIC_ORDER_OFFSET, MUSIC_ORDER_STEPS, MUSIC_PATTERN_COUNT,
    MUSIC_PATTERN_ROWS,
};
pub use caiven_core::memory::{MUSIC_RAM_BASE as MUSIC_BANK_BASE, SFX_RAM_BASE as SFX_BANK_BASE};
const SFX_STEPS: u8 = 16;

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

pub fn note_to_freq(note: u8) -> f32 {
    440.0 * 2.0f32.powf((note as f32 - 49.0) / 12.0)
}

const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

pub fn note_name(note: u8) -> String {
    if note == 0 {
        return "---".to_string();
    }
    // Note 1 is 27.5 Hz (A0), so names count from A, not C.
    let semitones_from_c0 = note as usize + 8;
    format!(
        "{}{}",
        NOTE_NAMES[semitones_from_c0 % 12],
        semitones_from_c0 / 12
    )
}

#[cfg(test)]
mod note_name_tests {
    use super::{note_name, note_to_freq};

    #[test]
    fn names_match_frequencies() {
        assert_eq!(note_name(49), "A4");
        assert!((note_to_freq(49) - 440.0).abs() < 0.01);
        assert_eq!(note_name(1), "A0");
        assert_eq!(note_name(4), "C1");
    }
}

#[derive(Clone)]
pub struct SfxPlayer {
    pub active: bool,
    pub sfx_id: u8,
    pub step: u8,
    pub tick_count: u8,
    pub ticks_per_step: u8,
}

impl Default for SfxPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl SfxPlayer {
    pub fn new() -> Self {
        Self {
            active: false,
            sfx_id: 0,
            step: 0,
            tick_count: 0,
            ticks_per_step: 4,
        }
    }

    pub fn start(&mut self, id: u8) {
        self.sfx_id = id;
        self.step = 0;
        self.tick_count = 0;
        self.active = true;
    }

    pub fn stop(&mut self) {
        self.active = false;
    }

    pub fn sfx_bytes_base(sfx_id: u8, step: u8) -> usize {
        SFX_BANK_BASE + (sfx_id as usize) * (SFX_STEPS as usize * 4) + (step as usize) * 4
    }
}

#[derive(Clone)]
pub struct MusicPlayer {
    pub active: bool,
    pub pattern_id: u8,
    pub row: u8,
    pub tick_count: u8,
    pub ticks_per_row: u8,
    pub loop_on: bool,
    /// True while playback is sequenced by the bank's song order table rather
    /// than looping a single pattern.
    pub song_active: bool,
    /// Current index into the song order table while `song_active`.
    pub song_step: u8,
    /// One player per typed music channel, in tracker column order. Each
    /// drives the voice at the same index — see `audio::MUSIC_VOICE_KINDS`.
    pub channels: [SfxPlayer; MUSIC_VOICE_COUNT],
}

impl Default for MusicPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MusicPlayer {
    pub fn new() -> Self {
        Self {
            active: false,
            pattern_id: 0,
            row: 0,
            tick_count: 0,
            ticks_per_row: 64,
            loop_on: true,
            song_active: false,
            song_step: 0,
            channels: std::array::from_fn(|_| SfxPlayer::new()),
        }
    }

    pub fn start(&mut self, pattern_id: u8) {
        self.pattern_id = pattern_id.min(MUSIC_PATTERN_COUNT as u8 - 1);
        self.row = 0;
        self.tick_count = 0;
        self.active = true;
        // Single-pattern playback takes over from any song in progress.
        self.song_active = false;
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.song_active = false;
        for channel in &mut self.channels {
            channel.active = false;
        }
    }

    /// Byte address of `row`'s first channel cell. Rows are
    /// `MUSIC_VOICE_COUNT` bytes wide, one SFX reference per typed channel.
    pub fn pattern_row_base(pattern_id: u8, row: u8) -> usize {
        MUSIC_BANK_BASE
            + (pattern_id as usize) * (MUSIC_PATTERN_ROWS * MUSIC_VOICE_COUNT)
            + (row as usize) * MUSIC_VOICE_COUNT
    }
}

/// Resolves a song-order step to the pattern it should play, honoring the
/// bank's loop point exactly once (never chases a chain of empty/looping
/// steps — an all-empty or self-looping-to-empty song simply stops).
/// Returns `(pattern_id, resolved_step)` on success.
pub fn resolve_song_step(memory: &Memory, step: u8) -> Option<(u8, u8)> {
    let order_base = MUSIC_BANK_BASE + MUSIC_ORDER_OFFSET;
    let step_pattern = |s: u8| -> Option<u8> {
        if (s as usize) >= MUSIC_ORDER_STEPS {
            return None;
        }
        match memory.read(order_base + s as usize).unwrap_or(0) {
            // A slot holds `pattern id + 1`, so 0 means "no step here".
            b if (1..=MUSIC_PATTERN_COUNT as u8).contains(&b) => Some(b - 1),
            _ => None,
        }
    };
    if let Some(pattern) = step_pattern(step) {
        return Some((pattern, step));
    }
    let loop_byte = memory
        .read(MUSIC_BANK_BASE + MUSIC_LOOP_POINT_OFFSET)
        .unwrap_or(0);
    if !(1..=MUSIC_ORDER_STEPS as u8).contains(&loop_byte) {
        return None;
    }
    let loop_step = loop_byte - 1;
    step_pattern(loop_step).map(|pattern| (pattern, loop_step))
}

#[cfg(test)]
mod song_order_tests {
    use super::*;
    use caiven_core::memory::RAM_SIZE;

    fn memory_with(order: &[(usize, u8)], loop_byte: u8) -> Memory {
        let mut memory = Memory::new(RAM_SIZE);
        for (step, value) in order {
            let _ = memory.write(MUSIC_BANK_BASE + MUSIC_ORDER_OFFSET + step, *value);
        }
        let _ = memory.write(MUSIC_BANK_BASE + MUSIC_LOOP_POINT_OFFSET, loop_byte);
        memory
    }

    #[test]
    fn empty_song_resolves_to_nothing() {
        assert_eq!(resolve_song_step(&memory_with(&[], 0), 0), None);
    }

    #[test]
    fn valid_step_resolves_to_its_pattern() {
        let memory = memory_with(&[(0, 3)], 0);
        assert_eq!(resolve_song_step(&memory, 0), Some((2, 0)));
    }

    #[test]
    fn out_of_range_slot_byte_is_treated_as_empty() {
        let memory = memory_with(&[(0, MUSIC_PATTERN_COUNT as u8 + 1)], 0);
        assert_eq!(resolve_song_step(&memory, 0), None);
    }

    #[test]
    fn falling_off_the_end_jumps_to_the_loop_point() {
        let memory = memory_with(&[(0, 1), (1, 2)], 1);
        assert_eq!(resolve_song_step(&memory, 2), Some((0, 0)));
    }

    #[test]
    fn loop_point_at_an_empty_step_does_not_jump_again() {
        let memory = memory_with(&[(0, 1)], 5);
        assert_eq!(resolve_song_step(&memory, 1), None);
    }

    #[test]
    fn zero_loop_byte_means_no_loop() {
        let memory = memory_with(&[(0, 1)], 0);
        assert_eq!(resolve_song_step(&memory, 1), None);
    }

    #[test]
    fn step_past_the_order_table_still_honors_the_loop_point() {
        let memory = memory_with(&[(0, 1)], 1);
        assert_eq!(
            resolve_song_step(&memory, MUSIC_ORDER_STEPS as u8),
            Some((0, 0))
        );
    }
}

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
        assert_eq!(pan, PAN_TABLE[1]);
    }

    #[test]
    fn attack_and_release_bits_select_envelope_levels() {
        let byte3 = 0b1000_0000 | 0b0001_0000; // release=level2(bit7), attack=level1(bit4)
        let (_, attack, release) = decode_byte3(byte3);
        assert_eq!(attack, ENVELOPE_MS[1]);
        assert_eq!(release, ENVELOPE_MS[2]);
    }
}
