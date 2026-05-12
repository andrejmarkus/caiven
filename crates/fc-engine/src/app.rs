use crate::cart_save::{CartMeta, SectionLayout};
use crate::debugger::{DebugMode, Debugger};
use crate::editors::SpriteEditor;
use crate::hot_reload::HotReload;
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
use winit::event::{ElementState, Modifiers, MouseButton};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    window::{Window, WindowAttributes},
};

const SPRITE_SHEET_RAM_BASE: usize = 0x4000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppMode {
    Run,
    SpriteEditor,
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
    cart_meta: Option<CartMeta>,
    mouse_x: f64,
    mouse_y: f64,
    mouse_left: bool,
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
            cart_meta: None,
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_left: false,
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
            if section.kind == SectionKind::SpriteSheet {
                self.vm
                    .load_section_to_ram(SPRITE_SHEET_RAM_BASE, &section.data);
                sections.push(SectionLayout {
                    kind: SectionKind::SpriteSheet,
                    ram_base: SPRITE_SHEET_RAM_BASE,
                    len: section.data.len(),
                });
                info!(
                    "SpriteSheet section loaded to RAM at 0x{:04X} ({} bytes)",
                    SPRITE_SHEET_RAM_BASE,
                    section.data.len()
                );
            }
        }

        self.cart_meta = Some(CartMeta {
            path: path.to_path_buf(),
            header: rom.header,
            program: rom.program,
            sections,
        });

        info!("ROM loaded from {}", path.display());
        Ok(())
    }

    fn load_source(&mut self, path: &Path) -> Result<()> {
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read source {}", path.display()))?;
        let out = fc_asm::assemble_with_sections(&source)
            .with_context(|| format!("failed to assemble {}", path.display()))?;
        self.vm.load_rom_with_source_map(out.program, out.source_map);
        for (wire_id, data) in &out.extra_sections {
            if *wire_id == fc_rom::SectionKind::SpriteSheet.to_u16() {
                self.vm.load_section_to_ram(SPRITE_SHEET_RAM_BASE, data);
            }
        }
        self.debugger.set_fcdbg_path(path.with_extension("fcdbg"));
        info!("source assembled from {}", path.display());
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
        let Some(meta) = &self.cart_meta else {
            warn!("Ctrl+S: no cart loaded");
            return;
        };
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
                match self.mode {
                    AppMode::SpriteEditor => {
                        let cursor = self.logical_mouse_pos();
                        self.sprite_editor
                            .render(self.screen.get_debug_layer(), &self.vm, &self.font, cursor);
                    }
                    AppMode::Run => {
                        if self.debugger.get_mode() == DebugMode::Paused {
                            self.debugger.draw_overlay(
                                self.screen.get_debug_layer(),
                                &self.vm,
                                &self.font,
                            );
                        }
                    }
                }
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
                if self.mouse_left && self.mode == AppMode::SpriteEditor {
                    let (sx, sy) = self.logical_mouse_pos();
                    self.sprite_editor.handle_click(sx, sy, &mut self.vm);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    self.mouse_left = state == ElementState::Pressed;
                    if self.mouse_left && self.mode == AppMode::SpriteEditor {
                        let (sx, sy) = self.logical_mouse_pos();
                        self.sprite_editor.handle_click(sx, sy, &mut self.vm);
                    }
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
                    let paused = self.debugger.get_mode() == DebugMode::Paused;
                    let ctrl = self.modifiers.state().control_key();
                    match code {
                        KeyCode::KeyS if pressed && !event.repeat && ctrl => {
                            self.save_cart();
                        }
                        KeyCode::F1 if pressed && !event.repeat => {
                            self.mode = AppMode::Run;
                        }
                        KeyCode::F2 if pressed && !event.repeat => {
                            self.mode = AppMode::SpriteEditor;
                        }
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
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
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
        Some(Command::Build { .. }) | Some(Command::Inspect { .. }) => unreachable!(),
    }

    let event_loop = EventLoop::new().context("failed to create event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).context("event loop error")?;

    Ok(())
}
