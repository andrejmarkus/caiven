use crate::cart_save::{CartMeta, SectionLayout};
use crate::debugger::{DebugClickAction, DebugMode, Debugger};
use crate::editors::{BrowserEditor, CodeEditor, CodeEditorAction, Editor, MapEditor, MetaEditor, MusicEditor, PaletteEditor, SfxEditor, SpriteEditor};
use crate::hot_reload::HotReload;
use crate::tabs;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use fc_rom::{RomHeader, SectionKind};
use fc_vm::default_instruction_set;
use fc_vm::input::{Input, InputMap};
use fc_vm::rendering::font::Font;
use fc_vm::rendering::screen::Screen;
use fc_vm::settings::NAME;
use fc_vm::timing::FixedTimestep;
use fc_vm::vm::audio::{Audio, AudioPeripheral};
use fc_vm::{Vm, VmConfig};
use log::{error, info, warn};
use pixels::{Pixels, SurfaceTexture};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use winit::event::{ElementState, Modifiers, MouseButton, MouseScrollDelta};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    window::{Window, WindowAttributes},
};

const SPRITE_SHEET_RAM_BASE: usize = 0x4000;
const MAP_RAM_BASE: usize = 0x5000;
const PALETTE_RAM_BASE: usize = 0x5800;
const SFX_RAM_BASE: usize = 0x5C00;
const SFX_BANK_LEN: usize = 16 * 64;
const MUSIC_RAM_BASE: usize = 0x6000;
const MUSIC_BANK_LEN: usize = 8 * 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Browser,
    Code,
    Run,
    Sprite,
    Map,
    Sfx,
    Music,
    Palette,
    Meta,
}

#[derive(Parser)]
#[command(name = "fc-engine", about = "Fantasy Console — development environment")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    /// Enable debugger overlay
    #[arg(short, long, global = true)]
    debug: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Assemble source and write a ROM file
    Build {
        /// Path to the .asm source file
        source: PathBuf,
        /// Output .rom path
        output: PathBuf,
    },
    /// Inspect a ROM file and print its section table
    Inspect {
        /// Path to the .rom file
        rom: PathBuf,
    },
    /// Run a .asm or .rom file
    Run {
        /// Path to .asm source (hot reload) or .rom file
        file: PathBuf,
    },
    /// Publish a .rom file to a cart sharing hub
    Publish {
        /// Path to the .rom file
        rom: PathBuf,
        /// Hub base URL
        #[arg(long, env = "FC_HUB_URL", default_value = "http://localhost:8080")]
        url: String,
        /// API key for upload authentication
        #[arg(long, env = "FC_HUB_API_KEY", default_value = "changeme")]
        api_key: String,
        /// Cart title (defaults to ROM header title)
        #[arg(long)]
        title: Option<String>,
        /// Author name (defaults to ROM header author)
        #[arg(long)]
        author: Option<String>,
        /// Short description
        #[arg(long, default_value = "")]
        description: String,
        /// Comma-separated tags
        #[arg(long, default_value = "")]
        tags: String,
        /// Frames to run before capturing screenshot
        #[arg(long, default_value_t = 30)]
        frames: u32,
        /// Skip screenshot capture and upload
        #[arg(long)]
        no_screenshot: bool,
    },
}

pub struct App {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    screen: Screen,
    input: Input,
    input_map: InputMap,
    vm: Vm,
    font: Font,
    config: VmConfig,
    #[allow(dead_code)]
    audio: Option<Audio>,
    debugger: Debugger,
    timing: FixedTimestep,
    last_tick: Instant,
    hot_reload: HotReload,
    mode: AppMode,
    sprite_editor: SpriteEditor,
    map_editor: MapEditor,
    palette_editor: PaletteEditor,
    meta_editor: MetaEditor,
    sfx_editor: SfxEditor,
    music_editor: MusicEditor,
    browser_editor: BrowserEditor,
    code_editor: CodeEditor,
    cart_meta: Option<CartMeta>,
    mouse_x: f64,
    mouse_y: f64,
    mouse_left: bool,
    mouse_right: bool,
    modifiers: Modifiers,
}

