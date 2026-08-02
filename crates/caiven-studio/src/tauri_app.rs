//! Tauri shell for Caiven Studio.
//!
//! `mlua::Lua` intentionally stays on one dedicated actor thread. Tauri
//! commands exchange owned messages with that thread and read framebuffer
//! snapshots, so webview workers never own or lock VM internals.

use crate::app::cart_io::{self, CartMeta};
use crate::debugger::{Breakpoint, Debugger};
use crate::studio::{SourceFile, asset_index, cart, examples, recent, templates};
use caiven_cart::{SectionKind, encode_asset_bank};
use caiven_core::Color;
use caiven_core::memory::{
    COLLISION_LEN, COLLISION_RAM_BASE, MAP_LEN, MAP_RAM_BASE, MUSIC_BANK_LEN, MUSIC_RAM_BASE,
    PALETTE_RAM_BASE, PALETTE_SIZE, RAM_SIZE, SFX_BANK_LEN, SFX_RAM_BASE, SPRITE_BYTES,
    SPRITE_SHEET_LEN, SPRITE_SHEET_RAM_BASE,
};
use caiven_vm::input::Button;
use caiven_vm::runtime::ConsoleCore;
use caiven_vm::vm::api_registry;
use caiven_vm::{AssetBankKind, LuaBreakpoint, LuaRunOutcome};
use serde::{Deserialize, Serialize};
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

    fn error(source: String, line: Option<usize>, message: String) -> Self {
        Self {
            kind: "error".to_string(),
            source: Some(source),
            line,
            message: Some(message),
        }
    }

    fn as_breakpoint(&self) -> Option<Breakpoint> {
        (self.kind == "breakpoint").then(|| Breakpoint {
            source: self.source.clone().unwrap_or_default(),
            line: self.line.unwrap_or_default(),
        })
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
    ids: Vec<u8>,
    active: u8,
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
    solid: bool,
}

impl From<&caiven_core::CollisionType> for CollisionTypePayload {
    fn from(t: &caiven_core::CollisionType) -> Self {
        Self {
            id: t.id,
            name: t.name.clone(),
            color: t.color,
            solid: t.flags.is_solid(),
        }
    }
}

