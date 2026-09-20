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
pub mod save_data;
pub mod sfx;

pub use camera::*;
pub use config::VmConfig;
pub use fault::VmFault;
pub use lua_exec::{
    DebugValue, LuaBreakpoint, LuaRunOutcome, describe_lua_error, describe_lua_error_location,
    prelude_module_catalog,
};
pub use palette::*;
pub use save_data::{SAVE_DATA_BLOB_MAX_BYTES, SaveData, SaveDataError};

use self::memory::Memory;
use self::sfx::{MusicPlayer, SfxPlayer, resolve_song_step};
use crate::peripheral::{Peripheral, PeripheralRegistry};
use crate::rendering::screen::ScreenLayer;
use crate::vm::Camera;
use crate::vm::audio::{SFX_VOICE_COUNT, Sound};
use caiven_cart::{
    CartSection, DEFAULT_BANK_NAME, SectionKind, decode_asset_bank, is_valid_bank_name,
};
use caiven_core::memory::{
    COLLISION_LEN, COLLISION_RAM_BASE, MAP_LEN, MAP_RAM_BASE, MUSIC_BANK_LEN, MUSIC_RAM_BASE,
    PALETTE_RAM_BASE, PALETTE_SIZE, SFX_BANK_LEN, SFX_RAM_BASE, SPRITE_SHEET_LEN,
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
    Palette,
    Sfx,
    Music,
    /// Per-cell collision layer that shadows the active Map bank — a
    /// "companion" bank: created, selected, and removed in lockstep with
    /// its primary (see `AssetBankKind::companion`), no independent Lua
    /// selector, always follows `load_map_bank`. Cell bytes index the
    /// cart's collision-type table (`Vm::collision_types`, ids `0`/`1`/`2`
    /// built-in as walkable/solid/hazard, `3..=255` free for custom types).
    Collision,
}

impl AssetBankKind {
    /// The bank this kind shadows, if any. Whenever the primary is
    /// selected, created, or removed, the same operation is applied to its
    /// companion at the same bank id — see `Vm::{select,create,remove}_asset_bank`.
    pub fn companion(self) -> Option<AssetBankKind> {
        match self {
            AssetBankKind::Map => Some(AssetBankKind::Collision),
            _ => None,
        }
    }
}

struct AssetBanks {
    banks: BTreeMap<AssetBankKind, BTreeMap<String, Vec<u8>>>,
    active: BTreeMap<AssetBankKind, String>,
}

impl AssetBanks {
    const KINDS: [AssetBankKind; 6] = [
        AssetBankKind::Sprites,
        AssetBankKind::Map,
        AssetBankKind::Palette,
        AssetBankKind::Sfx,
        AssetBankKind::Music,
        AssetBankKind::Collision,
    ];

    fn new() -> Self {
        let mut banks = BTreeMap::new();
        let mut active = BTreeMap::new();
        for kind in Self::KINDS {
            let (_, len) = Self::region(kind);
            banks.insert(
                kind,
                BTreeMap::from([(DEFAULT_BANK_NAME.to_string(), vec![0; len])]),
            );
            active.insert(kind, DEFAULT_BANK_NAME.to_string());
        }
        Self { banks, active }
    }

    fn region(kind: AssetBankKind) -> (usize, usize) {
        match kind {
            AssetBankKind::Sprites => (SPRITE_SHEET_RAM_BASE, SPRITE_SHEET_LEN),
            AssetBankKind::Map => (MAP_RAM_BASE, MAP_LEN),
            AssetBankKind::Palette => (PALETTE_RAM_BASE, PALETTE_SIZE * 3),
            AssetBankKind::Sfx => (SFX_RAM_BASE, SFX_BANK_LEN),
            AssetBankKind::Music => (MUSIC_RAM_BASE, MUSIC_BANK_LEN),
            AssetBankKind::Collision => (COLLISION_RAM_BASE, COLLISION_LEN),
        }
    }

    fn banks(&self, kind: AssetBankKind) -> &BTreeMap<String, Vec<u8>> {
        self.banks
            .get(&kind)
            .expect("all AssetBankKind variants are seeded in AssetBanks::new")
    }

    fn banks_mut(&mut self, kind: AssetBankKind) -> &mut BTreeMap<String, Vec<u8>> {
        self.banks
            .get_mut(&kind)
            .expect("all AssetBankKind variants are seeded in AssetBanks::new")
    }

    fn active(&self, kind: AssetBankKind) -> &str {
        self.active
            .get(&kind)
            .expect("all AssetBankKind variants are seeded in AssetBanks::new")
    }

    fn set_active(&mut self, kind: AssetBankKind, name: &str) {
        self.active.insert(kind, name.to_string());
    }

    fn normalized(data: &[u8], len: usize) -> Vec<u8> {
        let mut out = vec![0; len];
        let copy_len = len.min(data.len());
        out[..copy_len].copy_from_slice(&data[..copy_len]);
        out
    }

