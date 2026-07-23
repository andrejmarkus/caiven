//! StudioApp: eframe application state — headless console core, cart state,
//! tab selection and per-frame VM stepping + framebuffer texture upload.

use super::{
    browser_panel, cart, code_panel, command_palette, debug_panel, export, game_panel, help_panel,
    map_panel, menu_bar, meta_panel, music_panel, palette_panel, recent, sfx_panel, sprite_panel,
    templates, theme, toolbar, welcome_panel,
};
use crate::app::cart_io::{self, CartMeta};
use anyhow::Result;
use caiven_vm::input::Button;
use caiven_vm::runtime::ConsoleCore;
use log::info;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Code,
    Sprite,
    Map,
    Sfx,
    Music,
    Palette,
    Meta,
    Browser,
    Help,
}

impl Tab {
    pub const ALL: [Tab; 9] = [
        Tab::Code,
        Tab::Sprite,
        Tab::Map,
        Tab::Sfx,
        Tab::Music,
        Tab::Palette,
        Tab::Meta,
        Tab::Browser,
        Tab::Help,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Code => "CODE",
            Tab::Sprite => "SPRITE",
            Tab::Map => "MAP",
            Tab::Sfx => "SFX",
            Tab::Music => "MUSIC",
            Tab::Palette => "PALETTE",
            Tab::Meta => "META",
            Tab::Browser => "BROWSER",
            Tab::Help => "HELP",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Stopped,
    Running,
    Paused,
}

pub struct SourceFile {
    pub path: PathBuf,
    pub text: String,
    pub dirty: bool,
}

/// GIF export: 3 seconds of VM frames at the fixed 60Hz tick rate, sampled
/// down to 30fps output (the VM's own frame timing already runs
/// wall-clock-accurate via `ConsoleCore::frame_steps`, so sampling every Nth
/// tick — rather than every eframe UI repaint — keeps the recording's speed
/// correct regardless of display refresh rate).
const GIF_RECORD_VM_FRAMES: usize = 180;
const GIF_SAMPLE_EVERY: usize = 2;
const GIF_DELAY_MS: u64 = 1000 / 30;

struct GifRecording {
    frames: Vec<Vec<u8>>,
    vm_frame_count: usize,
}

/// A project action deferred behind the unsaved-changes confirmation modal.
enum PendingAction {
    New(&'static str),
    Open(PathBuf),
    Close,
    Exit,
}

pub struct StudioApp {
    core: ConsoleCore,
    cart: Option<CartMeta>,
    source: Option<SourceFile>,
    tab: Tab,
    run_state: RunState,
    game_tex: Option<egui::TextureHandle>,
    compose_buf: Vec<u8>,
    status: String,
    status_is_error: bool,
    code: code_panel::CodeState,
    sprite: sprite_panel::SpriteState,
    map: map_panel::MapState,
    palette: palette_panel::PaletteState,
    sfx: sfx_panel::SfxState,
    music: music_panel::MusicState,
    browser: browser_panel::BrowserState,
    debug: debug_panel::DebugState,
    help: help_panel::HelpState,
    cmd_palette: command_palette::PaletteState,
    gif_recording: Option<GifRecording>,
    pending_action: Option<PendingAction>,
    recent: Vec<PathBuf>,
    last_title: String,
    /// Set right before re-issuing `ViewportCommand::Close` after the user
    /// already answered the unsaved-changes modal, so the close-requested
    /// check below doesn't treat our own follow-up close as a fresh request
    /// and loop back into the same modal forever.
    exit_confirmed: bool,
}

impl StudioApp {
    pub fn new(cc: &eframe::CreationContext<'_>, file: Option<PathBuf>) -> Result<Self> {
        theme::apply(&cc.egui_ctx);

        let mut app = Self {
            core: ConsoleCore::new()?,
            cart: None,
            source: None,
            tab: Tab::Code,
            run_state: RunState::Stopped,
            game_tex: None,
            compose_buf: Vec::new(),
            status: "no cart loaded — caiven-studio edit <file.cav>".into(),
            status_is_error: false,
            code: code_panel::CodeState::default(),
            sprite: sprite_panel::SpriteState::default(),
            map: map_panel::MapState::default(),
            palette: palette_panel::PaletteState::default(),
            sfx: sfx_panel::SfxState::default(),
            music: music_panel::MusicState::default(),
            browser: browser_panel::BrowserState::default(),
            debug: debug_panel::DebugState::default(),
            help: help_panel::HelpState::default(),
            cmd_palette: command_palette::PaletteState::default(),
            gif_recording: None,
            pending_action: None,
            recent: recent::load(),
            last_title: String::new(),
            exit_confirmed: false,
        };

        if let Some(path) = file
            && let Err(e) = app.open_file(&path)
        {
            app.set_status(format!("{e:#}"), true);
        }
        // `open_file` starts the game running (the normal behavior when
        // opening a cart mid-session); on startup we want it loaded but
        // paused until the user hits run. `_init()` may already have
        // triggered audio during that load, so silence it too.
        app.run_state = RunState::Stopped;
        app.core.vm.stop_audio();

        Ok(app)
    }

