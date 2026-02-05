mod assembler;
mod debugger;
mod input;
mod instructions;
mod rendering;
mod settings;
mod vm;

use crate::debugger::Debugger;
use crate::instructions::default_instruction_set;
use crate::vm::Vm;
use input::Input;
use pixels::{Pixels, SurfaceTexture};
use rendering::screen::Screen;
use settings::{HEIGHT, NAME, SCREEN_HEIGHT, SCREEN_WIDTH, WIDTH};
use std::sync::Arc;
use winit::event::WindowEvent;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

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
        let instruction_set = Arc::new(default_instruction_set());
        let mut vm = Vm::new(instruction_set);
        vm.load_program(&std::fs::read_to_string("games/movement.asm").unwrap());

        Self {
            window: None,
            pixels: None,
            screen: Screen::new(),
            input: Input::new(),
            vm,
            debugger: Debugger::new(),
        }
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
                match self.debugger.get_mode() {
                    debugger::DebugMode::Running => {
                        self.vm
                            .run_frame(&self.input, &mut self.screen.get_world_layer());
                    }
                    debugger::DebugMode::Paused => {
                        // Do nothing
                    }
                    debugger::DebugMode::Step => {
                        self.vm
                            .step(&self.input, &mut self.screen.get_world_layer());
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
                        KeyCode::KeyN => {
                            if pressed && !event.repeat {
                                self.debugger.step();
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

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}