impl App {
    fn new() -> Result<Self> {
        let font = Font::from_image(
            "assets/font.png",
            " 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ!?\"'()+-=.:,[]<>",
            3,
            5,
        )
        .context("failed to initialize font")?;

        let config = VmConfig::default();
        let instruction_set = Arc::new(default_instruction_set());
        let mut vm = Vm::new(instruction_set, config);

        let audio = match Audio::new(vm.get_sound_shared()) {
            Ok(a) => Some(a),
            Err(e) => {
                error!("failed to initialize audio: {e}");
                None
            }
        };

        vm.register_peripheral(AudioPeripheral::new(vm.get_sound_shared()));

        info!("fantasy console engine initialized");

        Ok(Self {
            window: None,
            pixels: None,
            screen: Screen::new(config.width, config.height),
            input: Input::new(),
            input_map: InputMap::load("controls.toml"),
            vm,
            font,
            config,
            audio,
            debugger: Debugger::new(false),
            timing: FixedTimestep::new(60),
            last_tick: Instant::now(),
            hot_reload: HotReload::new(),
            mode: AppMode::Run,
            sprite_editor: SpriteEditor::new(),
            map_editor: MapEditor::new(),
            palette_editor: PaletteEditor::new(),
            meta_editor: MetaEditor::new(),
            sfx_editor: SfxEditor::new(),
            music_editor: MusicEditor::new(),
            browser_editor: BrowserEditor::new(),
            code_editor: CodeEditor::new(),
            cart_meta: None,
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_left: false,
            mouse_right: false,
            modifiers: Modifiers::default(),
        })
    }

    fn set_debug_enabled(&mut self, enabled: bool) {
        self.debugger.set_enabled(enabled);
    }

    fn load_rom(&mut self, path: &Path) -> Result<()> {
        let rom = fc_rom::load(path)
            .with_context(|| format!("failed to load ROM from {}", path.display()))?;

        for section in &rom.sections {
            if section.kind == SectionKind::ModManifest {
                let manifest = String::from_utf8_lossy(&section.data);
                let registered = self.vm.registered_peripheral_names();
                for required in manifest.lines().map(str::trim).filter(|s| !s.is_empty()) {
                    if !registered.contains(&required) {
                        anyhow::bail!("ROM requires mod '{}' but it is not loaded", required);
                    }
                }
            }
        }

        self.vm.load_rom(rom.program.clone());
        self.debugger.set_fcdbg_path(path.with_extension("fcdbg"));

        let mut sections: Vec<SectionLayout> = Vec::new();
        for section in &rom.sections {
            match section.kind {
                SectionKind::SpriteSheet => {
                    self.vm.load_section_to_ram(SPRITE_SHEET_RAM_BASE, &section.data);
                    sections.push(SectionLayout {
                        kind: SectionKind::SpriteSheet,
                        ram_base: SPRITE_SHEET_RAM_BASE,
                        len: section.data.len(),
                    });
                    info!(
                        "SpriteSheet loaded to RAM at 0x{:04X} ({} bytes)",
                        SPRITE_SHEET_RAM_BASE,
                        section.data.len()
                    );
                }
                SectionKind::Map => {
                    self.vm.load_section_to_ram(MAP_RAM_BASE, &section.data);
                    sections.push(SectionLayout {
                        kind: SectionKind::Map,
                        ram_base: MAP_RAM_BASE,
                        len: section.data.len(),
                    });
                    info!("Map loaded to RAM at 0x{:04X} ({} bytes)", MAP_RAM_BASE, section.data.len());
                }
                SectionKind::Palette => {
                    self.vm.load_section_to_ram(PALETTE_RAM_BASE, &section.data);
                    self.vm.set_palette_from_bytes(&section.data);
                    sections.push(SectionLayout {
                        kind: SectionKind::Palette,
                        ram_base: PALETTE_RAM_BASE,
                        len: section.data.len(),
                    });
                    info!("Palette loaded to RAM at 0x{:04X} ({} bytes)", PALETTE_RAM_BASE, section.data.len());
                }
                SectionKind::SfxBank => {
                    self.vm.load_section_to_ram(SFX_RAM_BASE, &section.data);
                    sections.push(SectionLayout {
                        kind: SectionKind::SfxBank,
                        ram_base: SFX_RAM_BASE,
                        len: section.data.len(),
                    });
                    info!("SfxBank loaded to RAM at 0x{:04X} ({} bytes)", SFX_RAM_BASE, section.data.len());
                }
                SectionKind::MusicBank => {
                    self.vm.load_section_to_ram(MUSIC_RAM_BASE, &section.data);
                    sections.push(SectionLayout {
                        kind: SectionKind::MusicBank,
                        ram_base: MUSIC_RAM_BASE,
                        len: section.data.len(),
                    });
                    info!("MusicBank loaded to RAM at 0x{:04X} ({} bytes)", MUSIC_RAM_BASE, section.data.len());
                }
                _ => {}
            }
        }

        // If no Palette section was in the ROM, sync VM's default palette to RAM
        if !sections.iter().any(|s| s.kind == SectionKind::Palette) {
            let palette_bytes: Vec<u8> = self
                .vm
                .get_palette()
                .iter()
                .flat_map(|c| [c.get_r(), c.get_g(), c.get_b()])
                .collect();
            self.vm.load_section_to_ram(PALETTE_RAM_BASE, &palette_bytes);
            sections.push(SectionLayout {
                kind: SectionKind::Palette,
                ram_base: PALETTE_RAM_BASE,
                len: palette_bytes.len(),
            });
        }

        // If no Map section was in the ROM, register it so Ctrl+S persists it
        if !sections.iter().any(|s| s.kind == SectionKind::Map) {
            sections.push(SectionLayout {
                kind: SectionKind::Map,
                ram_base: MAP_RAM_BASE,
                len: 64 * 32,
            });
        }

        // If no SfxBank section, register for Ctrl+S persistence
        if !sections.iter().any(|s| s.kind == SectionKind::SfxBank) {
            sections.push(SectionLayout {
                kind: SectionKind::SfxBank,
                ram_base: SFX_RAM_BASE,
                len: SFX_BANK_LEN,
            });
        }

        // If no MusicBank section, register for Ctrl+S persistence
        if !sections.iter().any(|s| s.kind == SectionKind::MusicBank) {
            sections.push(SectionLayout {
                kind: SectionKind::MusicBank,
                ram_base: MUSIC_RAM_BASE,
                len: MUSIC_BANK_LEN,
            });
        }

        self.meta_editor.set_header(
            &rom.header.title,
            &rom.header.author,
            rom.header.entry_point,
            rom.header.flags,
        );

        self.cart_meta = Some(CartMeta {
            path: path.to_path_buf(),
            header: rom.header,
            program: rom.program,
            sections,
        });

        if let Some(dir) = path.parent() {
            self.browser_editor.set_scan_dir(dir.to_path_buf());
        }

        info!("ROM loaded from {}", path.display());
        Ok(())
    }

