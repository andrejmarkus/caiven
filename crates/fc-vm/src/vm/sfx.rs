pub use fc_core::memory::{MUSIC_RAM_BASE as MUSIC_BANK_BASE, SFX_RAM_BASE as SFX_BANK_BASE};
const SFX_STEPS: u8 = 16;
const MUSIC_ROWS: u8 = 16;

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
    let idx = (note - 1) % 12;
    let octave = (note - 1) / 12;
    format!("{}{}", NOTE_NAMES[idx as usize], octave)
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
    pub ch0: SfxPlayer,
    pub ch1: SfxPlayer,
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
            ch0: SfxPlayer::new(),
            ch1: SfxPlayer::new(),
        }
    }

    pub fn start(&mut self, pattern_id: u8) {
        self.pattern_id = pattern_id.min(7);
        self.row = 0;
        self.tick_count = 0;
        self.active = true;
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.ch0.active = false;
        self.ch1.active = false;
    }

    pub fn pattern_row_base(pattern_id: u8, row: u8) -> usize {
        MUSIC_BANK_BASE + (pattern_id as usize) * (MUSIC_ROWS as usize * 2) + (row as usize) * 2
    }
}