    fn set_status(&mut self, msg: impl Into<String>, is_error: bool) {
        self.status = msg.into();
        self.status_is_error = is_error;
    }

    /// Opens a `.cav` cart file. Returns `Err` only for hard failures (I/O,
    /// bad cart); a cart whose embedded Lua fails to run still opens in the
    /// editor with the error shown, so it can be fixed in place.
    fn open_file(&mut self, path: &std::path::Path) -> Result<()> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "cav" {
            anyhow::bail!("unsupported file type: {} (expected .cav)", ext);
        }
        let meta = cart::load_cart(&mut self.core.vm, path, &self.core.input, &self.core.font)?;
        info!("studio: cart loaded from {}", path.display());
        // Show the embedded Lua source in the code tab too, so it's
        // viewable/editable — edits get folded back into `meta.lua_source`
        // on save().
        self.source = meta.lua_source.clone().map(|text| SourceFile {
            path: path.to_path_buf(),
            text,
            dirty: false,
        });
        self.cart = Some(meta);
        self.code.error = None;
        self.run_state = RunState::Running;
        self.set_status(format!("loaded {}", path.display()), false);
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            self.browser.set_scan_dir(dir.to_path_buf());
        }
        self.debug.on_cart_loaded(path);
        recent::push(&mut self.recent, path);
        Ok(())
    }

    /// Starts editing a brand-new cart seeded with `source` (the blank
    /// stub, or one of `templates::TEMPLATES`). Not yet on disk — `Ctrl+S`
    /// writes it to a free `untitled*.cav` name in the browser's current
    /// folder.
    fn new_cart_from(&mut self, source: &str) {
        self.core.reset_vm();
        let path = free_untitled_path(self.browser.scan_dir());
        self.cart = Some(CartMeta {
            path: path.clone(),
            header: caiven_cart::CartHeader::default_for("untitled"),
            program: Vec::new(),
            sections: cart::default_section_layout(),
            lua_source: Some(source.to_string()),
        });
        self.source = Some(SourceFile {
            path,
            text: source.to_string(),
            dirty: true,
        });
        self.code.error = None;
        self.run_source();
        self.tab = Tab::Code;
    }

    /// Compiles the current editor buffer and (re)starts the game.
    fn run_source(&mut self) {
        let Some(src) = &self.source else {
            self.set_status("no .lua source loaded", true);
            return;
        };
        match cart::compile_lua_into_vm(
            &mut self.core.vm,
            &src.text,
            &self.core.input,
            &self.core.font,
        ) {
            Ok(()) => {
                let name = src.path.display().to_string();
                self.code.error = None;
                self.run_state = RunState::Running;
                self.set_status(format!("compiled {name}"), false);
            }
            Err(e) => {
                let first = e
                    .message
                    .lines()
                    .next()
                    .unwrap_or("compile error")
                    .to_string();
                self.code.error = Some(e);
                self.run_state = RunState::Stopped;
                self.core.vm.stop_audio();
                self.set_status(format!("compile error: {first}"), true);
            }
        }
    }

    /// Starts the game running, recompiling first if the source was never
    /// compiled (or last failed to). Shared by the toolbar's RUN button and
    /// the command palette's "Run" entry.
    fn run_or_resume(&mut self) {
        if self.source.is_some() && self.run_state == RunState::Stopped {
            self.run_source();
        } else if self.cart.is_some() || self.source.is_some() {
            self.run_state = RunState::Running;
        } else {
            self.set_status("no cart loaded", true);
        }
    }

