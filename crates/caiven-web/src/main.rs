//! WASM cart player for the browser. Built as a `bin` target (not `cdylib` —
//! emscripten's cdylib "side module" mode requires -fPIC objects, which the
//! vendored Lua C build isn't); `extern "C"` exports are still reachable from
//! JS via `Module.ccall`/`cwrap`, same as a cdylib would be.
//!
//! Single global `Player` behind a `thread_local!` — emscripten's default
//! build is single-threaded, and the JS side only ever holds one instance.

use caiven_core::memory::RGBA_BYTES;
use caiven_vm::input::{Button, Input};
use caiven_vm::rendering::font::Font;
use caiven_vm::rendering::screen::Screen;
use caiven_vm::vm::audio::{AudioPeripheral, Sound, Synth};
use caiven_vm::{LuaRunOutcome, Vm, VmConfig};
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

const FONT_BYTES: &[u8] = include_bytes!("../../../assets/font.png");
const FONT_GLYPHS: &str = " 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ!?\"'()+-=.:,[]<>";

struct Player {
    vm: Vm,
    screen: Screen,
    input: Input,
    font: Font,
    out: Vec<u8>,
    width: u32,
    height: u32,
    sound: Arc<Mutex<Sound>>,
    synth: Synth,
    audio_buf: Vec<f32>,
    fault: bool,
    fault_bytes: Vec<u8>,
}

impl Player {
    fn new() -> anyhow::Result<Self> {
        let font = Font::from_bytes(FONT_BYTES, FONT_GLYPHS, 3, 5)?;
        let config = VmConfig::default();
        let mut vm = Vm::new(config);
        let sound = vm.get_sound_shared();
        vm.register_peripheral(AudioPeripheral::new(sound.clone()));

        Ok(Self {
            screen: Screen::new(config.width, config.height),
            input: Input::new(),
            vm,
            font,
            out: vec![0; config.width as usize * config.height as usize * RGBA_BYTES],
            width: config.width,
            height: config.height,
            sound,
            synth: Synth::new(),
            audio_buf: Vec::new(),
            fault: false,
            fault_bytes: Vec::new(),
        })
    }

    fn load_cart(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        let cart = caiven_cart::parse(bytes)?;

        for section in &cart.sections {
            if section.kind == caiven_cart::SectionKind::ModManifest {
                let manifest = String::from_utf8_lossy(&section.data);
                let registered = self.vm.registered_peripheral_names();
                check_mod_manifest(&manifest, &registered)?;
            }
        }

        let lua_source = self
            .vm
            .load_cart_sections(&cart.sections)
            .ok_or_else(|| anyhow::anyhow!("cart has no Lua source section"))?;
        self.vm
            .load_lua_source(&lua_source, &self.input, &self.font)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(())
    }

    fn tick(&mut self, frames: u32) {
        for _ in 0..frames {
            if self.fault {
                break;
            }
            if let LuaRunOutcome::Error(line, message) =
                self.vm.run_frame_lua_bp(&self.input, &self.font, &[])
            {
                let text = match line {
                    Some(l) => format!("line {l}: {message}"),
                    None => message,
                };
                self.fault_bytes = text.into_bytes();
                self.fault = true;
            }
            self.input.end_frame();
        }
        self.screen
            .construct(&mut self.out, self.vm.world_pixels(), self.vm.ui_pixels());
    }

    fn set_button(&mut self, idx: u8, down: bool) {
        if let Some(button) = Button::from_u8(idx) {
            self.input.set_button(button, down);
        }
    }

    /// Fills `audio_buf` with `num_frames` mono samples at `sample_rate`,
    /// growing the buffer if needed. Silence (not an error) if the shared
    /// `Sound` state is momentarily locked elsewhere.
    fn fill_audio(&mut self, num_frames: usize, sample_rate: f32) {
        if self.audio_buf.len() < num_frames {
            self.audio_buf.resize(num_frames, 0.0);
        }
        match self.sound.try_lock() {
            Ok(s) => {
                for sample in self.audio_buf[..num_frames].iter_mut() {
                    *sample = self.synth.next_sample(&s, sample_rate);
                }
            }
            Err(_) => {
                self.audio_buf[..num_frames].fill(0.0);
            }
        }
    }
}

/// Same rule `caiven-machine`/`caiven-studio` enforce before loading a cart:
/// every peripheral its `ModManifest` section names must be registered.
fn check_mod_manifest(manifest: &str, registered: &[&str]) -> anyhow::Result<()> {
    for required in manifest.lines().map(str::trim).filter(|s| !s.is_empty()) {
        if !registered.contains(&required) {
            anyhow::bail!("cart requires mod '{}' but it is not loaded", required);
        }
    }
    Ok(())
}

