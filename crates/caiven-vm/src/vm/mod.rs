pub mod api_registry;
pub mod audio;
pub mod camera;
pub mod config;
mod execution;
pub mod fault;
mod lua_exec;
pub mod memory;
pub mod palette;
mod rtc;
pub mod sfx;

pub use camera::*;
pub use config::VmConfig;
pub use fault::VmFault;
pub use lua_exec::{LuaBreakpoint, LuaRunOutcome, describe_lua_error, describe_lua_error_location};
pub use palette::*;

use self::memory::Memory;
use self::sfx::{MusicPlayer, SfxPlayer};
use crate::peripheral::{Peripheral, PeripheralRegistry};
use crate::rendering::screen::ScreenLayer;
use crate::vm::Camera;
use crate::vm::audio::{NoiseChannel, Sound, SquareChannel};
use caiven_cart::{CartSection, SectionKind, decode_asset_bank};
use caiven_core::memory::{
    MAP_LEN, MAP_RAM_BASE, MUSIC_BANK_LEN, MUSIC_RAM_BASE, PALETTE_RAM_BASE, PALETTE_SIZE,
    SFX_BANK_LEN, SFX_RAM_BASE, SPRITE_FLAGS_LEN, SPRITE_FLAGS_RAM_BASE, SPRITE_SHEET_LEN,
    SPRITE_SHEET_RAM_BASE,
};
use caiven_core::{Color, Vec2};
use log::error;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssetBankKind {
    Sprites,
    Map,
    /// Behavior-flag table that shadows the active Sprites bank. A
    /// "companion" bank: created, selected, and removed in lockstep with
    /// its primary (see `AssetBankKind::companion`) and has no independent
    /// Lua selector — it always follows `load_sprite_bank`.
    SpriteFlags,
    Palette,
    Sfx,
    Music,
}

impl AssetBankKind {
    /// The bank this kind shadows, if any. Whenever the primary is
    /// selected, created, or removed, the same operation is applied to its
    /// companion at the same bank id — see `Vm::{select,create,remove}_asset_bank`.
    fn companion(self) -> Option<AssetBankKind> {
        match self {
            AssetBankKind::Sprites => Some(AssetBankKind::SpriteFlags),
            _ => None,
        }
    }
}

struct AssetBanks {
    banks: BTreeMap<AssetBankKind, BTreeMap<u8, Vec<u8>>>,
    active: BTreeMap<AssetBankKind, u8>,
}

impl AssetBanks {
    const KINDS: [AssetBankKind; 6] = [
        AssetBankKind::Sprites,
        AssetBankKind::Map,
        AssetBankKind::SpriteFlags,
        AssetBankKind::Palette,
        AssetBankKind::Sfx,
        AssetBankKind::Music,
    ];

    fn new() -> Self {
        let mut banks = BTreeMap::new();
        let mut active = BTreeMap::new();
        for kind in Self::KINDS {
            let (_, len) = Self::region(kind);
            banks.insert(kind, BTreeMap::from([(0, vec![0; len])]));
            active.insert(kind, 0);
        }
        Self { banks, active }
    }

    fn region(kind: AssetBankKind) -> (usize, usize) {
        match kind {
            AssetBankKind::Sprites => (SPRITE_SHEET_RAM_BASE, SPRITE_SHEET_LEN),
            AssetBankKind::Map => (MAP_RAM_BASE, MAP_LEN),
            AssetBankKind::SpriteFlags => (SPRITE_FLAGS_RAM_BASE, SPRITE_FLAGS_LEN),
            AssetBankKind::Palette => (PALETTE_RAM_BASE, PALETTE_SIZE * 3),
            AssetBankKind::Sfx => (SFX_RAM_BASE, SFX_BANK_LEN),
            AssetBankKind::Music => (MUSIC_RAM_BASE, MUSIC_BANK_LEN),
        }
    }

    fn banks(&self, kind: AssetBankKind) -> &BTreeMap<u8, Vec<u8>> {
        self.banks
            .get(&kind)
            .expect("all AssetBankKind variants are seeded in AssetBanks::new")
    }