    fn pause(&mut self) {
        self.run_state = RunState::Paused;
        self.core.vm.stop_audio();
    }

    fn reset(&mut self) {
        if self.source.is_some() {
            self.run_source();
            return;
        }
        let Some(path) = self.cart.as_ref().map(|c| c.path.clone()) else {
            self.set_status("nothing to reset", true);
            return;
        };
        match self.open_file(&path) {
            Ok(()) => self.set_status(format!("reset {}", path.display()), false),
            Err(e) => self.set_status(format!("{e:#}"), true),
        }
    }

    fn save(&mut self) {
        let Some(meta) = &mut self.cart else {
            self.set_status("nothing to save", true);
            return;
        };
        if let Some(src) = &self.source {
            meta.lua_source = Some(src.text.clone());
        }
        let result = cart_io::save(&self.core.vm, meta);
        let path = meta.path.clone();
        match result {
            Ok(()) => {
                if let Some(src) = &mut self.source {
                    src.dirty = false;
                }
                self.set_status(format!("saved {}", path.display()), false)
            }
            Err(e) => self.set_status(format!("save failed: {e:#}"), true),
        }
    }

    /// Saves the current cart under a new path, then behaves like `save()`.
    fn save_as(&mut self, path: PathBuf) {
        let Some(meta) = &mut self.cart else {
            self.set_status("nothing to save", true);
            return;
        };
        meta.path = path.clone();
        if let Some(src) = &mut self.source {
            src.path = path.clone();
        }
        self.save();
        recent::push(&mut self.recent, &path);
    }

    /// Unloads the current cart back to the empty/Browser state.
    fn close_cart(&mut self) {
        self.cart = None;
        self.source = None;
        self.core.reset_vm();
        self.run_state = RunState::Stopped;
        self.tab = Tab::Browser;
        self.set_status("no cart loaded — caiven-studio edit <file.cav>", false);
    }

    fn is_dirty(&self) -> bool {
        self.source.as_ref().is_some_and(|s| s.dirty)
    }

    /// Runs `action` now if there's nothing unsaved, otherwise defers it
    /// behind the unsaved-changes confirmation modal.
    fn guard(&mut self, action: PendingAction) {
        if self.is_dirty() {
            self.pending_action = Some(action);
        } else {
            self.run_pending(action);
        }
    }

    fn request_new(&mut self) {
        self.guard(PendingAction::New(templates::BLANK));
    }

    fn request_new_template(&mut self, source: &'static str) {
        self.guard(PendingAction::New(source));
    }

    fn request_open(&mut self, path: PathBuf) {
        self.guard(PendingAction::Open(path));
    }

    fn request_close(&mut self) {
        self.guard(PendingAction::Close);
    }

    fn request_exit(&mut self, ctx: &egui::Context) {
        if self.is_dirty() {
            self.pending_action = Some(PendingAction::Exit);
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn run_pending(&mut self, action: PendingAction) {
        match action {
            PendingAction::New(source) => self.new_cart_from(source),
            PendingAction::Open(path) => match self.open_file(&path) {
                Ok(()) => self.tab = Tab::Code,
                Err(e) => self.set_status(format!("{e:#}"), true),
            },
            PendingAction::Close => self.close_cart(),
            PendingAction::Exit => {}
        }
    }

    /// Renders the "Save changes to X?" modal if a project action is
    /// deferred behind unsaved changes.
    fn show_pending_modal(&mut self, ctx: &egui::Context) {
        let Some(action) = &self.pending_action else {
            return;
        };
        let is_exit = matches!(action, PendingAction::Exit);
        let name = self.cart_name();

        let modal = egui::Modal::new(egui::Id::new("unsaved_changes")).show(ctx, |ui| {
            ui.set_width(280.0);
            ui.label(format!("Save changes to {name}?"));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let save = ui.button("Save").clicked();
                let discard = ui.button("Don't Save").clicked();
                let cancel = ui.button("Cancel").clicked();
                (save, discard, cancel)
            })
            .inner
        });

        let (save, discard, cancel) = modal.inner;
        if save {
            self.save();
            let action = self.pending_action.take().unwrap();
            if is_exit {
                self.exit_confirmed = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                self.run_pending(action);
            }
        } else if discard {
            let action = self.pending_action.take().unwrap();
            if is_exit {
                self.exit_confirmed = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                self.run_pending(action);
            }
        } else if cancel || modal.should_close() {
            self.pending_action = None;
        }
    }

