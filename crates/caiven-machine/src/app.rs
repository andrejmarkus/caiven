use anyhow::{Context, Result, anyhow};
use caiven_cart::SectionKind;
use caiven_vm::runtime::ConsoleCore;
use caiven_vm::settings::NAME;
use clap::Parser;
use log::{error, info};
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::{Mod, Scancode};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::platform::audio::sdl_audio_factory;
use crate::platform::input::{Gamepads, key_from_scancode, pad_button_from_sdl};
use crate::platform::scaling::{AspectMode, ScaleMode};
use crate::platform::window::Display;

#[derive(Parser)]
#[command(name = "caiven-machine", about = "Caiven — cart runner")]
struct Cli {
    /// Path to a project dir, its caiven.toml, or a .cav cartridge
    file: PathBuf,

    /// Run fullscreen. What handhelds want, where the panel is the window.
    #[arg(long)]
    fullscreen: bool,

    /// How large the console framebuffer is drawn
    #[arg(long, value_enum, default_value_t = ScaleMode::Fit)]
    scale: ScaleMode,

    /// Whether console pixels stay square
    #[arg(long, value_enum, default_value_t = AspectMode::Square)]
    aspect: AspectMode,
}

pub struct App {
    core: ConsoleCore,
    cart_path: PathBuf,
}

impl App {
    fn new(core: ConsoleCore) -> Self {
        Self {
            core,
            cart_path: PathBuf::new(),
        }
    }

    fn load(&mut self, path: &Path) -> Result<()> {
        let cart = caiven_cart::open(path)
            .with_context(|| format!("failed to load cart from {}", path.display()))?;

        for section in &cart.sections {
            if section.kind == SectionKind::ModManifest {
                let manifest = String::from_utf8_lossy(&section.data);
                let registered = self.core.vm.registered_peripheral_names();
                check_mod_manifest(&manifest, &registered)?;
            }
        }

        // Asset RAM must be in place before the Lua load, since it runs
        // `_init()` immediately.
        let lua_source =
            self.core.vm.load_cart_sections(&cart.sections).context(
                "cart has no Lua source section (bytecode carts are no longer supported)",
            )?;
        info!(
            "loaded {} asset section(s) to RAM",
            cart.sections
                .iter()
                .filter(|s| s.kind != SectionKind::LuaSource)
                .count()
        );
        self.core
            .vm
            .load_lua_source(&lua_source, &self.core.input, &self.core.font)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("failed to load Lua cart {}", path.display()))?;

        info!("cart loaded from {}", path.display());
        self.cart_path = path.to_path_buf();
        Ok(())
    }

    /// Reloads the cart from disk into a fresh VM (Ctrl+R): the fast
    /// edit-in-editor / re-run loop the project-dir format is for.
    fn reload(&mut self) {
        let path = self.cart_path.clone();
        self.core.reset_vm();
        match self.load(&path) {
            Ok(()) => info!("reloaded {}", path.display()),
            Err(e) => error!("reload failed: {e:#}"),
        }
    }

    fn set_key(&mut self, scancode: Scancode, pressed: bool) {
        if let Some(key) = key_from_scancode(scancode)
            && let Some(button) = self.core.input_map.get_button(key)
        {
            self.core.input.set_button(button, pressed);
        }
    }

    fn set_pad(&mut self, button: sdl2::controller::Button, pressed: bool) {
        if let Some(pad_button) = pad_button_from_sdl(button)
            && let Some(button) = self.core.input_map.get_pad_button(pad_button)
        {
            self.core.input.set_button(button, pressed);
        }
    }
}

/// Checks that every peripheral a cart's `ModManifest` section declares it
/// needs is present in `registered`. Blank lines are ignored.
fn check_mod_manifest(manifest: &str, registered: &[&str]) -> Result<()> {
    for required in manifest.lines().map(str::trim).filter(|s| !s.is_empty()) {
        if !registered.contains(&required) {
            anyhow::bail!("cart requires mod '{}' but it is not loaded", required);
        }
    }
    Ok(())
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let sdl = sdl2::init().map_err(|e| anyhow!("failed to initialize SDL: {e}"))?;
    let video = sdl
        .video()
        .map_err(|e| anyhow!("failed to initialize SDL video: {e}"))?;
    let controller_subsystem = sdl
        .game_controller()
        .map_err(|e| anyhow!("failed to initialize SDL game controller support: {e}"))?;

    // Audio is optional: a device with no output still runs carts, silently.
    let audio_factory = match sdl.audio() {
        Ok(audio) => sdl_audio_factory(audio),
        Err(e) => {
            error!("failed to initialize SDL audio: {e}");
            Box::new(|_| Err(anyhow!("SDL audio subsystem unavailable")))
        }
    };

    let mut app = App::new(ConsoleCore::with_audio_factory(audio_factory)?);
    app.load(&cli.file)?;

    let mut display = Display::new(
        &video,
        &app.core.config,
        NAME,
        cli.fullscreen,
        cli.scale,
        cli.aspect,
    )?;
    let texture_creator = display.texture_creator();
    let mut texture = Display::create_console_texture(&texture_creator, &app.core.config)?;

    let mut gamepads = Gamepads::new();
    gamepads.open_attached(&controller_subsystem);

    let mut event_pump = sdl
        .event_pump()
        .map_err(|e| anyhow!("failed to create SDL event pump: {e}"))?;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::Window {
                    win_event: WindowEvent::Close,
                    ..
                } => break 'running,

                Event::KeyDown {
                    scancode: Some(scancode),
                    keymod,
                    repeat,
                    ..
                } => {
                    // Ctrl+R reloads. It is a host shortcut, so it must not
                    // also reach the cart as a button press.
                    if !repeat
                        && scancode == Scancode::R
                        && keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD)
                    {
                        app.reload();
                        continue;
                    }
                    app.set_key(scancode, true);
                }
                Event::KeyUp {
                    scancode: Some(scancode),
                    ..
                } => app.set_key(scancode, false),

                Event::ControllerDeviceAdded { which, .. } => {
                    gamepads.open(&controller_subsystem, which)
                }
                Event::ControllerDeviceRemoved { which, .. } => gamepads.close(which),
                Event::ControllerButtonDown { button, .. } => app.set_pad(button, true),
                Event::ControllerButtonUp { button, .. } => app.set_pad(button, false),

                _ => {}
            }
        }

        let steps = app.core.frame_steps();
        if steps == 0 {
            // Nothing to advance yet. Without an accelerated renderer there
            // is no vsync to block on, so yield instead of spinning a core —
            // which on a handheld is battery burned for nothing.
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }
        for _ in 0..steps {
            app.core.run_frame();
        }

        app.core.screen.get_debug_layer().clear();
        display.present(&mut texture, &app.core.screen, &app.core.vm)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::check_mod_manifest;

    #[test]
    fn passes_when_all_required_peripherals_registered() {
        assert!(check_mod_manifest("rtc\ninput", &["rtc", "input", "audio"]).is_ok());
    }

    #[test]
    fn fails_when_a_peripheral_is_missing() {
        let err = check_mod_manifest("rtc\nmissing_mod", &["rtc"]).unwrap_err();
        assert!(err.to_string().contains("missing_mod"));
    }

    #[test]
    fn ignores_blank_lines_and_surrounding_whitespace() {
        assert!(check_mod_manifest("\n  rtc  \n\n", &["rtc"]).is_ok());
    }

    #[test]
    fn empty_manifest_always_passes() {
        assert!(check_mod_manifest("", &[]).is_ok());
    }
}