    fn banks_mut(&mut self, kind: AssetBankKind) -> &mut BTreeMap<u8, Vec<u8>> {
        self.banks
            .get_mut(&kind)
            .expect("all AssetBankKind variants are seeded in AssetBanks::new")
    }

    fn active(&self, kind: AssetBankKind) -> u8 {
        *self
            .active
            .get(&kind)
            .expect("all AssetBankKind variants are seeded in AssetBanks::new")
    }

    fn set_active(&mut self, kind: AssetBankKind, id: u8) {
        self.active.insert(kind, id);
    }

    fn normalized(data: &[u8], len: usize) -> Vec<u8> {
        let mut out = vec![0; len];
        let copy_len = len.min(data.len());
        out[..copy_len].copy_from_slice(&data[..copy_len]);
        out
    }

    fn sync(&mut self, kind: AssetBankKind, memory: &Memory) {
        let (base, len) = Self::region(kind);
        let active = self.active(kind);
        let data: Vec<u8> = (0..len)
            .map(|offset| memory.read(base + offset).unwrap_or(0))
            .collect();
        self.banks_mut(kind).insert(active, data);
    }

    fn select(&mut self, kind: AssetBankKind, id: u8, memory: &mut Memory) -> bool {
        if self.active(kind) == id {
            return self.banks(kind).contains_key(&id);
        }
        let Some(data) = self.banks(kind).get(&id).cloned() else {
            return false;
        };
        self.sync(kind, memory);
        let (base, _) = Self::region(kind);
        for (offset, byte) in data.into_iter().enumerate() {
            let _ = memory.write(base + offset, byte);
        }
        self.set_active(kind, id);
        true
    }
}

pub struct Vm {
    memory: Memory,
    camera: Camera,
    palette: Palette,
    sound: Arc<Mutex<Sound>>,
    sfx_player: SfxPlayer,
    music_player: MusicPlayer,
    peripherals: PeripheralRegistry,
    frame_count: u32,
    waiting: bool,
    fault: Option<VmFault>,
    world: ScreenLayer,
    ui: ScreenLayer,
    config: VmConfig,
    script: Option<lua_exec::LuaScript>,
    capture_lua_output: bool,
    call_stack: Vec<(String, String)>,
    asset_banks: AssetBanks,
}

impl Vm {
    pub fn new(config: VmConfig) -> Self {
        let mut memory = Memory::new(config.memory_size);
        let mut peripherals = PeripheralRegistry::new();
        peripherals.register(rtc::RealTimeClock);
        peripherals.init_all(&mut memory);

        Self {
            memory,
            camera: Camera::new(Vec2::new(0, 0)),
            palette: Palette::new(config.palette_size),
            sound: Arc::new(Mutex::new(Sound {
                square: SquareChannel {
                    enabled: false,
                    frequency: 440.0,
                    volume: 0.0,
                    duration: 0,
                },
                noise: NoiseChannel {
                    enabled: false,
                    volume: 0.0,
                    rate: 2000.0,
                    duration: 0,
                },
            })),
            sfx_player: SfxPlayer::new(),
            music_player: MusicPlayer::new(),
            peripherals,
            frame_count: 0,
            waiting: false,
            fault: None,
            world: ScreenLayer::new(config.width, config.height),
            ui: ScreenLayer::new(config.width, config.height),
            config,
            script: None,
            capture_lua_output: false,
            call_stack: Vec::new(),
            asset_banks: AssetBanks::new(),
        }
    }

    /// Enables VM-owned `print()` capture for subsequently loaded Lua code.
    /// Disabled by default so machine/web clients keep native Lua stdout.
    pub fn set_lua_output_capture(&mut self, enabled: bool) {
        self.capture_lua_output = enabled;
    }

    pub fn lua_output_capture_enabled(&self) -> bool {
        self.capture_lua_output
    }

    pub fn register_peripheral(&mut self, p: impl Peripheral + 'static) {
        self.peripherals.register(p);
    }