impl From<CollisionTypePayload> for caiven_core::CollisionType {
    fn from(p: CollisionTypePayload) -> Self {
        let bits = if p.solid {
            caiven_core::CollisionTypeFlags::SOLID
        } else {
            0
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
    sprite_banks: Vec<u8>,
    map_banks: Vec<u8>,
    active_sprite_bank: u8,
    active_map_bank: u8,
    collision: Vec<u8>,
    collision_types: Vec<CollisionTypePayload>,
    sfx: Vec<u8>,
    music: Vec<u8>,
    palette_banks: Vec<u8>,
    active_palette_bank: u8,
    sfx_banks: Vec<u8>,
    active_sfx_bank: u8,
    music_banks: Vec<u8>,
    active_music_bank: u8,
    ram: Vec<u8>,
    globals: Vec<GlobalPayload>,
    watches: Vec<GlobalPayload>,
    call_stack: Vec<CallFramePayload>,
    breakpoints: Vec<Breakpoint>,
    pause_reason: Option<PauseReasonPayload>,
    diagnostics: Vec<DiagnosticPayload>,
    output: Vec<String>,
    meta: MetaPayload,
    asset_index: asset_index::AssetIndex,
    audio: AudioPayload,
    recent: Vec<String>,
    api: Vec<ApiEntryPayload>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TickPayload {
    run_state: RunState,
    frame: u64,
    fps: f32,
    frame_time_ms: f32,
    globals: Vec<GlobalPayload>,
    watches: Vec<GlobalPayload>,
    call_stack: Vec<CallFramePayload>,
    pause_reason: Option<PauseReasonPayload>,
    audio: AudioPayload,
    diagnostics: Vec<DiagnosticPayload>,
    output: Vec<String>,
    active_sprite_bank: u8,
    active_map_bank: u8,
    active_palette_bank: u8,
    active_sfx_bank: u8,
    active_music_bank: u8,
}

#[derive(Clone)]
struct SharedSnapshot {
    frame: Vec<u8>,
    tick: TickPayload,
}

impl Default for SharedSnapshot {
    fn default() -> Self {
        Self {
            frame: vec![0; 128 * 128 * 4],
            tick: TickPayload {
                run_state: RunState::Stopped,
                frame: 0,
                fps: 0.0,
                frame_time_ms: 0.0,
                globals: Vec::new(),
                watches: Vec::new(),
                call_stack: Vec::new(),
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
                active_sprite_bank: 0,
                active_map_bank: 0,
                active_palette_bank: 0,
                active_sfx_bank: 0,
                active_music_bank: 0,
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
    Save(mpsc::Sender<Result<Vec<String>, String>>),
    Export {
        path: PathBuf,
        reply: mpsc::Sender<Result<(), String>>,
    },
    ExportWeb {
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
        id: Option<u8>,
        reply: mpsc::Sender<Result<AssetBankPayload, String>>,
    },
    PreparePublish(mpsc::Sender<Result<PathBuf, String>>),
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
    suppress_breakpoint_once: Option<Breakpoint>,
    needs_compile: bool,
    diagnostics: Vec<DiagnosticPayload>,
    output: Vec<String>,
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
            suppress_breakpoint_once: None,
            needs_compile: false,
            diagnostics: Vec::new(),
            output: Vec::new(),
        };
        if let Some(path) = initial_path {
            studio.open(&path)?;
        }
        Ok(studio)
    }

    fn open(&mut self, path: &Path) -> anyhow::Result<()> {
        self.console.reset_vm();
        let meta = cart::load_cart(
            &mut self.console.vm,
            path,
            &self.console.input,
            &self.console.font,
        )?;
        self.sources = if caiven_cart::is_project(path) {
            cart::load_project_sources(path)?
        } else {
            meta.lua_source
                .as_ref()
                .map(|text| {
                    vec![SourceFile {
                        path: path.to_path_buf(),
                        text: text.clone(),
                        dirty: false,
                    }]
                })
                .unwrap_or_default()
        };
        self.cart = Some(meta);
        let entry_source = self.source_name(0);
        self.debugger.set_dbg_path(debug_path(path), entry_source);
        self.diagnostics.clear();
        self.output = vec![format!("Opened {}", path.display())];
        self.collect_vm_output();
        self.run_state = RunState::Paused;
        self.pause_reason = Some(PauseReasonPayload::manual());
        self.suppress_breakpoint_once = None;
        self.needs_compile = false;
        self.frame = 0;
        self.fps = 0.0;
        self.frame_time_ms = 0.0;
        self.console.vm.stop_audio();
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
            program: Vec::new(),
            sections: cart::default_section_layout(),
            lua_source: Some(source.to_string()),
        });
        let entry_source = self.source_name(0);
        self.debugger.set_dbg_path(debug_path(path), entry_source);
        self.diagnostics.clear();
        self.output = vec![format!("Created {}", path.display())];
        self.run_state = RunState::Stopped;
        self.pause_reason = None;
        self.suppress_breakpoint_once = None;
        self.needs_compile = true;
        self.frame = 0;
        self.fps = 0.0;
        self.frame_time_ms = 0.0;
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
                self.collect_vm_output();
                self.output.push("Build succeeded".to_string());
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

    fn bootstrap(&self) -> BootstrapPayload {
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
            sprite_banks: self.console.vm.asset_bank_ids(AssetBankKind::Sprites),
            map_banks: self.console.vm.asset_bank_ids(AssetBankKind::Map),
            active_sprite_bank: self.console.vm.active_asset_bank(AssetBankKind::Sprites),
            active_map_bank: self.console.vm.active_asset_bank(AssetBankKind::Map),
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
            palette_banks: self.console.vm.asset_bank_ids(AssetBankKind::Palette),
            active_palette_bank: self.console.vm.active_asset_bank(AssetBankKind::Palette),
            sfx_banks: self.console.vm.asset_bank_ids(AssetBankKind::Sfx),
            active_sfx_bank: self.console.vm.active_asset_bank(AssetBankKind::Sfx),
            music_banks: self.console.vm.asset_bank_ids(AssetBankKind::Music),
            active_music_bank: self.console.vm.active_asset_bank(AssetBankKind::Music),
            ram: read_region(&self.console, 0, RAM_SIZE),
            globals: self.globals(),
            watches: self.watches(),
            call_stack: self.call_stack(),
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
            api: [
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
            .collect(),
        }
    }

    fn cart_size(&self) -> CartSizePayload {
        let packed_bytes = self.cart.as_ref().map_or(0, |meta| {
            let modules = self.modules();
            let entry = self.sources.first().map(|source| source.text.as_str());
            cart_io::packed_size(&self.console.vm, meta, entry, &modules)
        });
        CartSizePayload {
            packed_bytes,
            max_bytes: caiven_cart::MAX_CART_BYTES,
        }
    }

    fn tick_payload(&self) -> TickPayload {
        TickPayload {
            run_state: self.run_state,
            frame: self.frame,
            fps: self.fps,
            frame_time_ms: self.frame_time_ms,
            globals: self.globals(),
            watches: self.watches(),
            call_stack: self.call_stack(),
            pause_reason: self.pause_reason.clone(),
            audio: self.audio_payload(),
            diagnostics: self.diagnostics.clone(),
            output: self.output.clone(),
            active_sprite_bank: self.console.vm.active_asset_bank(AssetBankKind::Sprites),
            active_map_bank: self.console.vm.active_asset_bank(AssetBankKind::Map),
            active_palette_bank: self.console.vm.active_asset_bank(AssetBankKind::Palette),
            active_sfx_bank: self.console.vm.active_asset_bank(AssetBankKind::Sfx),
            active_music_bank: self.console.vm.active_asset_bank(AssetBankKind::Music),
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

    fn globals(&self) -> Vec<GlobalPayload> {
        self.console
            .vm
            .lua_globals()
            .into_iter()
            .map(|(name, value)| GlobalPayload { name, value })
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
        id: Option<u8>,
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
                let id = id.ok_or_else(|| "Bank id required".to_string())?;
                if !self.console.vm.select_asset_bank(bank_kind, id) {
                    return Err(format!("{label} bank {id} does not exist"));
                }
            }
            "create" => {
                let ids = self.console.vm.asset_bank_ids(bank_kind);
                let id = (1..=u8::MAX)
                    .find(|id| !ids.contains(id))
                    .ok_or_else(|| format!("No free {label} bank ids"))?;
                if !self.console.vm.create_asset_bank(bank_kind, id) {
                    return Err(format!("Could not create {label} bank {id}"));
                }
                let meta = self
                    .cart
                    .as_mut()
                    .ok_or_else(|| "No cart open".to_string())?;
                meta.sections.push(crate::app::cart_io::SectionLayout {
                    kind: section_kind,
                    ram_base: 0,
                    len: 1,
                    preserved_data: Some(encode_asset_bank(id, &[])),
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
                        preserved_data: Some(encode_asset_bank(id, &[])),
                    });
                }
            }
            "delete" => {
                let id = id.ok_or_else(|| "Bank id required".to_string())?;
                if !self.console.vm.remove_asset_bank(bank_kind, id) {
                    return Err(format!("Cannot delete {label} bank {id}"));
                }
                if let Some(meta) = self.cart.as_mut() {
                    let companion_section = bank_kind.companion().map(section_kind_for_bank);
                    meta.sections.retain(|section| {
                        let tracked_kind =
                            section.kind == section_kind || Some(section.kind) == companion_section;
                        let matches_id = section
                            .preserved_data
                            .as_deref()
                            .and_then(caiven_cart::decode_asset_bank)
                            .is_some_and(|(bank_id, _)| bank_id == id);
                        !(tracked_kind && matches_id)
                    });
                }
            }
            _ => return Err(format!("Unknown asset bank action: {action}")),
        }
        let active = self.console.vm.active_asset_bank(bank_kind);
        let data = self
            .console
            .vm
            .asset_bank_bytes(bank_kind, active)
            .unwrap_or_default();
        Ok(AssetBankPayload {
            kind: kind.to_string(),
            ids: self.console.vm.asset_bank_ids(bank_kind),
            active,
            data,
        })
    }

    fn watches(&self) -> Vec<GlobalPayload> {
        self.debugger
            .watches()
            .iter()
            .map(|expression| GlobalPayload {
                name: expression.clone(),
                value: self
                    .console
                    .vm
                    .lua_watch(expression)
                    .unwrap_or_else(|error| format!("<{error}>")),
            })
            .collect()
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
                if self.run_state == RunState::Stopped || self.needs_compile {
                    self.compile()?;
                } else {
                    self.suppress_breakpoint_once = self
                        .pause_reason
                        .as_ref()
                        .and_then(PauseReasonPayload::as_breakpoint);
                }
                self.pause_reason = None;
                self.run_state = RunState::Running;
            }
            "pause" => {
                self.run_state = RunState::Paused;
                self.pause_reason = Some(PauseReasonPayload::manual());
                self.suppress_breakpoint_once = None;
                self.console.vm.stop_audio();
            }
            "reset" => {
                self.compile()?;
                self.frame = 0;
                self.pause_reason = None;
                self.suppress_breakpoint_once = None;
                self.run_state = RunState::Running;
            }
            "step" => {
                if self.run_state == RunState::Stopped || self.needs_compile {
                    self.compile()?;
                } else {
                    self.suppress_breakpoint_once = self
                        .pause_reason
                        .as_ref()
                        .and_then(PauseReasonPayload::as_breakpoint);
                }
                self.run_state = RunState::Paused;
                self.pause_reason = None;
                if self.run_one_frame() {
                    self.pause_reason = Some(PauseReasonPayload::manual());
                }
                self.console.vm.stop_audio();
            }
            _ => return Err(format!("Unknown transport action: {action}")),
        }
        Ok(self.tick_payload())
    }

