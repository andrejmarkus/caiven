use crate::debugger::{DebugMode, Debugger};
use crate::input::Input;
use crate::isa::default_instruction_set;
use crate::rendering::font::Font;
use crate::rendering::screen::Screen;
use crate::settings::{HEIGHT, NAME, SCREEN_HEIGHT, SCREEN_WIDTH, WIDTH};
use crate::timing::FixedTimestep;
use crate::vm::Vm;
use crate::vm::audio::Audio;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use fc_rom::RomHeader;
use log::{error, info};
use pixels::{Pixels, SurfaceTexture};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    window::{Window, WindowAttributes},
};

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
    /// Development mode: load games/asm/movement.asm
    Dev,
}

pub struct App {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    screen: Screen,
    input: Input,
    vm: Vm,
    #[allow(dead_code)]
    audio: Option<Audio>,
    debugger: Debugger,
    timing: FixedTimestep,
    last_tick: Instant,
}

impl App {
    fn new() -> Result<Self> {
        Font::init_global(
            "assets/font.png",
            " 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ!?\"'()+-=.:,[]<>",
            3,
            5,
        )
        .context("failed to initialize font")?;

        let instruction_set = Arc::new(default_instruction_set());
        let vm = Vm::new(instruction_set);

        let audio = match Audio::new(vm.get_sound_shared()) {
            Ok(a) => Some(a),
            Err(e) => {
                error!("failed to initialize audio: {e}");
                None
            }
        };

        info!("fantasy console initialized");

        Ok(Self {
            window: None,
            pixels: None,
            screen: Screen::new(),
            input: Input::new(),
            vm,
            audio,
            debugger: Debugger::new(false),
            timing: FixedTimestep::new(60),
            last_tick: Instant::now(),
        })
    }

    fn set_debug_enabled(&mut self, enabled: bool) {
        self.debugger.set_enabled(enabled);
    }

    fn load_rom(&mut self, path: &PathBuf) -> Result<()> {
        let rom = fc_rom::load(path)
            .with_context(|| format!("failed to load ROM from {}", path.display()))?;
        self.vm.load_rom(rom.program);
        info!("ROM loaded from {}", path.display());
        Ok(())
    }

    fn load_source(&mut self, path: &PathBuf) -> Result<()> {
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read source {}", path.display()))?;
        self.vm
            .load_program(&source)
            .with_context(|| format!("failed to assemble {}", path.display()))?;
        info!("source assembled from {}", path.display());
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attrs = WindowAttributes::default()
            .with_title(NAME)
            .with_inner_size(LogicalSize::new(SCREEN_WIDTH as f64, SCREEN_HEIGHT as f64))
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
        let pixels = match Pixels::new(WIDTH, HEIGHT, surface) {
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
                        .draw_overlay(self.screen.get_debug_layer(), &self.vm);
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
                    match code {
                        KeyCode::ArrowUp | KeyCode::KeyW => self.input.up = pressed,
                        KeyCode::ArrowDown | KeyCode::KeyS => self.input.down = pressed,
                        KeyCode::ArrowLeft | KeyCode::KeyA => self.input.left = pressed,
                        KeyCode::ArrowRight | KeyCode::KeyD => self.input.right = pressed,
                        KeyCode::KeyJ => self.input.a = pressed,
                        KeyCode::KeyK => self.input.b = pressed,
                        KeyCode::Space => {
                            if pressed && !event.repeat {
                                self.debugger.toggle_pause();
                            }
                        }
                        KeyCode::KeyC => {
                            if pressed && !event.repeat {
                                self.debugger.step();
                            }
                        }
                        KeyCode::KeyB => {
                            if pressed && !event.repeat {
                                if let Some(state) = self.debugger.pop_state() {
                                    self.vm.restore(&state);
                                    self.debugger.pause();
                                }
                            }
                        }
                        KeyCode::KeyN => {
                            if pressed && !event.repeat {
                                self.debugger.prev_ram_page();
                            }
                        }
                        KeyCode::KeyM => {
                            if pressed && !event.repeat {
                                self.debugger.next_ram_page();
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick);
        self.last_tick = now;

        match self.debugger.get_mode() {
            DebugMode::Running => {
                let steps = self.timing.tick(dt);
                for _ in 0..steps {
                    self.vm.run_frame(&self.input);
                    self.debugger.push_state(self.vm.snapshot());
                }
            }
            DebugMode::Step => {
                self.vm.step(&self.input);
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

    if let Command::Build { source, output } = command {
        info!("building ROM: {} → {}", source.display(), output.display());

        let instruction_set = Arc::new(default_instruction_set());
        let vm = Vm::new(instruction_set);

        let src = std::fs::read_to_string(&source)
            .with_context(|| format!("cannot read {}", source.display()))?;

        let program = vm
            .assemble(&src)
            .map_err(|e| anyhow::anyhow!("assembly failed: {e}"))?;

        let stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let header = RomHeader::default_for(stem);

        fc_rom::write(&output, &header, &program)
            .with_context(|| format!("cannot write ROM to {}", output.display()))?;

        info!("ROM written to {}", output.display());
        return Ok(());
    }

    let mut app = App::new()?;

    match command {
        Command::Dev => {
            info!("development mode");
            let path = PathBuf::from("games/asm/movement.asm");
            if path.exists() {
                app.load_source(&path)?;
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
        Command::Build { .. } => unreachable!(),
    }

    let event_loop = EventLoop::new().context("failed to create event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).context("event loop error")?;

    Ok(())
}