    pub fn registered_peripheral_names(&self) -> Vec<&'static str> {
        self.peripherals.names()
    }

    pub fn set_fault(&mut self, fault: VmFault) {
        error!("VM FAULT: {:?}", fault);
        self.fault = Some(fault);
        self.waiting = true;
    }

    pub fn get_sound_shared(&self) -> Arc<Mutex<Sound>> {
        self.sound.clone()
    }

    /// Stops any active SFX/music playback and silences the output
    /// immediately. `tick_audio_players` keeps advancing playback even while
    /// the game isn't running (so SFX/Music editor previews stay audible),
    /// which otherwise means audio the game itself triggered — including
    /// from `_init()` on cart load — just keeps sounding forever once
    /// nothing else is stepping the VM to wind it down.
    pub fn stop_audio(&mut self) {
        self.sfx_player.stop();
        self.music_player.stop();
        if let Ok(mut sound) = self.sound.lock() {
            sound.square.enabled = false;
            sound.noise.enabled = false;
        }
    }

    pub fn load_section_to_ram(&mut self, base: usize, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            if let Err(e) = self.memory.write(base + i, byte) {
                log::error!("load_section_to_ram: write fault at {}: {:?}", base + i, e);
                break;
            }
        }
    }

    /// Copies every RAM-backed asset section a cart may carry (SpriteSheet,
    /// Map, SpriteFlags, Palette, SfxBank, MusicBank) to its fixed RAM base,
    /// and returns the cart's Lua source text if present. Single source of
    /// truth for "which section kind goes where" — every cart-loading call
    /// site (Studio, `caiven-machine`, the port screenshot capturer) must go
    /// through this instead of re-listing the mapping, so they can't drift
    /// apart the way the audio/peripheral per-frame tick paths already did
    /// once each grew a second, independently-written call site.
    pub fn load_cart_sections(&mut self, sections: &[CartSection]) -> Option<String> {
        self.asset_banks = AssetBanks::new();
        for section in sections {
            let ram_base = match section.kind {
                SectionKind::SpriteSheet => {
                    self.asset_banks
                        .banks_mut(AssetBankKind::Sprites)
                        .insert(0, AssetBanks::normalized(&section.data, SPRITE_SHEET_LEN));
                    continue;
                }
                SectionKind::Map => {
                    self.asset_banks
                        .banks_mut(AssetBankKind::Map)
                        .insert(0, AssetBanks::normalized(&section.data, MAP_LEN));
                    continue;
                }
                SectionKind::SpriteBank => {
                    if let Some((id, data)) = decode_asset_bank(&section.data) {
                        self.asset_banks
                            .banks_mut(AssetBankKind::Sprites)
                            .insert(id, AssetBanks::normalized(data, SPRITE_SHEET_LEN));
                    }
                    continue;
                }
                SectionKind::MapBank => {
                    if let Some((id, data)) = decode_asset_bank(&section.data) {
                        self.asset_banks
                            .banks_mut(AssetBankKind::Map)
                            .insert(id, AssetBanks::normalized(data, MAP_LEN));
                    }
                    continue;
                }
                // Additional (id != 0) banks for the kinds whose bank-0 data
                // still loads straight to RAM below. Bank 0 self-heals into
                // `asset_banks` on first `select`/`sync`; see `AssetBanks::sync`.
                SectionKind::SpriteFlagsBank => {
                    if let Some((id, data)) = decode_asset_bank(&section.data) {
                        self.asset_banks
                            .banks_mut(AssetBankKind::SpriteFlags)
                            .insert(id, AssetBanks::normalized(data, SPRITE_FLAGS_LEN));
                    }
                    continue;
                }
                SectionKind::PaletteBank => {
                    if let Some((id, data)) = decode_asset_bank(&section.data) {
                        self.asset_banks
                            .banks_mut(AssetBankKind::Palette)
                            .insert(id, AssetBanks::normalized(data, PALETTE_SIZE * 3));
                    }
                    continue;
                }
                SectionKind::SfxBanks => {
                    if let Some((id, data)) = decode_asset_bank(&section.data) {
                        self.asset_banks
                            .banks_mut(AssetBankKind::Sfx)
                            .insert(id, AssetBanks::normalized(data, SFX_BANK_LEN));
                    }
                    continue;
                }
                SectionKind::MusicBanks => {
                    if let Some((id, data)) = decode_asset_bank(&section.data) {
                        self.asset_banks
                            .banks_mut(AssetBankKind::Music)
                            .insert(id, AssetBanks::normalized(data, MUSIC_BANK_LEN));
                    }
                    continue;
                }
                SectionKind::SpriteFlags => SPRITE_FLAGS_RAM_BASE,
                SectionKind::Palette => PALETTE_RAM_BASE,
                SectionKind::SfxBank => SFX_RAM_BASE,
                SectionKind::MusicBank => MUSIC_RAM_BASE,
                SectionKind::Program
                | SectionKind::Meta
                | SectionKind::ModManifest
                | SectionKind::LuaSource
                | SectionKind::Custom(_) => continue,
            };
            self.load_section_to_ram(ram_base, &section.data);
            if section.kind == SectionKind::Palette {
                self.set_palette_from_bytes(&section.data);
            }
        }
        for kind in [AssetBankKind::Sprites, AssetBankKind::Map] {
            let Some(data) = self.asset_banks.banks(kind).get(&0).cloned() else {
                continue;
            };
            let (base, _) = AssetBanks::region(kind);
            for (offset, byte) in data.into_iter().enumerate() {
                let _ = self.memory.write(base + offset, byte);
            }
            self.asset_banks.set_active(kind, 0);
        }
        sections
            .iter()
            .find(|s| s.kind == SectionKind::LuaSource)
            .map(|s| String::from_utf8_lossy(&s.data).into_owned())
    }

    pub fn asset_bank_ids(&self, kind: AssetBankKind) -> Vec<u8> {
        self.asset_banks.banks(kind).keys().copied().collect()
    }

    pub fn active_asset_bank(&self, kind: AssetBankKind) -> u8 {
        self.asset_banks.active(kind)
    }

    /// Selects bank `id` for `kind`. If `kind` has a companion (see
    /// `AssetBankKind::companion`), the companion follows to the same id —
    /// e.g. selecting a Sprites bank also selects its SpriteFlags bank.
    pub fn select_asset_bank(&mut self, kind: AssetBankKind, id: u8) -> bool {
        let selected = self.asset_banks.select(kind, id, &mut self.memory);
        if selected {
            if let Some(companion) = kind.companion() {
                self.asset_banks.select(companion, id, &mut self.memory);
            }
        }
        selected
    }

    /// Creates and selects bank `id` for `kind`. A companion bank (if any)
    /// is created and selected alongside it at the same id, so the two stay
    /// in lockstep for the rest of their lifetime.
    pub fn create_asset_bank(&mut self, kind: AssetBankKind, id: u8) -> bool {
        if id == 0 || self.asset_banks.banks(kind).contains_key(&id) {
            return false;
        }
        let (_, len) = AssetBanks::region(kind);
        self.asset_banks.banks_mut(kind).insert(id, vec![0; len]);
        let created = self.asset_banks.select(kind, id, &mut self.memory);
        if created {
            if let Some(companion) = kind.companion() {
                if !self.asset_banks.banks(companion).contains_key(&id) {
                    let (_, companion_len) = AssetBanks::region(companion);
                    self.asset_banks
                        .banks_mut(companion)
                        .insert(id, vec![0; companion_len]);
                }
                self.asset_banks.select(companion, id, &mut self.memory);
            }
        }
        created
    }

    pub fn replace_asset_bank(&mut self, kind: AssetBankKind, id: u8, data: &[u8]) {
        let (_, len) = AssetBanks::region(kind);
        let data = AssetBanks::normalized(data, len);
        self.asset_banks.banks_mut(kind).insert(id, data.clone());
        if self.asset_banks.active(kind) == id {
            let (base, _) = AssetBanks::region(kind);
            for (offset, byte) in data.into_iter().enumerate() {
                let _ = self.memory.write(base + offset, byte);
            }
        }
    }

    /// Removes bank `id` for `kind`, falling back to bank 0 if it was
    /// active. A companion bank (if any) is removed alongside it.
    pub fn remove_asset_bank(&mut self, kind: AssetBankKind, id: u8) -> bool {
        if id == 0 || !self.asset_banks.banks(kind).contains_key(&id) {
            return false;
        }
        if self.asset_banks.active(kind) == id {
            let _ = self.asset_banks.select(kind, 0, &mut self.memory);
        }
        let removed = self.asset_banks.banks_mut(kind).remove(&id).is_some();
        if removed {
            if let Some(companion) = kind.companion() {
                if self.asset_banks.active(companion) == id {
                    let _ = self.asset_banks.select(companion, 0, &mut self.memory);
                }
                self.asset_banks.banks_mut(companion).remove(&id);
            }
        }
        removed
    }

    /// Returns current bank bytes, including unswitched RAM edits for active bank.
    pub fn asset_bank_bytes(&self, kind: AssetBankKind, id: u8) -> Option<Vec<u8>> {
        if self.asset_banks.active(kind) == id {
            let (base, len) = AssetBanks::region(kind);
            Some(
                (0..len)
                    .map(|offset| self.memory.read(base + offset).unwrap_or(0))
                    .collect(),
            )
        } else {
            self.asset_banks.banks(kind).get(&id).cloned()
        }
    }

    pub fn get_memory_length(&self) -> usize {
        self.memory.get_length()
    }

    pub fn peek_memory(&self, address: usize) -> u8 {
        self.memory.read(address).unwrap_or(0)
    }

    pub fn get_camera_x(&self) -> u32 {
        self.camera.get_x()
    }

    pub fn get_camera_y(&self) -> u32 {
        self.camera.get_y()
    }

    pub fn is_waiting(&self) -> bool {
        self.waiting
    }

    pub fn get_fault(&self) -> Option<VmFault> {
        self.fault
    }

    pub fn world_pixels(&self) -> &[u8] {
        self.world.get_pixels()
    }

    pub fn ui_pixels(&self) -> &[u8] {
        self.ui.get_pixels()
    }

    pub fn get_palette(&self) -> &[Color] {
        self.palette.get_colors()
    }

    pub fn set_palette_color(&mut self, index: usize, color: Color) {
        self.palette.set_color(index, color);
    }

    pub fn set_palette_from_bytes(&mut self, bytes: &[u8]) {
        for i in 0..16.min(bytes.len() / 3) {
            let r = bytes[i * 3];
            let g = bytes[i * 3 + 1];
            let b = bytes[i * 3 + 2];
            self.palette.set_color(i, Color::new_rgb(r, g, b));
        }
    }

    pub fn poke_memory(&mut self, address: usize, value: u8) {
        let _ = self.memory.write(address, value);
    }

    pub fn start_sfx(&mut self, id: u8) {
        self.sfx_player.start(id);
    }

    pub fn stop_sfx(&mut self) {
        self.sfx_player.stop();
        if let Ok(mut s) = self.sound.try_lock() {
            s.square.enabled = false;
            s.noise.enabled = false;
        }
    }

    pub fn start_music(&mut self, pattern_id: u8) {
        self.music_player.start(pattern_id);
    }

    pub fn stop_music(&mut self) {
        self.music_player.stop();
        if let Ok(mut s) = self.sound.try_lock() {
            s.square.enabled = false;
            s.noise.enabled = false;
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
}

#[cfg(test)]
mod asset_bank_tests {
    use super::*;
    use crate::input::Input;
    use crate::rendering::font::Font;
    use caiven_cart::encode_asset_bank;

    #[test]
    fn bank_switches_copy_ram_and_preserve_runtime_edits() {
        let mut vm = Vm::new(VmConfig::default());
        vm.load_cart_sections(&[
            CartSection {
                kind: SectionKind::SpriteSheet,
                data: vec![1; SPRITE_SHEET_LEN],
            },
            CartSection {
                kind: SectionKind::SpriteBank,
                data: encode_asset_bank(2, &vec![7; SPRITE_SHEET_LEN]),
            },
        ]);

        assert_eq!(vm.asset_bank_ids(AssetBankKind::Sprites), vec![0, 2]);
        assert_eq!(vm.peek_memory(SPRITE_SHEET_RAM_BASE), 1);
        assert!(vm.select_asset_bank(AssetBankKind::Sprites, 2));
        assert_eq!(vm.peek_memory(SPRITE_SHEET_RAM_BASE), 7);
        vm.poke_memory(SPRITE_SHEET_RAM_BASE, 9);
        assert!(vm.select_asset_bank(AssetBankKind::Sprites, 2));
        assert_eq!(vm.peek_memory(SPRITE_SHEET_RAM_BASE), 9);
        assert!(vm.select_asset_bank(AssetBankKind::Sprites, 0));
        assert_eq!(vm.peek_memory(SPRITE_SHEET_RAM_BASE), 1);
        assert!(vm.select_asset_bank(AssetBankKind::Sprites, 2));
        assert_eq!(vm.peek_memory(SPRITE_SHEET_RAM_BASE), 9);
        assert!(!vm.select_asset_bank(AssetBankKind::Sprites, 3));
    }

    #[test]
    fn sprite_flags_bank_follows_sprites_as_companion() {
        let mut vm = Vm::new(VmConfig::default());
        // Bank 0's flags start out with some data written.
        vm.poke_memory(SPRITE_FLAGS_RAM_BASE, 5);

        // Creating a new Sprites bank creates and selects a fresh,
        // zero-filled SpriteFlags bank at the same id — not a copy of bank 0.
        assert!(vm.create_asset_bank(AssetBankKind::Sprites, 2));
        assert_eq!(vm.active_asset_bank(AssetBankKind::SpriteFlags), 2);
        assert_eq!(vm.peek_memory(SPRITE_FLAGS_RAM_BASE), 0);

        // Switching Sprites banks carries SpriteFlags along in lockstep,
        // and runtime edits to each bank's flags are preserved independently.
        vm.poke_memory(SPRITE_FLAGS_RAM_BASE, 9);
        assert!(vm.select_asset_bank(AssetBankKind::Sprites, 0));
        assert_eq!(vm.active_asset_bank(AssetBankKind::SpriteFlags), 0);
        assert_eq!(vm.peek_memory(SPRITE_FLAGS_RAM_BASE), 5);
        assert!(vm.select_asset_bank(AssetBankKind::Sprites, 2));
        assert_eq!(vm.active_asset_bank(AssetBankKind::SpriteFlags), 2);
        assert_eq!(vm.peek_memory(SPRITE_FLAGS_RAM_BASE), 9);

        // Removing the Sprites bank removes its companion flags bank too.
        assert!(vm.remove_asset_bank(AssetBankKind::Sprites, 2));
        assert_eq!(vm.active_asset_bank(AssetBankKind::SpriteFlags), 0);
        assert!(!vm.asset_bank_ids(AssetBankKind::SpriteFlags).contains(&2));
    }

    #[test]
    fn lua_can_switch_asset_banks() {
        let mut vm = Vm::new(VmConfig::default());
        vm.load_cart_sections(&[CartSection {
            kind: SectionKind::MapBank,
            data: encode_asset_bank(4, &vec![6; MAP_LEN]),
        }]);
        vm.load_lua_source(
            "function _init() switched = load_map_bank(4) end\nfunction _update() end",
            &Input::new(),
            &Font::empty(),
        )
        .expect("Lua banking fixture should load");

        assert_eq!(vm.active_asset_bank(AssetBankKind::Map), 4);
        assert_eq!(vm.peek_memory(MAP_RAM_BASE), 6);
        assert_eq!(
            vm.lua_watch("switched")
                .expect("Lua global should be readable"),
            "true"
        );
    }
}
