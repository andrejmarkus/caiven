use crate::debugger::{DebugMode, Debugger};
use crate::input::{Input, InputMap};
use crate::isa::default_instruction_set;
use crate::rendering::font::Font;
use crate::rendering::screen::Screen;
use crate::settings::NAME;
use crate::timing::FixedTimestep;
use crate::vm::audio::{Audio, AudioPeripheral};
use crate::vm::{Vm, VmConfig};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use fc_rom::{RomHeader, SectionKind};
use log::{error, info, warn};
use pixels::{Pixels, SurfaceTexture};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    window::{Window, WindowAttributes},
};

const SPRITE_SHEET_RAM_BASE: usize = 0x4000;

#[derive(Parser)]
#[command(name = "fc-host", about = "Fantasy Console emulator and toolchain")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run a compiled ROM file
    Run {
        /// Path to the .rom file
        rom: PathBuf,
    },
    /// Assemble source and write a ROM file
    Build {
        /// Path to the .asm source file
        source: PathBuf,
        /// Output .rom path
        output: PathBuf,
    },
    /// Run source file with debugger enabled
    Debug {
        /// Path to the .asm source file (assembled in memory)
        source: PathBuf,
    },
    /// Inspect a ROM file and print its section table
    Inspect {
        /// Path to the .rom file
        rom: PathBuf,
    },
    /// Development mode: load games/asm/movement.asm
    Dev,
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
    watch_path: Option<PathBuf>,
    watch_mtime: Option<SystemTime>,
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

        info!("fantasy console initialized");

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
            watch_path: None,
            watch_mtime: None,
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
                        anyhow::bail!(
                            "ROM requires mod '{}' but it is not loaded",
                            required
                        );
                    }
                }
            }
        }

        self.vm.load_rom(rom.program);

        for section in &rom.sections {
            if section.kind == SectionKind::SpriteSheet {
                self.vm.load_section_to_ram(SPRITE_SHEET_RAM_BASE, &section.data);
                info!("SpriteSheet section loaded to RAM at 0x{:04X} ({} bytes)", SPRITE_SHEET_RAM_BASE, section.data.len());
            }
        }
        info!("ROM loaded from {}", path.display());
        Ok(())
    }

    fn load_source(&mut self, path: &Path) -> Result<()> {
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read source {}", path.display()))?;
        self.vm
            .load_program(&source)
            .with_context(|| format!("failed to assemble {}", path.display()))?;
        self.watch_mtime = path.metadata().ok().and_then(|m| m.modified().ok());
        info!("source assembled from {}", path.display());
        Ok(())
    }

    fn watch_source(&mut self, path: PathBuf) -> Result<()> {
        self.load_source(&path.clone())?;
        self.watch_path = Some(path);
        Ok(())
    }

    fn poll_hot_reload(&mut self) {
        let Some(path) = self.watch_path.clone() else { return };
        let Ok(meta) = path.metadata() else { return };
        let Ok(mtime) = meta.modified() else { return };
        if Some(mtime) != self.watch_mtime {
            info!("hot-reload: {}", path.display());
            if let Err(e) = self.load_source(&path) {
                warn!("hot-reload failed: {e}");
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
                if self.debugger.get_mode() == DebugMode::Paused {
                    self.debugger
                        .draw_overlay(self.screen.get_debug_layer(), &self.vm, &self.font);
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
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state.is_pressed();

                if let PhysicalKey::Code(code) = event.physical_key {
                    if let Some(button) = self.input_map.get_button(code) {
                        self.input.set_button(button, pressed);
                    }
                    match code {
                        KeyCode::Space if pressed && !event.repeat => {
                            self.debugger.toggle_pause();
                        }
                        KeyCode::KeyC if pressed && !event.repeat => {
                            self.debugger.step();
                        }
                        KeyCode::KeyB => {
                            if pressed
                                && !event.repeat
                                && let Some(state) = self.debugger.pop_state()
                            {
                                self.vm.restore(&state);
                                self.debugger.pause();
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
                self.debugger.pause();
            }
            DebugMode::Paused => {}
        }

        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    let command = cli.command.unwrap_or(Command::Dev);

    let log_level = match &command {
        Command::Debug { .. } => "debug",
        _ => "info",
    };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    match &command {
        Command::Build { source, output } => {
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

            info!("ROM written to {} ({} extra sections)", output.display(), extra.len());
            return Ok(());
        }
        Command::Inspect { rom } => {
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

    match command {
        Command::Dev => {
            info!("development mode (hot-reload active)");
            let path = PathBuf::from("games/asm/movement.asm");
            if path.exists() {
                app.watch_source(path)?;
            }
        }
        Command::Debug { source } => {
            info!("debug mode: {}", source.display());
            app.set_debug_enabled(true);
            app.load_source(&source)?;
        }
        Command::Run { rom } => {
            info!("running ROM: {}", rom.display());
            app.load_rom(&rom)?;
        }
        Command::Build { .. } | Command::Inspect { .. } => unreachable!(),
    }

    let event_loop = EventLoop::new().context("failed to create event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).context("event loop error")?;

    Ok(())
}
