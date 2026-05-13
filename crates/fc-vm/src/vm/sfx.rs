pub const SFX_BANK_BASE: usize = 0x5C00;
const SFX_STEPS: u8 = 16;

pub fn note_to_freq(note: u8) -> f32 {
    440.0 * 2.0f32.powf((note as f32 - 49.0) / 12.0)
}

const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

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