thread_local! {
    static PLAYER: RefCell<Option<Player>> = const { RefCell::new(None) };
}

#[unsafe(no_mangle)]
pub extern "C" fn caiven_new() -> i32 {
    match Player::new() {
        Ok(p) => {
            PLAYER.with(|cell| *cell.borrow_mut() = Some(p));
            0
        }
        Err(e) => {
            eprintln!("caiven_new failed: {e}");
            -1
        }
    }
}

/// `ptr`/`len` address a byte buffer JS has already copied into the wasm
/// heap (e.g. via `HEAPU8.set` at a pointer from `malloc`). Returns 0 on
/// success, -1 if the cart failed to parse or load.
///
/// # Safety
/// Caller must ensure `ptr` ..`ptr + len` is a valid, initialized region of
/// the wasm linear memory for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn caiven_load_cart(ptr: *const u8, len: usize) -> i32 {
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    PLAYER.with(|cell| {
        let Some(player) = cell.borrow_mut().as_mut().map(|p| p as *mut Player) else {
            return -1;
        };
        // SAFETY: pointer is derived from the RefCell we're already inside;
        // no re-entrant access happens while it's live.
        let player = unsafe { &mut *player };
        match player.load_cart(bytes) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("caiven_load_cart failed: {e}");
                -1
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn caiven_set_button(idx: u32, down: i32) {
    PLAYER.with(|cell| {
        if let Some(player) = cell.borrow_mut().as_mut() {
            player.set_button(idx as u8, down != 0);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn caiven_tick(frames: u32) {
    PLAYER.with(|cell| {
        if let Some(player) = cell.borrow_mut().as_mut() {
            player.tick(frames);
        }
    });
}

/// Pointer into the wasm heap where the composited RGBA framebuffer lives.
/// Valid until the next `caiven_tick` call (the buffer is reused in place).
#[unsafe(no_mangle)]
pub extern "C" fn caiven_pixels() -> *const u8 {
    PLAYER.with(|cell| {
        cell.borrow()
            .as_ref()
            .map_or(std::ptr::null(), |p| p.out.as_ptr())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn caiven_width() -> u32 {
    PLAYER.with(|cell| cell.borrow().as_ref().map_or(0, |p| p.width))
}

#[unsafe(no_mangle)]
pub extern "C" fn caiven_height() -> u32 {
    PLAYER.with(|cell| cell.borrow().as_ref().map_or(0, |p| p.height))
}

/// Synthesizes `num_frames` mono samples at `sample_rate` into an internal
/// buffer, read back via [`caiven_audio_ptr`]. Called from JS once per
/// audio-callback tick (`ScriptProcessorNode`/`AudioWorklet`).
#[unsafe(no_mangle)]
pub extern "C" fn caiven_audio_fill(num_frames: u32, sample_rate: f32) {
    PLAYER.with(|cell| {
        if let Some(player) = cell.borrow_mut().as_mut() {
            player.fill_audio(num_frames as usize, sample_rate);
        }
    });
}

/// Pointer into the wasm heap where the last [`caiven_audio_fill`] wrote its
/// `f32` samples. Valid until the next `caiven_audio_fill` call.
#[unsafe(no_mangle)]
pub extern "C" fn caiven_audio_ptr() -> *const f32 {
    PLAYER.with(|cell| {
        cell.borrow()
            .as_ref()
            .map_or(std::ptr::null(), |p| p.audio_buf.as_ptr())
    })
}

/// Non-zero once the cart's Lua script has hit a runtime error. `caiven_tick`
/// stops advancing frames after this; the last-rendered framebuffer stays put.
#[unsafe(no_mangle)]
pub extern "C" fn caiven_has_fault() -> i32 {
    PLAYER.with(|cell| cell.borrow().as_ref().is_some_and(|p| p.fault) as i32)
}

#[unsafe(no_mangle)]
pub extern "C" fn caiven_fault_ptr() -> *const u8 {
    PLAYER.with(|cell| {
        cell.borrow()
            .as_ref()
            .map_or(std::ptr::null(), |p| p.fault_bytes.as_ptr())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn caiven_fault_len() -> u32 {
    PLAYER.with(|cell| {
        cell.borrow()
            .as_ref()
            .map_or(0, |p| p.fault_bytes.len() as u32)
    })
}

fn main() {}
