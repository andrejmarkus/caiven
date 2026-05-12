use anyhow::{Context, Result};
use clap::Parser;
use fc_rom::SectionKind;
use fc_vm::default_instruction_set;
use fc_vm::input::{Input, InputMap};
use fc_vm::rendering::font::Font;
use fc_vm::rendering::screen::Screen;
use fc_vm::settings::NAME;
use fc_vm::timing::FixedTimestep;
use fc_vm::vm::audio::{Audio, AudioPeripheral};
use fc_vm::{Vm, VmConfig};
use log::{error, info};
use pixels::{Pixels, SurfaceTexture};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    window::{Window, WindowAttributes},
};

const SPRITE_SHEET_RAM_BASE: usize = 0x4000;

#[derive(Parser)]
#[command(name = "fc-host", about = "Fantasy Console — ROM runner")]
struct Cli {
    rom: PathBuf,
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
    timing: FixedTimestep,
    last_tick: Instant,
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
            timing: FixedTimestep::new(60),
            last_tick: Instant::now(),
        })
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

        self.vm.load_rom(rom.program);

        for section in &rom.sections {
            if section.kind == SectionKind::SpriteSheet {
                self.vm
                    .load_section_to_ram(SPRITE_SHEET_RAM_BASE, &section.data);
                info!(
                    "SpriteSheet section loaded to RAM at 0x{:04X} ({} bytes)",
                    SPRITE_SHEET_RAM_BASE,
                    section.data.len()
                );
            }
        }
        info!("ROM loaded from {}", path.display());
        Ok(())
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
                if let PhysicalKey::Code(code) = event.physical_key
                    && let Some(button) = self.input_map.get_button(code)
                {
                    self.input.set_button(button, pressed);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick);
        self.last_tick = now;

        let steps = self.timing.tick(dt);
        for _ in 0..steps {
            self.vm.run_frame(&self.input, &self.font);
        }

        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let mut app = App::new()?;
    app.load_rom(&cli.rom)?;

    let event_loop = EventLoop::new().context("failed to create event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).context("event loop error")?;

    Ok(())
}