    fn sync(&mut self, kind: AssetBankKind, memory: &Memory) {
        let (base, len) = Self::region(kind);
        let active = self.active(kind).to_string();
        let data: Vec<u8> = (0..len)
            .map(|offset| memory.read(base + offset).unwrap_or(0))
            .collect();
        self.banks_mut(kind).insert(active, data);
    }

    fn select(&mut self, kind: AssetBankKind, name: &str, memory: &mut Memory) -> bool {
        if self.active(kind) == name {
            return self.banks(kind).contains_key(name);
        }
        let Some(data) = self.banks(kind).get(name).cloned() else {
            return false;
        };
        self.sync(kind, memory);
        let (base, _) = Self::region(kind);
        for (offset, byte) in data.into_iter().enumerate() {
            let _ = memory.write(base + offset, byte);
        }
        self.set_active(kind, name);
        true
    }

    /// Selects `kind`'s bank `name`, cascading to its companion (if any) at
    /// the same name — creating a fresh zero-filled companion bank first if
    /// one doesn't exist yet, so a companion can never lag behind on stale
    /// data from whatever bank was previously active (e.g. switching to a
    /// Map bank that has no matching Collision bank must not leave the old
    /// bank's collision governing the new map). Shared by
    /// `Vm::{select,create}_asset_bank` and the Lua `load_*_bank` builtins
    /// (`lua_exec.rs`) so the three call paths can't drift on this.
    fn select_with_companion(
        &mut self,
        kind: AssetBankKind,
        name: &str,
        memory: &mut Memory,
    ) -> bool {
        let selected = self.select(kind, name, memory);
        if selected && let Some(companion) = kind.companion() {
            if !self.banks(companion).contains_key(name) {
                let (_, len) = Self::region(companion);
                self.banks_mut(companion)
                    .insert(name.to_string(), vec![0; len]);
            }
            self.select(companion, name, memory);
        }
        selected
    }
}

pub struct Vm {
    memory: Memory,
    camera: Camera,
    palette: Palette,
    sound: Arc<Mutex<Sound>>,
    music_player: MusicPlayer,
    sfx_pool: [PooledSfx; SFX_VOICE_COUNT],
    /// Handle of the voice Studio's SFX-editor preview is holding, if any.
    /// The preview borrows an ordinary sfx voice rather than owning one of
    /// its own — with only two, a reserved third would be a third of the
    /// console's polyphony spent on an editor.
    preview_sfx: Option<u32>,
    next_sfx_age: u64,
    peripherals: PeripheralRegistry,
    frame_count: u32,
    waiting: bool,
    fault: Option<VmFault>,
    /// Plain-language description of `fault`, when there's more to say than
    /// the variant itself carries — set alongside `fault` by `set_fault`.
    /// `VmFault::LuaError` carries no text of its own, so a host wanting to
    /// show *why* a cart crashed (not just that it did) reads this via
    /// [`Vm::fault_message`] instead.
    fault_message: Option<String>,
    world: ScreenLayer,
    ui: ScreenLayer,
    config: VmConfig,
    script: Option<lua_exec::LuaScript>,
    capture_lua_output: bool,
    call_stack: Vec<(String, String)>,
    /// Local variables at the innermost frame, captured at the moment the
    /// last breakpoint hit — cleared once execution resumes past a
    /// breakpoint, same lifecycle as `call_stack`. See
    /// `Vm::run_frame_lua_bp` for how these are read via raw FFI.
    locals: Vec<lua_exec::RawLocal>,
    /// Table/function values rooted for the debugger's expand-on-demand
    /// inspector, keyed by node id (see [`lua_exec::DebugValue`]). Cleared
    /// once per tick via [`Vm::clear_debug_roots`] so ids from a prior
    /// pause never stay valid across a step/resume.
    debug_roots: std::collections::HashMap<String, mlua::Value>,
    asset_banks: AssetBanks,
    save_data: SaveData,
    /// Cart-global collision-type table (names/colors/solid flags). Small
    /// metadata, not RAM-backed — see `caiven_core::collision` and
    /// `COLLISION_RAM_BASE`'s doc comment. Seeded with the built-in types
    /// and overwritten wholesale by a `SectionKind::CollisionTypes` section
    /// on cart load, so old carts without one still get valid defaults.
    collision_types: Vec<caiven_core::CollisionType>,
    /// Cart's opt-in gameplay-stdlib selection (`[stdlib] modules` in
    /// `caiven.toml`), set via `Vm::set_prelude_modules` before the first
    /// `load_lua_source`. Empty means core-only — there is no "load
    /// everything" default. See `lua_exec::PRELUDE_MODULES`.
    active_prelude_modules: Vec<&'static str>,
}

