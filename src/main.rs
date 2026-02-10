mod assembler;
mod debugger;
mod input;
mod rendering;
mod rom;
mod settings;
mod utils;
mod vm;

use crate::assembler::default_instruction_set;
use crate::debugger::Debugger;
use crate::rendering::font::Font;
use crate::rom::rom_loader::load_rom;
use crate::rom::rom_writer::write_rom;
use crate::vm::Vm;
use input::Input;
use log::info;
use pixels::{Pixels, SurfaceTexture};
use rendering::screen::Screen;
use settings::{HEIGHT, NAME, SCREEN_HEIGHT, SCREEN_WIDTH, WIDTH};
use std::path::PathBuf;
use std::sync::Arc;
use winit::event::WindowEvent;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

enum AppMode {
    Dev,
    Debug(PathBuf),
    Run(PathBuf),
    Build(PathBuf, PathBuf),
}

struct App {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    screen: Screen,
    input: Input,
    vm: Vm,
    debugger: Debugger,
}

impl App {
    fn new() -> Self {
        Font::init_global(
            "assets/font.png",
            " 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ!?\"'()+-=.:,[]<>",
            3,
            5,
        );
        let instruction_set = Arc::new(default_instruction_set());
        let vm = Vm::new(instruction_set);
        // vm.load_program(&std::fs::read_to_string("games/tiles.asm").unwrap());

        info!("Fantasy Console initialized");

        Self {
            window: None,
            pixels: None,
            screen: Screen::new(),
            input: Input::new(),
            vm,
            debugger: Debugger::new(false),
        }
    }

    pub fn set_debug_enabled(&mut self, enabled: bool) {
        self.debugger.set_enabled(enabled);
    }

    pub fn load_rom(&mut self, path: &PathBuf) {
        let program = load_rom(path).expect("Failed to load ROM file");
        self.vm.load_rom(program);
        info!("ROM loaded successfully from {}", path.display());
    }

    pub fn load_source(&mut self, path: &PathBuf) {
        let source = std::fs::read_to_string(path).expect("Failed to read source file");
        self.vm.load_program(&source);
        info!(
            "Source loaded and assembled successfully from {}",
            path.display()
        );
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attrs = WindowAttributes::default()
            .with_title(NAME)
            .with_inner_size(LogicalSize::new(SCREEN_WIDTH as f64, SCREEN_HEIGHT as f64))
            .with_resizable(false);
        let window = Arc::new(event_loop.create_window(window_attrs).unwrap());

        let size = window.inner_size();
        let surface = SurfaceTexture::new(size.width, size.height, window.clone());
        let pixels = Pixels::new(WIDTH, HEIGHT, surface).unwrap();

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

                match self.debugger.get_mode() {
                    debugger::DebugMode::Running => {
                        let (world, ui) = self.screen.get_layers_mut();
                        self.vm.run_frame(&self.input, world, ui);
                        self.debugger.push_state(self.vm.snapshot());
                    }
                    debugger::DebugMode::Paused => {
                        self.debugger
                            .draw_overlay(self.screen.get_debug_layer(), &self.vm);
                    }
                    debugger::DebugMode::Step => {
                        let (world, ui) = self.screen.get_layers_mut();
                        self.vm.step(&self.input, world, ui);
                        self.debugger.check_breakpoint(self.vm.get_pc());
                        self.debugger.dump_state(&self.vm);
                        self.debugger.pause();
                    }
                }

                if let Some(pixels) = self.pixels.as_mut() {
                    self.screen.construct(pixels.frame_mut());
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
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

fn parse_args() -> AppMode {
    let mut args = std::env::args().skip(1);

    match args.next() {
        Some(arg) if arg == "--dev" => AppMode::Dev,
        Some(arg) if arg == "--debug" => {
            let path = args.next().expect("Expected path after --debug");
            AppMode::Debug(PathBuf::from(path))
        }
        Some(arg) if arg == "--run" => {
            let path = args.next().expect("Expected path after --run");
            AppMode::Run(PathBuf::from(path))
        }
        Some(arg) if arg == "--build" => {
            let input_path = args.next().expect("Expected input path after --build");
            let output_path = args.next().expect("Expected output path after input path");
            AppMode::Build(PathBuf::from(input_path), PathBuf::from(output_path))
        }
        Some(arg) => panic!("Unknown argument: {}", arg),
        None => AppMode::Dev,
    }
}

fn main() {
    let mode = parse_args();

    let log_level = match mode {
        AppMode::Debug(_) => "debug",
        _ => "info",
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    if let AppMode::Build(input_path, output_path) = mode {
        info!(
            "Building ROM from {} to {}",
            input_path.display(),
            output_path.display()
        );

        let instruction_set = Arc::new(default_instruction_set());
        let vm = Vm::new(instruction_set);

        let source = std::fs::read_to_string(&input_path).unwrap_or_else(|e| {
            log::error!("Failed to read input file {}: {}", input_path.display(), e);
            std::process::exit(1);
        });

        let program = vm.assemble(&source).unwrap_or_else(|e| {
            log::error!("Assembly failed: {}", e);
            std::process::exit(1);
        });

        write_rom(&output_path, &program).unwrap_or_else(|e| {
            log::error!("Failed to write ROM file {}: {}", output_path.display(), e);
            std::process::exit(1);
        });

        info!("ROM built successfully to {}", output_path.display());
        return;
    }

    let mut app = App::new();

    match mode {
        AppMode::Dev => {
            info!("Starting in development mode...");
            let default_source = PathBuf::from("games/asm/movement.asm");
            if default_source.exists() {
                app.load_source(&default_source);
            }
        }
        AppMode::Debug(path) => {
            info!("Starting in debug mode with ROM: {}", path.display());
            app.set_debug_enabled(true);
            app.load_rom(&path);
        }
        AppMode::Run(path) => {
            info!("Starting with ROM: {}", path.display());
            app.load_rom(&path);
        }
        AppMode::Build(_, _) => unreachable!(),
    }

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    event_loop.run_app(&mut app).unwrap();
}
