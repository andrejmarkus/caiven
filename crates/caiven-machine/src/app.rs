use anyhow::{Context, Result, anyhow};
use caiven_cart::SectionKind;
use caiven_vm::input::{Button, ControlsFile, InputMap, Key, PadButton, SystemButton};
use caiven_vm::runtime::ConsoleCore;
use caiven_vm::settings::NAME;
use chrono::Timelike;
use clap::Parser;
use log::{error, info};
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::{Mod, Scancode};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::platform::audio::sdl_audio_factory;
use crate::platform::input::{Gamepads, key_from_scancode, pad_button_from_sdl};
use crate::platform::power;
use crate::platform::scaling::{AspectMode, ScaleMode};
use crate::platform::window::Display;
use crate::port_client::{self, PortEntry};
use crate::shell::input::{ShellInput, cart_button, shell_button, shell_button_from_system};
use crate::shell::library::{self as cart_library, CartMeta};
use crate::shell::screens::chrome::{self, StatusInfo};
use crate::shell::screens::loading::{self, LoadProgress};
use crate::shell::screens::{
    boot, controls as controls_screen, crash, detail, library as library_screen, pause, playing,
    port as port_screen, settings as settings_screen,
};
use crate::shell::settings::Settings;
use crate::shell::state::{BIND_ORDER, BOOT_DURATION, Effect, Screen, ShellButton, ShellState};
use crate::shell::surface::Surface;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "caiven-machine", about = "Caiven — cart runner")]
struct Cli {
    /// Path to a project dir, its caiven.toml, or a .cav cartridge. Omit to
    /// boot into the console shell and browse the cart library instead —
    /// the same experience a handheld gives a player.
    file: Option<PathBuf>,

    /// Run fullscreen. What handhelds want, where the panel is the window.
    #[arg(long)]
    fullscreen: bool,

    /// How large the console framebuffer is drawn. Overrides whatever the
    /// persisted `settings.toml` (or the default) has for this session;
    /// omit it to let a value set from the Settings screen stick across
    /// runs.
    #[arg(long, value_enum)]
    scale: Option<ScaleMode>,

    /// Whether console pixels stay square. Same override behavior as
    /// `--scale`.
    #[arg(long, value_enum)]
    aspect: Option<AspectMode>,

    /// Show the fps counter on the Playing screen. Only forces it on for
    /// this session — a persisted "off" is never silently overridden to
    /// off by the flag's absence.
    #[arg(long)]
    show_fps: bool,
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
}

/// Where persisted settings live: a `settings.toml` beside the binary, the
/// same exe-relative bargain `cart_library::default_dir()` makes for
/// `carts/` — one folder a player can copy off a card wholesale.
fn settings_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("settings.toml")
}

/// Loads persisted settings, falling back to defaults on a missing or
/// corrupt file rather than failing to boot — the same tolerance
/// `cart_library::scan` has for a bad `.cav` (SPEC V54).
fn load_settings(path: &Path) -> Settings {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Settings::default();
    };
    match toml::from_str(&content) {
        Ok(settings) => settings,
        Err(e) => {
            error!("failed to parse {}: {e}", path.display());
            Settings::default()
        }
    }
}

/// Writes settings to `path`, logging (not failing) on error — a save
/// that can't land shouldn't take the settings screen down with it.
fn save_settings(path: &Path, settings: &Settings) {
    match toml::to_string_pretty(settings) {
        Ok(content) => {
            if let Err(e) = std::fs::write(path, content) {
                error!("failed to write {}: {e}", path.display());
            }
        }
        Err(e) => error!("failed to serialize settings: {e}"),
    }
}

/// Where `controls.toml` lives: the same bare, CWD-relative name
/// `caiven_vm::runtime::ConsoleCore` already reads it from — writing
/// anywhere else would mean the remap screen's changes silently don't
/// survive a restart (SPEC V40 round-trip).
fn controls_path() -> PathBuf {
    PathBuf::from("controls.toml")
}