/// One slot of the round-robin SFX voice pool backing the polyphonic
/// `play_sfx`/`stop_sfx` Lua API. `age` is a monotonically increasing
/// counter set on every (re)trigger — the pool steals the slot with the
/// smallest `age` when all slots are busy, i.e. the one triggered longest
/// ago. `epoch` is a separate counter, also bumped on every (re)trigger,
/// used only to validate a handle returned to Lua — kept distinct from the
/// shared `Voice.epoch` (bumped independently by `tick_sfx_channel` on
/// every note-start) so a handle doesn't go stale the instant the pool's
/// player ticks its first step; those are two different questions ("is
/// this handle still this call's voice" vs. "has the audio thread's
/// envelope/phase seen a retrigger").
struct PooledSfx {
    player: SfxPlayer,
    age: u64,
    epoch: u32,
    volume_scale: f32,
}

impl PooledSfx {
    fn new() -> Self {
        Self {
            player: SfxPlayer::new(),
            age: 0,
            epoch: 0,
            volume_scale: 1.0,
        }
    }
}

/// Packs a pool slot index and its current allocation epoch into a single
/// handle returned to Lua. `release_sfx_voice` decodes both and only acts
/// if the epoch still matches — a handle for a voice since stolen by
/// another `play_sfx` call is a silent no-op instead of stopping the wrong
/// sound.
fn pack_sfx_handle(slot: u32, epoch: u32) -> u32 {
    (epoch << 3) | (slot & 0x7)
}

fn unpack_sfx_handle(handle: u32) -> (u32, u32) {
    (handle & 0x7, handle >> 3)
}

/// Starts sound effect `id` on a free (or, if all are busy, the
/// least-recently-triggered) pool voice. `volume` is a `[0, 1]` multiplier
/// layered on top of each step's authored volume. Returns an opaque handle
/// for `release_sfx_voice`. Free function (not a `Vm` method) so both
/// `Vm::play_sfx_voice` and the Lua `play_sfx` closure in `lua_exec.rs`
/// (which can only borrow individual fields, never `&mut Vm`, from inside
/// `lua.scope`) share one implementation.
fn allocate_sfx_voice(
    pool: &mut [PooledSfx; SFX_VOICE_COUNT],
    next_age: &mut u64,
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
    pool[slot].epoch = pool[slot].epoch.wrapping_add(1);
    pool[slot].volume_scale = volume;
    pool[slot].player.start(id);

    pack_sfx_handle(slot as u32, pool[slot].epoch)
}

