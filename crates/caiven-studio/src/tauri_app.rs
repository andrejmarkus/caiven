//! Tauri shell for Caiven Studio.
//!
//! `mlua::Lua` intentionally stays on one dedicated actor thread. Tauri
//! commands exchange owned messages with that thread and read framebuffer
//! snapshots, so webview workers never own or lock VM internals.

use crate::app::cart_io::{self, CartMeta};
use crate::debugger::{Breakpoint, Debugger};
use crate::studio::{SourceFile, asset_index, cart, examples, recent, templates};
use caiven_cart::{DEFAULT_BANK_NAME, SectionKind, encode_asset_bank};
use caiven_core::Color;
use caiven_core::memory::{
    COLLISION_LEN, COLLISION_RAM_BASE, MAP_LEN, MAP_RAM_BASE, MUSIC_BANK_LEN, MUSIC_ORDER_STEPS,
    MUSIC_RAM_BASE, PALETTE_RAM_BASE, PALETTE_SIZE, RAM_SIZE, RGBA_BYTES, SCREEN_HEIGHT,
    SCREEN_WIDTH, SFX_BANK_LEN, SFX_RAM_BASE, SPRITE_BYTES, SPRITE_SHEET_LEN,
    SPRITE_SHEET_RAM_BASE,
};
use caiven_vm::input::Button;
use caiven_vm::runtime::ConsoleCore;
use caiven_vm::vm::SaveData;
use caiven_vm::vm::api_registry;
use caiven_vm::{AssetBankKind, BreakableLines, LuaBreakpoint, LuaRunOutcome, LuaStep, Vm};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, mpsc};
use std::time::{Duration, Instant};
#[cfg(debug_assertions)]
use tauri::Manager;
use tauri::{Emitter, State};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum RunState {
    Running,
    Paused,
    Stopped,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourcePayload {
    path: String,
    name: String,
    text: String,
    dirty: bool,
}

#[derive(Clone, Serialize)]
struct ApiParamPayload {
    name: String,
    ty: String,
}

#[derive(Clone, Serialize)]
struct ApiEntryPayload {
    name: String,
    params: Vec<ApiParamPayload>,
    returns: String,
    doc: String,
    category: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PreludeModulePayload {
    name: String,
    /// Conventional local for the module's table, e.g. `Camera`.
    export: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveResult {
    output: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct MetaPayload {
    description: String,
    tags: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticPayload {
    severity: String,
    title: String,
    detail: String,
    path: String,
    line: Option<usize>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GlobalPayload {
    name: String,
    value: String,
    node_id: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DebugChildPayload {
    key: String,
    value: String,
    node_id: Option<String>,
}

impl From<(String, caiven_vm::DebugValue)> for DebugChildPayload {
    fn from((key, value): (String, caiven_vm::DebugValue)) -> Self {
        Self {
            key,
            value: value.text,
            node_id: value.node_id,
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CallFramePayload {
    label: String,
    location: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PauseReasonPayload {
    kind: String,
    source: Option<String>,
    line: Option<usize>,
    message: Option<String>,
}

impl PauseReasonPayload {
    fn manual() -> Self {
        Self {
            kind: "manual".to_string(),
            source: None,
            line: None,
            message: None,
        }
    }

    fn breakpoint(breakpoint: &Breakpoint) -> Self {
        Self {
            kind: "breakpoint".to_string(),
            source: Some(breakpoint.source.clone()),
            line: Some(breakpoint.line),
            message: None,
        }
    }

    fn step(location: &Breakpoint) -> Self {
        Self {
            kind: "step".to_string(),
            ..Self::breakpoint(location)
        }
    }

    fn error(source: String, line: Option<usize>, message: String) -> Self {
        Self {
            kind: "error".to_string(),
            source: Some(source),
            line,
            message: Some(message),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AudioPayload {
    sfx_active: bool,
    sfx_id: u8,
    sfx_step: u8,
    music_active: bool,
    music_pattern: u8,
    music_row: u8,
    music_loop: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CartSizePayload {
    packed_bytes: usize,
    max_bytes: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssetBankPayload {
    kind: String,
    names: Vec<String>,
    active: String,
    data: Vec<u8>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MapCellPayload {
    offset: usize,
    tile: u8,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CollisionCellPayload {
    offset: usize,
    value: u8,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CollisionTypePayload {
    id: u8,
    name: String,
    color: [u8; 3],
    shape: String,
}

impl From<&caiven_core::CollisionType> for CollisionTypePayload {
    fn from(t: &caiven_core::CollisionType) -> Self {
        let shape = if t.flags.is_solid() {
            "solid"
        } else if t.flags.is_one_way() {
            "one_way"
        } else if t.flags.is_slope_left() {
            "slope_left"
        } else if t.flags.is_slope_right() {
            "slope_right"
        } else {
            "none"
        };
        Self {
            id: t.id,
            name: t.name.clone(),
            color: t.color,
            shape: shape.to_string(),
        }
    }
}

impl From<CollisionTypePayload> for caiven_core::CollisionType {
    fn from(p: CollisionTypePayload) -> Self {
        let bits = match p.shape.as_str() {
            "solid" => caiven_core::CollisionTypeFlags::SOLID,
            "one_way" => caiven_core::CollisionTypeFlags::ONE_WAY,
            "slope_left" => caiven_core::CollisionTypeFlags::SLOPE_LEFT,
            "slope_right" => caiven_core::CollisionTypeFlags::SLOPE_RIGHT,
            _ => 0,
        };
        Self {
            id: p.id,
            name: p.name,
            color: p.color,
            flags: caiven_core::CollisionTypeFlags::from_bits(bits),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapPayload {
    connected: bool,
    title: String,
    path: String,
    author: String,
    run_state: RunState,
    frame: u64,
    fps: f32,
    cart_size: CartSizePayload,
    sources: Vec<SourcePayload>,
    palette: Vec<String>,
    sprite_sheet: Vec<u8>,
    map: Vec<u8>,
    sprite_banks: Vec<String>,
    map_banks: Vec<String>,
    active_sprite_bank: String,
    active_map_bank: String,
    collision: Vec<u8>,
    collision_types: Vec<CollisionTypePayload>,
    sfx: Vec<u8>,
    music: Vec<u8>,
    palette_banks: Vec<String>,
    active_palette_bank: String,
    sfx_banks: Vec<String>,
    active_sfx_bank: String,
    music_banks: Vec<String>,
    active_music_bank: String,
    ram: Vec<u8>,
    globals: Vec<GlobalPayload>,
    watches: Vec<GlobalPayload>,
    call_stack: Vec<CallFramePayload>,
    locals: Vec<GlobalPayload>,
    breakpoints: Vec<Breakpoint>,
    pause_reason: Option<PauseReasonPayload>,
    diagnostics: Vec<DiagnosticPayload>,
    output: Vec<String>,
    meta: MetaPayload,
    asset_index: asset_index::AssetIndex,
    audio: AudioPayload,
    recent: Vec<String>,
    api: Vec<ApiEntryPayload>,
    prelude_modules: Vec<PreludeModulePayload>,
    /// Whether an asset/meta edit (sprite, palette, map, collision, bank,
    /// title/author) happened since the last save — code
    /// edits are tracked separately per-`SourcePayload`. See review STU-04.
    asset_dirty: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TickPayload {
    run_state: RunState,
    frame: u64,
    lua_runs: u64,
    fps: f32,
    frame_time_ms: f32,
    globals: Vec<GlobalPayload>,
    watches: Vec<GlobalPayload>,
    call_stack: Vec<CallFramePayload>,
    /// Index into `call_stack` whose locals `locals` shows.
    selected_frame: usize,
    locals: Vec<GlobalPayload>,
    pause_reason: Option<PauseReasonPayload>,
    audio: AudioPayload,
    diagnostics: Vec<DiagnosticPayload>,
    output: Vec<String>,
    active_sprite_bank: String,
    active_map_bank: String,
    active_palette_bank: String,
    active_sfx_bank: String,
    active_music_bank: String,
    asset_dirty: bool,
}

#[derive(Clone)]
struct SharedSnapshot {
    frame: Vec<u8>,
    tick: TickPayload,
}

impl Default for SharedSnapshot {
    fn default() -> Self {
        Self {
            frame: vec![0; SCREEN_WIDTH as usize * SCREEN_HEIGHT as usize * RGBA_BYTES],
            tick: TickPayload {
                run_state: RunState::Stopped,
                frame: 0,
                lua_runs: 0,
                fps: 0.0,
                frame_time_ms: 0.0,
                globals: Vec::new(),
                watches: Vec::new(),
                call_stack: Vec::new(),
                selected_frame: 0,
                locals: Vec::new(),
                pause_reason: None,
                audio: AudioPayload {
                    sfx_active: false,
                    sfx_id: 0,
                    sfx_step: 0,
                    music_active: false,
                    music_pattern: 0,
                    music_row: 0,
                    music_loop: true,
                },
                diagnostics: Vec::new(),
                output: Vec::new(),
                active_sprite_bank: DEFAULT_BANK_NAME.to_string(),
                active_map_bank: DEFAULT_BANK_NAME.to_string(),
                active_palette_bank: DEFAULT_BANK_NAME.to_string(),
                active_sfx_bank: DEFAULT_BANK_NAME.to_string(),
                active_music_bank: DEFAULT_BANK_NAME.to_string(),
                asset_dirty: false,
            },
        }
    }
}

enum CoreCommand {
    Bootstrap(mpsc::Sender<Result<BootstrapPayload, String>>),
    CartSize(mpsc::Sender<Result<CartSizePayload, String>>),
    OpenProject {
        path: PathBuf,
        reply: mpsc::Sender<Result<BootstrapPayload, String>>,
    },
    NewProject {
        path: PathBuf,
        template_id: String,
        reply: mpsc::Sender<Result<BootstrapPayload, String>>,
    },
    RemixExample {
        path: PathBuf,
        example_id: String,
        reply: mpsc::Sender<Result<BootstrapPayload, String>>,
    },
    WriteBuffer {
        path: String,
        text: String,
        reply: mpsc::Sender<Result<(), String>>,
    },
    Save(mpsc::Sender<Result<SaveResult, String>>),
    Export {
        path: PathBuf,
        reply: mpsc::Sender<Result<(), String>>,
    },
    ExportWeb {
        path: PathBuf,
        reply: mpsc::Sender<Result<(), String>>,
    },
    ExportScreenshot {
        path: PathBuf,
        reply: mpsc::Sender<Result<(), String>>,
    },
    ExportSourceZip {
        path: PathBuf,
        reply: mpsc::Sender<Result<(), String>>,
    },
    Transport {
        action: String,
        reply: mpsc::Sender<Result<TickPayload, String>>,
    },
    SetInput {
        button: u8,
        pressed: bool,
        reply: mpsc::Sender<Result<(), String>>,
    },
    WriteSprite {
        sprite: usize,
        pixels: Vec<u8>,
        reply: mpsc::Sender<Result<(), String>>,
    },
    WritePalette {
        slot: usize,
        hex: String,
        reply: mpsc::Sender<Result<(), String>>,
    },
    ToggleBreakpoint {
        source: String,
        line: usize,
        reply: mpsc::Sender<Result<Vec<Breakpoint>, String>>,
    },
    AddWatch {
        expression: String,
        reply: mpsc::Sender<Result<Vec<GlobalPayload>, String>>,
    },
    RemoveWatch {
        expression: String,
        reply: mpsc::Sender<Result<Vec<GlobalPayload>, String>>,
    },
    ExpandDebugValue {
        node_id: String,
        reply: mpsc::Sender<Result<Vec<DebugChildPayload>, String>>,
    },
    SelectFrame {
        index: usize,
        reply: mpsc::Sender<Result<TickPayload, String>>,
    },
    PeekValue {
        expression: String,
        reply: mpsc::Sender<Result<String, String>>,
    },
    ClearOutput {
        reply: mpsc::Sender<Result<(), String>>,
    },
    RemoveRecent {
        path: PathBuf,
        reply: mpsc::Sender<Result<Vec<String>, String>>,
    },
    ReadMemory {
        address: usize,
        len: usize,
        reply: mpsc::Sender<Result<Vec<u8>, String>>,
    },
    WriteMemory {
        address: usize,
        bytes: Vec<u8>,
        reply: mpsc::Sender<Result<(), String>>,
    },
    WriteMapCells {
        cells: Vec<MapCellPayload>,
        reply: mpsc::Sender<Result<(), String>>,
    },
    WriteCollisionCells {
        cells: Vec<CollisionCellPayload>,
        reply: mpsc::Sender<Result<(), String>>,
    },
    ReadCollisionTypes {
        reply: mpsc::Sender<Result<Vec<CollisionTypePayload>, String>>,
    },
    /// Replaces the whole collision-type table — the editor's "manage
    /// types" UI always sends the full set it computed, rather than deltas,
    /// so there's no ordering/race concern between concurrent edits.
    WriteCollisionTypes {
        types: Vec<CollisionTypePayload>,
        reply: mpsc::Sender<Result<(), String>>,
    },
    WriteMeta {
        title: String,
        author: String,
        meta: MetaPayload,
        reply: mpsc::Sender<Result<(), String>>,
    },
    CreateModule {
        name: String,
        reply: mpsc::Sender<Result<SourcePayload, String>>,
    },
    CloseProject(mpsc::Sender<Result<BootstrapPayload, String>>),
    AudioTransport {
        kind: String,
        id: u8,
        action: String,
        loop_on: Option<bool>,
        reply: mpsc::Sender<Result<AudioPayload, String>>,
    },
    AssetIndex(mpsc::Sender<Result<asset_index::AssetIndex, String>>),
    AssetBank {
        kind: String,
        action: String,
        name: Option<String>,
        reply: mpsc::Sender<Result<AssetBankPayload, String>>,
    },
    PreparePublish {
        minify: bool,
        reply: mpsc::Sender<Result<(PathBuf, PathBuf), String>>,
    },
    IsDirty(mpsc::Sender<bool>),
}

struct StudioBridge {
    tx: mpsc::Sender<CoreCommand>,
    snapshot: Arc<RwLock<SharedSnapshot>>,
}

impl StudioBridge {
    fn request<T>(
        &self,
        build: impl FnOnce(mpsc::Sender<Result<T, String>>) -> CoreCommand,
    ) -> Result<T, String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(build(reply_tx))
            .map_err(|_| "Studio core stopped".to_string())?;
        reply_rx
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "Studio core did not respond".to_string())?
    }
}

struct StudioCore {
    console: ConsoleCore,
    cart: Option<CartMeta>,
    sources: Vec<SourceFile>,
    run_state: RunState,
    frame: u64,
    fps: f32,
    frame_time_ms: f32,
    debugger: Debugger,
    pause_reason: Option<PauseReasonPayload>,
    /// Set by `open()`, whose boot ran without the debugger: the first
    /// Run/Step compiles again so breakpoints reach top-level code and `_init()`.
    boot_on_run: bool,
    /// Where breakpoints stop, per source name, for the code last compiled.
    breakable: HashMap<String, BreakableLines>,
    /// Lua runs so far; expanded debugger rows refetch when it changes.
    lua_runs: u64,
    needs_compile: bool,
    diagnostics: Vec<DiagnosticPayload>,
    output: Vec<String>,
    /// Pristine copy of the cart's asset sections, refreshed only by an
    /// explicit editor edit (sprite/palette/map/collision write, asset-bank
    /// change, meta change) or a fresh compile — never by gameplay.
    /// `save()` writes this instead of reading live VM RAM, and Run/Reset
    /// restore it into RAM before compiling, so a mid-play mutation (e.g.
    /// `set_tile` clearing a collected coin) can never reach disk and never
    /// survives a Reset. See `.claude/rules/vm-runtime.md` / review ST-01.
    asset_snapshot: Vec<(SectionKind, Vec<u8>)>,
    /// Asset banks deleted this session but not yet flushed to disk — passed
    /// to `caiven_cart::save_project` so it deletes exactly these files
    /// instead of sweeping the project directory for anything bank-shaped
    /// (see review CART-04). Cleared once a save actually persists them.
    removed_banks: Vec<(SectionKind, String)>,
    /// True once an asset/meta edit happened since the last save (code edits
    /// are tracked per-`SourceFile`). Drives the unsaved-changes prompts.
    asset_dirty: bool,
}

impl StudioCore {
    fn new(initial_path: Option<PathBuf>) -> anyhow::Result<Self> {
        let mut console = ConsoleCore::new()?;
        console.vm.set_lua_output_capture(true);
        let mut studio = Self {
            console,
            cart: None,
            sources: Vec::new(),
            run_state: RunState::Stopped,
            frame: 0,
            fps: 0.0,
            frame_time_ms: 0.0,
            debugger: Debugger::new(),
            pause_reason: None,
            boot_on_run: false,
            breakable: HashMap::new(),
            lua_runs: 0,
            needs_compile: false,
            diagnostics: Vec::new(),
            output: Vec::new(),
            asset_snapshot: Vec::new(),
            removed_banks: Vec::new(),
            asset_dirty: false,
        };
        if let Some(path) = initial_path
            && let Err(error) = studio.open(&path)
        {
            log::warn!("could not open {}: {error:#}", path.display());
            studio
                .output
                .push(format!("Could not open {}: {error:#}", path.display()));
        }
        Ok(studio)
    }

    /// Recaptures `asset_snapshot` from the VM's current RAM/bank state.
    /// Call this after any editor-driven edit (never after a Lua frame ran),
    /// so the pristine copy always reflects the last thing a human authored.
    fn mark_asset_edit(&mut self) {
        self.asset_dirty = true;
        self.refresh_asset_snapshot();
    }

    fn refresh_asset_snapshot(&mut self) {
        if let Some(meta) = self.cart.as_ref() {
            self.asset_snapshot = cart_io::gather_sections(&self.console.vm, meta);
        }
    }

    fn open(&mut self, path: &Path) -> anyhow::Result<()> {
        // Validate into a scratch VM first — load_cart and reading project
        // sources can both fail. Committing to `self.console`/`self.cart`
        // only after both succeed means a failed open leaves the previously
        // open cart's VM and metadata untouched, instead of a `reset_vm`'d
        // blank VM paired with the old cart's still-in-place metadata (a
        // follow-up Ctrl+S would then write that old metadata's project
        // over blank RAM). See review STU-01.
        let capture_lua_output = self.console.vm.lua_output_capture_enabled();
        let mut vm = Vm::new(self.console.config);
        vm.set_lua_output_capture(capture_lua_output);
        let meta = cart::load_cart(&mut vm, path, &self.console.input, &self.console.font)?;
        let sources = if caiven_cart::is_project(path) {
            cart::load_project_sources(path)?
        } else {
            vec![SourceFile {
                path: path.to_path_buf(),
                text: meta.lua_source.clone(),
                dirty: false,
            }]
        };

        self.console.adopt_vm(vm);
        self.sources = sources;
        self.cart = Some(meta);
        if let Ok(bytes) = std::fs::read(save_data_path(path))
            && let Some(data) = SaveData::decode(&bytes)
        {
            *self.console.vm.save_data_mut() = data;
        }
        // Snapshot before the boot, so `_init()` changes to RAM stay out of it.
        self.refresh_asset_snapshot();
        // Boot now so editors show what `_init()` sets up, e.g. palette colors.
        self.console
            .vm
            .finish_lua_boot(&self.console.input, &self.console.font);
        self.debugger.set_dbg_path(debug_path(path));
        self.diagnostics.clear();
        self.output = vec![format!("Opened {}", path.display())];
        self.collect_vm_output();
        self.run_state = RunState::Paused;
        self.pause_reason = Some(PauseReasonPayload::manual());
        self.boot_on_run = true;
        self.needs_compile = false;
        self.frame = 0;
        self.fps = 0.0;
        self.frame_time_ms = 0.0;
        // Parked: `_init()` music waits for Run instead of playing now.
        self.console.vm.suspend_audio();
        self.removed_banks.clear();
        self.asset_dirty = false;
        recent::push(&mut recent::load(), path);
        Ok(())
    }

    fn new_project(&mut self, path: &Path, template_id: &str) -> Result<(), String> {
        let template = templates::find(template_id)
            .ok_or_else(|| format!("Unknown cart template: {template_id}"))?;
        if path.exists() {
            let mut entries =
                std::fs::read_dir(path).map_err(|error| format!("{}: {error}", path.display()))?;
            if entries
                .next()
                .transpose()
                .map_err(|error| error.to_string())?
                .is_some()
            {
                return Err(format!("New cart folder must be empty: {}", path.display()));
            }
        }
        self.console.reset_vm();
        // Seed the sprite sheet so the template's first Run shows something
        // visible instead of an invisible `sprite(0, ...)` — the template's
        // own _init() still sets the palette colors these pixels reference.
        let sprite_seed = templates::sprite_sheet_bytes(template);
        if !template.sprite_seed.is_empty() {
            cart::apply_sections(
                &mut self.console.vm,
                &[(SectionKind::SpriteSheet, sprite_seed)],
            );
        }
        let title = path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("untitled")
            .to_string();
        let source = template.source;
        self.sources = vec![SourceFile {
            path: path.join("main.lua"),
            text: source.to_string(),
            dirty: true,
        }];
        self.cart = Some(CartMeta {
            path: path.to_path_buf(),
            header: caiven_cart::CartHeader::default_for(&title),
            sections: cart::default_section_layout(),
            lua_source: source.to_string(),
        });
        self.debugger.set_dbg_path(debug_path(path));
        self.diagnostics.clear();
        self.output = vec![format!("Created {}", path.display())];
        self.run_state = RunState::Stopped;
        self.pause_reason = None;
        self.needs_compile = true;
        self.frame = 0;
        self.fps = 0.0;
        self.frame_time_ms = 0.0;
        self.removed_banks.clear();
        self.asset_dirty = false;
        self.refresh_asset_snapshot();
        self.save()?;
        recent::push(&mut recent::load(), path);
        Ok(())
    }

    /// Unpacks a bundled example cart into an empty project folder the user
    /// picked, then opens it — a fully editable "remix" of the example,
    /// exactly like opening a `.cav` someone shared. Code, sprites, and
    /// sound all round-trip because `cart::unpack_cart` writes a normal
    /// project directory, not a read-only copy of the packed cartridge.
    fn remix_example(&mut self, path: &Path, example_id: &str) -> Result<(), String> {
        let example = examples::find(example_id)
            .ok_or_else(|| format!("Unknown example cart: {example_id}"))?;
        let temp_cav = cart::temp_cav_path();
        std::fs::write(&temp_cav, example.bytes)
            .map_err(|error| format!("{}: {error}", temp_cav.display()))?;
        let unpack_result =
            cart::unpack_cart(&temp_cav, path).map_err(|error| format!("{error:#}"));
        let _ = std::fs::remove_file(&temp_cav);
        unpack_result?;
        self.open(path).map_err(|error| format!("{error:#}"))
    }

    fn project_dir(&self) -> Option<&Path> {
        let meta = self.cart.as_ref()?;
        (meta.path.extension().and_then(|value| value.to_str()) != Some("cav"))
            .then_some(meta.path.as_path())
    }

    fn modules(&self) -> Vec<(PathBuf, String)> {
        let Some(dir) = self.project_dir() else {
            return Vec::new();
        };
        self.sources
            .get(1..)
            .unwrap_or_default()
            .iter()
            .map(|source| {
                (
                    source
                        .path
                        .strip_prefix(dir)
                        .unwrap_or(&source.path)
                        .to_path_buf(),
                    source.text.clone(),
                )
            })
            .collect()
    }

    fn compile(&mut self) -> Result<(), String> {
        // Roots pin the previous run's Lua values; drop them with that run.
        self.console.vm.clear_debug_roots();
        // A fresh run starts from silence, not the previous run's music.
        self.console.vm.stop_audio();
        // Every fresh compile — Run from Stopped, Reset, Step from Stopped —
        // restores the pristine asset snapshot into RAM before `_init()`
        // runs, so gameplay's mutations from a previous play session (e.g.
        // `set_tile` clearing a collected coin) never leak into the next
        // run. `hot_reload` deliberately skips this: it's the
        // state-preserving Ctrl+S-while-running path. See review ST-01.
        if !self.asset_snapshot.is_empty() {
            cart::apply_sections(&mut self.console.vm, &self.asset_snapshot);
        }
        let project_dir = self.project_dir().map(Path::to_path_buf);
        match cart::compile_sources_into_vm(
            &mut self.console.vm,
            project_dir.as_deref(),
            &self.sources,
            &self.console.input,
            &self.console.font,
        ) {
            Ok(()) => {
                self.diagnostics.clear();
                self.pause_reason = None;
                self.needs_compile = false;
                self.boot_on_run = false;
                self.breakable = self.breakable_lines();
                self.collect_vm_output();
                self.output.push("Build succeeded".to_string());
                trim_output(&mut self.output);
                self.refresh_asset_snapshot();
                // The build's time must not come back as a catch-up burst
                // that runs the first notes before the audio thread hears them.
                self.console.reset_timing();
                Ok(())
            }
            Err(error) => {
                self.collect_vm_output();
                let source = match error.source.as_deref() {
                    Some("cart") | None => self.source_name(0),
                    Some(source) => source.to_string(),
                };
                let detail = match error.line {
                    Some(line) => format!("{source}:{line}: {}", error.message),
                    None => error.message.clone(),
                };
                self.run_state = RunState::Stopped;
                self.console.vm.stop_audio();
                self.diagnostics = vec![DiagnosticPayload {
                    severity: "error".to_string(),
                    title: "Build failed".to_string(),
                    detail: detail.clone(),
                    path: source.clone(),
                    line: error.line,
                }];
                self.pause_reason =
                    Some(PauseReasonPayload::error(source, error.line, error.message));
                self.output.push(format!("Error: {detail}"));
                trim_output(&mut self.output);
                Err(detail)
            }
        }
    }

    fn source_name(&self, index: usize) -> String {
        let Some(source) = self.sources.get(index) else {
            return "main.lua".to_string();
        };
        let name = match self.project_dir() {
            Some(dir) => source
                .path
                .strip_prefix(dir)
                .unwrap_or(&source.path)
                .display()
                .to_string(),
            // No project folder (a single .cav file) — a full absolute path
            // is never useful in the tree/editor tab, so show just the
            // file's own name (e.g. "main.lua").
            None => source
                .path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| source.path.display().to_string()),
        };
        name.replace('\\', "/")
    }

    fn bootstrap(&mut self) -> BootstrapPayload {
        self.console.vm.clear_debug_roots();
        let (title, path, author) = self
            .cart
            .as_ref()
            .map(|meta| {
                (
                    meta.header.title.clone(),
                    meta.path.display().to_string(),
                    meta.header.author.clone(),
                )
            })
            .unwrap_or_else(|| ("No cart open".into(), String::new(), String::new()));
        let sprite_sheet = read_region(&self.console, SPRITE_SHEET_RAM_BASE, SPRITE_SHEET_LEN);
        let map = read_region(&self.console, MAP_RAM_BASE, MAP_LEN);
        let collision = read_region(&self.console, COLLISION_RAM_BASE, COLLISION_LEN);
        let sfx = read_region(&self.console, SFX_RAM_BASE, SFX_BANK_LEN);
        let music = read_region(&self.console, MUSIC_RAM_BASE, MUSIC_BANK_LEN);
        BootstrapPayload {
            connected: true,
            title,
            path,
            author,
            run_state: self.run_state,
            frame: self.frame,
            fps: self.fps,
            cart_size: self.cart_size(),
            sources: self
                .sources
                .iter()
                .enumerate()
                .map(|(index, source)| SourcePayload {
                    path: source.path.display().to_string(),
                    name: self.source_name(index),
                    text: source.text.clone(),
                    dirty: source.dirty,
                })
                .collect(),
            palette: palette_hex(&self.console),
            sprite_sheet: sprite_sheet.clone(),
            map: map.clone(),
            sprite_banks: self.console.vm.asset_bank_names(AssetBankKind::Sprites),
            map_banks: self.console.vm.asset_bank_names(AssetBankKind::Map),
            active_sprite_bank: self
                .console
                .vm
                .active_asset_bank(AssetBankKind::Sprites)
                .to_string(),
            active_map_bank: self
                .console
                .vm
                .active_asset_bank(AssetBankKind::Map)
                .to_string(),
            collision,
            collision_types: self
                .console
                .vm
                .collision_types()
                .iter()
                .map(CollisionTypePayload::from)
                .collect(),
            sfx: sfx.clone(),
            music: music.clone(),
            palette_banks: self.console.vm.asset_bank_names(AssetBankKind::Palette),
            active_palette_bank: self
                .console
                .vm
                .active_asset_bank(AssetBankKind::Palette)
                .to_string(),
            sfx_banks: self.console.vm.asset_bank_names(AssetBankKind::Sfx),
            active_sfx_bank: self
                .console
                .vm
                .active_asset_bank(AssetBankKind::Sfx)
                .to_string(),
            music_banks: self.console.vm.asset_bank_names(AssetBankKind::Music),
            active_music_bank: self
                .console
                .vm
                .active_asset_bank(AssetBankKind::Music)
                .to_string(),
            ram: read_region(&self.console, 0, RAM_SIZE),
            globals: self.globals(),
            watches: self.watches(),
            call_stack: self.call_stack(),
            locals: self.locals(),
            breakpoints: self.debugger.breakpoints().to_vec(),
            pause_reason: self.pause_reason.clone(),
            diagnostics: self.diagnostics.clone(),
            output: self.output.clone(),
            meta: self.meta_payload(),
            asset_index: self.asset_index(),
            audio: self.audio_payload(),
            recent: recent::load()
                .into_iter()
                .map(|path| path.display().to_string())
                .collect(),
            api: self.api_payload(),
            prelude_modules: self.prelude_modules_payload(),
            asset_dirty: self.asset_dirty,
        }
    }

    fn cart_size(&self) -> CartSizePayload {
        let packed_bytes = self.cart.as_ref().map_or(0, |meta| {
            let modules = self.modules();
            let entry = self
                .sources
                .first()
                .map_or(meta.lua_source.as_str(), |source| source.text.as_str());
            cart_io::packed_size(&self.console.vm, meta, entry, &modules)
        });
        CartSizePayload {
            packed_bytes,
            max_bytes: caiven_cart::MAX_CART_BYTES,
        }
    }

    fn tick_payload(&mut self) -> TickPayload {
        self.console.vm.clear_debug_roots();
        TickPayload {
            run_state: self.run_state,
            lua_runs: self.lua_runs,
            frame: self.frame,
            fps: self.fps,
            frame_time_ms: self.frame_time_ms,
            globals: self.globals(),
            watches: self.watches(),
            call_stack: self.call_stack(),
            selected_frame: self.console.vm.lua_selected_frame(),
            locals: self.locals(),
            pause_reason: self.pause_reason.clone(),
            audio: self.audio_payload(),
            diagnostics: self.diagnostics.clone(),
            output: self.output.clone(),
            active_sprite_bank: self
                .console
                .vm
                .active_asset_bank(AssetBankKind::Sprites)
                .to_string(),
            active_map_bank: self
                .console
                .vm
                .active_asset_bank(AssetBankKind::Map)
                .to_string(),
            active_palette_bank: self
                .console
                .vm
                .active_asset_bank(AssetBankKind::Palette)
                .to_string(),
            active_sfx_bank: self
                .console
                .vm
                .active_asset_bank(AssetBankKind::Sfx)
                .to_string(),
            active_music_bank: self
                .console
                .vm
                .active_asset_bank(AssetBankKind::Music)
                .to_string(),
            asset_dirty: self.asset_dirty,
        }
    }

    fn call_stack(&self) -> Vec<CallFramePayload> {
        self.console
            .vm
            .lua_call_stack()
            .into_iter()
            .map(|(label, location)| {
                let location = location.trim_start_matches(['@', '=']);
                let location = location.rsplit_once(':').map_or_else(
                    || location.to_string(),
                    |(source, line)| {
                        format!(
                            "{}:{line}",
                            if source == "cart" {
                                self.source_name(0)
                            } else {
                                source.to_string()
                            }
                        )
                    },
                );
                CallFramePayload { label, location }
            })
            .collect()
    }

    fn globals(&mut self) -> Vec<GlobalPayload> {
        self.console
            .vm
            .lua_globals()
            .into_iter()
            .map(|(name, value)| GlobalPayload {
                name,
                value: value.text,
                node_id: value.node_id,
            })
            .collect()
    }

    fn locals(&mut self) -> Vec<GlobalPayload> {
        self.console
            .vm
            .lua_debug_locals()
            .into_iter()
            .map(|(name, value)| GlobalPayload {
                name,
                value: value.text,
                node_id: value.node_id,
            })
            .collect()
    }

    fn asset_index(&self) -> asset_index::AssetIndex {
        let sources: Vec<_> = self
            .sources
            .iter()
            .enumerate()
            .map(|(index, source)| (self.source_name(index), source.text.clone()))
            .collect();
        asset_index::build(
            &sources,
            &read_region(&self.console, SPRITE_SHEET_RAM_BASE, SPRITE_SHEET_LEN),
            &read_region(&self.console, MAP_RAM_BASE, MAP_LEN),
            &read_region(&self.console, SFX_RAM_BASE, SFX_BANK_LEN),
            &read_region(&self.console, MUSIC_RAM_BASE, MUSIC_BANK_LEN),
            &read_region(&self.console, PALETTE_RAM_BASE, PALETTE_SIZE * 3),
        )
    }

    fn asset_bank(
        &mut self,
        kind: &str,
        action: &str,
        name: Option<String>,
    ) -> Result<AssetBankPayload, String> {
        // Only kinds a user can pick from Studio's UI are dispatchable here.
        // Collision has no entry — it's a *companion* bank (see
        // `AssetBankKind::companion`) that always follows Map in lockstep,
        // so it's created/selected/deleted below as a side effect of the
        // Map bank operation rather than through its own kind.
        let bank_kind = match kind {
            "sprites" => AssetBankKind::Sprites,
            "map" => AssetBankKind::Map,
            "palette" => AssetBankKind::Palette,
            "sfx" => AssetBankKind::Sfx,
            "music" => AssetBankKind::Music,
            _ => return Err(format!("Unknown asset bank kind: {kind}")),
        };
        let section_kind = section_kind_for_bank(bank_kind);
        let label = kind;
        match action {
            "read" => {}
            "select" => {
                let name = name.ok_or_else(|| "Bank name required".to_string())?;
                if !self.console.vm.select_asset_bank(bank_kind, &name) {
                    return Err(format!("{label} bank \"{name}\" does not exist"));
                }
            }
            "create" => {
                let name = name.ok_or_else(|| "Bank name required".to_string())?;
                if !caiven_cart::is_valid_bank_name(&name) {
                    return Err(format!(
                        "\"{name}\" is not a valid bank name (1-{} letters, digits, _, or -)",
                        caiven_cart::MAX_BANK_NAME_LEN
                    ));
                }
                if !self.console.vm.create_asset_bank(bank_kind, &name) {
                    return Err(format!("Could not create {label} bank \"{name}\""));
                }
                let meta = self
                    .cart
                    .as_mut()
                    .ok_or_else(|| "No cart open".to_string())?;
                meta.sections.push(crate::app::cart_io::SectionLayout {
                    kind: section_kind,
                    ram_base: 0,
                    len: 1,
                    preserved_data: Some(encode_asset_bank(&name, &[])),
                });
                // The VM already created the companion bank's live data
                // (`create_asset_bank` cascades); track its section too so
                // `gather_sections` actually saves it instead of silently
                // dropping the companion on the next write.
                if let Some(companion_kind) = bank_kind.companion() {
                    meta.sections.push(crate::app::cart_io::SectionLayout {
                        kind: section_kind_for_bank(companion_kind),
                        ram_base: 0,
                        len: 1,
                        preserved_data: Some(encode_asset_bank(&name, &[])),
                    });
                }
            }
            "delete" => {
                let name = name.ok_or_else(|| "Bank name required".to_string())?;
                if !self.console.vm.remove_asset_bank(bank_kind, &name) {
                    return Err(format!("Cannot delete {label} bank \"{name}\""));
                }
                if let Some(meta) = self.cart.as_mut() {
                    let companion_section = bank_kind.companion().map(section_kind_for_bank);
                    meta.sections.retain(|section| {
                        let tracked_kind =
                            section.kind == section_kind || Some(section.kind) == companion_section;
                        let matches_name = section
                            .preserved_data
                            .as_deref()
                            .and_then(caiven_cart::decode_asset_bank)
                            .is_some_and(|(bank_name, _)| bank_name == name);
                        !(tracked_kind && matches_name)
                    });
                }
                // Explicit removal list for `save_project` (review CART-04):
                // it deletes exactly these bank files, never sweeps the
                // project directory for anything bank-name-shaped.
                self.removed_banks.push((section_kind, name.clone()));
                if let Some(companion_kind) = bank_kind.companion() {
                    self.removed_banks
                        .push((section_kind_for_bank(companion_kind), name));
                }
            }
            _ => return Err(format!("Unknown asset bank action: {action}")),
        }
        if action != "read" {
            self.mark_asset_edit();
        }
        let active = self.console.vm.active_asset_bank(bank_kind).to_string();
        let data = self
            .console
            .vm
            .asset_bank_bytes(bank_kind, &active)
            .unwrap_or_default();
        Ok(AssetBankPayload {
            kind: kind.to_string(),
            names: self.console.vm.asset_bank_names(bank_kind),
            active,
            data,
        })
    }

    fn watches(&mut self) -> Vec<GlobalPayload> {
        let expressions = self.debugger.watches().to_vec();
        expressions
            .into_iter()
            .map(|expression| {
                let watch = self.console.vm.lua_watch(&expression);
                match watch {
                    Ok(value) => GlobalPayload {
                        name: expression,
                        value: value.text,
                        node_id: value.node_id,
                    },
                    Err(error) => GlobalPayload {
                        name: expression,
                        value: format!("<{error}>"),
                        node_id: None,
                    },
                }
            })
            .collect()
    }

    /// A watch path's current value as text, for the editor's hover while paused.
    fn peek_value(&mut self, expression: &str) -> Result<String, String> {
        if self.run_state != RunState::Paused {
            return Err("Pause the cart to read values".to_string());
        }
        match self.console.vm.lua_watch(expression) {
            Ok(value) => Ok(value.text),
            Err(error) if error == "nil" => Ok("nil".to_string()),
            Err(error) => Err(error),
        }
    }

    /// Returns a previously rooted table/function's immediate children —
    /// read-only, same posture as `lua_watch`: never evaluates cart Lua,
    /// only walks an already-captured value (see `Vm::expand_debug_node`).
    fn expand_debug_value(&mut self, node_id: &str) -> Result<Vec<DebugChildPayload>, String> {
        self.console
            .vm
            .expand_debug_node(node_id)
            .map(|children| children.into_iter().map(DebugChildPayload::from).collect())
    }

    fn audio_active(&self) -> bool {
        self.console.vm.sfx_player().active || self.console.vm.music_player().active
    }

    fn audio_payload(&self) -> AudioPayload {
        let sfx = self.console.vm.sfx_player();
        let music = self.console.vm.music_player();
        AudioPayload {
            sfx_active: sfx.active,
            sfx_id: sfx.sfx_id,
            sfx_step: sfx.step,
            music_active: music.active,
            music_pattern: music.pattern_id,
            music_row: music.row,
            music_loop: music.loop_on,
        }
    }

    fn meta_payload(&self) -> MetaPayload {
        self.cart
            .as_ref()
            .and_then(|cart| {
                cart.sections
                    .iter()
                    .find(|section| section.kind == SectionKind::Meta)
            })
            .and_then(|section| section.preserved_data.as_deref())
            .and_then(|bytes| serde_json::from_slice(bytes).ok())
            .unwrap_or_default()
    }

    fn transport(&mut self, action: &str) -> Result<TickPayload, String> {
        match action {
            "run" => {
                if self.sources.is_empty() {
                    return Err("No cart open".to_string());
                }
                if self.run_state == RunState::Stopped || self.needs_compile || self.boot_on_run {
                    self.compile()?;
                } else {
                    self.console.vm.resume_audio();
                }
                self.pause_reason = None;
                self.run_state = RunState::Running;
            }
            "pause" => {
                self.run_state = RunState::Paused;
                self.pause_reason = Some(PauseReasonPayload::manual());
                self.console.vm.suspend_audio();
            }
            "reset" => {
                self.compile()?;
                self.frame = 0;
                self.pause_reason = None;
                self.run_state = RunState::Running;
            }
            "step" | "stepInto" | "stepOver" | "stepOut" => {
                if self.run_state == RunState::Stopped || self.needs_compile || self.boot_on_run {
                    self.compile()?;
                }
                let step = match action {
                    "stepInto" => Some(LuaStep::Into),
                    "stepOver" => Some(LuaStep::Over),
                    "stepOut" => Some(LuaStep::Out),
                    _ => None,
                };
                self.run_state = RunState::Paused;
                self.pause_reason = None;
                // The step advances the game's audio by one frame, then parks it.
                self.console.vm.resume_audio();
                // A line step that ends the frame stops at the next frame's first line.
                if self.run_one_frame(step)
                    && (step.is_none() || self.run_one_frame(Some(LuaStep::Into)))
                {
                    self.pause_reason = Some(PauseReasonPayload::manual());
                }
                self.console.vm.suspend_audio();
            }
            _ => return Err(format!("Unknown transport action: {action}")),
        }
        Ok(self.tick_payload())
    }

    fn runtime_breakpoints(&self) -> Vec<LuaBreakpoint> {
        let entry_source = self.source_name(0);
        self.debugger
            .breakpoints()
            .iter()
            .map(|breakpoint| {
                // A header, `end` or blank line stops at the code it marks.
                let line = self
                    .breakable
                    .get(&breakpoint.source)
                    .map_or(breakpoint.line, |lines| lines.resolve(breakpoint.line));
                LuaBreakpoint::new(
                    if breakpoint.source == entry_source {
                        "cart".to_string()
                    } else {
                        breakpoint.source.clone()
                    },
                    line,
                )
            })
            .collect()
    }

    fn breakable_lines(&self) -> HashMap<String, BreakableLines> {
        self.sources
            .iter()
            .enumerate()
            .map(|(index, source)| (self.source_name(index), BreakableLines::of(&source.text)))
            .collect()
    }

    fn source_breakpoint(&self, breakpoint: LuaBreakpoint) -> Breakpoint {
        Breakpoint {
            source: if breakpoint.source == "cart" {
                self.source_name(0)
            } else {
                breakpoint.source
            },
            line: breakpoint.line,
        }
    }

    fn run_one_frame(&mut self, step: Option<LuaStep>) -> bool {
        let started = Instant::now();
        let breakpoints = self.runtime_breakpoints();
        let outcome = self.console.run_frame_lua_step(&breakpoints, step);
        self.collect_vm_output();
        match outcome {
            LuaRunOutcome::Completed => {
                self.frame = self.frame.wrapping_add(1);
                self.frame_time_ms = started.elapsed().as_secs_f32() * 1000.0;
                true
            }
            LuaRunOutcome::Breakpoint(runtime_breakpoint) => {
                let breakpoint = self.source_breakpoint(runtime_breakpoint);
                self.run_state = RunState::Paused;
                self.pause_reason = Some(PauseReasonPayload::breakpoint(&breakpoint));
                self.console.vm.suspend_audio();
                self.output.push(format!(
                    "Paused at {}:{}",
                    breakpoint.source, breakpoint.line
                ));
                trim_output(&mut self.output);
                false
            }
            LuaRunOutcome::Step(location) => {
                let location = self.source_breakpoint(location);
                self.run_state = RunState::Paused;
                self.pause_reason = Some(PauseReasonPayload::step(&location));
                self.console.vm.suspend_audio();
                false
            }
            LuaRunOutcome::Error(location, message) => {
                self.run_state = RunState::Paused;
                self.console.vm.suspend_audio();
                let source = location
                    .as_ref()
                    .map(|location| {
                        if location.source == "cart" {
                            self.source_name(0)
                        } else {
                            location.source.clone()
                        }
                    })
                    .unwrap_or_else(|| self.source_name(0));
                let line = location.as_ref().map(|location| location.line);
                let detail = match line {
                    Some(line) => format!("{source}:{line}: {message}"),
                    None => message.clone(),
                };
                self.pause_reason = Some(PauseReasonPayload::error(source.clone(), line, message));
                self.diagnostics = vec![DiagnosticPayload {
                    severity: "error".to_string(),
                    title: "Runtime error".to_string(),
                    detail: detail.clone(),
                    path: source,
                    line,
                }];
                self.output.push(format!("Error: {detail}"));
                trim_output(&mut self.output);
                false
            }
        }
    }

    /// Every Lua run ends here, so it also counts runs for the debugger.
    fn collect_vm_output(&mut self) {
        self.lua_runs = self.lua_runs.wrapping_add(1);
        self.output.extend(self.console.vm.take_lua_output());
        trim_output(&mut self.output);
    }

    fn write_meta(
        &mut self,
        title: String,
        author: String,
        meta_payload: MetaPayload,
    ) -> Result<(), String> {
        let Some(cart) = self.cart.as_mut() else {
            return Err("No cart open".to_string());
        };
        cart.header.title = title.trim().to_string();
        cart.header.author = author.trim().to_string();
        let bytes = serde_json::to_vec(&meta_payload).map_err(|error| error.to_string())?;
        if let Some(section) = cart
            .sections
            .iter_mut()
            .find(|section| section.kind == SectionKind::Meta)
        {
            section.len = bytes.len();
            section.preserved_data = Some(bytes);
        } else {
            cart.sections.push(crate::app::cart_io::SectionLayout {
                kind: SectionKind::Meta,
                ram_base: 0,
                len: bytes.len(),
                preserved_data: Some(bytes),
            });
        }
        self.mark_asset_edit();
        Ok(())
    }

    /// The API entries available in the Studio editor, opt-in modules
    /// included so autocomplete can teach them before they are required.
    fn api_payload(&self) -> Vec<ApiEntryPayload> {
        [
            (api_registry::BUILTINS, "Console builtins"),
            (api_registry::PRELUDE, "Gameplay stdlib"),
            (api_registry::STDLIB, "Lua standard library"),
        ]
        .into_iter()
        .flat_map(|(entries, category)| entries.iter().map(move |entry| (entry, category)))
        .map(|(entry, category)| ApiEntryPayload {
            name: entry.name.to_string(),
            params: entry
                .params
                .iter()
                .map(|param| ApiParamPayload {
                    name: param.name.to_string(),
                    ty: param.ty.to_string(),
                })
                .collect(),
            returns: entry.returns.to_string(),
            doc: entry.doc.to_string(),
            category: category.to_string(),
        })
        .collect()
    }

    /// The opt-in prelude module catalog — drives the editor's
    /// missing-`require` diagnostic.
    fn prelude_modules_payload(&self) -> Vec<PreludeModulePayload> {
        caiven_vm::prelude_module_catalog()
            .into_iter()
            .map(|(name, export)| PreludeModulePayload {
                name: name.to_string(),
                export: export.to_string(),
            })
            .collect()
    }

    fn create_module(&mut self, name: &str) -> Result<SourcePayload, String> {
        let Some(dir) = self.project_dir().map(Path::to_path_buf) else {
            return Err("Modules require a project folder".to_string());
        };
        let relative = normalized_module_path(name)?;
        let path = dir.join(&relative);
        if self.sources.iter().any(|source| source.path == path) || path.exists() {
            return Err(format!("Module already exists: {}", relative.display()));
        }
        let source = SourceFile {
            path: path.clone(),
            text: "return {}\n".to_string(),
            dirty: true,
        };
        let payload = SourcePayload {
            path: path.display().to_string(),
            name: relative.display().to_string().replace('\\', "/"),
            text: source.text.clone(),
            dirty: true,
        };
        self.sources.push(source);
        self.needs_compile = true;
        Ok(payload)
    }

    fn close_project(&mut self) {
        self.console.vm.clear_debug_roots();
        self.console.reset_vm();
        self.cart = None;
        self.sources.clear();
        self.run_state = RunState::Stopped;
        self.pause_reason = None;
        self.boot_on_run = false;
        self.needs_compile = false;
        self.frame = 0;
        self.fps = 0.0;
        self.frame_time_ms = 0.0;
        self.diagnostics.clear();
        self.debugger.clear();
        self.asset_snapshot.clear();
        self.removed_banks.clear();
        self.asset_dirty = false;
        self.output.push("Closed project".to_string());
        trim_output(&mut self.output);
    }

    fn audio_transport(
        &mut self,
        kind: &str,
        id: u8,
        action: &str,
        loop_on: Option<bool>,
    ) -> Result<AudioPayload, String> {
        if let Some(loop_on) = loop_on {
            self.console.vm.set_music_loop(loop_on);
        }
        match (kind, action) {
            ("sfx", "play") if id < 16 => self.console.vm.start_sfx(id),
            ("sfx", "stop") => self.console.vm.stop_sfx(),
            ("music", "play") if id < 8 => self.console.vm.start_music(id),
            // `id` is a song-order step here, not a pattern id.
            ("music", "play_song") if (id as usize) < MUSIC_ORDER_STEPS => {
                self.console.vm.start_music_song(id)
            }
            ("music", "stop") => self.console.vm.stop_music(),
            ("sfx", "play") => return Err(format!("SFX id out of range: {id}")),
            ("music", "play") => return Err(format!("Music id out of range: {id}")),
            ("music", "play_song") => return Err(format!("Song step out of range: {id}")),
            _ => return Err(format!("Unknown audio action: {kind}/{action}")),
        }
        Ok(self.audio_payload())
    }

    fn save(&mut self) -> Result<SaveResult, String> {
        let modules = self.modules();
        let entry = self.sources.first().map(|source| source.text.clone());
        let Some(meta) = self.cart.as_mut() else {
            return Err("Nothing to save".to_string());
        };
        if let Some(entry) = entry {
            meta.lua_source = entry;
        }
        // Writes the pristine `asset_snapshot`, not live VM RAM: a cart
        // running mid-play may have mutated map/sprite RAM (collected
        // coins, procedural changes), and none of that gameplay state may
        // reach disk. See review ST-01.
        cart_io::save_pristine(&self.asset_snapshot, meta, &modules, &self.removed_banks)
            .map_err(|error| format!("{error:#}"))?;
        self.removed_banks.clear();
        self.asset_dirty = false;
        for source in &mut self.sources {
            source.dirty = false;
        }
        self.output.push("Saved project".to_string());
        trim_output(&mut self.output);

        // Ctrl+S while the cart is already running hot-reloads it in place
        // (state-preserving) instead of leaving the new code stranded until
        // the next Run/Reset. Best-effort: a reload failure is surfaced via
        // diagnostics/output but must not fail the save itself — the disk
        // write already succeeded, and the previous script keeps running.
        if self.needs_compile && self.run_state != RunState::Stopped {
            let _ = self.hot_reload();
        }

        let output = self
            .sources
            .iter()
            .enumerate()
            .map(|(index, _)| self.source_name(index))
            .collect();
        Ok(SaveResult { output })
    }

    /// Hot-reloads the running script in place, preserving state — see
    /// [`cart::hot_reload_sources_into_vm`]. Mirrors [`StudioCore::compile`]'s
    /// diagnostics/output bookkeeping on success. On failure, unlike
    /// `compile()`, `run_state`/audio are left untouched: the previous script
    /// keeps running exactly as it was, only diagnostics and output change.
    fn hot_reload(&mut self) -> Result<(), String> {
        let project_dir = self.project_dir().map(Path::to_path_buf);
        match cart::hot_reload_sources_into_vm(
            &mut self.console.vm,
            project_dir.as_deref(),
            &self.sources,
            &self.console.input,
            &self.console.font,
        ) {
            Ok(()) => {
                self.diagnostics.clear();
                self.pause_reason = None;
                self.needs_compile = false;
                self.breakable = self.breakable_lines();
                self.collect_vm_output();
                self.output.push("Hot-reloaded".to_string());
                trim_output(&mut self.output);
                Ok(())
            }
            Err(error) => {
                self.collect_vm_output();
                let source = match error.source.as_deref() {
                    Some("cart") | None => self.source_name(0),
                    Some(source) => source.to_string(),
                };
                let detail = match error.line {
                    Some(line) => format!("{source}:{line}: {}", error.message),
                    None => error.message.clone(),
                };
                self.diagnostics = vec![DiagnosticPayload {
                    severity: "error".to_string(),
                    title: "Hot-reload failed".to_string(),
                    detail: detail.clone(),
                    path: source.clone(),
                    line: error.line,
                }];
                self.output.push(format!(
                    "Hot-reload error, previous version still running: {detail}"
                ));
                trim_output(&mut self.output);
                Err(detail)
            }
        }
    }

    fn export(&mut self, path: &Path, minify: bool) -> Result<(), String> {
        let modules = self.modules();
        let entry = self.sources.first().map(|source| source.text.clone());
        let Some(meta) = self.cart.as_mut() else {
            return Err("Nothing to export".to_string());
        };
        if let Some(entry) = entry {
            meta.lua_source = entry;
        }
        cart_io::export_binary(&self.console.vm, meta, path, &modules, minify)
            .map_err(|error| format!("{error:#}"))
    }

    fn export_web(&mut self, path: &Path) -> Result<(), String> {
        let modules = self.modules();
        let entry = self.sources.first().map(|source| source.text.clone());
        let Some(meta) = self.cart.as_mut() else {
            return Err("Nothing to export".to_string());
        };
        if let Some(entry) = entry {
            meta.lua_source = entry;
        }
        cart_io::export_web(&self.console.vm, meta, path, &modules)
            .map_err(|error| format!("{error:#}"))
    }

    fn export_screenshot(&mut self, path: &Path) -> Result<(), String> {
        let modules = self.modules();
        let entry = self.sources.first().map(|source| source.text.clone());
        let Some(meta) = self.cart.as_mut() else {
            return Err("Nothing to export".to_string());
        };
        if let Some(entry) = entry {
            meta.lua_source = entry;
        }
        cart_io::export_screenshot(&self.console.vm, meta, path, &modules)
            .map_err(|error| format!("{error:#}"))
    }

    fn export_source_zip(&mut self, path: &Path) -> Result<(), String> {
        let modules = self.modules();
        let entry = self.sources.first().map(|source| source.text.clone());
        let Some(meta) = self.cart.as_mut() else {
            return Err("Nothing to export".to_string());
        };
        if let Some(entry) = entry {
            meta.lua_source = entry;
        }
        cart_io::export_source_zip(&self.console.vm, meta, path, &modules)
            .map_err(|error| format!("{error:#}"))
    }
}

/// The additional-bank `SectionKind` (id != 0 wrapper) that round-trips a
/// given `AssetBankKind` to disk. Single source of truth shared by
/// `StudioCore::asset_bank`'s primary dispatch and its companion-bank
/// bookkeeping, so the two can't drift apart on which section a bank kind
/// serializes as.
fn section_kind_for_bank(kind: AssetBankKind) -> SectionKind {
    match kind {
        AssetBankKind::Sprites => SectionKind::SpriteBank,
        AssetBankKind::Map => SectionKind::MapBank,
        AssetBankKind::Palette => SectionKind::PaletteBank,
        AssetBankKind::Sfx => SectionKind::SfxBanks,
        AssetBankKind::Music => SectionKind::MusicBanks,
        AssetBankKind::Collision => SectionKind::CollisionBank,
    }
}

fn palette_hex(console: &ConsoleCore) -> Vec<String> {
    console
        .vm
        .get_palette()
        .iter()
        .map(|color| {
            format!(
                "#{:02X}{:02X}{:02X}",
                color.get_r(),
                color.get_g(),
                color.get_b()
            )
        })
        .collect()
}

fn read_region(console: &ConsoleCore, address: usize, len: usize) -> Vec<u8> {
    (0..len)
        .map(|offset| console.vm.peek_memory(address + offset))
        .collect()
}

fn debug_path(path: &Path) -> PathBuf {
    if path.extension().and_then(|value| value.to_str()) == Some("cav") {
        path.with_extension("cav.dbg")
    } else {
        path.join(".caiven.dbg")
    }
}

/// Same sidecar convention as [`debug_path`]: a `.cav` file gets a
/// same-named sibling, a project directory gets a dotfile inside it.
fn save_data_path(path: &Path) -> PathBuf {
    if path.extension().and_then(|value| value.to_str()) == Some("cav") {
        path.with_extension("cav.data")
    } else {
        path.join(".caiven.data")
    }
}

fn normalized_module_path(name: &str) -> Result<PathBuf, String> {
    let trimmed = name.trim().trim_start_matches('/');
    if trimmed.is_empty() {
        return Err("Module name cannot be empty".to_string());
    }
    let mut path = PathBuf::from(trimmed);
    if path.extension().is_none() {
        path.set_extension("lua");
    }
    if path.extension().and_then(|value| value.to_str()) != Some("lua")
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(format!("Invalid Lua module path: {name}"));
    }
    Ok(path)
}

fn trim_output(output: &mut Vec<String>) {
    const MAX_LINES: usize = 200;
    if output.len() > MAX_LINES {
        output.drain(..output.len() - MAX_LINES);
    }
}

fn write_shared_snapshot(studio: &mut StudioCore, snapshot: &Arc<RwLock<SharedSnapshot>>) {
    let mut frame = vec![0; SCREEN_WIDTH as usize * SCREEN_HEIGHT as usize * RGBA_BYTES];
    studio.console.screen.construct(
        &mut frame,
        studio.console.vm.world_pixels(),
        studio.console.vm.ui_pixels(),
    );
    if let Ok(mut shared) = snapshot.write() {
        shared.frame = frame;
        shared.tick = studio.tick_payload();
    }
}

fn handle_command(studio: &mut StudioCore, command: CoreCommand) {
    match command {
        CoreCommand::Bootstrap(reply) => {
            let _ = reply.send(Ok(studio.bootstrap()));
        }
        CoreCommand::CartSize(reply) => {
            let _ = reply.send(Ok(studio.cart_size()));
        }
        CoreCommand::OpenProject { path, reply } => {
            let result = studio
                .open(&path)
                .map(|()| studio.bootstrap())
                .map_err(|error| format!("{error:#}"));
            let _ = reply.send(result);
        }
        CoreCommand::NewProject {
            path,
            template_id,
            reply,
        } => {
            let result = studio
                .new_project(&path, &template_id)
                .map(|()| studio.bootstrap());
            let _ = reply.send(result);
        }
        CoreCommand::RemixExample {
            path,
            example_id,
            reply,
        } => {
            let result = studio
                .remix_example(&path, &example_id)
                .map(|()| studio.bootstrap());
            let _ = reply.send(result);
        }
        CoreCommand::WriteBuffer { path, text, reply } => {
            let result = if let Some(source) = studio
                .sources
                .iter_mut()
                .find(|source| source.path.display().to_string() == path)
            {
                source.text = text;
                source.dirty = true;
                studio.needs_compile = true;
                Ok(())
            } else {
                Err(format!("Unknown source buffer: {path}"))
            };
            let _ = reply.send(result);
        }
        CoreCommand::Save(reply) => {
            let _ = reply.send(studio.save());
        }
        CoreCommand::Export { path, reply } => {
            let _ = reply.send(studio.export(&path, true));
        }
        CoreCommand::ExportWeb { path, reply } => {
            let _ = reply.send(studio.export_web(&path));
        }
        CoreCommand::ExportScreenshot { path, reply } => {
            let _ = reply.send(studio.export_screenshot(&path));
        }
        CoreCommand::ExportSourceZip { path, reply } => {
            let _ = reply.send(studio.export_source_zip(&path));
        }
        CoreCommand::Transport { action, reply } => {
            let _ = reply.send(studio.transport(&action));
        }
        CoreCommand::SetInput {
            button,
            pressed,
            reply,
        } => {
            let result = Button::from_u8(button)
                .ok_or_else(|| format!("Unknown input button: {button}"))
                .map(|button| studio.console.input.set_button(button, pressed));
            let _ = reply.send(result);
        }
        CoreCommand::WriteSprite {
            sprite,
            pixels,
            reply,
        } => {
            let result = if sprite >= 256 {
                Err(format!("Sprite id out of range: {sprite}"))
            } else if pixels.len() != SPRITE_BYTES {
                Err(format!(
                    "Sprite needs {SPRITE_BYTES} pixels, got {}",
                    pixels.len()
                ))
            } else {
                let base = SPRITE_SHEET_RAM_BASE + sprite * SPRITE_BYTES;
                for (offset, value) in pixels.into_iter().enumerate() {
                    studio.console.vm.poke_memory(base + offset, value.min(15));
                }
                studio.mark_asset_edit();
                Ok(())
            };
            let _ = reply.send(result);
        }
        CoreCommand::WritePalette { slot, hex, reply } => {
            let result = parse_hex(&hex).and_then(|(red, green, blue)| {
                if slot >= 16 {
                    return Err(format!("Palette slot out of range: {slot}"));
                }
                studio
                    .console
                    .vm
                    .set_palette_color(slot, Color::new_rgb(red, green, blue));
                for (offset, value) in [red, green, blue].into_iter().enumerate() {
                    studio
                        .console
                        .vm
                        .poke_memory(PALETTE_RAM_BASE + slot * 3 + offset, value);
                }
                studio.mark_asset_edit();
                Ok(())
            });
            let _ = reply.send(result);
        }
        CoreCommand::ToggleBreakpoint {
            source,
            line,
            reply,
        } => {
            let result = if line == 0 {
                Err("Breakpoint line starts at 1".to_string())
            } else if !studio
                .sources
                .iter()
                .enumerate()
                .any(|(index, _)| studio.source_name(index) == source)
            {
                Err(format!("Unknown breakpoint source: {source}"))
            } else {
                studio.debugger.toggle_line_breakpoint(source, line);
                Ok(studio.debugger.breakpoints().to_vec())
            };
            let _ = reply.send(result);
        }
        CoreCommand::AddWatch { expression, reply } => {
            let result = if !caiven_vm::is_watch_expression(&expression) {
                Err("Watch must be a name like player.x or items[1]".to_string())
            } else if studio.debugger.add_watch(expression) {
                Ok(studio.watches())
            } else {
                Err("Watch is empty or already exists".to_string())
            };
            let _ = reply.send(result);
        }
        CoreCommand::RemoveWatch { expression, reply } => {
            let result = if studio.debugger.remove_watch(&expression) {
                Ok(studio.watches())
            } else {
                Err(format!("Unknown watch: {expression}"))
            };
            let _ = reply.send(result);
        }
        CoreCommand::ExpandDebugValue { node_id, reply } => {
            let _ = reply.send(studio.expand_debug_value(&node_id));
        }
        CoreCommand::SelectFrame { index, reply } => {
            let result = studio
                .console
                .vm
                .select_lua_frame(index)
                .map(|()| studio.tick_payload());
            let _ = reply.send(result);
        }
        CoreCommand::PeekValue { expression, reply } => {
            let _ = reply.send(studio.peek_value(&expression));
        }
        CoreCommand::ClearOutput { reply } => {
            studio.output.clear();
            let _ = reply.send(Ok(()));
        }
        CoreCommand::RemoveRecent { path, reply } => {
            let mut list = recent::load();
            let result = recent::remove(&mut list, &path)
                .map_err(|error| format!("Could not update recent carts: {error}"))
                .map(|_| {
                    list.into_iter()
                        .map(|path| path.display().to_string())
                        .collect()
                });
            let _ = reply.send(result);
        }
        CoreCommand::ReadMemory {
            address,
            len,
            reply,
        } => {
            let result = address
                .checked_add(len)
                .filter(|&end| end <= RAM_SIZE)
                .ok_or_else(|| format!("Memory range out of bounds: 0x{address:04X} + {len}"))
                .map(|_| read_region(&studio.console, address, len));
            let _ = reply.send(result);
        }
        CoreCommand::WriteMemory {
            address,
            bytes,
            reply,
        } => {
            let write_len = bytes.len();
            let result = address
                .checked_add(write_len)
                .filter(|&end| end <= RAM_SIZE)
                .ok_or_else(|| {
                    format!(
                        "Memory range out of bounds: 0x{address:04X} + {}",
                        write_len
                    )
                })
                .map(|_| {
                    for (offset, byte) in bytes.into_iter().enumerate() {
                        studio.console.vm.poke_memory(address + offset, byte);
                    }
                    if address < PALETTE_RAM_BASE + PALETTE_SIZE * 3
                        && address + write_len > PALETTE_RAM_BASE
                    {
                        let palette =
                            read_region(&studio.console, PALETTE_RAM_BASE, PALETTE_SIZE * 3);
                        studio.console.vm.set_palette_from_bytes(&palette);
                    }
                    studio.mark_asset_edit();
                });
            let _ = reply.send(result);
        }
        CoreCommand::WriteMapCells { cells, reply } => {
            let result = if let Some(cell) = cells.iter().find(|cell| cell.offset >= MAP_LEN) {
                Err(format!("Map cell out of range: {}", cell.offset))
            } else {
                for cell in cells {
                    studio
                        .console
                        .vm
                        .poke_memory(MAP_RAM_BASE + cell.offset, cell.tile);
                }
                studio.mark_asset_edit();
                Ok(())
            };
            let _ = reply.send(result);
        }
        CoreCommand::WriteCollisionCells { cells, reply } => {
            let result = if let Some(cell) = cells.iter().find(|cell| cell.offset >= COLLISION_LEN)
            {
                Err(format!("Collision cell out of range: {}", cell.offset))
            } else {
                for cell in cells {
                    studio
                        .console
                        .vm
                        .poke_memory(COLLISION_RAM_BASE + cell.offset, cell.value);
                }
                studio.mark_asset_edit();
                Ok(())
            };
            let _ = reply.send(result);
        }
        CoreCommand::ReadCollisionTypes { reply } => {
            let types = studio
                .console
                .vm
                .collision_types()
                .iter()
                .map(CollisionTypePayload::from)
                .collect();
            let _ = reply.send(Ok(types));
        }
        CoreCommand::WriteCollisionTypes { types, reply } => {
            studio
                .console
                .vm
                .set_collision_types(types.into_iter().map(Into::into).collect());
            studio.mark_asset_edit();
            let _ = reply.send(Ok(()));
        }
        CoreCommand::WriteMeta {
            title,
            author,
            meta,
            reply,
        } => {
            let _ = reply.send(studio.write_meta(title, author, meta));
        }
        CoreCommand::CreateModule { name, reply } => {
            let _ = reply.send(studio.create_module(&name));
        }
        CoreCommand::CloseProject(reply) => {
            studio.close_project();
            let _ = reply.send(Ok(studio.bootstrap()));
        }
        CoreCommand::AudioTransport {
            kind,
            id,
            action,
            loop_on,
            reply,
        } => {
            let _ = reply.send(studio.audio_transport(&kind, id, &action, loop_on));
        }
        CoreCommand::AssetIndex(reply) => {
            let _ = reply.send(Ok(studio.asset_index()));
        }
        CoreCommand::AssetBank {
            kind,
            action,
            name,
            reply,
        } => {
            let _ = reply.send(studio.asset_bank(&kind, &action, name));
        }
        CoreCommand::IsDirty(reply) => {
            let _ = reply.send(studio.asset_dirty || studio.sources.iter().any(|s| s.dirty));
        }
        CoreCommand::PreparePublish { minify, reply } => {
            let path = cart::temp_cav_path();
            let project = studio.cart.as_ref().map(|cart| cart.path.clone());
            let result = studio.export(&path, minify).and_then(|()| {
                project
                    .map(|project| (path, project))
                    .ok_or_else(|| "No cart open".to_string())
            });
            let _ = reply.send(result);
        }
    }
}

fn parse_hex(value: &str) -> Result<(u8, u8, u8), String> {
    let value = value
        .strip_prefix('#')
        .ok_or_else(|| format!("Invalid color: {value}"))?;
    if value.len() != 6 {
        return Err(format!("Invalid color: #{value}"));
    }
    let parse = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&value[range], 16).map_err(|_| format!("Invalid color: #{value}"))
    };
    Ok((parse(0..2)?, parse(2..4)?, parse(4..6)?))
}

fn spawn_core(initial_path: Option<PathBuf>) -> StudioBridge {
    let (tx, rx) = mpsc::channel();
    let snapshot = Arc::new(RwLock::new(SharedSnapshot::default()));
    let actor_snapshot = Arc::clone(&snapshot);

    std::thread::Builder::new()
        .name("caiven-studio-core".to_string())
        .spawn(move || {
            let mut studio = match StudioCore::new(initial_path) {
                Ok(studio) => studio,
                Err(error) => {
                    log::error!("failed to start Studio core: {error:#}");
                    return;
                }
            };
            let mut last_snapshot = Instant::now() - Duration::from_secs(1);
            let mut fps_started = Instant::now();
            let mut fps_frames = 0_u32;
            let mut snapshot_stale = true;

            loop {
                match rx.recv_timeout(Duration::from_millis(2)) {
                    Ok(command) => {
                        handle_command(&mut studio, command);
                        snapshot_stale = true;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                while let Ok(command) = rx.try_recv() {
                    handle_command(&mut studio, command);
                    snapshot_stale = true;
                }

                let was_running = studio.run_state == RunState::Running;
                let steps = studio.console.frame_steps();
                for _ in 0..steps {
                    if studio.run_state == RunState::Running {
                        if !studio.run_one_frame(None) {
                            break;
                        }
                        fps_frames += 1;
                    } else {
                        studio.console.vm.tick_audio_players();
                    }
                }
                if studio.console.vm.save_data().is_dirty()
                    && let Some(meta) = studio.cart.as_ref()
                {
                    let path = save_data_path(&meta.path);
                    if std::fs::write(&path, studio.console.vm.save_data().encode()).is_ok() {
                        studio.console.vm.save_data_mut().clear_dirty();
                    }
                }
                if fps_started.elapsed() >= Duration::from_secs(1) {
                    studio.fps = fps_frames as f32 / fps_started.elapsed().as_secs_f32();
                    fps_frames = 0;
                    fps_started = Instant::now();
                }
                snapshot_stale |= was_running || studio.audio_active();
                if snapshot_stale && last_snapshot.elapsed() >= Duration::from_millis(16) {
                    write_shared_snapshot(&mut studio, &actor_snapshot);
                    last_snapshot = Instant::now();
                    snapshot_stale = false;
                }
            }
        })
        .expect("failed to spawn Studio core actor");

    StudioBridge { tx, snapshot }
}

/// Frontend calls this after the user confirms discarding unsaved changes;
/// `destroy` skips the close-requested guard so it cannot loop.
#[tauri::command]
fn studio_force_close(window: tauri::Window) -> Result<(), String> {
    window.destroy().map_err(|error| error.to_string())
}

#[tauri::command]
fn studio_bootstrap(state: State<'_, StudioBridge>) -> Result<BootstrapPayload, String> {
    state.request(CoreCommand::Bootstrap)
}

#[tauri::command]
fn studio_cart_size(state: State<'_, StudioBridge>) -> Result<CartSizePayload, String> {
    state.request(CoreCommand::CartSize)
}

#[tauri::command]
fn studio_open_project(
    path: PathBuf,
    state: State<'_, StudioBridge>,
) -> Result<BootstrapPayload, String> {
    state.request(|reply| CoreCommand::OpenProject { path, reply })
}

#[tauri::command]
fn studio_new_project(
    path: PathBuf,
    template_id: String,
    state: State<'_, StudioBridge>,
) -> Result<BootstrapPayload, String> {
    state.request(|reply| CoreCommand::NewProject {
        path,
        template_id,
        reply,
    })
}

#[tauri::command]
fn studio_list_templates() -> Vec<templates::CartTemplateSummary> {
    templates::summaries()
}

#[tauri::command]
fn studio_remix_example(
    path: PathBuf,
    example_id: String,
    state: State<'_, StudioBridge>,
) -> Result<BootstrapPayload, String> {
    state.request(|reply| CoreCommand::RemixExample {
        path,
        example_id,
        reply,
    })
}

#[tauri::command]
fn studio_list_examples() -> Vec<examples::ExampleSummary> {
    examples::summaries()
}

#[tauri::command]
fn studio_write_buffer(
    path: String,
    text: String,
    state: State<'_, StudioBridge>,
) -> Result<(), String> {
    state.request(|reply| CoreCommand::WriteBuffer { path, text, reply })
}

#[tauri::command]
fn studio_save(state: State<'_, StudioBridge>) -> Result<SaveResult, String> {
    state.request(CoreCommand::Save)
}

#[tauri::command]
fn studio_export(path: PathBuf, state: State<'_, StudioBridge>) -> Result<(), String> {
    state.request(|reply| CoreCommand::Export { path, reply })
}

/// Exports the current project as a single self-contained, offline-playable
/// `.html` (SPEC §I `export-web`) — inlines the `caiven-web` WASM runtime,
/// the packed cart, and the audio worklet; no rebuild, no network at
/// runtime. `path` is a full destination file path chosen by the frontend
/// via `tauri-plugin-dialog`'s save dialog, same trust boundary as
/// `studio_export` (V9 — this is IPC input, not re-validated beyond what
/// `std::fs::write` itself enforces).
#[tauri::command]
fn studio_export_web(path: PathBuf, state: State<'_, StudioBridge>) -> Result<(), String> {
    state.request(|reply| CoreCommand::ExportWeb { path, reply })
}

/// Runs the current project headlessly for a fixed frame count and writes a
/// PNG screenshot to `path`, same trust boundary as `studio_export` (V9).
#[tauri::command]
fn studio_export_screenshot(path: PathBuf, state: State<'_, StudioBridge>) -> Result<(), String> {
    state.request(|reply| CoreCommand::ExportScreenshot { path, reply })
}

/// Zips the current project's `caiven.toml` + Lua source + assets to `path`;
/// errors for binary `.cav` carts, which have no source tree. Same trust
/// boundary as `studio_export` (V9).
#[tauri::command]
fn studio_export_source_zip(path: PathBuf, state: State<'_, StudioBridge>) -> Result<(), String> {
    state.request(|reply| CoreCommand::ExportSourceZip { path, reply })
}

#[tauri::command]
fn studio_transport(action: String, state: State<'_, StudioBridge>) -> Result<TickPayload, String> {
    state.request(|reply| CoreCommand::Transport { action, reply })
}

#[tauri::command]
fn studio_set_input(
    button: u8,
    pressed: bool,
    state: State<'_, StudioBridge>,
) -> Result<(), String> {
    state.request(|reply| CoreCommand::SetInput {
        button,
        pressed,
        reply,
    })
}

#[tauri::command]
fn studio_write_sprite(
    sprite: usize,
    pixels: Vec<u8>,
    state: State<'_, StudioBridge>,
) -> Result<(), String> {
    state.request(|reply| CoreCommand::WriteSprite {
        sprite,
        pixels,
        reply,
    })
}

#[tauri::command]
fn studio_write_palette(
    slot: usize,
    hex: String,
    state: State<'_, StudioBridge>,
) -> Result<(), String> {
    state.request(|reply| CoreCommand::WritePalette { slot, hex, reply })
}

#[tauri::command]
fn studio_toggle_breakpoint(
    source: String,
    line: usize,
    state: State<'_, StudioBridge>,
) -> Result<Vec<Breakpoint>, String> {
    state.request(|reply| CoreCommand::ToggleBreakpoint {
        source,
        line,
        reply,
    })
}

#[tauri::command]
fn studio_add_watch(
    expression: String,
    state: State<'_, StudioBridge>,
) -> Result<Vec<GlobalPayload>, String> {
    state.request(|reply| CoreCommand::AddWatch { expression, reply })
}

#[tauri::command]
fn studio_remove_watch(
    expression: String,
    state: State<'_, StudioBridge>,
) -> Result<Vec<GlobalPayload>, String> {
    state.request(|reply| CoreCommand::RemoveWatch { expression, reply })
}

#[tauri::command]
fn studio_expand_debug_value(
    node_id: String,
    state: State<'_, StudioBridge>,
) -> Result<Vec<DebugChildPayload>, String> {
    state.request(|reply| CoreCommand::ExpandDebugValue { node_id, reply })
}

#[tauri::command]
fn studio_select_frame(
    index: usize,
    state: State<'_, StudioBridge>,
) -> Result<TickPayload, String> {
    state.request(|reply| CoreCommand::SelectFrame { index, reply })
}

#[tauri::command]
fn studio_peek_value(expression: String, state: State<'_, StudioBridge>) -> Result<String, String> {
    state.request(|reply| CoreCommand::PeekValue { expression, reply })
}

#[tauri::command]
fn studio_clear_output(state: State<'_, StudioBridge>) -> Result<(), String> {
    state.request(|reply| CoreCommand::ClearOutput { reply })
}

#[tauri::command]
fn studio_remove_recent(
    path: PathBuf,
    state: State<'_, StudioBridge>,
) -> Result<Vec<String>, String> {
    state.request(|reply| CoreCommand::RemoveRecent { path, reply })
}

#[tauri::command]
fn studio_read_memory(
    address: usize,
    len: usize,
    state: State<'_, StudioBridge>,
) -> Result<Vec<u8>, String> {
    state.request(|reply| CoreCommand::ReadMemory {
        address,
        len,
        reply,
    })
}

#[tauri::command]
fn studio_write_memory(
    address: usize,
    bytes: Vec<u8>,
    state: State<'_, StudioBridge>,
) -> Result<(), String> {
    state.request(|reply| CoreCommand::WriteMemory {
        address,
        bytes,
        reply,
    })
}

#[tauri::command]
fn studio_write_map_cells(
    cells: Vec<MapCellPayload>,
    state: State<'_, StudioBridge>,
) -> Result<(), String> {
    state.request(|reply| CoreCommand::WriteMapCells { cells, reply })
}

#[tauri::command]
fn studio_write_collision_cells(
    cells: Vec<CollisionCellPayload>,
    state: State<'_, StudioBridge>,
) -> Result<(), String> {
    state.request(|reply| CoreCommand::WriteCollisionCells { cells, reply })
}

#[tauri::command]
fn studio_read_collision_types(
    state: State<'_, StudioBridge>,
) -> Result<Vec<CollisionTypePayload>, String> {
    state.request(|reply| CoreCommand::ReadCollisionTypes { reply })
}

#[tauri::command]
fn studio_write_collision_types(
    types: Vec<CollisionTypePayload>,
    state: State<'_, StudioBridge>,
) -> Result<(), String> {
    state.request(|reply| CoreCommand::WriteCollisionTypes { types, reply })
}

#[tauri::command]
fn studio_write_meta(
    title: String,
    author: String,
    meta: MetaPayload,
    state: State<'_, StudioBridge>,
) -> Result<(), String> {
    state.request(|reply| CoreCommand::WriteMeta {
        title,
        author,
        meta,
        reply,
    })
}

#[tauri::command]
fn studio_create_module(
    name: String,
    state: State<'_, StudioBridge>,
) -> Result<SourcePayload, String> {
    state.request(|reply| CoreCommand::CreateModule { name, reply })
}

#[tauri::command]
fn studio_close_project(state: State<'_, StudioBridge>) -> Result<BootstrapPayload, String> {
    state.request(CoreCommand::CloseProject)
}

#[tauri::command]
fn studio_audio_transport(
    kind: String,
    id: u8,
    action: String,
    loop_on: Option<bool>,
    state: State<'_, StudioBridge>,
) -> Result<AudioPayload, String> {
    state.request(|reply| CoreCommand::AudioTransport {
        kind,
        id,
        action,
        loop_on,
        reply,
    })
}

#[tauri::command]
fn studio_asset_index(state: State<'_, StudioBridge>) -> Result<asset_index::AssetIndex, String> {
    state.request(CoreCommand::AssetIndex)
}

#[tauri::command]
fn studio_asset_bank(
    kind: String,
    action: String,
    name: Option<String>,
    state: State<'_, StudioBridge>,
) -> Result<AssetBankPayload, String> {
    state.request(|reply| CoreCommand::AssetBank {
        kind,
        action,
        name,
        reply,
    })
}

#[allow(clippy::too_many_arguments)]
#[tauri::command(async)]
fn studio_port_publish(
    app: tauri::AppHandle,
    state: State<'_, StudioBridge>,
    title: String,
    description: String,
    tags: Vec<String>,
    changelog: String,
    target_cart_id: Option<String>,
    as_new: Option<bool>,
    remixable: Option<bool>,
    frames: u32,
) -> Result<crate::port_api::PublishResult, String> {
    let remixable = remixable.unwrap_or(false);
    let emit = |progress: crate::port_api::PublishProgress| {
        let _ = app.emit("publish:progress", progress);
    };
    emit(crate::port_api::PublishProgress {
        step: "pack".into(),
        pct: 5,
        note: "Packing live buffers".into(),
    });
    // Remixers read this source in the browser.
    let (packed, project) = state.request(|reply| CoreCommand::PreparePublish {
        minify: !remixable,
        reply,
    })?;
    emit(crate::port_api::PublishProgress {
        step: "pack".into(),
        pct: 20,
        note: "Cartridge packed".into(),
    });
    let result = crate::port_api::publish(
        &packed,
        &project,
        as_new.unwrap_or(false),
        crate::port_api::PublishMeta {
            remixable,
            title,
            description,
            tags,
            changelog,
            target_cart_id,
            frames: frames.clamp(1, 600),
        },
        emit,
    );
    let _ = std::fs::remove_file(&packed);
    if let Ok(done) = &result {
        let _ = app.emit("publish:done", done);
    } else if let Err(message) = &result {
        let _ = app.emit("publish:error", serde_json::json!({ "message": message }));
    }
    result
}

#[tauri::command]
fn studio_frame(state: State<'_, StudioBridge>) -> Result<tauri::ipc::Response, String> {
    state
        .snapshot
        .read()
        .map(|snapshot| tauri::ipc::Response::new(snapshot.frame.clone()))
        .map_err(|_| "Framebuffer snapshot poisoned".to_string())
}

#[tauri::command]
fn studio_tick(state: State<'_, StudioBridge>) -> Result<TickPayload, String> {
    state
        .snapshot
        .read()
        .map(|snapshot| snapshot.tick.clone())
        .map_err(|_| "Studio snapshot poisoned".to_string())
}

fn build_menu(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};

    // A custom `.menu()` replaces Tauri's auto-generated default entirely, so
    // the standard macOS App menu (Quit/Hide/Services, Cmd+Q et al.) and the
    // Window menu have to be rebuilt here or those shortcuts silently die.
    #[cfg(target_os = "macos")]
    let app_menu = Submenu::with_items(
        app,
        app.package_info().name.clone(),
        true,
        &[
            &PredefinedMenuItem::about(app, None, Some(tauri::menu::AboutMetadata::default()))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;

    let new_item = MenuItem::with_id(app, "file_new", "New", true, Some("CmdOrCtrl+N"))?;
    let open_item = MenuItem::with_id(app, "file_open", "Open...", true, Some("CmdOrCtrl+O"))?;
    let save_item = MenuItem::with_id(app, "file_save", "Save", true, Some("CmdOrCtrl+S"))?;
    let export_item = MenuItem::with_id(
        app,
        "file_export",
        "Export Cartridge...",
        true,
        None::<&str>,
    )?;
    let export_web_item = MenuItem::with_id(
        app,
        "file_export_web",
        "Export to Web (.html)...",
        true,
        None::<&str>,
    )?;
    let export_screenshot_item = MenuItem::with_id(
        app,
        "file_export_screenshot",
        "Export Screenshot (.png)...",
        true,
        None::<&str>,
    )?;
    let export_source_zip_item = MenuItem::with_id(
        app,
        "file_export_source_zip",
        "Export Source (.zip)...",
        true,
        None::<&str>,
    )?;
    let close_item = MenuItem::with_id(app, "file_close", "Close", true, None::<&str>)?;
    let file_menu = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &new_item,
            &open_item,
            &PredefinedMenuItem::separator(app)?,
            &save_item,
            &export_item,
            &export_web_item,
            &export_screenshot_item,
            &export_source_zip_item,
            &PredefinedMenuItem::separator(app)?,
            &close_item,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;

    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;

    let run_item = MenuItem::with_id(app, "run_toggle", "Run / Pause", true, Some("CmdOrCtrl+R"))?;
    let palette_item = MenuItem::with_id(
        app,
        "command_palette",
        "Command Palette...",
        true,
        Some("CmdOrCtrl+K"),
    )?;
    let view_menu = Submenu::with_items(app, "View", true, &[&run_item, &palette_item])?;

    let window_menu = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            #[cfg(target_os = "macos")]
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;

    #[cfg(target_os = "macos")]
    return Menu::with_items(
        app,
        &[&app_menu, &file_menu, &edit_menu, &view_menu, &window_menu],
    );
    #[cfg(not(target_os = "macos"))]
    Menu::with_items(app, &[&file_menu, &edit_menu, &view_menu, &window_menu])
}

pub fn run(initial_path: Option<PathBuf>) -> anyhow::Result<()> {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init());
    #[cfg(feature = "automation")]
    let builder = builder.plugin(tauri_plugin_webdriver_automation::init());
    builder
        .manage(spawn_core(initial_path))
        .menu(build_menu)
        .on_menu_event(|app, event| {
            use tauri::Emitter;
            let action = match event.id().as_ref() {
                "file_new" => "new",
                "file_open" => "open",
                "file_save" => "save",
                "file_export" => "export",
                "file_export_web" => "export_web",
                "file_export_screenshot" => "export_screenshot",
                "file_export_source_zip" => "export_source_zip",
                "file_close" => "close",
                "run_toggle" => "run_toggle",
                "command_palette" => "palette",
                _ => return,
            };
            let _ = app.emit("menu-action", action);
        })
        .on_window_event(|window, event| {
            use tauri::{Emitter, Manager};
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let (tx, rx) = mpsc::channel();
                let bridge = window.state::<StudioBridge>();
                // Unreachable core counts as dirty: better an extra prompt
                // than a silent loss. Review STU-05.
                let dirty = bridge.tx.send(CoreCommand::IsDirty(tx)).is_ok()
                    && rx.recv_timeout(Duration::from_secs(2)).unwrap_or(true);
                if dirty {
                    api.prevent_close();
                    let _ = window.emit("close-requested", ());
                }
            }
        })
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                if std::env::var_os("CAIVEN_STUDIO_DEVTOOLS").is_some()
                    && let Some(window) = app.get_webview_window("main")
                {
                    window.open_devtools();
                }
            }
            #[cfg(not(debug_assertions))]
            let _ = app;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            studio_bootstrap,
            studio_force_close,
            studio_cart_size,
            studio_open_project,
            studio_new_project,
            studio_list_templates,
            studio_remix_example,
            studio_list_examples,
            studio_write_buffer,
            studio_save,
            studio_export,
            studio_export_web,
            studio_export_screenshot,
            studio_export_source_zip,
            studio_transport,
            studio_set_input,
            studio_write_sprite,
            studio_write_palette,
            studio_toggle_breakpoint,
            studio_add_watch,
            studio_remove_watch,
            studio_expand_debug_value,
            studio_select_frame,
            studio_peek_value,
            studio_clear_output,
            studio_remove_recent,
            studio_read_memory,
            studio_write_memory,
            studio_write_map_cells,
            studio_write_collision_cells,
            studio_read_collision_types,
            studio_write_collision_types,
            studio_write_meta,
            studio_create_module,
            studio_close_project,
            studio_audio_transport,
            studio_asset_index,
            studio_asset_bank,
            studio_port_publish,
            crate::port_api::port_session,
            crate::port_api::port_link_start,
            crate::port_api::port_link_poll,
            crate::port_api::port_link_cancel,
            crate::port_api::port_logout,
            crate::port_api::port_set_url,
            crate::port_api::port_list_carts,
            crate::port_api::port_publish_target,
            crate::port_api::port_download,
            crate::port_api::studio_scan_library,
            studio_frame,
            studio_tick,
        ])
        .run(tauri::generate_context!())
        .map_err(|error| anyhow::anyhow!("Tauri error: {error}"))
}

#[cfg(test)]
mod creator_workflow;

#[cfg(test)]
mod tests {
    use super::{
        Breakpoint, CoreCommand, RunState, SharedSnapshot, StudioCore, debug_path, handle_command,
        normalized_module_path, parse_hex, save_data_path, trim_output, write_shared_snapshot,
    };
    use caiven_cart::DEFAULT_BANK_NAME;
    use caiven_core::memory::{RGBA_BYTES, SCREEN_HEIGHT, SCREEN_WIDTH};
    use caiven_vm::AssetBankKind;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, RwLock, mpsc};

    fn temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "caiven-tauri-app-test-{label}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    /// Drives `handle_command` exactly as the actor thread does, without a
    /// real Tauri runtime: builds a `CoreCommand` around a fresh reply
    /// channel and returns what the handler sent back.
    pub(super) fn dispatch<T>(
        studio: &mut StudioCore,
        build: impl FnOnce(mpsc::Sender<Result<T, String>>) -> CoreCommand,
    ) -> Result<T, String> {
        let (tx, rx) = mpsc::channel();
        handle_command(studio, build(tx));
        rx.try_recv().expect("handler always replies")
    }

    #[test]
    fn shared_frame_buffer_matches_screen_dimensions() {
        // Regression: this buffer was hardcoded to the pre-redesign 128x128
        // screen, so `ImageData` construction in the frontend (which uses
        // the real 192x128 dims) threw on a length mismatch and the console
        // stayed blank on Run.
        let expected_len = SCREEN_WIDTH as usize * SCREEN_HEIGHT as usize * RGBA_BYTES;
        assert_eq!(SharedSnapshot::default().frame.len(), expected_len);

        let dir = temp_dir("shared-frame-buffer");
        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&dir, "blank").expect("new project");
        let snapshot = Arc::new(RwLock::new(SharedSnapshot::default()));
        write_shared_snapshot(&mut studio, &snapshot);
        assert_eq!(snapshot.read().unwrap().frame.len(), expected_len);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn music_started_in_init_plays_after_open_and_after_pause() {
        // Open and Pause used to stop the game's audio, and Run from Paused
        // does not recompile, so `_init()` music came back only after an edit.
        let dir = temp_dir("init-music");
        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&dir, "blank").expect("new project");
        studio.sources[0].text =
            "function _init() play_music(0) end\nfunction _update() end\n".to_string();
        studio.save().expect("save");

        let mut reopened = StudioCore::new(None).expect("studio core");
        reopened.open(&dir).expect("open");
        reopened.transport("run").expect("run");
        // `_init()` runs on the first frame, as in the core loop.
        assert!(reopened.run_one_frame(None));
        assert!(reopened.console.vm.music_player().active, "Run after open");

        reopened.transport("pause").expect("pause");
        assert!(
            !reopened.console.vm.music_player().active,
            "paused is silent"
        );
        reopened.transport("step").expect("step");
        assert!(
            !reopened.console.vm.music_player().active,
            "stepping is silent"
        );
        reopened.transport("run").expect("resume");
        assert!(reopened.console.vm.music_player().active, "Run resumes it");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reset_does_not_carry_the_previous_runs_music() {
        let dir = temp_dir("reset-music");
        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&dir, "blank").expect("new project");
        studio.sources[0].text =
            "function _init() play_music(0) end\nfunction _update() end\n".to_string();
        studio.needs_compile = true;
        studio.transport("run").expect("run");
        assert!(studio.run_one_frame(None));
        assert!(studio.console.vm.music_player().active);

        studio.sources[0].text = "function _init() end\nfunction _update() end\n".to_string();
        studio.transport("reset").expect("reset");
        assert!(!studio.console.vm.music_player().active);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn parses_palette_hex() {
        assert_eq!(parse_hex("#FEB05D"), Ok((254, 176, 93)));
        assert_eq!(parse_hex("#000000"), Ok((0, 0, 0)));
        assert!(parse_hex("FEB05D").is_err());
        assert!(parse_hex("#XYZXYZ").is_err());
    }

    #[test]
    fn save_data_persists_across_reopen_via_disk() {
        let dir = temp_dir("save-data-round-trip");

        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&dir, "blank").expect("new project");
        studio.save().expect("save project to disk");
        studio
            .console
            .vm
            .save_data_mut()
            .set_blob(serde_json::json!({ "level": 9 }))
            .expect("blob within size cap");
        let path = save_data_path(&dir);
        std::fs::write(&path, studio.console.vm.save_data().encode()).expect("write save data");

        let mut studio2 = StudioCore::new(None).expect("studio core");
        studio2.open(&dir).expect("reopen project");
        assert_eq!(
            studio2.console.vm.save_data().blob(),
            &serde_json::json!({ "level": 9 })
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn remix_example_unpacks_into_empty_project_and_opens_it() {
        let dir = std::env::temp_dir().join(format!(
            "caiven-remix-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        let mut studio = StudioCore::new(None).expect("studio core");
        studio
            .remix_example(&dir, "movement")
            .expect("remix example");

        assert!(dir.join("main.lua").exists());
        assert!(std::fs::read_to_string(dir.join("main.lua")).is_ok_and(|s| !s.is_empty()));
        let sprite_bank = studio
            .console
            .vm
            .asset_bank_bytes(AssetBankKind::Sprites, DEFAULT_BANK_NAME)
            .expect("default sprite bank");
        assert!(sprite_bank.iter().any(|&pixel| pixel != 0));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn remix_example_rejects_nonempty_destination() {
        let dir = std::env::temp_dir().join(format!(
            "caiven-remix-nonempty-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("create dir");
        std::fs::write(dir.join("existing.txt"), b"hi").expect("write file");

        let mut studio = StudioCore::new(None).expect("studio core");
        assert!(studio.remix_example(&dir, "movement").is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn remix_example_rejects_unknown_id() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let dir = std::env::temp_dir().join("caiven-remix-unknown-test");
        assert!(studio.remix_example(&dir, "not-an-example").is_err());
    }

    #[test]
    fn a_failed_open_leaves_the_previous_cart_and_vm_untouched() {
        // Review STU-01: `open` used to `reset_vm()` before validating the
        // new cart, so a failed open still left the VM blank even though
        // `self.cart` kept pointing at the previous project — a follow-up
        // Ctrl+S would then write that project's metadata out over blank
        // RAM. Loading into a scratch VM first and only committing on
        // success means a failed open must change nothing.
        let good = temp_dir("stu01-good");
        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&good, "blank").expect("new project");
        let path_before = studio.cart.as_ref().unwrap().path.clone();
        let sprite_before = studio
            .console
            .vm
            .asset_bank_bytes(AssetBankKind::Sprites, DEFAULT_BANK_NAME)
            .unwrap();

        // A directory that looks like a project (has caiven.toml) but whose
        // manifest version this build cannot load — load_cart is guaranteed
        // to fail on it.
        let broken = temp_dir("stu01-broken");
        std::fs::create_dir_all(&broken).expect("create broken dir");
        std::fs::write(
            broken.join("caiven.toml"),
            "[cart]\ntitle = \"Broken\"\nversion = 9999\n",
        )
        .expect("write broken manifest");

        assert!(studio.open(&broken).is_err());

        assert_eq!(studio.cart.as_ref().unwrap().path, path_before);
        assert_eq!(
            studio
                .console
                .vm
                .asset_bank_bytes(AssetBankKind::Sprites, DEFAULT_BANK_NAME)
                .unwrap(),
            sprite_before
        );

        std::fs::remove_dir_all(&good).ok();
        std::fs::remove_dir_all(&broken).ok();
    }

    #[test]
    fn saving_mid_play_never_writes_gameplay_mutated_map_state() {
        // Review ST-01: a running cart's own `set_tile` mutates live map RAM
        // directly, and `save()` used to read that live RAM — so a mid-play
        // Ctrl+S baked whatever gameplay had just done (e.g. clearing a
        // collected coin) into the saved map. `save()` must instead write
        // the pristine `asset_snapshot`, captured before any frame ran.
        let dir = temp_dir("st01-map-mutation");
        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&dir, "blank").expect("new project");
        studio.sources[0].text =
            "function _init() end\nfunction _update() set_tile(0, 0, 5) end\n".to_string();
        studio.needs_compile = true;

        studio.transport("run").expect("run");
        assert!(
            studio.run_one_frame(None),
            "the update frame must not fault"
        );
        assert_eq!(
            studio
                .console
                .vm
                .peek_memory(caiven_core::memory::MAP_RAM_BASE),
            5,
            "gameplay did mutate live map RAM, as expected"
        );

        studio.save().expect("save mid-play");

        let mut reopened = StudioCore::new(None).expect("studio core");
        reopened.open(&dir).expect("reopen saved project");
        assert_eq!(
            reopened
                .console
                .vm
                .peek_memory(caiven_core::memory::MAP_RAM_BASE),
            0,
            "the saved map must not contain the mid-play mutation"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn full_edit_run_mutate_save_reset_reopen_round_trip_preserves_authored_state() {
        // Manual smoke test from the review plan, automated: draw a sprite in
        // the editor, Run, let gameplay mutate the map via set_tile, Ctrl+S
        // mid-play, Reset, close and reopen from disk. The authored sprite
        // must survive and the runtime map mutation must not leak into the
        // save (ST-01), and Reset must return to a fresh, unmutated run.
        let dir = temp_dir("smoke-full-round-trip");
        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&dir, "blank").expect("new project");

        let drawn_sprite: Vec<u8> = (0..caiven_core::memory::SPRITE_BYTES)
            .map(|i| (i % 16) as u8)
            .collect();
        dispatch(&mut studio, |reply| CoreCommand::WriteSprite {
            sprite: 3,
            pixels: drawn_sprite.clone(),
            reply,
        })
        .expect("draw sprite");

        studio.sources[0].text = "function _init() end
function _update() set_tile(0, 0, 5) end
"
        .to_string();
        studio.needs_compile = true;
        studio.transport("run").expect("run");
        assert!(
            studio.run_one_frame(None),
            "the update frame must not fault"
        );
        assert_eq!(
            studio
                .console
                .vm
                .peek_memory(caiven_core::memory::MAP_RAM_BASE),
            5,
            "gameplay did mutate live map RAM, as expected"
        );

        studio.save().expect("save mid-play");

        studio.transport("reset").expect("reset");
        assert_eq!(
            studio
                .console
                .vm
                .peek_memory(caiven_core::memory::MAP_RAM_BASE),
            0,
            "Reset must return to the pristine, unmutated map"
        );
        assert_eq!(
            studio
                .console
                .vm
                .asset_bank_bytes(AssetBankKind::Sprites, DEFAULT_BANK_NAME)
                .map(|bytes| bytes
                    [3 * caiven_core::memory::SPRITE_BYTES..4 * caiven_core::memory::SPRITE_BYTES]
                    .to_vec()),
            Some(drawn_sprite.clone()),
            "Reset must not lose the authored sprite"
        );

        let mut reopened = StudioCore::new(None).expect("studio core");
        reopened.open(&dir).expect("reopen saved project");
        assert_eq!(
            reopened
                .console
                .vm
                .peek_memory(caiven_core::memory::MAP_RAM_BASE),
            0,
            "the saved map must not contain the mid-play mutation"
        );
        assert_eq!(
            reopened
                .console
                .vm
                .asset_bank_bytes(AssetBankKind::Sprites, DEFAULT_BANK_NAME)
                .map(|bytes| bytes
                    [3 * caiven_core::memory::SPRITE_BYTES..4 * caiven_core::memory::SPRITE_BYTES]
                    .to_vec()),
            Some(drawn_sprite),
            "the reopened project must keep the authored sprite"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    // -- normalized_module_path -------------------------------------------

    #[test]
    fn normalized_module_path_adds_lua_extension() {
        let path = normalized_module_path("entities/player").expect("valid module path");
        assert_eq!(path, Path::new("entities/player.lua"));
    }

    #[test]
    fn normalized_module_path_strips_leading_slash() {
        let path = normalized_module_path("/util").expect("valid module path");
        assert_eq!(path, Path::new("util.lua"));
    }

    #[test]
    fn normalized_module_path_rejects_empty() {
        assert!(normalized_module_path("").is_err());
        assert!(normalized_module_path("   ").is_err());
    }

    #[test]
    fn normalized_module_path_rejects_parent_dir_traversal() {
        assert!(normalized_module_path("../../etc/passwd").is_err());
        assert!(normalized_module_path("nested/../../escape").is_err());
    }

    #[test]
    fn normalized_module_path_rejects_non_lua_extension() {
        assert!(normalized_module_path("main.txt").is_err());
    }

    // -- debug_path ---------------------------------------------------------

    #[test]
    fn debug_path_for_cav_file_replaces_extension() {
        let path = debug_path(Path::new("/carts/game.cav"));
        assert_eq!(path, Path::new("/carts/game.cav.dbg"));
    }

    #[test]
    fn debug_path_for_project_dir_appends_dbg_file() {
        let path = debug_path(Path::new("/carts/game"));
        assert_eq!(path, Path::new("/carts/game/.caiven.dbg"));
    }

    // -- trim_output ----------------------------------------------------------

    #[test]
    fn trim_output_keeps_last_200_lines() {
        let mut output: Vec<String> = (0..250).map(|i| i.to_string()).collect();
        trim_output(&mut output);
        assert_eq!(output.len(), 200);
        assert_eq!(output.first(), Some(&"50".to_string()));
        assert_eq!(output.last(), Some(&"249".to_string()));
    }

    #[test]
    fn trim_output_leaves_short_output_untouched() {
        let mut output: Vec<String> = (0..10).map(|i| i.to_string()).collect();
        trim_output(&mut output);
        assert_eq!(output.len(), 10);
    }

    // -- handle_command: memory bounds --------------------------------------

    #[test]
    fn write_memory_rejects_out_of_range_address() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::WriteMemory {
            address: usize::MAX,
            bytes: vec![1, 2, 3],
            reply,
        });
        assert!(result.is_err());
    }

    #[test]
    fn write_memory_accepts_in_range_address() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::WriteMemory {
            address: 0,
            bytes: vec![1, 2, 3],
            reply,
        });
        assert!(result.is_ok());
    }

    #[test]
    fn read_memory_rejects_out_of_range_length() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::ReadMemory {
            address: caiven_core::memory::RAM_SIZE - 1,
            len: 10,
            reply,
        });
        assert!(result.is_err());
    }

    #[test]
    fn read_memory_accepts_in_range_length() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::ReadMemory {
            address: 0,
            len: 4,
            reply,
        });
        assert_eq!(result.map(|bytes| bytes.len()), Ok(4));
    }

    // -- handle_command: sprite/palette validation ---------------------------

    #[test]
    fn write_sprite_rejects_out_of_range_sprite_id() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::WriteSprite {
            sprite: 256,
            pixels: vec![0; caiven_core::memory::SPRITE_BYTES],
            reply,
        });
        assert!(result.is_err());
    }

    #[test]
    fn write_sprite_rejects_wrong_pixel_count() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::WriteSprite {
            sprite: 0,
            pixels: vec![0; 4],
            reply,
        });
        assert!(result.is_err());
    }

    #[test]
    fn write_palette_rejects_invalid_hex() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::WritePalette {
            slot: 0,
            hex: "not-a-color".to_string(),
            reply,
        });
        assert!(result.is_err());
    }

    #[test]
    fn write_palette_rejects_out_of_range_slot() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::WritePalette {
            slot: 16,
            hex: "#FFFFFF".to_string(),
            reply,
        });
        assert!(result.is_err());
    }

    // -- handle_command: breakpoints / input ---------------------------------

    #[test]
    fn core_starts_when_initial_project_is_missing() {
        let missing = std::env::temp_dir().join("caiven-missing-project-does-not-exist");
        let studio = StudioCore::new(Some(missing)).expect("core starts without project");
        assert!(studio.cart.is_none());
        assert!(
            studio
                .output
                .iter()
                .any(|line| line.contains("Could not open"))
        );
    }

    #[test]
    fn toggle_breakpoint_rejects_line_zero() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::ToggleBreakpoint {
            source: "main.lua".to_string(),
            line: 0,
            reply,
        });
        assert!(result.is_err());
    }

    #[test]
    fn toggle_breakpoint_rejects_unknown_source() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::ToggleBreakpoint {
            source: "does-not-exist.lua".to_string(),
            line: 1,
            reply,
        });
        assert!(result.is_err());
    }

    #[test]
    fn set_input_rejects_unknown_button() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::SetInput {
            button: 255,
            pressed: true,
            reply,
        });
        assert!(result.is_err());
    }

    // -- handle_command: map/collision cell bounds ---------------------------

    #[test]
    fn write_map_cells_rejects_out_of_range_offset() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::WriteMapCells {
            cells: vec![super::MapCellPayload {
                offset: caiven_core::memory::MAP_LEN,
                tile: 1,
            }],
            reply,
        });
        assert!(result.is_err());
    }

    #[test]
    fn write_collision_cells_rejects_out_of_range_offset() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::WriteCollisionCells {
            cells: vec![super::CollisionCellPayload {
                offset: caiven_core::memory::COLLISION_LEN,
                value: 1,
            }],
            reply,
        });
        assert!(result.is_err());
    }

    // -- handle_command: audio transport -------------------------------------

    #[test]
    fn audio_transport_rejects_out_of_range_sfx_id() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::AudioTransport {
            kind: "sfx".to_string(),
            id: 16,
            action: "play".to_string(),
            loop_on: None,
            reply,
        });
        assert!(result.is_err());
    }

    #[test]
    fn audio_transport_rejects_out_of_range_music_id() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::AudioTransport {
            kind: "music".to_string(),
            id: 8,
            action: "play".to_string(),
            loop_on: None,
            reply,
        });
        assert!(result.is_err());
    }

    #[test]
    fn audio_transport_rejects_unknown_action() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::AudioTransport {
            kind: "sfx".to_string(),
            id: 0,
            action: "dance".to_string(),
            loop_on: None,
            reply,
        });
        assert!(result.is_err());
    }

    // -- handle_command: asset banks ------------------------------------------

    #[test]
    fn asset_bank_rejects_unknown_kind() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::AssetBank {
            kind: "not-a-kind".to_string(),
            action: "read".to_string(),
            name: None,
            reply,
        });
        assert!(result.is_err());
    }

    #[test]
    fn asset_bank_rejects_unknown_action() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::AssetBank {
            kind: "sprites".to_string(),
            action: "not-an-action".to_string(),
            name: None,
            reply,
        });
        assert!(result.is_err());
    }

    #[test]
    fn asset_bank_select_rejects_missing_id() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::AssetBank {
            kind: "sprites".to_string(),
            action: "select".to_string(),
            name: None,
            reply,
        });
        assert!(result.is_err());
    }

    /// Documents a real inconsistency in `StudioCore::asset_bank`'s "create"
    /// path: it creates the bank in the live VM *before* checking that a
    /// cart is open to track the new section against. With no cart open,
    /// the handler returns `Err("No cart open")` yet the VM is left holding
    /// a bank id 1 that the cart's section list will never know about — a
    /// silent VM/cart-metadata desync (see tauri_app.rs `asset_bank`,
    /// "create" arm). This test pins today's (buggy) behavior so a fix is
    /// visible as a test change, not a silent behavior drift.
    #[test]
    fn asset_bank_create_without_cart_leaves_vm_bank_orphaned() {
        let mut studio = StudioCore::new(None).expect("studio core");
        let result = dispatch(&mut studio, |reply| CoreCommand::AssetBank {
            kind: "sprites".to_string(),
            action: "create".to_string(),
            name: Some("forest".to_string()),
            reply,
        });
        assert!(result.is_err(), "no cart open, so create should fail");
        assert!(
            studio
                .console
                .vm
                .asset_bank_names(AssetBankKind::Sprites)
                .contains(&"forest".to_string()),
            "known bug: the VM bank is created before the cart-open check"
        );
    }

    #[test]
    fn asset_edits_set_dirty_until_saved() {
        // Review STU-04: only code edits used to count as unsaved.
        let dir = temp_dir("stu04-dirty");
        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&dir, "blank").expect("new project");
        assert!(!studio.bootstrap().asset_dirty);

        dispatch(&mut studio, |reply| CoreCommand::WriteSprite {
            sprite: 0,
            pixels: vec![1; caiven_core::memory::SPRITE_BYTES],
            reply,
        })
        .expect("write sprite");
        assert!(studio.bootstrap().asset_dirty);

        studio.save().expect("save");
        assert!(!studio.bootstrap().asset_dirty);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn deleting_a_bank_removes_only_that_banks_files_on_save() {
        // Review CART-04: `save_project` used to sweep the whole project
        // directory for anything matching `{stem}_{name}.png`, deleting a
        // hand-placed same-shaped file (e.g. reference art) that was never
        // one of the cart's own banks. `removed_banks` now names exactly
        // the banks Studio itself deleted this session.
        let dir = temp_dir("cart04-bank-delete");
        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&dir, "blank").expect("new project");

        dispatch(&mut studio, |reply| CoreCommand::AssetBank {
            kind: "sprites".to_string(),
            action: "create".to_string(),
            name: Some("forest".to_string()),
            reply,
        })
        .expect("create bank");
        studio.save().expect("save with new bank");
        assert!(dir.join("sprites_forest.png").is_file());

        // A file that merely looks like a bank but was never loaded/created
        // through Studio — must survive every future save untouched.
        std::fs::write(dir.join("sprites_reference.png"), b"not a real bank")
            .expect("write reference file");

        dispatch(&mut studio, |reply| CoreCommand::AssetBank {
            kind: "sprites".to_string(),
            action: "delete".to_string(),
            name: Some("forest".to_string()),
            reply,
        })
        .expect("delete bank");
        studio.save().expect("save after delete");

        assert!(!dir.join("sprites_forest.png").exists());
        assert!(dir.join("sprites_reference.png").is_file());

        std::fs::remove_dir_all(&dir).ok();
    }

    // -- StudioCore::new_project ----------------------------------------------

    #[test]
    fn new_project_rejects_nonempty_destination() {
        let dir = temp_dir("new-project-nonempty");
        std::fs::create_dir_all(&dir).expect("create dir");
        std::fs::write(dir.join("existing.txt"), b"hi").expect("write file");

        let mut studio = StudioCore::new(None).expect("studio core");
        assert!(studio.new_project(&dir, "blank").is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn new_project_rejects_unknown_template() {
        let dir = temp_dir("new-project-unknown-template");
        let mut studio = StudioCore::new(None).expect("studio core");
        assert!(studio.new_project(&dir, "not-a-template").is_err());
    }

    // -- transport / breakpoint / locals IPC path ------------------------------

    #[test]
    fn step_transport_pauses_at_breakpoint_with_locals() {
        let dir = temp_dir("transport-breakpoint-locals");
        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&dir, "blank").expect("new project");
        studio.sources[0].text =
            "function _init()\nend\n\nfunction _update()\n  local hp = 42\n  clear_screen()\nend\n"
                .to_string();
        studio.needs_compile = true;

        let toggled = dispatch(&mut studio, |reply| CoreCommand::ToggleBreakpoint {
            source: "main.lua".to_string(),
            line: 6,
            reply,
        })
        .expect("toggle breakpoint");
        assert_eq!(
            toggled,
            vec![Breakpoint {
                source: "main.lua".to_string(),
                line: 6
            }]
        );

        let tick = dispatch(&mut studio, |reply| CoreCommand::Transport {
            action: "step".to_string(),
            reply,
        })
        .expect("step transport");

        assert_eq!(tick.run_state, RunState::Paused);
        let pause_reason = tick.pause_reason.expect("paused at breakpoint");
        assert_eq!(pause_reason.kind, "breakpoint");
        assert_eq!(pause_reason.source, Some("main.lua".to_string()));
        assert_eq!(pause_reason.line, Some(6));
        let locals: Vec<(String, String)> = tick
            .locals
            .iter()
            .map(|local| (local.name.clone(), local.value.clone()))
            .collect();
        assert!(
            locals
                .iter()
                .any(|(name, value)| name == "hp" && value == "42"),
            "expected hp=42 in locals, got {locals:?}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn breakpoint_inside_init_stops_the_first_run_after_open() {
        let dir = temp_dir("init-breakpoint");
        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&dir, "blank").expect("new project");
        studio.sources[0].text =
            "function _init()\n  ready = true\nend\nfunction _update() end\n".to_string();
        studio.save().expect("save");

        let mut reopened = StudioCore::new(None).expect("studio core");
        reopened.open(&dir).expect("open");
        reopened
            .debugger
            .toggle_line_breakpoint("main.lua".to_string(), 2);
        reopened.transport("run").expect("run");
        assert!(!reopened.run_one_frame(None), "_init() must stop at line 2");
        let tick = reopened.tick_payload();
        let pause_reason = tick.pause_reason.expect("paused inside _init");
        assert_eq!(pause_reason.line, Some(2));
        assert_eq!(tick.call_stack[0].label, "_init");

        // Run finishes `_init()` instead of skipping it.
        reopened.transport("run").expect("resume");
        assert!(reopened.run_one_frame(None));
        let ready = reopened.console.vm.lua_watch("ready").expect("watch ready");
        assert_eq!(ready.text, "true");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn header_breakpoint_stops_in_the_body_and_run_continues_the_loop() {
        let dir = temp_dir("header-breakpoint");
        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&dir, "blank").expect("new project");
        studio.sources[0].text =
            "total = 0\nfunction _update()\n  local step = 1\n  for i = 1, 2 do\n    total = total + i * step\n  end\nend\n"
                .to_string();
        studio.needs_compile = true;
        studio
            .debugger
            .toggle_line_breakpoint("main.lua".to_string(), 2);
        studio
            .debugger
            .toggle_line_breakpoint("main.lua".to_string(), 5);

        studio.transport("run").expect("run");
        let mut stops = Vec::new();
        while !studio.run_one_frame(None) {
            stops.push(studio.pause_reason.as_ref().and_then(|reason| reason.line));
            studio.transport("run").expect("continue");
        }
        // The header stops at the body's first line; the loop stops each pass.
        assert_eq!(stops, [Some(3), Some(5), Some(5)]);
        let total = studio.console.vm.lua_watch("total").expect("watch total");
        assert_eq!(total.text, "3");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn step_transport_from_stopped_compiles_and_runs_one_frame() {
        let dir = temp_dir("transport-step-from-stopped");
        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&dir, "blank").expect("new project");

        let tick = dispatch(&mut studio, |reply| CoreCommand::Transport {
            action: "step".to_string(),
            reply,
        })
        .expect("step transport");

        assert_eq!(tick.run_state, RunState::Paused);
        assert_eq!(tick.frame, 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn step_transport_reports_runtime_error_pause_reason() {
        let dir = temp_dir("transport-step-runtime-error");
        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&dir, "blank").expect("new project");
        studio.sources[0].text =
            "function _init()\nend\n\nfunction _update()\n  error(\"boom\")\nend\n".to_string();
        studio.needs_compile = true;

        let tick = dispatch(&mut studio, |reply| CoreCommand::Transport {
            action: "step".to_string(),
            reply,
        })
        .expect("step transport");

        assert_eq!(tick.run_state, RunState::Paused);
        let pause_reason = tick.pause_reason.expect("paused on runtime error");
        assert_eq!(pause_reason.kind, "error");
        assert!(pause_reason.message.unwrap_or_default().contains("boom"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn toggle_breakpoint_accepts_known_source_and_line() {
        let dir = temp_dir("toggle-breakpoint-happy-path");
        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&dir, "blank").expect("new project");

        let toggled = dispatch(&mut studio, |reply| CoreCommand::ToggleBreakpoint {
            source: "main.lua".to_string(),
            line: 5,
            reply,
        })
        .expect("toggle on");
        assert_eq!(
            toggled,
            vec![Breakpoint {
                source: "main.lua".to_string(),
                line: 5
            }]
        );

        let toggled_off = dispatch(&mut studio, |reply| CoreCommand::ToggleBreakpoint {
            source: "main.lua".to_string(),
            line: 5,
            reply,
        })
        .expect("toggle off");
        assert!(toggled_off.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn add_and_remove_watch_round_trip() {
        let dir = temp_dir("watch-round-trip");
        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&dir, "blank").expect("new project");

        let added = dispatch(&mut studio, |reply| CoreCommand::AddWatch {
            expression: "player_score".to_string(),
            reply,
        })
        .expect("add watch");
        assert!(added.iter().any(|watch| watch.name == "player_score"));

        let removed = dispatch(&mut studio, |reply| CoreCommand::RemoveWatch {
            expression: "player_score".to_string(),
            reply,
        })
        .expect("remove watch");
        assert!(removed.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn watches_reach_frame_locals_array_items_and_nested_tables() {
        let dir = temp_dir("watch-paths");
        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&dir, "blank").expect("new project");
        studio.sources[0].text = "player = { bag = { gem = { hp = 1 } } }\nlist = { { x = 7 } }\nfunction _update()\n  local b = list[1]\n  player.bag.gem.hp = player.bag.gem.hp + 1\nend\n".to_string();
        studio.needs_compile = true;
        studio
            .debugger
            .toggle_line_breakpoint("main.lua".to_string(), 5);
        for expression in ["player", "b.x", "list[1].x"] {
            dispatch(&mut studio, |reply| CoreCommand::AddWatch {
                expression: expression.to_string(),
                reply,
            })
            .expect("add watch");
        }
        studio.transport("run").expect("run");
        assert!(!studio.run_one_frame(None), "stops at line 5");

        let watches = studio.tick_payload().watches;
        let value = |name: &str| {
            watches
                .iter()
                .find(|watch| watch.name == name)
                .map(|watch| watch.value.clone())
        };
        assert_eq!(value("b.x").as_deref(), Some("7"), "frame local");
        assert_eq!(value("list[1].x").as_deref(), Some("7"), "array item");

        // A child stays expandable across ticks and shows the live value.
        let player = studio.expand_debug_value("watch:player").expect("player");
        let bag = player.iter().find(|child| child.key == "bag").expect("bag");
        let bag_id = bag.node_id.clone().expect("bag is a table");
        studio.tick_payload();
        let gem_id = studio
            .expand_debug_value(&bag_id)
            .expect("bag after a tick")[0]
            .node_id
            .clone()
            .expect("gem is a table");
        studio.transport("run").expect("continue");
        assert!(studio.run_one_frame(None), "frame finishes");
        studio.tick_payload();
        let gem = studio
            .expand_debug_value(&gem_id)
            .expect("gem after a frame");
        assert_eq!(gem[0].value, "2");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn line_steps_select_frames_and_peek_values_while_paused() {
        let dir = temp_dir("line-steps");
        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&dir, "blank").expect("new project");
        studio.sources[0].text = "function twice(n)\n  return n * 2\nend\nfunction _update()\n  local hp = 3\n  hp = twice(hp)\nend\n".to_string();
        studio.needs_compile = true;
        studio
            .debugger
            .toggle_line_breakpoint("main.lua".to_string(), 6);
        assert!(
            studio.peek_value("hp").is_err(),
            "nothing to read while stopped"
        );
        studio.transport("run").expect("run");
        assert!(!studio.run_one_frame(None), "stops at line 6");

        let step = |studio: &mut StudioCore, action: &str| {
            let reason = studio.transport(action).expect(action).pause_reason;
            let reason = reason.expect("paused");
            (reason.kind, reason.line)
        };
        assert_eq!(step(&mut studio, "stepInto"), ("step".to_string(), Some(2)));
        assert_eq!(studio.tick_payload().call_stack[0].label, "twice");
        assert_eq!(studio.peek_value("n"), Ok("3".to_string()));

        let tick = dispatch(&mut studio, |reply| CoreCommand::SelectFrame {
            index: 1,
            reply,
        })
        .expect("select the caller");
        assert_eq!(tick.selected_frame, 1);
        assert!(
            tick.locals
                .iter()
                .any(|local| local.name == "hp" && local.value == "3")
        );
        assert_eq!(studio.peek_value("hp"), Ok("3".to_string()));
        assert_eq!(studio.peek_value("missing"), Ok("nil".to_string()));

        assert_eq!(step(&mut studio, "stepOut"), ("step".to_string(), Some(7)));
        assert_eq!(studio.tick_payload().selected_frame, 0);
        assert_eq!(studio.peek_value("hp"), Ok("6".to_string()));
        // Stepping off the frame's end stops at the next frame's first line.
        assert_eq!(step(&mut studio, "stepOver"), ("step".to_string(), Some(5)));

        std::fs::remove_dir_all(&dir).ok();
    }

    // -- StudioCore::create_module ---------------------------------------------

    #[test]
    fn create_module_requires_project_folder() {
        let mut studio = StudioCore::new(None).expect("studio core");
        assert!(studio.create_module("extra").is_err());
    }

    #[test]
    fn create_module_rejects_duplicate_name() {
        let dir = temp_dir("create-module-duplicate");
        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&dir, "blank").expect("new project");

        studio
            .create_module("extra")
            .expect("first create succeeds");
        let result = studio.create_module("extra");
        assert!(result.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn create_module_rejects_path_traversal_name() {
        let dir = temp_dir("create-module-traversal");
        let mut studio = StudioCore::new(None).expect("studio core");
        studio.new_project(&dir, "blank").expect("new project");

        assert!(studio.create_module("../escape").is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn api_payload_offers_opt_in_modules_before_they_are_required() {
        let studio = StudioCore::new(None).expect("studio core");
        let api = studio.api_payload();
        assert!(api.iter().any(|e| e.name == "tween.new"));
        assert!(api.iter().any(|e| e.name == "lerp"));
    }
}