    fn route_game_input(&mut self, ctx: &egui::Context) {
        use egui::Key;
        const BINDINGS: [(Button, &[Key]); 6] = [
            (Button::Up, &[Key::ArrowUp, Key::W]),
            (Button::Down, &[Key::ArrowDown, Key::S]),
            (Button::Left, &[Key::ArrowLeft, Key::A]),
            (Button::Right, &[Key::ArrowRight, Key::D]),
            (Button::A, &[Key::J]),
            (Button::B, &[Key::K]),
        ];

        let editors_have_keyboard =
            self.run_state != RunState::Running || ctx.wants_keyboard_input();
        ctx.input(|i| {
            for (button, keys) in BINDINGS {
                let pressed =
                    !editors_have_keyboard && keys.iter().any(|k| i.keys_down.contains(k));
                self.core.input.set_button(button, pressed);
            }
        });
    }

    fn step_vm(&mut self) {
        let steps = self.core.frame_steps();
        if self.run_state != RunState::Running {
            // Game stopped/paused: keep SFX/music editor previews audible.
            for _ in 0..steps {
                self.core.vm.tick_audio_players();
            }
            return;
        }

        let bps = self.debug.dbg.breakpoints().to_vec();
        let mut outcome = caiven_vm::LuaRunOutcome::Completed;
        for _ in 0..steps {
            outcome = self.core.run_frame_lua_bp(&bps);
            if self.gif_recording.is_some() {
                self.record_gif_frame();
            }
            if !matches!(outcome, caiven_vm::LuaRunOutcome::Completed) {
                break;
            }
        }
        match outcome {
            caiven_vm::LuaRunOutcome::Completed => {}
            caiven_vm::LuaRunOutcome::Breakpoint(line) => {
                self.run_state = RunState::Paused;
                self.core.vm.stop_audio();
                self.debug.on_break(line);
                self.debug.last_error = None;
                self.set_status(format!("breakpoint hit at line {line}"), false);
            }
            caiven_vm::LuaRunOutcome::Error(line, message) => {
                self.run_state = RunState::Paused;
                self.core.vm.stop_audio();
                self.set_status(format!("lua error: {message}"), true);
                self.debug.last_error = Some(super::debug_panel::LuaError { line, message });
            }
        }
    }

    /// Switches to the Code tab and scrolls to the last runtime error's
    /// source line, from the game view's error-overlay button.
    fn jump_to_last_error(&mut self) {
        let Some(line) = self.debug.last_error.as_ref().and_then(|e| e.line) else {
            return;
        };
        if let Some(src) = &self.source {
            self.code.goto_line(&src.text, line);
        }
        self.tab = Tab::Code;
    }

    /// Splices `text` into the code editor at its last known cursor
    /// position and switches to the Code tab — used by both the API
    /// reference panel and the command palette's "insert builtin" entries.
    fn insert_at_cursor(&mut self, ctx: &egui::Context, text: &str) {
        let Some(src) = &mut self.source else {
            self.set_status("open a cart to insert code", true);
            return;
        };
        code_panel::insert_at_cursor(ctx, src, text);
        self.tab = Tab::Code;
    }

    /// Composites the current world+UI framebuffers into one RGBA frame —
    /// the exact image shown in the game preview. Shared by the preview
    /// texture upload and screenshot/GIF export, so exports always match
    /// what's on screen.
    fn compose_frame(&self) -> Vec<u8> {
        let w = self.core.config.width as usize;
        let h = self.core.config.height as usize;
        let mut buf = vec![0u8; w * h * 4];
        self.core.screen.construct(
            &mut buf,
            self.core.vm.world_pixels(),
            self.core.vm.ui_pixels(),
        );
        for px in buf.chunks_exact_mut(4) {
            px[3] = 255;
        }
        buf
    }