/// Stops the voice `handle` refers to, if it's still the current occupant
/// of that pool slot. Silent no-op for a handle whose voice already
/// finished or was stolen by a later `allocate_sfx_voice` call.
fn release_sfx_voice(
    pool: &mut [PooledSfx; SFX_VOICE_COUNT],
    sound: &Arc<Mutex<Sound>>,
    handle: u32,
) {
    let (slot, epoch) = unpack_sfx_handle(handle);
    let slot = slot as usize;
    if slot >= pool.len() || pool[slot].epoch != epoch {
        return;
    }

    pool[slot].player.stop();
    if let Ok(mut s) = sound.try_lock() {
        s.voices[audio::SFX_VOICE_START + slot].gate = false;
    }
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
            sound: Arc::new(Mutex::new(Sound::default())),
            music_player: MusicPlayer::new(),
            sfx_pool: std::array::from_fn(|_| PooledSfx::new()),
            preview_sfx: None,
            next_sfx_age: 0,
            peripherals,
            frame_count: 0,
            waiting: false,
            fault: None,
            fault_message: None,
            world: ScreenLayer::new(config.width, config.height),
            ui: ScreenLayer::new(config.width, config.height),
            config,
            script: None,
            capture_lua_output: false,
            call_stack: Vec::new(),
            locals: Vec::new(),
            debug_roots: std::collections::HashMap::new(),
            asset_banks: AssetBanks::new(),
            save_data: SaveData::new(),
            collision_types: caiven_core::builtin_collision_types(),
            active_prelude_modules: Vec::new(),
        }
    }

    /// The cart's current collision-type table (built-ins plus any custom
    /// types), in id order as loaded/set.
    pub fn collision_types(&self) -> &[caiven_core::CollisionType] {
        &self.collision_types
    }

    /// Replaces the collision-type table wholesale (editor "manage types"
    /// commits a full table rather than deltas — see `caiven-studio`).
    pub fn set_collision_types(&mut self, types: Vec<caiven_core::CollisionType>) {
        self.collision_types = types;
    }

    pub fn save_data(&self) -> &SaveData {
        &self.save_data
    }

    pub fn save_data_mut(&mut self) -> &mut SaveData {
        &mut self.save_data
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
        // `LuaError` callers always know more than the bare variant (the
        // actual error text) and go through `set_fault_with_message`
        // directly; `ExecutionBudgetExceeded` has one fixed plain-language
        // message regardless of call site, so it's filled in here instead of
        // making every caller repeat it.
        let message = match fault {
            VmFault::ExecutionBudgetExceeded => {
                Some(lua_exec::EXECUTION_BUDGET_MESSAGE.to_string())
            }
            VmFault::LuaError | VmFault::MemoryOutOfBounds(_) => None,
        };
        self.set_fault_with_message(fault, message);
    }

    /// Same as [`Vm::set_fault`], additionally recording a plain-language
    /// description for [`Vm::fault_message`] — used for `VmFault::LuaError`,
    /// whose variant alone carries no text a host could show a player.
    pub fn set_fault_with_message(&mut self, fault: VmFault, message: Option<String>) {
        error!("VM FAULT: {:?}", fault);
        self.fault = Some(fault);
        self.fault_message = message;
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
        self.preview_sfx = None;
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

    pub fn load_section_to_ram(&mut self, base: usize, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            if let Err(e) = self.memory.write(base + i, byte) {
                log::error!("load_section_to_ram: write fault at {}: {:?}", base + i, e);
                break;
            }
        }
    }

    /// Copies every RAM-backed asset section a cart may carry (SpriteSheet,
    /// Map, Palette, SfxBank, MusicBank) to its fixed RAM base,
    /// and returns the cart's Lua source text if present. Single source of
    /// truth for "which section kind goes where" — every cart-loading call
    /// site (Studio, `caiven-machine`, the port screenshot capturer) must go
    /// through this instead of re-listing the mapping, so they can't drift
    /// apart the way the audio/peripheral per-frame tick paths already did
    /// once each grew a second, independently-written call site.
    pub fn load_cart_sections(&mut self, sections: &[CartSection]) -> Option<String> {
        self.asset_banks = AssetBanks::new();
        // Reset to built-ins; a `CollisionTypes` section below overwrites
        // this wholesale. Carts saved before this feature existed carry no
        // such section and simply keep the built-in three.
        self.collision_types = caiven_core::builtin_collision_types();
        for section in sections {
            let ram_base = match section.kind {
                SectionKind::SpriteSheet => {
                    self.asset_banks.banks_mut(AssetBankKind::Sprites).insert(
                        DEFAULT_BANK_NAME.to_string(),
                        AssetBanks::normalized(&section.data, SPRITE_SHEET_LEN),
                    );
                    continue;
                }
                SectionKind::Map => {
                    self.asset_banks.banks_mut(AssetBankKind::Map).insert(
                        DEFAULT_BANK_NAME.to_string(),
                        AssetBanks::normalized(&section.data, MAP_LEN),
                    );
                    continue;
                }
                SectionKind::SpriteBank => {
                    if let Some((name, data)) = decode_asset_bank(&section.data) {
                        self.asset_banks.banks_mut(AssetBankKind::Sprites).insert(
                            name.to_string(),
                            AssetBanks::normalized(data, SPRITE_SHEET_LEN),
                        );
                    }
                    continue;
                }
                SectionKind::MapBank => {
                    if let Some((name, data)) = decode_asset_bank(&section.data) {
                        self.asset_banks
                            .banks_mut(AssetBankKind::Map)
                            .insert(name.to_string(), AssetBanks::normalized(data, MAP_LEN));
                    }
                    continue;
                }
                SectionKind::Collision => {
                    self.asset_banks.banks_mut(AssetBankKind::Collision).insert(
                        DEFAULT_BANK_NAME.to_string(),
                        AssetBanks::normalized(&section.data, COLLISION_LEN),
                    );
                    continue;
                }
                SectionKind::CollisionBank => {
                    if let Some((name, data)) = decode_asset_bank(&section.data) {
                        self.asset_banks.banks_mut(AssetBankKind::Collision).insert(
                            name.to_string(),
                            AssetBanks::normalized(data, COLLISION_LEN),
                        );
                    }
                    continue;
                }
                SectionKind::CollisionTypes => {
                    self.collision_types = caiven_cart::decode_collision_types(&section.data);
                    continue;
                }
                // Additional (name != default) banks for the kinds whose
                // default-bank data still loads straight to RAM below. The
                // default bank self-heals into `asset_banks` on first
                // `select`/`sync`; see `AssetBanks::sync`.
                SectionKind::PaletteBank => {
                    if let Some((name, data)) = decode_asset_bank(&section.data) {
                        self.asset_banks.banks_mut(AssetBankKind::Palette).insert(
                            name.to_string(),
                            AssetBanks::normalized(data, PALETTE_SIZE * 3),
                        );
                    }
                    continue;
                }
                SectionKind::SfxBanks => {
                    if let Some((name, data)) = decode_asset_bank(&section.data) {
                        self.asset_banks
                            .banks_mut(AssetBankKind::Sfx)
                            .insert(name.to_string(), AssetBanks::normalized(data, SFX_BANK_LEN));
                    }
                    continue;
                }
                SectionKind::MusicBanks => {
                    if let Some((name, data)) = decode_asset_bank(&section.data) {
                        self.asset_banks.banks_mut(AssetBankKind::Music).insert(
                            name.to_string(),
                            AssetBanks::normalized(data, MUSIC_BANK_LEN),
                        );
                    }
                    continue;
                }
                SectionKind::Palette => PALETTE_RAM_BASE,
                SectionKind::SfxBank => SFX_RAM_BASE,
                SectionKind::MusicBank => MUSIC_RAM_BASE,
                SectionKind::Program
                | SectionKind::Meta
                | SectionKind::ModManifest
                | SectionKind::PreludeModules
                | SectionKind::LuaSource
                | SectionKind::Custom(_) => continue,
            };
            self.load_section_to_ram(ram_base, &section.data);
            if section.kind == SectionKind::Palette {
                self.set_palette_from_bytes(&section.data);
            }
        }
        for kind in [
            AssetBankKind::Sprites,
            AssetBankKind::Map,
            AssetBankKind::Collision,
        ] {
            let Some(data) = self.asset_banks.banks(kind).get(DEFAULT_BANK_NAME).cloned() else {
                continue;
            };
            let (base, _) = AssetBanks::region(kind);
            for (offset, byte) in data.into_iter().enumerate() {
                let _ = self.memory.write(base + offset, byte);
            }
            self.asset_banks.set_active(kind, DEFAULT_BANK_NAME);
        }
        sections
            .iter()
            .find(|s| s.kind == SectionKind::LuaSource)
            .map(|s| String::from_utf8_lossy(&s.data).into_owned())
    }

    pub fn asset_bank_names(&self, kind: AssetBankKind) -> Vec<String> {
        self.asset_banks.banks(kind).keys().cloned().collect()
    }

    pub fn active_asset_bank(&self, kind: AssetBankKind) -> &str {
        self.asset_banks.active(kind)
    }

    /// Selects bank `name` for `kind`. If `kind` has a companion (see
    /// `AssetBankKind::companion`), the companion follows to the same name —
    /// e.g. selecting a Map bank also selects its Collision bank.
    pub fn select_asset_bank(&mut self, kind: AssetBankKind, name: &str) -> bool {
        let selected = self
            .asset_banks
            .select_with_companion(kind, name, &mut self.memory);
        if selected && kind == AssetBankKind::Palette {
            self.sync_palette_from_ram();
        }
        selected
    }

    /// Refreshes the render-time `Palette` (parsed `Color` list) from
    /// whatever bytes are currently at `PALETTE_RAM_BASE`. Palette bank
    /// switches only move raw bytes through `AssetBanks`/`Memory` — without
    /// this, rendering would keep using the *previous* bank's colors since
    /// `self.palette` is otherwise only synced from RAM at cart-load time
    /// or by explicit `set_palette_color` pokes.
    fn sync_palette_from_ram(&mut self) {
        let bytes: Vec<u8> = (0..PALETTE_SIZE * 3)
            .map(|offset| self.memory.read(PALETTE_RAM_BASE + offset).unwrap_or(0))
            .collect();
        self.set_palette_from_bytes(&bytes);
    }

    /// Creates and selects bank `name` for `kind`. `name` must satisfy
    /// [`is_valid_bank_name`] and not already exist. A companion bank (if
    /// any) is created and selected alongside it at the same name, so the
    /// two stay in lockstep for the rest of their lifetime.
    pub fn create_asset_bank(&mut self, kind: AssetBankKind, name: &str) -> bool {
        if !is_valid_bank_name(name) || self.asset_banks.banks(kind).contains_key(name) {
            return false;
        }
        let (_, len) = AssetBanks::region(kind);
        self.asset_banks
            .banks_mut(kind)
            .insert(name.to_string(), vec![0; len]);
        let created = self
            .asset_banks
            .select_with_companion(kind, name, &mut self.memory);
        if created && kind == AssetBankKind::Palette {
            self.sync_palette_from_ram();
        }
        created
    }

    pub fn replace_asset_bank(&mut self, kind: AssetBankKind, name: &str, data: &[u8]) {
        let (_, len) = AssetBanks::region(kind);
        let data = AssetBanks::normalized(data, len);
        self.asset_banks
            .banks_mut(kind)
            .insert(name.to_string(), data.clone());
        if self.asset_banks.active(kind) == name {
            let (base, _) = AssetBanks::region(kind);
            for (offset, byte) in data.into_iter().enumerate() {
                let _ = self.memory.write(base + offset, byte);
            }
            if kind == AssetBankKind::Palette {
                self.sync_palette_from_ram();
            }
        }
    }

    /// Removes bank `name` for `kind`, falling back to the default bank if
    /// it was active. A companion bank (if any) is removed alongside it.
    /// The default bank itself can never be removed.
    pub fn remove_asset_bank(&mut self, kind: AssetBankKind, name: &str) -> bool {
        if name == DEFAULT_BANK_NAME || !self.asset_banks.banks(kind).contains_key(name) {
            return false;
        }
        if self.asset_banks.active(kind) == name {
            let _ = self
                .asset_banks
                .select(kind, DEFAULT_BANK_NAME, &mut self.memory);
            if kind == AssetBankKind::Palette {
                self.sync_palette_from_ram();
            }
        }
        let removed = self.asset_banks.banks_mut(kind).remove(name).is_some();
        if removed && let Some(companion) = kind.companion() {
            if self.asset_banks.active(companion) == name {
                let _ = self
                    .asset_banks
                    .select(companion, DEFAULT_BANK_NAME, &mut self.memory);
            }
            self.asset_banks.banks_mut(companion).remove(name);
        }
        removed
    }

    /// Returns current bank bytes, including unswitched RAM edits for the
    /// active bank.
    pub fn asset_bank_bytes(&self, kind: AssetBankKind, name: &str) -> Option<Vec<u8>> {
        if self.asset_banks.active(kind) == name {
            let (base, len) = AssetBanks::region(kind);
            Some(
                (0..len)
                    .map(|offset| self.memory.read(base + offset).unwrap_or(0))
                    .collect(),
            )
        } else {
            self.asset_banks.banks(kind).get(name).cloned()
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

    /// Frames run since load — the console-time clock a host can attach to
    /// a fault report (e.g. "crashed at frame N") without duplicating its
    /// own counter alongside the VM's.
    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }

    pub fn get_fault(&self) -> Option<VmFault> {
        self.fault
    }

    /// Plain-language description of the current fault, when there's more to
    /// say than the [`VmFault`] variant itself — set by
    /// [`Vm::set_fault_with_message`]. `None` for a fault-free VM, and for a
    /// fault that didn't record extra text.
    pub fn fault_message(&self) -> Option<&str> {
        self.fault_message.as_deref()
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

    /// Full RAM snapshot — the flat buffer backing sprites, map, palette
    /// region, sfx/music banks, collision and heap. Used by front-ends for
    /// save-state persistence; RTC's 3 live-register bytes ride along
    /// harmlessly, since the RTC peripheral overwrites them every tick
    /// regardless of what a restore puts there.
    pub fn ram(&self) -> &[u8] {
        self.memory.get_ram()
    }

    /// Restores a RAM snapshot taken from [`Vm::ram`]. `bytes` is untrusted
    /// (a save file may be truncated or hand-edited) — a length mismatch is
    /// rejected rather than resizing the buffer, which would desync every
    /// hardcoded region offset in `caiven_core::memory`.
    pub fn load_ram(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() != self.memory.get_length() {
            return false;
        }
        self.memory.set_ram(bytes.to_vec());
        true
    }

    /// Previews sound effect `id` for Studio's SFX editor on an ordinary
    /// sfx voice, replacing whatever the previous preview was playing.
    pub fn start_sfx(&mut self, id: u8) {
        self.stop_sfx();
        self.preview_sfx = Some(self.play_sfx_voice(id, 1.0));
    }

    pub fn stop_sfx(&mut self) {
        if let Some(handle) = self.preview_sfx.take() {
            self.stop_sfx_voice(handle);
        }
    }

    pub fn start_music(&mut self, pattern_id: u8) {
        self.music_player.start(pattern_id);
    }

    /// Starts song-order playback from `start_step`, the same way the Lua
    /// `play_music_song` builtin does. A song with nothing to play at that
    /// step (or its loop point) stops playback instead of starting it.
    pub fn start_music_song(&mut self, start_step: u8) {
        match resolve_song_step(&self.memory, start_step) {
            Some((pattern, resolved_step)) => {
                self.music_player.pattern_id = pattern;
                self.music_player.song_step = resolved_step;
                self.music_player.row = 0;
                self.music_player.tick_count = 0;
                self.music_player.active = true;
                self.music_player.song_active = true;
            }
            None => {
                self.music_player.active = false;
                self.music_player.song_active = false;
            }
        }
    }

    pub fn stop_music(&mut self) {
        self.music_player.stop();
        if let Ok(mut s) = self.sound.try_lock() {
            for v in s
                .voices
                .iter_mut()
                .skip(audio::MUSIC_VOICE_START)
                .take(audio::MUSIC_VOICE_COUNT)
            {
                v.gate = false;
                v.epoch = v.epoch.wrapping_add(1);
            }
        }
    }

    /// Snapshot of the voice Studio's SFX-editor preview is holding. Idle
    /// once the preview finished, was stopped, or had its voice stolen by a
    /// louder claim on the same two sfx voices.
    pub fn sfx_player(&self) -> SfxPlayer {
        self.preview_sfx
            .and_then(|handle| {
                let (slot, epoch) = unpack_sfx_handle(handle);
                self.sfx_pool
                    .get(slot as usize)
                    .filter(|pooled| pooled.epoch == epoch && pooled.player.active)
                    .map(|pooled| pooled.player.clone())
            })
            .unwrap_or_default()
    }

    pub fn music_player(&self) -> &MusicPlayer {
        &self.music_player
    }

    pub fn set_music_loop(&mut self, on: bool) {
        self.music_player.loop_on = on;
    }

    /// Starts sound effect `id` on a free (or, if all are busy, oldest)
    /// pool voice. `volume` is a `[0, 1]` multiplier layered on top of the
    /// step's authored volume. Returns a handle for `stop_sfx_voice`.
    pub fn play_sfx_voice(&mut self, id: u8, volume: f32) -> u32 {
        allocate_sfx_voice(&mut self.sfx_pool, &mut self.next_sfx_age, id, volume)
    }

    /// Stops the voice `handle` refers to. Silent no-op if that voice
    /// already finished or was reused by a later `play_sfx_voice` call.
    pub fn stop_sfx_voice(&mut self, handle: u32) {
        release_sfx_voice(&mut self.sfx_pool, &self.sound, handle)
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
                data: encode_asset_bank("forest", &vec![7; SPRITE_SHEET_LEN]),
            },
        ]);

        assert_eq!(
            vm.asset_bank_names(AssetBankKind::Sprites),
            vec![DEFAULT_BANK_NAME.to_string(), "forest".to_string()]
        );
        assert_eq!(vm.peek_memory(SPRITE_SHEET_RAM_BASE), 1);
        assert!(vm.select_asset_bank(AssetBankKind::Sprites, "forest"));
        assert_eq!(vm.peek_memory(SPRITE_SHEET_RAM_BASE), 7);
        vm.poke_memory(SPRITE_SHEET_RAM_BASE, 9);
        assert!(vm.select_asset_bank(AssetBankKind::Sprites, "forest"));
        assert_eq!(vm.peek_memory(SPRITE_SHEET_RAM_BASE), 9);
        assert!(vm.select_asset_bank(AssetBankKind::Sprites, DEFAULT_BANK_NAME));
        assert_eq!(vm.peek_memory(SPRITE_SHEET_RAM_BASE), 1);
        assert!(vm.select_asset_bank(AssetBankKind::Sprites, "forest"));
        assert_eq!(vm.peek_memory(SPRITE_SHEET_RAM_BASE), 9);
        assert!(!vm.select_asset_bank(AssetBankKind::Sprites, "missing"));
    }

    #[test]
    fn ram_round_trips_through_load_ram() {
        let mut vm = Vm::new(VmConfig::default());
        vm.poke_memory(SPRITE_SHEET_RAM_BASE, 42);
        let snapshot = vm.ram().to_vec();

        vm.poke_memory(SPRITE_SHEET_RAM_BASE, 7);
        assert_eq!(vm.peek_memory(SPRITE_SHEET_RAM_BASE), 7);

        assert!(vm.load_ram(&snapshot));
        assert_eq!(vm.peek_memory(SPRITE_SHEET_RAM_BASE), 42);
    }

    #[test]
    fn load_ram_rejects_a_length_mismatch() {
        let mut vm = Vm::new(VmConfig::default());
        vm.poke_memory(SPRITE_SHEET_RAM_BASE, 42);

        assert!(!vm.load_ram(&[0; 4]));
        assert_eq!(vm.peek_memory(SPRITE_SHEET_RAM_BASE), 42);
    }

    #[test]
    fn lua_can_switch_asset_banks() {
        let mut vm = Vm::new(VmConfig::default());
        vm.load_cart_sections(&[CartSection {
            kind: SectionKind::MapBank,
            data: encode_asset_bank("cave", &vec![6; MAP_LEN]),
        }]);
        vm.load_lua_source(
            "function _init() switched = load_map_bank(\"cave\") end\nfunction _update() end",
            &Input::new(),
            &Font::empty(),
        )
        .expect("Lua banking fixture should load");

        assert_eq!(vm.active_asset_bank(AssetBankKind::Map), "cave");
        assert_eq!(vm.peek_memory(MAP_RAM_BASE), 6);
        assert_eq!(
            vm.lua_watch("switched")
                .expect("Lua global should be readable"),
            "true"
        );
    }

    #[test]
    fn collision_bank_follows_map_as_companion_and_preserves_edits() {
        let mut vm = Vm::new(VmConfig::default());
        // Creating a new Map bank creates and selects a fresh, zero-filled
        // Collision bank at the same name — not a copy of the default.
        assert!(vm.create_asset_bank(AssetBankKind::Map, "cave"));
        assert_eq!(vm.active_asset_bank(AssetBankKind::Collision), "cave");
        assert_eq!(vm.peek_memory(COLLISION_RAM_BASE), 0);

        // Switching Map banks carries Collision along in lockstep, and
        // runtime edits to each bank's collision are preserved independently.
        vm.poke_memory(COLLISION_RAM_BASE, 1);
        assert!(vm.select_asset_bank(AssetBankKind::Map, DEFAULT_BANK_NAME));
        assert_eq!(
            vm.active_asset_bank(AssetBankKind::Collision),
            DEFAULT_BANK_NAME
        );
        assert_eq!(vm.peek_memory(COLLISION_RAM_BASE), 0);
        assert!(vm.select_asset_bank(AssetBankKind::Map, "cave"));
        assert_eq!(vm.active_asset_bank(AssetBankKind::Collision), "cave");
        assert_eq!(vm.peek_memory(COLLISION_RAM_BASE), 1);

        // Removing the Map bank removes its companion collision bank too.
        assert!(vm.remove_asset_bank(AssetBankKind::Map, "cave"));
        assert_eq!(
            vm.active_asset_bank(AssetBankKind::Collision),
            DEFAULT_BANK_NAME
        );
        assert!(
            !vm.asset_bank_names(AssetBankKind::Collision)
                .contains(&"cave".to_string())
        );
    }

    #[test]
    fn lua_load_map_bank_carries_collision_companion() {
        let mut vm = Vm::new(VmConfig::default());
        let mut collision = vec![0u8; COLLISION_LEN];
        collision[0] = 1;
        vm.load_cart_sections(&[
            CartSection {
                kind: SectionKind::MapBank,
                data: encode_asset_bank("cave", &vec![0; MAP_LEN]),
            },
            CartSection {
                kind: SectionKind::CollisionBank,
                data: encode_asset_bank("cave", &collision),
            },
        ]);
        vm.load_lua_source(
            "function _init() switched = load_map_bank(\"cave\") end\nfunction _update() end",
            &Input::new(),
            &Font::empty(),
        )
        .expect("Lua banking fixture should load");

        assert_eq!(vm.active_asset_bank(AssetBankKind::Collision), "cave");
        assert_eq!(vm.peek_memory(COLLISION_RAM_BASE), 1);
        assert_eq!(
            vm.lua_watch("switched")
                .expect("Lua global should be readable"),
            "true"
        );
    }

    #[test]
    fn selecting_palette_bank_updates_render_time_colors() {
        let mut vm = Vm::new(VmConfig::default());
        vm.load_cart_sections(&[CartSection {
            kind: SectionKind::PaletteBank,
            data: encode_asset_bank("alt", &[11, 22, 33]),
        }]);

        // Before the switch, slot 0 has whatever the default palette is —
        // definitely not (11, 22, 33).
        assert_ne!(vm.get_palette()[0].to_rgb(), [11, 22, 33]);
        assert!(vm.select_asset_bank(AssetBankKind::Palette, "alt"));
        // Raw RAM moved (this part already worked)...
        assert_eq!(vm.peek_memory(PALETTE_RAM_BASE), 11);
        // ...and critically, so did the render-time Color cache actually
        // used by drawing — this is the bug this test guards against.
        assert_eq!(vm.get_palette()[0].to_rgb(), [11, 22, 33]);
    }

    #[test]
    fn lua_can_switch_palette_sfx_and_music_banks() {
        let mut vm = Vm::new(VmConfig::default());
        vm.load_cart_sections(&[
            CartSection {
                kind: SectionKind::PaletteBank,
                data: encode_asset_bank("alt", &[9, 9, 9]),
            },
            CartSection {
                kind: SectionKind::SfxBanks,
                data: encode_asset_bank("alt", &[5; SFX_BANK_LEN]),
            },
            CartSection {
                kind: SectionKind::MusicBanks,
                data: encode_asset_bank("alt", &[3; MUSIC_BANK_LEN]),
            },
        ]);
        vm.load_lua_source(
            "function _init()\n\
             palette_ok = load_palette_bank(\"alt\")\n\
             sfx_ok = load_sfx_bank(\"alt\")\n\
             music_ok = load_music_bank(\"alt\")\n\
             end\n\
             function _update() end",
            &Input::new(),
            &Font::empty(),
        )
        .expect("Lua banking fixture should load");

        assert_eq!(vm.active_asset_bank(AssetBankKind::Palette), "alt");
        assert_eq!(vm.active_asset_bank(AssetBankKind::Sfx), "alt");
        assert_eq!(vm.active_asset_bank(AssetBankKind::Music), "alt");
        assert_eq!(vm.peek_memory(PALETTE_RAM_BASE), 9);
        assert_eq!(vm.peek_memory(SFX_RAM_BASE), 5);
        assert_eq!(vm.peek_memory(MUSIC_RAM_BASE), 3);
        for global in ["palette_ok", "sfx_ok", "music_ok"] {
            assert_eq!(
                vm.lua_watch(global)
                    .unwrap_or_else(|_| panic!("{global} should be readable")),
                "true"
            );
        }
        // The Lua-driven switch must refresh render-time colors too, not
        // just RAM — same bug class as select_asset_bank, fixed separately
        // in the load_palette_bank builtin itself.
        assert_eq!(vm.get_palette()[0].to_rgb(), [9, 9, 9]);
    }
}
