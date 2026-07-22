use anyhow::{Context, Result};
use clap::Parser;
use fc_core::memory::SPRITE_SHEET_RAM_BASE;
use fc_rom::SectionKind;
use fc_vm::runtime::{ConsoleCore, WindowGfx};
use log::info;
use std::path::{Path, PathBuf};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::{application::ApplicationHandler, event::WindowEvent};

#[derive(Parser)]
#[command(name = "fc-host", about = "Fantasy Console — cart runner")]
struct Cli {
    /// Path to a .rom or .lua file
    file: PathBuf,
}

pub struct App {
    core: ConsoleCore,
    gfx: WindowGfx,
}

impl App {
    fn new() -> Result<Self> {
        Ok(Self {
            core: ConsoleCore::new()?,
            gfx: WindowGfx::default(),
        })
    }

    fn load(&mut self, path: &Path) -> Result<()> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("lua") => self.load_lua(path),
            Some("rom") => self.load_rom(path),
            ext => anyhow::bail!(
                "unsupported file extension '{}' (expected .rom or .lua): {}",
                ext.unwrap_or(""),
                path.display()
            ),
        }
    }

    /// Loads a bare `.lua` file straight into the VM's embedded Lua path.
    /// No asset sections and no ROM packaging — for that, build a `.rom`.
    fn load_lua(&mut self, path: &Path) -> Result<()> {
        let src = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read Lua source from {}", path.display()))?;
        self.core
            .vm
            .load_lua_source(&src, &self.core.input, &self.core.font)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("failed to load Lua script {}", path.display()))
    }

    fn load_rom(&mut self, path: &Path) -> Result<()> {
        let rom = fc_rom::load(path)
            .with_context(|| format!("failed to load ROM from {}", path.display()))?;

        for section in &rom.sections {
            if section.kind == SectionKind::ModManifest {
                let manifest = String::from_utf8_lossy(&section.data);
                let registered = self.core.vm.registered_peripheral_names();
                for required in manifest.lines().map(str::trim).filter(|s| !s.is_empty()) {
                    if !registered.contains(&required) {
                        anyhow::bail!("ROM requires mod '{}' but it is not loaded", required);
                    }
                }
            }
        }

        // Asset RAM must be in place before the Lua load, since it runs
        // `_init()` immediately.
        for section in &rom.sections {
            if section.kind == SectionKind::SpriteSheet {
                self.core
                    .vm
                    .load_section_to_ram(SPRITE_SHEET_RAM_BASE, &section.data);
                info!(
                    "SpriteSheet section loaded to RAM at 0x{:04X} ({} bytes)",
                    SPRITE_SHEET_RAM_BASE,
                    section.data.len()
                );
            }
        }

        let lua_source = rom
            .sections
            .iter()
            .find(|s| s.kind == SectionKind::LuaSource)
            .map(|s| String::from_utf8_lossy(&s.data).into_owned())
            .context("ROM has no Lua source section (bytecode carts are no longer supported)")?;
        self.core
            .vm
            .load_lua_source(&lua_source, &self.core.input, &self.core.font)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("failed to load Lua ROM {}", path.display()))?;

        info!("ROM loaded from {}", path.display());
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.gfx.resume(event_loop, &self.core.config);
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
                self.gfx.resize(new_size);
            }
            WindowEvent::RedrawRequested => {
                self.core.screen.get_debug_layer().clear();
                self.gfx.present(&self.core.screen, &self.core.vm);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state.is_pressed();
                if let PhysicalKey::Code(code) = event.physical_key
                    && let Some(button) = self.core.input_map.get_button(code)
                {
                    self.core.input.set_button(button, pressed);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        for _ in 0..self.core.frame_steps() {
            self.core.run_frame();
        }
        self.gfx.request_redraw();
    }
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let mut app = App::new()?;
    app.load(&cli.file)?;

    let event_loop = EventLoop::new().context("failed to create event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).context("event loop error")?;

    Ok(())
}