    /// Called once per VM tick (not per UI repaint) while a GIF recording is
    /// in progress, so playback speed tracks the VM's real 60Hz timing
    /// regardless of the display's refresh rate.
    fn record_gif_frame(&mut self) {
        let buf = self.compose_frame();
        let done = {
            let Some(rec) = self.gif_recording.as_mut() else {
                return;
            };
            rec.vm_frame_count += 1;
            if rec.vm_frame_count % GIF_SAMPLE_EVERY == 0 {
                rec.frames.push(buf);
            }
            rec.vm_frame_count >= GIF_RECORD_VM_FRAMES
        };
        if done {
            let frames = self
                .gif_recording
                .take()
                .map(|r| r.frames)
                .unwrap_or_default();
            self.finish_gif_recording(frames);
        }
    }

    fn export_screenshot(&mut self) {
        if self.cart.is_none() && self.source.is_none() {
            self.set_status("no cart loaded", true);
            return;
        }
        let buf = self.compose_frame();
        let (w, h) = (self.core.config.width, self.core.config.height);
        match export::encode_png(w, h, &buf) {
            Ok(png) => self.save_export_bytes(&png, "png", "PNG Image"),
            Err(e) => self.set_status(format!("export failed: {e:#}"), true),
        }
    }

    fn start_gif_recording(&mut self) {
        if self.cart.is_none() && self.source.is_none() {
            self.set_status("no cart loaded", true);
            return;
        }
        if self.run_state != RunState::Running {
            self.run_or_resume();
        }
        self.gif_recording = Some(GifRecording {
            frames: Vec::new(),
            vm_frame_count: 0,
        });
        self.set_status("recording 3s GIF...", false);
    }

    fn finish_gif_recording(&mut self, frames: Vec<Vec<u8>>) {
        if frames.is_empty() {
            self.set_status("GIF recording produced no frames", true);
            return;
        }
        let (w, h) = (self.core.config.width, self.core.config.height);
        match export::encode_gif(w, h, &frames, GIF_DELAY_MS) {
            Ok(bytes) => self.save_export_bytes(&bytes, "gif", "GIF Image"),
            Err(e) => self.set_status(format!("GIF export failed: {e:#}"), true),
        }
    }

    /// Prompts for a destination and writes `bytes` there — shared tail end
    /// of both export actions.
    fn save_export_bytes(&mut self, bytes: &[u8], ext: &str, filter_name: &str) {
        let stem = self.cart_name();
        let stem = stem.strip_suffix(".cav").unwrap_or(&stem);
        let Some(path) = rfd::FileDialog::new()
            .add_filter(filter_name, &[ext])
            .set_file_name(format!("{stem}.{ext}"))
            .save_file()
        else {
            return;
        };
        match std::fs::write(&path, bytes) {
            Ok(()) => self.set_status(format!("exported {}", path.display()), false),
            Err(e) => self.set_status(format!("export failed: {e}"), true),
        }
    }

    fn update_game_texture(&mut self, ctx: &egui::Context) {
        let w = self.core.config.width as usize;
        let h = self.core.config.height as usize;
        self.compose_buf = self.compose_frame();
        let image = egui::ColorImage::from_rgba_unmultiplied([w, h], &self.compose_buf);
        match &mut self.game_tex {
            Some(tex) => tex.set(image, egui::TextureOptions::NEAREST),
            None => {
                self.game_tex = Some(ctx.load_texture("game", image, egui::TextureOptions::NEAREST))
            }
        }
    }

    fn window_title(&self) -> String {
        let dirty = if self.is_dirty() { " •" } else { "" };
        format!("{}{} — Caiven Studio", self.cart_name(), dirty)
    }

    fn cart_name(&self) -> String {
        let path = self
            .cart
            .as_ref()
            .map(|c| c.path.as_path())
            .or_else(|| self.source.as_ref().map(|s| s.path.as_path()));
        match path {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string()),
            None => "<no cart>".into(),
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // Shift+Ctrl+S must be checked before plain Ctrl+S: `consume_key`
        // ignores extra modifiers, so the plain pattern would also match it.
        let save_as = ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::S)
        });
        if save_as {
            if let Some(path) = pick_save_as_path(&self.cart_name()) {
                self.save_as(path);
            }
        } else {
            let save = ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::S));
            if save {
                self.save();
            }
        }
        let run = ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::R));
        if run {
            self.run_source();
        }
        let new = ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::N));
        if new {
            self.request_new();
        }
        let open = ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::O));
        if open && let Some(path) = pick_open_path() {
            self.request_open(path);
        }

        ctx.input(|i| {
            let f_keys = [
                (egui::Key::F1, Tab::Code),
                (egui::Key::F2, Tab::Sprite),
                (egui::Key::F3, Tab::Map),
                (egui::Key::F4, Tab::Sfx),
                (egui::Key::F5, Tab::Music),
                (egui::Key::F6, Tab::Palette),
                (egui::Key::F7, Tab::Meta),
                (egui::Key::F8, Tab::Browser),
                (egui::Key::F9, Tab::Help),
            ];
            for (key, tab) in f_keys {
                if i.key_pressed(key) {
                    self.tab = tab;
                }
            }
        });
    }
}

