//! Shared front-end runtime: the VM + peripherals bundle used by both the
//! editor (caiven-studio) and the cart runner (caiven-machine).
//!
//! Windowing, rendering surfaces and event loops belong to the front-end
//! binary, not here — `caiven-machine` owns them via SDL2, and Studio
//! composites frames into its own buffer.

use crate::input::{Input, InputMap};
use crate::rendering::font::Font;
use crate::rendering::screen::Screen;
use crate::timing::FixedTimestep;
use crate::vm::audio::{AudioFactory, AudioOut};
use crate::{Vm, VmConfig};
use anyhow::Result;
use log::{error, info};
use std::time::Instant;

pub use crate::rendering::font::FONT_GLYPHS;
/// Integer scale factor from console resolution to initial window size.
pub const WINDOW_SCALE: u32 = 4;

/// Opens an audio output, logging and swallowing failure — a console with
/// no available audio device still runs, just silently.
fn open_audio(
    factory: &AudioFactory,
    sound: std::sync::Arc<std::sync::Mutex<crate::vm::audio::Sound>>,
) -> Option<Box<dyn AudioOut>> {
    match factory(sound) {
        Ok(a) => Some(a),
        Err(e) => {
            // `{e}` only prints the top `anyhow::Context` message, discarding
            // the actual SDL error underneath it — `{e:#}` prints the full
            // chain, which is the only way to see why the device open failed.
            error!("failed to initialize audio: {e:#}");
            None
        }
    }
}

/// Everything a console front-end needs besides a window: a VM with the
/// audio peripheral registered, screen composition buffers, input state
/// and fixed-timestep frame timing.
pub struct ConsoleCore {
    pub screen: Screen,
    pub input: Input,
    pub input_map: InputMap,
    pub vm: Vm,
    pub font: Font,
    pub config: VmConfig,
    /// Owns the audio output; dropping it silences the console. `None` when
    /// no output device could be opened — the console still runs, silently.
    pub audio: Option<Box<dyn AudioOut>>,
    /// Opens the audio output; `adopt_vm` retries it while none is open.
    audio_factory: AudioFactory,
    pub timing: FixedTimestep,
    pub last_tick: Instant,
}

impl ConsoleCore {
    /// Builds a console using the default SDL2 audio backend, opening its
    /// own audio-only SDL context.
    #[cfg(any(feature = "sdl2-bundled", feature = "sdl2-dynamic"))]
    pub fn new() -> Result<Self> {
        Self::with_audio_factory(crate::vm::audio::sdl_default_audio_factory())
    }

    /// Builds a console whose audio output comes from `audio_factory`.
    ///
    /// Front-ends that already own a platform layer — `caiven-machine` and
    /// its SDL2 audio device — pass their own here rather than pulling a
    /// second audio stack in through the VM.
    pub fn with_audio_factory(audio_factory: AudioFactory) -> Result<Self> {
        let font = Font::builtin()?;

        let config = VmConfig::default();
        let vm = Vm::new(config);

        let audio = open_audio(&audio_factory, vm.get_sound_shared());

        info!("fantasy console initialized");

        Ok(Self {
            screen: Screen::new(config.width, config.height),
            input: Input::new(),
            // The host decides where controls.toml lives and installs it.
            input_map: InputMap::default(),
            vm,
            font,
            config,
            audio,
            audio_factory,
            timing: FixedTimestep::new(60),
            last_tick: Instant::now(),
        })
    }

    /// Replaces the VM and audio device with a blank state, keeping
    /// screen/input/font/timing. Used to start editing a brand-new cart
    /// without carrying over a previously loaded cart's RAM.
    pub fn reset_vm(&mut self) {
        let capture_lua_output = self.vm.lua_output_capture_enabled();
        let mut vm = Vm::new(self.config);
        vm.set_lua_output_capture(capture_lua_output);
        self.adopt_vm(vm);
    }