    fn runtime_breakpoints(&mut self) -> Vec<LuaBreakpoint> {
        let entry_source = self.source_name(0);
        let suppressed = self.suppress_breakpoint_once.take();
        self.debugger
            .breakpoints()
            .iter()
            .filter(|breakpoint| suppressed.as_ref() != Some(*breakpoint))
            .map(|breakpoint| {
                LuaBreakpoint::new(
                    if breakpoint.source == entry_source {
                        "cart".to_string()
                    } else {
                        breakpoint.source.clone()
                    },
                    breakpoint.line,
                )
            })
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

    fn run_one_frame(&mut self) -> bool {
        let started = Instant::now();
        let breakpoints = self.runtime_breakpoints();
        let outcome = self.console.run_frame_lua_bp(&breakpoints);
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
                self.console.vm.stop_audio();
                self.output.push(format!(
                    "Paused at {}:{}",
                    breakpoint.source, breakpoint.line
                ));
                trim_output(&mut self.output);
                false
            }
            LuaRunOutcome::Error(location, message) => {
                self.run_state = RunState::Paused;
                self.console.vm.stop_audio();
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

    fn collect_vm_output(&mut self) {
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
        Ok(())
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
        self.console.reset_vm();
        self.cart = None;
        self.sources.clear();
        self.run_state = RunState::Stopped;
        self.pause_reason = None;
        self.suppress_breakpoint_once = None;
        self.needs_compile = false;
        self.frame = 0;
        self.fps = 0.0;
        self.frame_time_ms = 0.0;
        self.diagnostics.clear();
        self.debugger.clear();
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
            ("music", "stop") => self.console.vm.stop_music(),
            ("sfx", "play") => return Err(format!("SFX id out of range: {id}")),
            ("music", "play") => return Err(format!("Music id out of range: {id}")),
            _ => return Err(format!("Unknown audio action: {kind}/{action}")),
        }
        Ok(self.audio_payload())
    }

    fn save(&mut self) -> Result<Vec<String>, String> {
        let modules = self.modules();
        let entry = self.sources.first().map(|source| source.text.clone());
        let Some(meta) = self.cart.as_mut() else {
            return Err("Nothing to save".to_string());
        };
        if let Some(entry) = entry {
            meta.lua_source = Some(entry);
        }
        cart_io::save(&self.console.vm, meta, &modules).map_err(|error| format!("{error:#}"))?;
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

        Ok(self
            .sources
            .iter()
            .enumerate()
            .map(|(index, _)| self.source_name(index))
            .collect())
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

    fn export(&mut self, path: &Path) -> Result<(), String> {
        let modules = self.modules();
        let entry = self.sources.first().map(|source| source.text.clone());
        let Some(meta) = self.cart.as_mut() else {
            return Err("Nothing to export".to_string());
        };
        if let Some(entry) = entry {
            meta.lua_source = Some(entry);
        }
        cart_io::export_binary(&self.console.vm, meta, path, &modules)
            .map_err(|error| format!("{error:#}"))
    }

    fn export_web(&mut self, path: &Path) -> Result<(), String> {
        let modules = self.modules();
        let entry = self.sources.first().map(|source| source.text.clone());
        let Some(meta) = self.cart.as_mut() else {
            return Err("Nothing to export".to_string());
        };
        if let Some(entry) = entry {
            meta.lua_source = Some(entry);
        }
        cart_io::export_web(&self.console.vm, meta, path, &modules)
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

fn write_shared_snapshot(studio: &StudioCore, snapshot: &Arc<RwLock<SharedSnapshot>>) {
    let mut frame = vec![0; 128 * 128 * 4];
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
            let _ = reply.send(studio.export(&path));
        }
        CoreCommand::ExportWeb { path, reply } => {
            let _ = reply.send(studio.export_web(&path));
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
            let result = if !valid_watch_expression(&expression) {
                Err("Watch must be a dotted identifier".to_string())
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
            id,
            reply,
        } => {
            let _ = reply.send(studio.asset_bank(&kind, &action, id));
        }
        CoreCommand::PreparePublish(reply) => {
            let path = cart::temp_cav_path();
            let result = studio.export(&path).map(|()| path);
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

fn valid_watch_expression(expression: &str) -> bool {
    let expression = expression.trim();
    !expression.is_empty()
        && expression.split('.').all(|part| {
            !part.is_empty()
                && !part.as_bytes()[0].is_ascii_digit()
                && part
                    .chars()
                    .all(|character| character == '_' || character.is_ascii_alphanumeric())
        })
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

            loop {
                match rx.recv_timeout(Duration::from_millis(2)) {
                    Ok(command) => handle_command(&mut studio, command),
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                while let Ok(command) = rx.try_recv() {
                    handle_command(&mut studio, command);
                }

                let steps = studio.console.frame_steps();
                for _ in 0..steps {
                    if studio.run_state == RunState::Running {
                        if !studio.run_one_frame() {
                            break;
                        }
                        fps_frames += 1;
                    } else {
                        studio.console.vm.tick_audio_players();
                    }
                }
                if fps_started.elapsed() >= Duration::from_secs(1) {
                    studio.fps = fps_frames as f32 / fps_started.elapsed().as_secs_f32();
                    fps_frames = 0;
                    fps_started = Instant::now();
                }
                if last_snapshot.elapsed() >= Duration::from_millis(16) {
                    write_shared_snapshot(&studio, &actor_snapshot);
                    last_snapshot = Instant::now();
                }
            }
        })
        .expect("failed to spawn Studio core actor");

    StudioBridge { tx, snapshot }
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
fn studio_save(state: State<'_, StudioBridge>) -> Result<Vec<String>, String> {
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
    id: Option<u8>,
    state: State<'_, StudioBridge>,
) -> Result<AssetBankPayload, String> {
    state.request(|reply| CoreCommand::AssetBank {
        kind,
        action,
        id,
        reply,
    })
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
fn studio_port_publish(
    app: tauri::AppHandle,
    state: State<'_, StudioBridge>,
    title: String,
    description: String,
    tags: Vec<String>,
    changelog: String,
    target_cart_id: Option<String>,
    frames: u32,
) -> Result<crate::port_api::PublishResult, String> {
    let emit = |progress: crate::port_api::PublishProgress| {
        let _ = app.emit("publish:progress", progress);
    };
    emit(crate::port_api::PublishProgress {
        step: "pack".into(),
        pct: 5,
        note: "Packing live buffers".into(),
    });
    let packed = state.request(CoreCommand::PreparePublish)?;
    emit(crate::port_api::PublishProgress {
        step: "pack".into(),
        pct: 20,
        note: "Cartridge packed".into(),
    });
    let result = crate::port_api::publish(
        &packed,
        crate::port_api::PublishMeta {
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
    use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};

    // A custom `.menu()` replaces Tauri's auto-generated default entirely, so
    // the standard macOS App menu (Quit/Hide/Services, Cmd+Q et al.) and the
    // Window menu have to be rebuilt here or those shortcuts silently die.
    #[cfg(target_os = "macos")]
    let app_menu = Submenu::with_items(
        app,
        app.package_info().name.clone(),
        true,
        &[
            &PredefinedMenuItem::about(app, None, Some(AboutMetadata::default()))?,
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
    #[cfg(debug_assertions)]
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
                "file_close" => "close",
                "run_toggle" => "run_toggle",
                "command_palette" => "palette",
                _ => return,
            };
            let _ = app.emit("menu-action", action);
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
            studio_transport,
            studio_set_input,
            studio_write_sprite,
            studio_write_palette,
            studio_toggle_breakpoint,
            studio_add_watch,
            studio_remove_watch,
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
            crate::port_api::port_download,
            crate::port_api::studio_scan_library,
            studio_frame,
            studio_tick,
        ])
        .run(tauri::generate_context!())
        .map_err(|error| anyhow::anyhow!("Tauri error: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{
        CoreCommand, StudioCore, debug_path, handle_command, normalized_module_path, parse_hex,
        trim_output, valid_watch_expression,
    };
    use caiven_vm::AssetBankKind;
    use std::path::{Path, PathBuf};
    use std::sync::mpsc;

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
    fn dispatch<T>(
        studio: &mut StudioCore,
        build: impl FnOnce(mpsc::Sender<Result<T, String>>) -> CoreCommand,
    ) -> Result<T, String> {
        let (tx, rx) = mpsc::channel();
        handle_command(studio, build(tx));
        rx.try_recv().expect("handler always replies")
    }

    #[test]
    fn parses_palette_hex() {
        assert_eq!(parse_hex("#FEB05D"), Ok((254, 176, 93)));
        assert_eq!(parse_hex("#000000"), Ok((0, 0, 0)));
        assert!(parse_hex("FEB05D").is_err());
        assert!(parse_hex("#XYZXYZ").is_err());
    }

    #[test]
    fn validates_safe_watch_paths() {
        assert!(valid_watch_expression("player.x"));
        assert!(valid_watch_expression("_state.enemy_2.hp"));
        assert!(!valid_watch_expression("player.x + 1"));
        assert!(!valid_watch_expression("player..x"));
        assert!(!valid_watch_expression("2player.x"));
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
            .asset_bank_bytes(AssetBankKind::Sprites, 0)
            .expect("sprite bank 0");
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
            id: None,
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
            id: None,
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
            id: None,
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
            id: None,
            reply,
        });
        assert!(result.is_err(), "no cart open, so create should fail");
        assert!(
            studio
                .console
                .vm
                .asset_bank_ids(AssetBankKind::Sprites)
                .contains(&1),
            "known bug: the VM bank is created before the cart-open check"
        );
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
}