    fn load_source(&mut self, path: &Path) -> Result<()> {
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read source {}", path.display()))?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext == "fc" {
            let out = fc_lang::compile(&source)
                .map_err(|e| anyhow::anyhow!("compile error in {}: {}", path.display(), e))?;
            self.vm.load_rom_with_source_map(out.program, out.source_map);
            self.vm.set_fc_source(&source);
            self.code_editor.set_source_path(path.to_path_buf());
            info!("fc-lang compiled from {}", path.display());
        } else {
            let out = fc_asm::assemble_with_sections(&source)
                .with_context(|| format!("failed to assemble {}", path.display()))?;
            self.vm.load_rom_with_source_map(out.program, out.source_map);
            for (wire_id, data) in &out.extra_sections {
                if *wire_id == fc_rom::SectionKind::SpriteSheet.to_u16() {
                    self.vm.load_section_to_ram(SPRITE_SHEET_RAM_BASE, data);
                }
            }
            info!("source assembled from {}", path.display());
        }
        self.debugger.set_fcdbg_path(path.with_extension("fcdbg"));
        Ok(())
    }

    fn watch_source(&mut self, path: PathBuf) -> Result<()> {
        self.load_source(&path)?;
        let mtime = path.metadata().ok().and_then(|m| m.modified().ok());
        self.hot_reload.watch(path, mtime);
        Ok(())
    }

    fn poll_hot_reload(&mut self) {
        if let Some(path) = self.hot_reload.poll() {
            info!("hot-reload: {}", path.display());
            if let Err(e) = self.load_source(&path) {
                warn!("hot-reload failed: {e}");
            }
        }
    }

    fn save_cart(&mut self) {
        let Some(meta) = &mut self.cart_meta else {
            warn!("Ctrl+S: no cart loaded");
            return;
        };
        meta.header.title = self.meta_editor.title.clone();
        meta.header.author = self.meta_editor.author.clone();
        match crate::cart_save::save(&self.vm, meta) {
            Ok(()) => info!("cart saved to {}", meta.path.display()),
            Err(e) => error!("cart save failed: {e}"),
        }
    }

    fn logical_mouse_pos(&self) -> (u32, u32) {
        let (pw, ph) = self
            .window
            .as_ref()
            .map(|w| {
                let s = w.inner_size();
                (s.width as f64, s.height as f64)
            })
            .unwrap_or((
                (self.config.width * 4) as f64,
                (self.config.height * 4) as f64,
            ));
        let sx = (self.mouse_x / pw * self.config.width as f64)
            .clamp(0.0, (self.config.width - 1) as f64) as u32;
        let sy = (self.mouse_y / ph * self.config.height as f64)
            .clamp(0.0, (self.config.height - 1) as f64) as u32;
        (sx, sy)
    }

    fn dispatch_editor_click(&mut self, x: u32, y: u32) {
        if let Some(new_mode) = tabs::hit_test(x, y) {
            self.mode = new_mode;
            return;
        }
        let vm = &mut self.vm;
        match self.mode {
            AppMode::Sprite => self.sprite_editor.handle_click(x, y, vm),
            AppMode::Map => self.map_editor.handle_click(x, y, vm),
            AppMode::Palette => self.palette_editor.handle_click(x, y, vm),
            AppMode::Meta => self.meta_editor.handle_click(x, y, vm),
            AppMode::Sfx => self.sfx_editor.handle_click(x, y, vm),
            AppMode::Music => self.music_editor.handle_click(x, y, vm),
            AppMode::Browser => self.browser_editor.handle_click(x, y, vm),
            AppMode::Code => self.code_editor.handle_click(x, y, vm),
            AppMode::Run => {}
        }
    }

    fn dispatch_editor_drag(&mut self, x: u32, y: u32) {
        if tabs::hit_test(x, y).is_some() {
            return;
        }
        let vm = &mut self.vm;
        match self.mode {
            AppMode::Sprite => self.sprite_editor.handle_drag(x, y, vm),
            AppMode::Map => self.map_editor.handle_drag(x, y, vm),
            AppMode::Palette => self.palette_editor.handle_drag(x, y, vm),
            AppMode::Meta => self.meta_editor.handle_drag(x, y, vm),
            AppMode::Sfx => self.sfx_editor.handle_drag(x, y, vm),
            AppMode::Music => self.music_editor.handle_drag(x, y, vm),
            AppMode::Browser => self.browser_editor.handle_drag(x, y, vm),
            AppMode::Code => self.code_editor.handle_drag(x, y, vm),
            AppMode::Run => {}
        }
    }

    fn dispatch_editor_mouse_up(&mut self, x: u32, y: u32) {
        let vm = &mut self.vm;
        match self.mode {
            AppMode::Sprite => self.sprite_editor.handle_mouse_up(x, y, vm),
            AppMode::Map => self.map_editor.handle_mouse_up(x, y, vm),
            AppMode::Palette => self.palette_editor.handle_mouse_up(x, y, vm),
            AppMode::Meta => self.meta_editor.handle_mouse_up(x, y, vm),
            AppMode::Sfx => self.sfx_editor.handle_mouse_up(x, y, vm),
            AppMode::Music => self.music_editor.handle_mouse_up(x, y, vm),
            AppMode::Browser => self.browser_editor.handle_mouse_up(x, y, vm),
            AppMode::Run | AppMode::Code => {}
        }
    }

    fn dispatch_editor_right_click(&mut self, x: u32, y: u32) {
        let vm = &mut self.vm;
        match self.mode {
            AppMode::Sprite => self.sprite_editor.handle_right_click(x, y, vm),
            AppMode::Map => self.map_editor.handle_right_click(x, y, vm),
            _ => {}
        }
    }

    fn dispatch_editor_right_drag(&mut self, x: u32, y: u32) {
        let vm = &mut self.vm;
        match self.mode {
            AppMode::Sprite => self.sprite_editor.handle_right_drag(x, y, vm),
            AppMode::Map => self.map_editor.handle_right_drag(x, y, vm),
            _ => {}
        }
    }

    fn dispatch_editor_scroll(&mut self, dx: f32, dy: f32) {
        let vm = &mut self.vm;
        match self.mode {
            AppMode::Sprite => self.sprite_editor.handle_scroll(dx, dy, vm),
            AppMode::Map => self.map_editor.handle_scroll(dx, dy, vm),
            AppMode::Sfx => self.sfx_editor.handle_scroll(dx, dy, vm),
            AppMode::Music => self.music_editor.handle_scroll(dx, dy, vm),
            AppMode::Browser => self.browser_editor.handle_scroll(dx, dy, vm),
            AppMode::Code => self.code_editor.handle_scroll(dx, dy, vm),
            _ => {}
        }
    }

    fn poll_browser_load(&mut self) {
        if let Some(path) = self.browser_editor.take_pending_load() {
            match self.load_rom(&path) {
                Ok(()) => {
                    self.mode = AppMode::Run;
                    info!("browser: loaded {}", path.display());
                }
                Err(e) => error!("browser: load failed: {e}"),
            }
        }
    }

    fn poll_code_editor_action(&mut self) {
        if let Some(action) = self.code_editor.pending_action.take() {
            self.apply_code_editor_action(action);
        }
    }

    fn apply_code_editor_action(&mut self, action: CodeEditorAction) {
        match action {
            CodeEditorAction::None => {}
            CodeEditorAction::Save => {
                if self.code_editor.save() {
                    info!("code editor: source saved");
                } else {
                    warn!("code editor: save failed (no path?)");
                }
            }
            CodeEditorAction::CompileAndRun => {
                let source = self.code_editor.get_source();
                match fc_lang::compile(&source) {
                    Ok(out) => {
                        self.vm.load_rom_with_source_map(out.program, out.source_map);
                        self.vm.set_fc_source(&source);
                        if let Some(path) = &self.code_editor.source_path {
                            let _ = std::fs::write(path, &source);
                        }
                        self.code_editor.error_msg = None;
                        self.mode = AppMode::Run;
                        info!("code editor: compiled and running");
                    }
                    Err(e) => {
                        let msg = format!("{e}");
                        warn!("code editor: compile error: {msg}");
                        self.code_editor.error_msg = Some(msg);
                    }
                }
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let screen_w = self.config.width * 4;
        let screen_h = self.config.height * 4;
        let window_attrs = WindowAttributes::default()
            .with_title(NAME)
            .with_inner_size(LogicalSize::new(screen_w as f64, screen_h as f64))
            .with_resizable(false);

        let window = match event_loop.create_window(window_attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                error!("failed to create window: {e}");
                event_loop.exit();
                return;
            }
        };

        let size = window.inner_size();
        let surface = SurfaceTexture::new(size.width, size.height, window.clone());
        let pixels = match Pixels::new(self.config.width, self.config.height, surface) {
            Ok(p) => p,
            Err(e) => {
                error!("failed to create pixel buffer: {e}");
                event_loop.exit();
                return;
            }
        };

        self.window = Some(window);
        self.pixels = Some(pixels);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if let Some(pixels) = self.pixels.as_mut() {
                    let _ = pixels.resize_surface(new_size.width, new_size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                self.screen.get_debug_layer().clear();
                let cursor = self.logical_mouse_pos();
                let font = &self.font;
                let vm = &self.vm;
                let debug_layer = self.screen.get_debug_layer();

                match self.mode {
                    AppMode::Sprite => {
                        self.sprite_editor.render(debug_layer, vm, font, cursor);
                    }
                    AppMode::Map => {
                        self.map_editor.render(debug_layer, vm, font, cursor);
                    }
                    AppMode::Palette => {
                        self.palette_editor.render(debug_layer, vm, font, cursor);
                    }
                    AppMode::Meta => {
                        self.meta_editor.render(debug_layer, vm, font, cursor);
                    }
                    AppMode::Sfx => {
                        self.sfx_editor.render(debug_layer, vm, font, cursor);
                    }
                    AppMode::Music => {
                        self.music_editor.render(debug_layer, vm, font, cursor);
                    }
                    AppMode::Code => {
                        self.code_editor.render(debug_layer, vm, font, cursor);
                    }
                    AppMode::Run => {
                        match self.debugger.get_mode() {
                            DebugMode::Paused | DebugMode::Step => {
                                self.debugger.draw_overlay(debug_layer, vm, font);
                            }
                            DebugMode::Running => {
                                self.debugger.draw_status_bar(debug_layer, vm, font);
                            }
                        }
                    }
                    AppMode::Browser => {
                        self.browser_editor.render(debug_layer, vm, font, cursor);
                    }
                }

                // Tab bar always visible
                tabs::draw_tab_bar(self.screen.get_debug_layer(), &self.font, self.mode);

                if let Some(pixels) = self.pixels.as_mut() {
                    self.screen.construct(
                        pixels.frame_mut(),
                        self.vm.world_pixels(),
                        self.vm.ui_pixels(),
                    );
                    let _ = pixels.render();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_x = position.x;
                self.mouse_y = position.y;
                let (sx, sy) = self.logical_mouse_pos();
                if self.mouse_left && self.mode != AppMode::Run {
                    self.dispatch_editor_drag(sx, sy);
                }
                if self.mouse_right && self.mode != AppMode::Run {
                    self.dispatch_editor_right_drag(sx, sy);
                }
                // Debugger timeline drag (Run mode)
                if self.mouse_left && self.mode == AppMode::Run && self.debugger.is_enabled() {
                    if let DebugClickAction::RestoreScrub = self.debugger.handle_click(sx, sy, &self.vm) {
                        if let Some(state) = self.debugger.current_scrub_snapshot() {
                            self.vm.restore(&state);
                        }
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if self.mode != AppMode::Run {
                    let (dx, dy) = match delta {
                        MouseScrollDelta::LineDelta(x, y) => (x, y),
                        MouseScrollDelta::PixelDelta(pos) => (pos.x as f32 / 20.0, pos.y as f32 / 20.0),
                    };
                    self.dispatch_editor_scroll(dx, dy);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                let (sx, sy) = self.logical_mouse_pos();
                match button {
                    MouseButton::Left => {
                        self.mouse_left = pressed;
                        if pressed {
                            // always dispatch so tab bar is clickable in Run mode
                            self.dispatch_editor_click(sx, sy);
                            self.poll_browser_load();
                            self.poll_code_editor_action();
                            // debugger overlay click (Run mode only)
                            if self.mode == AppMode::Run && self.debugger.is_enabled() {
                                let pc = self.vm.get_pc();
                                match self.debugger.handle_click(sx, sy, &self.vm) {
                                    DebugClickAction::TogglePause => self.debugger.toggle_pause(pc),
                                    DebugClickAction::Step => self.debugger.step(),
                                    DebugClickAction::RestoreScrub => {
                                        if let Some(state) = self.debugger.current_scrub_snapshot() {
                                            self.vm.restore(&state);
                                        }
                                    }
                                    DebugClickAction::None => {}
                                }
                            }
                        } else if !pressed && self.mode != AppMode::Run {
                            self.dispatch_editor_mouse_up(sx, sy);
                        }
                    }
                    MouseButton::Right => {
                        self.mouse_right = pressed;
                        if pressed && self.mode != AppMode::Run {
                            self.dispatch_editor_right_click(sx, sy);
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods;
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state.is_pressed();

                if let PhysicalKey::Code(code) = event.physical_key {
                    if let Some(button) = self.input_map.get_button(code) {
                        self.input.set_button(button, pressed);
                    }

                    let ctrl = self.modifiers.state().control_key();

                    if pressed && !event.repeat {
                        match code {
                            KeyCode::KeyS if ctrl => {
                                self.save_cart();
                                return;
                            }
                            // Tab-bar mode switches (F1–F7, F8=browser)
                            KeyCode::F1 => { self.mode = AppMode::Run; return; }
                            KeyCode::F2 => { self.mode = AppMode::Sprite; return; }
                            KeyCode::F3 => { self.mode = AppMode::Map; return; }
                            KeyCode::F4 => { self.mode = AppMode::Sfx; return; }
                            KeyCode::F5 => { self.mode = AppMode::Music; return; }
                            KeyCode::F6 => { self.mode = AppMode::Palette; return; }
                            KeyCode::F7 => { self.mode = AppMode::Meta; return; }
                            KeyCode::F8 => { self.mode = AppMode::Browser; return; }
                            KeyCode::F9 => { self.mode = AppMode::Code; return; }
                            _ => {}
                        }
                    }

                    // Code editor — handle directly with modifier state
                    if self.mode == AppMode::Code && pressed {
                        let shift = self.modifiers.state().shift_key();
                        let action = self.code_editor.handle_key_direct(code, shift, ctrl);
                        self.apply_code_editor_action(action);
                        return;
                    }

                    // Run-mode debugger controls
                    if self.mode == AppMode::Run {
                        let paused = self.debugger.get_mode() == DebugMode::Paused;
                        match code {
                            KeyCode::Space if pressed && !event.repeat => {
                                self.debugger.toggle_pause(self.vm.get_pc());
                            }
                            KeyCode::KeyC if pressed && !event.repeat => {
                                self.debugger.step();
                            }
                            KeyCode::F10 if pressed && !event.repeat && paused => {
                                self.debugger.step();
                            }
                            KeyCode::KeyB if pressed && !event.repeat && paused => {
                                self.debugger.toggle_bp_at_cursor();
                            }
                            KeyCode::ArrowUp if pressed && paused => {
                                self.debugger.cursor_up(&self.vm);
                            }
                            KeyCode::ArrowDown if pressed && paused => {
                                self.debugger.cursor_down(&self.vm);
                            }
                            KeyCode::ArrowLeft if pressed && paused => {
                                self.debugger.scrub_back();
                                if let Some(state) = self.debugger.current_scrub_snapshot() {
                                    self.vm.restore(&state);
                                }
                            }
                            KeyCode::ArrowRight if pressed && paused => {
                                self.debugger.scrub_forward();
                                if let Some(state) = self.debugger.current_scrub_snapshot() {
                                    self.vm.restore(&state);
                                }
                            }
                            KeyCode::KeyN if pressed && !event.repeat => {
                                self.debugger.prev_ram_page();
                            }
                            KeyCode::KeyM if pressed && !event.repeat => {
                                self.debugger.next_ram_page();
                            }
                            _ => {}
                        }
                    } else if pressed {
                        // Delegate key to active editor
                        let vm = &mut self.vm;
                        match self.mode {
                            AppMode::Sprite => self.sprite_editor.handle_key(code, vm),
                            AppMode::Map => self.map_editor.handle_key(code, vm),
                            AppMode::Palette => self.palette_editor.handle_key(code, vm),
                            AppMode::Meta => self.meta_editor.handle_key(code, vm),
                            AppMode::Sfx => self.sfx_editor.handle_key(code, vm),
                            AppMode::Music => self.music_editor.handle_key(code, vm),
                            AppMode::Browser => self.browser_editor.handle_key(code, vm),
                            AppMode::Run | AppMode::Code => {}
                        }
                        self.poll_browser_load();
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.browser_editor.poll_hub();
        self.poll_browser_load();
        self.poll_hot_reload();

        if self.mode == AppMode::Run {
            let now = Instant::now();
            let dt = now.duration_since(self.last_tick);
            self.last_tick = now;

            match self.debugger.get_mode() {
                DebugMode::Running => {
                    let steps = self.timing.tick(dt);
                    for _ in 0..steps {
                        self.vm.run_frame(&self.input, &self.font);
                        self.debugger.push_state(self.vm.snapshot());
                    }
                }
                DebugMode::Step => {
                    self.vm.step(&self.input, &self.font);
                    self.debugger.check_breakpoint(self.vm.get_pc());
                    self.debugger.dump_state(&self.vm);
                    self.debugger.pause(self.vm.get_pc());
                }
                DebugMode::Paused => {}
            }
        } else {
            self.last_tick = Instant::now();
        }

        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

fn build_multipart(boundary: &str, parts: &[(&str, Option<&str>, &str, &[u8])]) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, filename, content_type, data) in parts {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        let cd = match filename {
            Some(fname) => format!("Content-Disposition: form-data; name=\"{name}\"; filename=\"{fname}\"\r\n"),
            None => format!("Content-Disposition: form-data; name=\"{name}\"\r\n"),
        };
        body.extend_from_slice(cd.as_bytes());
        body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
        body.extend_from_slice(data);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

fn capture_screenshot(rom: &fc_rom::Rom, config: VmConfig, frames: u32) -> Result<Vec<u8>> {
    let instruction_set = Arc::new(default_instruction_set());
    let mut vm = Vm::new(instruction_set, config);

    vm.load_rom(rom.program.clone());
    for section in &rom.sections {
        match section.kind {
            SectionKind::SpriteSheet => vm.load_section_to_ram(SPRITE_SHEET_RAM_BASE, &section.data),
            SectionKind::Map => vm.load_section_to_ram(MAP_RAM_BASE, &section.data),
            SectionKind::Palette => {
                vm.load_section_to_ram(PALETTE_RAM_BASE, &section.data);
                vm.set_palette_from_bytes(&section.data);
            }
            SectionKind::SfxBank => vm.load_section_to_ram(SFX_RAM_BASE, &section.data),
            SectionKind::MusicBank => vm.load_section_to_ram(MUSIC_RAM_BASE, &section.data),
            _ => {}
        }
    }

    let font = Font::empty();
    let input = Input::new();
    for _ in 0..frames {
        vm.run_frame(&input, &font);
    }

    let world = vm.world_pixels();
    let ui = vm.ui_pixels();
    let pixel_count = (config.width * config.height) as usize;
    let mut rgba = vec![0u8; pixel_count * 4];
    for i in 0..pixel_count {
        let base = i * 4;
        if ui[base + 3] > 0 {
            rgba[base..base + 4].copy_from_slice(&ui[base..base + 4]);
        } else {
            rgba[base..base + 4].copy_from_slice(&world[base..base + 4]);
        }
    }

    let img = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(config.width, config.height, rgba)
        .context("failed to create image buffer")?;
    let mut png_bytes = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .context("failed to encode screenshot PNG")?;
    Ok(png_bytes)
}

fn publish_cart(
    rom_path: &Path,
    hub_url: &str,
    api_key: &str,
    title: Option<&str>,
    author: Option<&str>,
    description: &str,
    tags: &str,
    frames: u32,
    no_screenshot: bool,
) -> Result<()> {
    let rom = fc_rom::load(rom_path)
        .with_context(|| format!("failed to load ROM from {}", rom_path.display()))?;

    let title = title.unwrap_or(&rom.header.title);
    let author = author.unwrap_or(&rom.header.author);

    let meta_str = serde_json::json!({
        "title": title,
        "author": author,
        "description": description,
        "tags": tags,
    })
    .to_string();

    let rom_bytes = std::fs::read(rom_path)
        .with_context(|| format!("failed to read ROM bytes from {}", rom_path.display()))?;

    let boundary = "----FcHubBoundary7x3k9p";
    let filename = rom_path.file_name().and_then(|n| n.to_str()).unwrap_or("cart.rom");

    let body = build_multipart(
        boundary,
        &[
            ("meta", None, "application/json", meta_str.as_bytes()),
            ("rom", Some(filename), "application/octet-stream", &rom_bytes),
        ],
    );

    let content_type = format!("multipart/form-data; boundary={boundary}");
    let upload_url = format!("{hub_url}/api/carts");

    let response = ureq::post(&upload_url)
        .set("X-Api-Key", api_key)
        .set("Content-Type", &content_type)
        .send_bytes(&body)
        .context("failed to upload cart")?;

    let cart_id: String = {
        let val: serde_json::Value = serde_json::from_reader(response.into_reader())
            .context("failed to parse upload response")?;
        val["id"].as_str().context("upload response missing 'id'")?.to_string()
    };

    println!("published: {hub_url}/api/carts/{cart_id}");

    if !no_screenshot {
        let config = VmConfig::default();
        let png_bytes = capture_screenshot(&rom, config, frames)?;

        let boundary2 = "----FcHubScreenshotBoundary";
        let screenshot_body = build_multipart(
            boundary2,
            &[("screenshot", Some("screenshot.png"), "image/png", &png_bytes)],
        );
        let ct2 = format!("multipart/form-data; boundary={boundary2}");
        let screenshot_url = format!("{hub_url}/api/carts/{cart_id}/screenshot");

        ureq::post(&screenshot_url)
            .set("X-Api-Key", api_key)
            .set("Content-Type", &ct2)
            .send_bytes(&screenshot_body)
            .context("failed to upload screenshot")?;

        println!("screenshot uploaded");
    }

    Ok(())
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    let log_level = if cli.debug { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    let command = cli.command;

    match &command {
        Some(Command::Build { source, output }) => {
            info!("building ROM: {} → {}", source.display(), output.display());

            let out = fc_asm::assemble_file_with_sections(source)
                .map_err(|e| anyhow::anyhow!("assembly failed: {e}"))?;

            let stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let header = RomHeader::default_for(stem);

            let extra: Vec<(SectionKind, Vec<u8>)> = out
                .extra_sections
                .into_iter()
                .map(|(id, data)| (SectionKind::from_u16(id), data))
                .collect();

            fc_rom::write(output, &header, &out.program, &extra)
                .with_context(|| format!("cannot write ROM to {}", output.display()))?;

            info!(
                "ROM written to {} ({} extra sections)",
                output.display(),
                extra.len()
            );
            return Ok(());
        }
        Some(Command::Inspect { rom }) => {
            let loaded = fc_rom::load(rom)
                .with_context(|| format!("failed to load ROM from {}", rom.display()))?;
            println!("ROM: {}", rom.display());
            println!("  title:  {}", loaded.header.title);
            println!("  author: {}", loaded.header.author);
            println!("  program: {} bytes", loaded.program.len());
            println!("  sections ({}):", loaded.sections.len() + 1);
            println!("    [0] Program  {} bytes", loaded.program.len());
            for (i, s) in loaded.sections.iter().enumerate() {
                println!("    [{}] {:?}  {} bytes", i + 1, s.kind, s.data.len());
            }
            return Ok(());
        }
        Some(Command::Publish { rom, url, api_key, title, author, description, tags, frames, no_screenshot }) => {
            publish_cart(rom, url, api_key, title.as_deref(), author.as_deref(), description, tags, *frames, *no_screenshot)?;
            return Ok(());
        }
        _ => {}
    }

    let mut app = App::new()?;
    app.set_debug_enabled(cli.debug);

    match command {
        Some(Command::Run { file }) => {
            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext == "rom" {
                info!("running ROM: {}", file.display());
                app.load_rom(&file)?;
            } else {
                info!("running source: {} (hot-reload active)", file.display());
                app.watch_source(file)?;
            }
        }
        None => {
            info!("no file specified — open a .asm or .rom file with: fc-engine run <file>");
        }
        Some(Command::Build { .. }) | Some(Command::Inspect { .. }) | Some(Command::Publish { .. }) => unreachable!(),
    }

    let event_loop = EventLoop::new().context("failed to create event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).context("event loop error")?;

    Ok(())
}