    /// Replaces the VM with one already built and validated elsewhere, on
    /// the same audio output. Lets a caller load/validate a candidate cart
    /// into a scratch `Vm` off to the side and only commit it here once
    /// loading fully succeeded — the current VM (and whatever cart it holds)
    /// is left untouched by a failed load.
    pub fn adopt_vm(&mut self, mut vm: Vm) {
        // One output for the console's life: reopening it per cart restarts
        // SDL's whole audio stack, and a failed reopen leaves it silent.
        vm.attach_sound(self.vm.get_sound_shared());
        self.vm = vm;
        if self.audio.is_none() {
            self.audio = open_audio(&self.audio_factory, self.vm.get_sound_shared());
        }
    }

    /// Advances the fixed-timestep clock; returns how many frames to run now.
    pub fn frame_steps(&mut self) -> u32 {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick);
        self.last_tick = now;
        self.timing.tick(dt)
    }

    /// Drops any time banked while `frame_steps` wasn't called (e.g. paused)
    /// and resets the clock to now, so resuming doesn't replay the gap as a
    /// catch-up burst.
    pub fn reset_timing(&mut self) {
        self.timing.reset();
        self.last_tick = Instant::now();
    }

    /// Runs one VM frame with the current input state, then latches it so
    /// edge-triggered input (INP/INPR, `btnp`) sees per-frame transitions.
    pub fn run_frame(&mut self) {
        self.vm.run_frame(&self.input, &self.font);
        self.input.end_frame();
    }

    /// Runs one Lua-scripted frame honoring line breakpoints; input latches
    /// like `run_frame`. See [`crate::vm::Vm::run_frame_lua_bp`].
    pub fn run_frame_lua_bp(
        &mut self,
        breakpoints: &[crate::vm::LuaBreakpoint],
    ) -> crate::vm::LuaRunOutcome {
        let outcome = self
            .vm
            .run_frame_lua_bp(&self.input, &self.font, breakpoints);
        self.input.end_frame();
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::ConsoleCore;
    use crate::rendering::font::Font;

    #[test]
    fn embedded_font_decodes_without_working_directory_asset_lookup() {
        let font = Font::builtin().expect("embedded font should decode");
        assert_eq!(font.get_width(), 3);
        assert_eq!(font.get_height(), 5);
        assert!(font.get_glyph('A').is_some());
    }

    #[test]
    fn adopting_a_vm_keeps_the_open_output_and_silences_the_old_cart() {
        use crate::vm::audio::{AudioOut, SFX_VOICE_START, Sound};
        use std::sync::{Arc, Mutex};

        struct NullOut;
        impl AudioOut for NullOut {}
        let opened: std::rc::Rc<std::cell::RefCell<Vec<Arc<Mutex<Sound>>>>> = Default::default();
        let seen = std::rc::Rc::clone(&opened);
        let mut core = ConsoleCore::with_audio_factory(Box::new(move |sound| {
            seen.borrow_mut().push(sound);
            Ok(Box::new(NullOut) as Box<dyn AudioOut>)
        }))
        .expect("console core");

        core.vm
            .load_section_to_ram(caiven_core::memory::SFX_RAM_BASE, &[49, 12, 0, 0]);
        core.vm.start_sfx(0);
        core.vm.tick_audio_players();
        core.reset_vm();
        core.adopt_vm(crate::Vm::new(core.config));

        let opened = opened.borrow();
        assert_eq!(opened.len(), 1, "the output opens once, not per cart");
        assert!(Arc::ptr_eq(&opened[0], &core.vm.get_sound_shared()));
        let sound = opened[0].lock().expect("sound lock");
        assert!(
            !sound.voices[SFX_VOICE_START].gate,
            "old cart's note is gone"
        );
    }

    #[test]
    #[cfg(any(feature = "sdl2-bundled", feature = "sdl2-dynamic"))]
    fn reset_vm_preserves_output_capture_setting() {
        let mut core = ConsoleCore::new().expect("console core should initialize");
        core.vm.set_lua_output_capture(true);
        core.reset_vm();
        assert!(core.vm.lua_output_capture_enabled());
    }
}
