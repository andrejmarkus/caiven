mod assember;
mod buttons;
mod input;
mod roms;
mod screen;
mod settings;
mod vm;

use crate::assember::assemble;
use crate::vm::Vm;
use input::Input;
use pixels::{Pixels, SurfaceTexture};
use screen::Screen;
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
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            pixels: None,
            screen: Screen::new(),
            input: Input::new(),
            vm: Vm::new(assemble(
                r#"
                    ; R0 = x, R1 = y, R2 = left input, R3 = right input
                    ; Init state in RAM
                    MOV r0 10
                    STM 100 r0
                    MOV r1 10
                    STM 101 r1

                    loop:
                        CLS
                        ; Load state from RAM
                        LDM r0 100
                        LDM r1 101
                        
                        ; Read left and right input
                        IN r2 2
                        IN r3 3
                        
                        ; Check if left button is pressed
                        JNZ r2 move_left
                        
                        ; Check if right button is pressed
                        JNZ r3 move_right
                        
                        JMP draw

                    move_left:
                        DEC r0
                        STM 100 r0
                        JMP draw

                    move_right:
                        ADD r0 1
                        STM 100 r0

                    draw:
                        ; draw pixel at (R0, R1) with color red (255, 0, 0)
                        DPXR r0 r1 255,0,0
                        
                        WAIT
                        JMP loop
                "#,
            )),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attrs = WindowAttributes::default()
            .with_title(NAME)
            .with_inner_size(LogicalSize::new(SCREEN_WIDTH, SCREEN_HEIGHT));
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
            WindowEvent::RedrawRequested => {
                self.vm.run_frame(&self.input, &mut self.screen);

                if let Some(pixels) = self.pixels.as_mut() {
                    pixels.frame_mut().copy_from_slice(&self.screen.pixels);
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