/// The remap screen's per-row label for each of `BIND_ORDER`'s six buttons:
/// its keyboard names, its gamepad names, joined for display. Empty on both
/// sides reads as "—" rather than a blank row.
fn bind_labels(controls: &ControlsFile) -> [String; 6] {
    let mut labels: [String; 6] = Default::default();
    for (index, shell_button) in BIND_ORDER.into_iter().enumerate() {
        let Some(button) = cart_button(shell_button) else {
            continue;
        };
        let keys = controls.controls.names(button).join(", ");
        let pads = controls.gamepad.names(button).join(", ");
        labels[index] = match (keys.is_empty(), pads.is_empty()) {
            (true, true) => String::new(),
            (false, true) => keys,
            (true, false) => pads,
            (false, false) => format!("{keys} \u{b7} {pads}"),
        };
    }
    labels
}

/// A physical input captured while the remap screen is listening.
enum Captured {
    Key(Key),
    Pad(PadButton),
}

/// Applies one captured input to the button the remap screen has focused:
/// writes it into `controls_doc`, persists it, rebuilds the live
/// `InputMap` so the new binding works immediately, and tells the shell
/// what to show for it — a single fresh binding replaces whatever was
/// there, in the same `[controls]`/`[gamepad]` shape `controls.toml`
/// already uses (SPEC V40).
fn capture_bind(
    app: &mut App,
    shell: &mut ShellState,
    controls_doc: &mut ControlsFile,
    captured: Captured,
) {
    let index = shell.bind_index();
    let Some(button) = BIND_ORDER.get(index).copied().and_then(cart_button) else {
        shell.bind_captured(String::new());
        return;
    };
    let label = match captured {
        Captured::Key(key) => {
            controls_doc
                .controls
                .set(button, vec![key.name().to_string()]);
            key.name().to_string()
        }
        Captured::Pad(pad) => {
            controls_doc
                .gamepad
                .set(button, vec![pad.name().to_string()]);
            pad.name().to_string()
        }
    };
    app.core.input_map = controls_doc.to_input_map();
    if let Err(e) = controls_doc.save(&controls_path()) {
        error!("failed to write controls.toml: {e}");
    }
    shell.bind_captured(label);
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

/// What a physical key or pad button resolves to through `controls.toml`.
/// At most one of these ever comes back for a given input — a binding
/// shared by both a cart button and a `SystemButton` has already had the
/// collision resolved in the cart's favor (SPEC V51/V52).
enum Mapped {
    Cart(Button),
    System(SystemButton),
}

fn map_key(input_map: &InputMap, key: Key) -> Option<Mapped> {
    if let Some(sys) = input_map.get_system_button(key) {
        return Some(Mapped::System(sys));
    }
    input_map.get_button(key).map(Mapped::Cart)
}

fn map_pad(input_map: &InputMap, pad: PadButton) -> Option<Mapped> {
    if let Some(sys) = input_map.get_pad_system_button(pad) {
        return Some(Mapped::System(sys));
    }
    input_map.get_pad_button(pad).map(Mapped::Cart)
}

/// Runs one shell button event through `ShellState::press` and handles
/// whatever effect comes back.
fn dispatch(
    evt: ShellButton,
    app: &mut App,
    shell: &mut ShellState,
    carts: &mut Vec<CartMeta>,
    library_dir: &Path,
    port_entries: &mut Vec<PortEntry>,
) {
    if let Some(effect) = shell.press(evt) {
        handle_effect(effect, app, shell, carts, library_dir, port_entries);
    }
}

/// A button (key or pad) went down. During `Playing`, every cart button
/// (all six — SPEC V49) reaches the VM directly, in parallel with feeding
/// `ShellInput`: the same physical B also has to keep counting toward the
/// long-press-to-`Start` fallback (SPEC V53) no matter what screen is up —
/// `press_playing` already ignores every shell event but `Start`, so
/// routing every button through both paths unconditionally is safe.
fn on_down(
    mapped: Mapped,
    app: &mut App,
    shell: &mut ShellState,
    shell_input: &mut ShellInput,
    carts: &mut Vec<CartMeta>,
    library_dir: &Path,
    port_entries: &mut Vec<PortEntry>,
) {
    match mapped {
        Mapped::Cart(button) => {
            if shell.screen() == Screen::Playing {
                app.core.input.set_button(button, true);
            }
            if let Some(evt) = shell_input.press(shell_button(button)) {
                dispatch(evt, app, shell, carts, library_dir, port_entries);
            }
        }
        Mapped::System(sys) => {
            dispatch(
                shell_button_from_system(sys),
                app,
                shell,
                carts,
                library_dir,
                port_entries,
            );
        }
    }
}

fn on_up(
    mapped: Mapped,
    app: &mut App,
    shell: &mut ShellState,
    shell_input: &mut ShellInput,
    carts: &mut Vec<CartMeta>,
    library_dir: &Path,
    port_entries: &mut Vec<PortEntry>,
) {
    // SystemButton has no release semantics of its own — B's hold timer is
    // what carries the long-press fallback, and that lives on the Cart(B)
    // arm below.
    if let Mapped::Cart(button) = mapped {
        if shell.screen() == Screen::Playing {
            app.core.input.set_button(button, false);
        }
        if let Some(evt) = shell_input.release(shell_button(button)) {
            dispatch(evt, app, shell, carts, library_dir, port_entries);
        }
    }
}

/// Carries out what `ShellState::press` decided the host must do.
///
/// Screens that don't have real functionality behind them yet (save states,
/// T50) resolve their effects to a safe no-op rather than leaving the shell
/// unable to navigate. `ListenForBind` needs no action here — the run
/// loop's top-level listening check answers with `bind_captured` once a
/// physical input arrives, or `listening` would swallow every future press.
///
/// Port requests (`RefreshPort`, `StartDownload`) block on `ureq` just like
/// `LoadCart`/`DeleteCart` block on the filesystem — same synchronous
/// convention, see `port_client`'s module doc.
fn handle_effect(
    effect: Effect,
    app: &mut App,
    shell: &mut ShellState,
    carts: &mut Vec<CartMeta>,
    library_dir: &Path,
    port_entries: &mut Vec<PortEntry>,
) {
    match effect {
        Effect::LoadCart(index) => {
            let Some(cart) = carts.get(index) else {
                shell.cart_failed("selected cart no longer exists", None);
                return;
            };
            let path = cart.path.clone();
            match app.load(&path) {
                Ok(()) => shell.cart_ready(),
                Err(e) => {
                    error!("failed to load {}: {e:#}", path.display());
                    shell.cart_failed(e.to_string(), None);
                }
            }
        }
        Effect::CancelLoad => {
            // Loads above resolve synchronously in the same call that
            // requested them, so nothing is ever actually in flight to
            // cancel by the time this could fire.
        }
        Effect::DeleteCart(index) => {
            if let Some(cart) = carts.get(index)
                && let Err(e) = std::fs::remove_file(&cart.path)
            {
                error!("failed to delete {}: {e}", cart.path.display());
            }
            *carts = cart_library::scan(library_dir);
            shell.set_cart_count(carts.len());
        }
        Effect::ResetCart => app.reload(),
        Effect::QuitToLibrary => app.core.reset_vm(),
        Effect::SaveState | Effect::LoadState => {
            info!("save states are not implemented yet (SPEC T50)");
        }
        Effect::RefreshPort => match port_client::list(shell.port_sort()) {
            Ok(entries) => {
                *port_entries = entries;
                shell.set_port_count(port_entries.len());
            }
            Err(e) => {
                error!("Port listing failed: {e}");
                port_entries.clear();
                shell.set_port_count(0);
            }
        },
        Effect::StartDownload(index) => {
            let Some(entry) = port_entries.get(index) else {
                shell.download_failed();
                return;
            };
            let id = entry.id.clone();
            match port_client::download(&id) {
                Ok(bytes) => {
                    let path = library_dir.join(format!("{}.cav", port_client::safe_filename(&id)));
                    match std::fs::write(&path, &bytes) {
                        Ok(()) => {
                            *carts = cart_library::scan(library_dir);
                            shell.download_finished();
                            shell.set_cart_count(carts.len());
                        }
                        Err(e) => {
                            error!("failed to save downloaded cart to {}: {e}", path.display());
                            shell.download_failed();
                        }
                    }
                }
                Err(e) => {
                    error!("Port download failed for {id}: {e}");
                    shell.download_failed();
                }
            }
        }
        Effect::SettingsChanged => save_settings(&settings_path(), shell.settings()),
        // `press_controls` already flipped `listening`; the run loop's
        // top-level check does the rest by routing the next physical input
        // to `bind_captured` instead of normal navigation.
        Effect::ListenForBind(_) => {}
    }
}

/// Draws whichever screen `ShellState` is on.
fn draw_screen(
    surface: &mut Surface,
    shell: &ShellState,
    carts: &[CartMeta],
    port_entries: &[PortEntry],
    config: &caiven_vm::VmConfig,
    fps: u32,
    status: &StatusInfo,
) {
    match shell.screen() {
        Screen::Boot => boot::draw(surface, shell, VERSION, config),
        Screen::Library => library_screen::draw(surface, shell, carts),
        Screen::Detail => detail::draw(surface, shell, carts),
        Screen::Loading => {
            // In practice this never draws mid-progress: loads resolve
            // synchronously before the next frame, so by the time this
            // screen is on, the real work is already done (SPEC V35 — the
            // fraction below is real, not a faked tick count).
            let progress = LoadProgress {
                fraction: 1.0,
                stage: "running _init()".to_string(),
            };
            loading::draw(surface, shell, carts, &progress);
        }
        Screen::Playing => playing::draw(surface, shell, fps),
        Screen::Pause => pause::draw(surface, shell),
        Screen::Settings => settings_screen::draw(surface, shell, VERSION),
        Screen::Controls => controls_screen::draw(surface, shell),
        Screen::Port => port_screen::draw(surface, shell, port_entries),
        Screen::Crash => crash::draw(surface, shell),
    }
    chrome::draw(surface, shell, status);
}

/// Whether the content on screen right now animates every frame on its
/// own, independent of input — the shell only redraws on state change
/// (SPEC V33), and these are the states that change every tick.
fn animates_every_frame(shell: &ShellState) -> bool {
    matches!(shell.screen(), Screen::Boot | Screen::Loading)
        || (shell.screen() == Screen::Playing && shell.settings().show_fps)
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

    let mut shell_state = ShellState::new();
    let mut settings = load_settings(&settings_path());
    if let Some(scale) = cli.scale {
        settings.scaling = scale;
    }
    if let Some(aspect) = cli.aspect {
        settings.aspect = aspect;
    }
    settings.show_fps |= cli.show_fps;
    shell_state.set_settings(settings);

    let mut controls_doc = ControlsFile::load(&controls_path());
    shell_state.set_binds(bind_labels(&controls_doc));

    let library_dir = cart_library::default_dir();
    let mut carts: Vec<CartMeta> = Vec::new();
    let mut port_entries: Vec<PortEntry> = Vec::new();

    match &cli.file {
        // A cart given directly on the command line is the developer
        // hot-reload flow: load it eagerly, fail fast on error (this is a
        // dev tool, not a player-facing crash screen), and skip straight
        // past Boot/Library — neither ever draws, so the fake cart count
        // and discarded `LoadCart` effect below never surface anywhere.
        Some(file) => {
            app.load(file)?;
            shell_state.tick(BOOT_DURATION);
            shell_state.set_cart_count(1);
            let _ = shell_state.press(ShellButton::A);
            shell_state.cart_ready();
        }
        None => {
            carts = cart_library::scan(&library_dir);
            shell_state.set_cart_count(carts.len());
        }
    }

    let mut display = Display::new(&video, &app.core.config, NAME, cli.fullscreen)?;
    let texture_creator = display.texture_creator();
    let mut console_texture = Display::create_console_texture(&texture_creator, &app.core.config)?;

    let (win_w, win_h) = display.window_size();
    let mut surface = Surface::new(win_w, win_h).context("failed to build the shell surface")?;
    let mut shell_texture =
        Display::create_shell_texture(&texture_creator, surface.width(), surface.height())?;

    let mut gamepads = Gamepads::new();
    gamepads.open_attached(&controller_subsystem);

    let mut event_pump = sdl
        .event_pump()
        .map_err(|e| anyhow!("failed to create SDL event pump: {e}"))?;

    let mut shell_input = ShellInput::new();
    let mut last_tick = Instant::now();

    let mut fps_window_start = Instant::now();
    let mut fps_frames_in_window = 0u32;
    let mut current_fps = 0u32;

    'running: loop {
        let mut input_event_this_frame = false;

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::Window {
                    win_event: WindowEvent::Close,
                    ..
                } => break 'running,

                Event::Window {
                    win_event: WindowEvent::SizeChanged(w, h),
                    ..
                } => {
                    surface.resize(w as u32, h as u32)?;
                    shell_texture = Display::create_shell_texture(
                        &texture_creator,
                        surface.width(),
                        surface.height(),
                    )?;
                    surface.mark_dirty();
                }

                Event::KeyDown {
                    scancode: Some(scancode),
                    keymod,
                    repeat,
                    ..
                } => {
                    // Ctrl+R reloads. It is a host shortcut, so it must not
                    // also reach the cart or the shell as a button press —
                    // only meaningful once a cart is actually running.
                    if !repeat
                        && scancode == Scancode::R
                        && keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD)
                        && shell_state.screen() == Screen::Playing
                    {
                        app.reload();
                        continue;
                    }
                    // The remap screen wants the next physical input, not a
                    // mapped button — capture it here instead of routing it
                    // through the normal controls.toml lookup below.
                    if !repeat
                        && shell_state.is_listening()
                        && let Some(key) = key_from_scancode(scancode)
                    {
                        input_event_this_frame = true;
                        capture_bind(
                            &mut app,
                            &mut shell_state,
                            &mut controls_doc,
                            Captured::Key(key),
                        );
                        continue;
                    }
                    if !repeat
                        && let Some(key) = key_from_scancode(scancode)
                        && let Some(mapped) = map_key(&app.core.input_map, key)
                    {
                        input_event_this_frame = true;
                        on_down(
                            mapped,
                            &mut app,
                            &mut shell_state,
                            &mut shell_input,
                            &mut carts,
                            &library_dir,
                            &mut port_entries,
                        );
                    }
                }
                Event::KeyUp {
                    scancode: Some(scancode),
                    ..
                } => {
                    if let Some(key) = key_from_scancode(scancode)
                        && let Some(mapped) = map_key(&app.core.input_map, key)
                    {
                        input_event_this_frame = true;
                        on_up(
                            mapped,
                            &mut app,
                            &mut shell_state,
                            &mut shell_input,
                            &mut carts,
                            &library_dir,
                            &mut port_entries,
                        );
                    }
                }

                Event::ControllerDeviceAdded { which, .. } => {
                    gamepads.open(&controller_subsystem, which)
                }
                Event::ControllerDeviceRemoved { which, .. } => gamepads.close(which),
                Event::ControllerButtonDown { button, .. } => {
                    if shell_state.is_listening()
                        && let Some(pad) = pad_button_from_sdl(button)
                    {
                        input_event_this_frame = true;
                        capture_bind(
                            &mut app,
                            &mut shell_state,
                            &mut controls_doc,
                            Captured::Pad(pad),
                        );
                        continue;
                    }
                    if let Some(pad) = pad_button_from_sdl(button)
                        && let Some(mapped) = map_pad(&app.core.input_map, pad)
                    {
                        input_event_this_frame = true;
                        on_down(
                            mapped,
                            &mut app,
                            &mut shell_state,
                            &mut shell_input,
                            &mut carts,
                            &library_dir,
                            &mut port_entries,
                        );
                    }
                }
                Event::ControllerButtonUp { button, .. } => {
                    if let Some(pad) = pad_button_from_sdl(button)
                        && let Some(mapped) = map_pad(&app.core.input_map, pad)
                    {
                        input_event_this_frame = true;
                        on_up(
                            mapped,
                            &mut app,
                            &mut shell_state,
                            &mut shell_input,
                            &mut carts,
                            &library_dir,
                            &mut port_entries,
                        );
                    }
                }

                _ => {}
            }
        }

        let now = Instant::now();
        let dt = now.duration_since(last_tick);
        last_tick = now;

        shell_state.tick(dt);
        if let Some(evt) = shell_input.tick(dt) {
            dispatch(
                evt,
                &mut app,
                &mut shell_state,
                &mut carts,
                &library_dir,
                &mut port_entries,
            );
        }

        if shell_state.screen() == Screen::Playing {
            let steps = app.core.frame_steps();
            for _ in 0..steps {
                app.core.run_frame();
            }
            app.core.screen.get_debug_layer().clear();
        } else {
            // Nothing to advance yet outside gameplay. Without an
            // accelerated renderer there is no vsync to block on, so yield
            // instead of spinning a core — battery burned for nothing on a
            // handheld sitting in a menu.
            std::thread::sleep(Duration::from_millis(1));
        }

        fps_frames_in_window += 1;
        let window_elapsed = now.duration_since(fps_window_start);
        if window_elapsed >= Duration::from_secs(1) {
            current_fps =
                (fps_frames_in_window as f32 / window_elapsed.as_secs_f32()).round() as u32;
            fps_frames_in_window = 0;
            fps_window_start = now;
        }

        if animates_every_frame(&shell_state) || input_event_this_frame {
            surface.mark_dirty();
        }
        if surface.is_dirty() {
            let now = chrono::Local::now();
            let status = StatusInfo {
                hour: now.hour() as u8,
                minute: now.minute() as u8,
                battery: power::battery_fraction(),
                // The Machine has no wifi hardware to query — SDL exposes
                // no such API either way, so this is always false rather
                // than a fabricated reading.
                wifi: false,
            };
            draw_screen(
                &mut surface,
                &shell_state,
                &carts,
                &port_entries,
                &app.core.config,
                current_fps,
                &status,
            );
            surface.mark_clean();
        }

        display.present(
            &mut console_texture,
            &mut shell_texture,
            &app.core.screen,
            &app.core.vm,
            shell_state.settings().scaling,
            shell_state.settings().aspect,
            surface.rgba(),
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{check_mod_manifest, load_settings, save_settings};
    use crate::platform::scaling::{AspectMode, ScaleMode};
    use crate::shell::settings::Settings;

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

    #[test]
    fn saved_settings_round_trip_through_load() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.toml");
        let settings = Settings {
            scaling: ScaleMode::Integer2x,
            aspect: AspectMode::Stretch,
            show_fps: true,
            master_volume: 42,
            sfx_volume: 7,
            music_volume: 99,
        };
        save_settings(&path, &settings);
        assert_eq!(load_settings(&path), settings);
    }

    #[test]
    fn a_missing_settings_file_loads_as_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("does-not-exist.toml");
        assert_eq!(load_settings(&path), Settings::default());
    }

    #[test]
    fn a_corrupt_settings_file_loads_as_default_instead_of_panicking() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.toml");
        std::fs::write(&path, "not valid toml {{{").expect("write garbage");
        assert_eq!(load_settings(&path), Settings::default());
    }
}