impl eframe::App for StudioApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);
        self.route_game_input(ctx);
        self.browser.poll(ctx);
        if let Some(path) = self.browser.take_pending_load() {
            self.request_open(path);
        }
        if self.browser.take_pending_new() {
            self.request_new();
        }
        self.step_vm();
        self.update_game_texture(ctx);

        if self.pending_action.is_none()
            && self.is_dirty()
            && !self.exit_confirmed
            && ctx.input(|i| i.viewport().close_requested())
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending_action = Some(PendingAction::Exit);
        }

        match menu_bar::show(ctx, &self.recent) {
            menu_bar::MenuAction::New => self.request_new(),
            menu_bar::MenuAction::Open => {
                if let Some(path) = pick_open_path() {
                    self.request_open(path);
                }
            }
            menu_bar::MenuAction::OpenRecent(path) => self.request_open(path),
            menu_bar::MenuAction::ClearRecent => {
                self.recent.clear();
                recent::save(&self.recent);
            }
            menu_bar::MenuAction::Save => self.save(),
            menu_bar::MenuAction::SaveAs => {
                if let Some(path) = pick_save_as_path(&self.cart_name()) {
                    self.save_as(path);
                }
            }
            menu_bar::MenuAction::ExportScreenshot => self.export_screenshot(),
            menu_bar::MenuAction::ExportGif => self.start_gif_recording(),
            menu_bar::MenuAction::Close => self.request_close(),
            menu_bar::MenuAction::Exit => self.request_exit(ctx),
            menu_bar::MenuAction::None => {}
        }
        self.show_pending_modal(ctx);

        let fps = ctx.input(|i| 1.0 / i.stable_dt.max(1e-6));
        let action = toolbar::show(ctx, &self.cart_name(), self.run_state, fps);
        match action {
            toolbar::ToolbarAction::Run => self.run_or_resume(),
            toolbar::ToolbarAction::Pause => self.pause(),
            toolbar::ToolbarAction::Reset => self.reset(),
            toolbar::ToolbarAction::Save => self.save(),
            toolbar::ToolbarAction::None => {}
        }

        if let Some(action) = command_palette::show(
            ctx,
            &mut self.cmd_palette,
            self.run_state == RunState::Running,
        ) {
            match action {
                command_palette::PaletteAction::New => self.request_new(),
                command_palette::PaletteAction::NewTemplate(src) => self.request_new_template(src),
                command_palette::PaletteAction::Open => {
                    if let Some(path) = pick_open_path() {
                        self.request_open(path);
                    }
                }
                command_palette::PaletteAction::Save => self.save(),
                command_palette::PaletteAction::SaveAs => {
                    if let Some(path) = pick_save_as_path(&self.cart_name()) {
                        self.save_as(path);
                    }
                }
                command_palette::PaletteAction::Close => self.request_close(),
                command_palette::PaletteAction::Exit => self.request_exit(ctx),
                command_palette::PaletteAction::Run => self.run_or_resume(),
                command_palette::PaletteAction::Pause => self.pause(),
                command_palette::PaletteAction::Reset => self.reset(),
                command_palette::PaletteAction::SwitchTab(tab) => self.tab = tab,
                command_palette::PaletteAction::InsertBuiltin(text) => {
                    self.insert_at_cursor(ctx, &text)
                }
                command_palette::PaletteAction::ExportScreenshot => self.export_screenshot(),
                command_palette::PaletteAction::ExportGif => self.start_gif_recording(),
            }
        }

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                for tab in Tab::ALL {
                    ui.selectable_value(&mut self.tab, tab, tab.label());
                }
            });
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            let color = if self.status_is_error {
                theme::ERROR
            } else {
                theme::DIM
            };
            ui.colored_label(color, &self.status);
        });

        egui::SidePanel::right("game")
            .default_width(560.0)
            .min_width(160.0)
            .show(ctx, |ui| {
                egui::TopBottomPanel::bottom("debug")
                    .resizable(true)
                    .default_height(300.0)
                    .show_inside(ui, |ui| {
                        debug_panel::show(ui, &mut self.debug, &mut self.core, &mut self.run_state);
                    });
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    let action = game_panel::show(
                        ui,
                        self.game_tex.as_ref(),
                        self.core.config.width as f32,
                        self.run_state,
                        self.debug.last_error.as_ref(),
                    );
                    if action == game_panel::GamePanelAction::JumpToError {
                        self.jump_to_last_error();
                    }
                });
            });

        let mut help_insert = None;
        let welcome_action = egui::CentralPanel::default()
            .show(ctx, |ui| match self.tab {
                Tab::Code => match &mut self.source {
                    Some(src) => {
                        code_panel::show(ui, &mut self.code, src, &mut self.debug.dbg);
                        welcome_panel::WelcomeAction::None
                    }
                    None => welcome_panel::show(ui, &self.recent),
                },
                Tab::Sprite => {
                    sprite_panel::show(ui, &mut self.sprite, &mut self.core.vm);
                    welcome_panel::WelcomeAction::None
                }
                Tab::Map => {
                    map_panel::show(ui, &mut self.map, &mut self.core.vm);
                    welcome_panel::WelcomeAction::None
                }
                Tab::Palette => {
                    palette_panel::show(ui, &mut self.palette, &mut self.core.vm);
                    welcome_panel::WelcomeAction::None
                }
                Tab::Sfx => {
                    sfx_panel::show(ui, &mut self.sfx, &mut self.core.vm);
                    welcome_panel::WelcomeAction::None
                }
                Tab::Music => {
                    music_panel::show(ui, &mut self.music, &mut self.core.vm);
                    welcome_panel::WelcomeAction::None
                }
                Tab::Meta => {
                    meta_panel::show(ui, self.cart.as_mut());
                    welcome_panel::WelcomeAction::None
                }
                Tab::Browser => {
                    browser_panel::show(ui, &mut self.browser, ctx, self.cart.as_ref());
                    welcome_panel::WelcomeAction::None
                }
                Tab::Help => {
                    if let help_panel::HelpAction::Insert(text) =
                        help_panel::show(ui, &mut self.help)
                    {
                        help_insert = Some(text);
                    }
                    welcome_panel::WelcomeAction::None
                }
            })
            .inner;

        if let Some(text) = help_insert {
            self.insert_at_cursor(ctx, &text);
        }

        match welcome_action {
            welcome_panel::WelcomeAction::None => {}
            welcome_panel::WelcomeAction::NewBlank => self.request_new(),
            welcome_panel::WelcomeAction::NewTemplate(source) => self.request_new_template(source),
            welcome_panel::WelcomeAction::Open => {
                if let Some(path) = pick_open_path() {
                    self.request_open(path);
                }
            }
            welcome_panel::WelcomeAction::OpenRecent(path) => self.request_open(path),
        }

        let title = self.window_title();
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }

        // Running needs a steady redraw to animate the game view; idle
        // (Stopped/Paused) doesn't burn a full render loop for a static
        // editor UI — egui already repaints on input, text-cursor blink,
        // etc. on its own.
        if self.run_state == RunState::Running {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
    }
}

/// Opens a native "Open" dialog filtered to `.cav` carts.
fn pick_open_path() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Caiven cart", &["cav"])
        .pick_file()
}

/// Opens a native "Save As" dialog filtered to `.cav` carts, defaulting to
/// the current cart's name.
fn pick_save_as_path(current_name: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Caiven cart", &["cav"])
        .set_file_name(current_name)
        .save_file()
}

/// First non-colliding `untitled.cav` / `untitled-2.cav` / ... path in `dir`.
fn free_untitled_path(dir: &std::path::Path) -> PathBuf {
    let candidate = dir.join("untitled.cav");
    if !candidate.exists() {
        return candidate;
    }
    let mut n = 2;
    loop {
        let candidate = dir.join(format!("untitled-{n}.cav"));
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}
