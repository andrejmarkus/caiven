//! StudioApp: eframe application state — headless console core, cart state,
//! tab selection and per-frame VM stepping + framebuffer texture upload.

use super::{
    browser_panel, cart, code_panel, command_palette, debug_panel, export, game_panel, help_panel,
    map_panel, menu_bar, meta_panel, music_panel, palette_panel, project_tree, recent, sfx_panel,
    sprite_panel, templates, theme, toolbar, welcome_panel,
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
    /// Every editable Lua buffer for the current cart: `sources[0]` is
    /// always the entry file (`main.lua`), the rest are sibling `.lua`
    /// modules `require()`d by it. Empty when no cart is loaded.
    sources: Vec<SourceFile>,
    /// Index into `sources` shown in the Code tab.
    active_source: usize,
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
    /// `Some` while the "new module" naming popup is open; holds the text
    /// field's live buffer.
    new_module_name: Option<String>,
    last_title: String,
    /// Set right before re-issuing `ViewportCommand::Close` after the user
    /// already answered the unsaved-changes modal, so the close-requested
    /// check below doesn't treat our own follow-up close as a fresh request
    /// and loop back into the same modal forever.
    exit_confirmed: bool,
    /// Path of a `.cav` the user tried to open directly — Studio only edits
    /// project directories, so this drives a confirmation modal asking to
    /// unpack it first instead of opening it.
    unpack_prompt: Option<PathBuf>,
}

impl StudioApp {
    pub fn new(cc: &eframe::CreationContext<'_>, file: Option<PathBuf>) -> Result<Self> {
        theme::apply(&cc.egui_ctx);

        let mut app = Self {
            core: ConsoleCore::new()?,
            cart: None,
            sources: Vec::new(),
            active_source: 0,
            tab: Tab::Code,
            run_state: RunState::Stopped,
            game_tex: None,
            compose_buf: Vec::new(),
            status: "no cart loaded — caiven-studio edit <project dir | .cav>".into(),
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
            new_module_name: None,
            last_title: String::new(),
            exit_confirmed: false,
            unpack_prompt: None,
        };

        if let Some(path) = file {
            if wants_unpack_prompt(&path) {
                app.unpack_prompt = Some(path);
            } else if let Err(e) = app.open_file(&path) {
                app.set_status(format!("{e:#}"), true);
            }
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

    fn active_source_mut(&mut self) -> Option<&mut SourceFile> {
        self.sources.get_mut(self.active_source)
    }

    /// The project directory `self.sources` were loaded from, for bundling
    /// sibling modules — `None` for a single-buffer `.cav`-sourced cart,
    /// which has no directory of sibling `.lua` files to speak of.
    fn project_dir(&self) -> Option<&std::path::Path> {
        let meta = self.cart.as_ref()?;
        if meta.path.extension().and_then(|e| e.to_str()) == Some("cav") {
            None
        } else {
            Some(meta.path.as_path())
        }
    }

    /// `self.sources[1..]` relativized to the project dir, as
    /// `save_project`/`export_binary` expect. Empty for a `.cav`-sourced
    /// cart (no project dir, no separate module files).
    fn module_rel_pairs(&self) -> Vec<(PathBuf, String)> {
        let Some(dir) = self.project_dir() else {
            return Vec::new();
        };
        self.sources
            .get(1..)
            .unwrap_or_default()
            .iter()
            .map(|s| {
                let rel = s.path.strip_prefix(dir).unwrap_or(&s.path).to_path_buf();
                (rel, s.text.clone())
            })
            .collect()
    }

    /// Opens a project directory (or its `caiven.toml`) or a binary `.cav`
    /// cart file. Returns `Err` only for hard failures (I/O, bad cart); a
    /// cart whose embedded Lua fails to run still opens in the editor with
    /// the error shown, so it can be fixed in place.
    fn open_file(&mut self, path: &std::path::Path) -> Result<()> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !caiven_cart::is_project(path) && ext != "cav" {
            anyhow::bail!(
                "unsupported file type: {} (expected a project dir or .cav)",
                ext
            );
        }
        let meta = cart::load_cart(&mut self.core.vm, path, &self.core.input, &self.core.font)?;
        info!("studio: cart loaded from {}", path.display());
        // Project dir: load the entry file and every sibling module as its
        // own editable buffer. Binary .cav: a single buffer from the
        // already-bundled LuaSource section (module structure doesn't
        // survive the binary format — see the `unpack` CLI command).
        self.sources = if caiven_cart::is_project(path) {
            cart::load_project_sources(path)?
        } else {
            meta.lua_source
                .clone()
                .map(|text| {
                    vec![SourceFile {
                        path: path.to_path_buf(),
                        text,
                        dirty: false,
                    }]
                })
                .unwrap_or_default()
        };
        self.active_source = 0;
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

    /// Prompts for a project folder (native picker — the chosen folder's
    /// name becomes the project name) and starts editing a brand-new cart
    /// seeded with `source` (the blank stub, or one of
    /// `templates::TEMPLATES`), writing it to disk immediately rather than
    /// deferring to the first `Ctrl+S`.
    fn new_cart_from(&mut self, source: &str) {
        let Some(dir) = rfd::FileDialog::new()
            .set_directory(self.browser.scan_dir())
            .pick_folder()
        else {
            self.set_status("new project cancelled", false);
            return;
        };
        if caiven_cart::is_project(&dir) {
            self.set_status(
                format!(
                    "{} is already a Caiven project — use Open instead",
                    dir.display()
                ),
                true,
            );
            return;
        }

        self.core.reset_vm();
        let title = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled".to_string());
        self.cart = Some(CartMeta {
            path: dir.clone(),
            header: caiven_cart::CartHeader::default_for(&title),
            program: Vec::new(),
            sections: cart::default_section_layout(),
            lua_source: Some(source.to_string()),
        });
        self.sources = vec![SourceFile {
            path: dir.join("main.lua"),
            text: source.to_string(),
            dirty: false,
        }];
        self.active_source = 0;
        self.code.error = None;
        self.save();
        recent::push(&mut self.recent, &dir);
        if let Some(parent) = dir.parent().filter(|p| !p.as_os_str().is_empty()) {
            self.browser.set_scan_dir(parent.to_path_buf());
        }
        self.run_source();
        self.tab = Tab::Code;
    }

    /// Compiles the entry buffer plus any sibling module buffers (bundled
    /// together, so `require()` resolves in the preview exactly like it
    /// will in the shipped cart) and (re)starts the game.
    fn run_source(&mut self) {
        if self.sources.is_empty() {
            self.set_status("no .lua source loaded", true);
            return;
        }
        let dir = self.project_dir().map(std::path::Path::to_path_buf);
        match cart::compile_sources_into_vm(
            &mut self.core.vm,
            dir.as_deref(),
            &self.sources,
            &self.core.input,
            &self.core.font,
        ) {
            Ok(()) => {
                let name = self.sources[0].path.display().to_string();
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
        if !self.sources.is_empty() && self.run_state == RunState::Stopped {
            self.run_source();
        } else if self.cart.is_some() || !self.sources.is_empty() {
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
        if !self.sources.is_empty() {
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
        let modules = self.module_rel_pairs();
        let entry_text = self.sources.first().map(|s| s.text.clone());
        let Some(meta) = &mut self.cart else {
            self.set_status("nothing to save", true);
            return;
        };
        if let Some(text) = entry_text {
            meta.lua_source = Some(text);
        }
        let result = cart_io::save(&self.core.vm, meta, &modules);
        let path = meta.path.clone();
        match result {
            Ok(()) => {
                for src in &mut self.sources {
                    src.dirty = false;
                }
                self.set_status(format!("saved {}", path.display()), false)
            }
            Err(e) => self.set_status(format!("save failed: {e:#}"), true),
        }
    }

    /// Saves the current cart under a new path, then behaves like `save()`.
    /// Every source buffer's path is re-rooted under the new location,
    /// preserving its relative position (`ui/panel.lua` stays
    /// `ui/panel.lua` under the new project dir).
    fn save_as(&mut self, path: PathBuf) {
        let Some(meta) = &mut self.cart else {
            self.set_status("nothing to save", true);
            return;
        };
        let old_dir = meta.path.clone();
        meta.path = path.clone();
        for src in &mut self.sources {
            let rel = src
                .path
                .strip_prefix(&old_dir)
                .unwrap_or(&src.path)
                .to_path_buf();
            src.path = path.join(rel);
        }
        self.save();
        recent::push(&mut self.recent, &path);
    }

    /// Builds a binary `.cav` distribution cartridge at `path` from the
    /// current project without changing where the project itself is saved.
    fn export_cartridge(&mut self, path: PathBuf) {
        let modules = self.module_rel_pairs();
        let entry_text = self.sources.first().map(|s| s.text.clone());
        let Some(meta) = &mut self.cart else {
            self.set_status("nothing to export", true);
            return;
        };
        if let Some(text) = entry_text {
            meta.lua_source = Some(text);
        }
        match cart_io::export_binary(&self.core.vm, meta, &path, &modules) {
            Ok(()) => self.set_status(format!("exported cartridge {}", path.display()), false),
            Err(e) => self.set_status(format!("export failed: {e:#}"), true),
        }
    }

    /// Prompts for a `.cav` file, then delegates to `unpack_from`.
    fn unpack_cartridge(&mut self) {
        let Some(src) = pick_unpack_source() else {
            self.set_status("unpack cancelled", false);
            return;
        };
        self.unpack_from(src);
    }

    /// Unpacks `src` (a binary `.cav`) into a project directory the user
    /// picks, then opens the freshly-unpacked project for editing.
    fn unpack_from(&mut self, src: PathBuf) {
        let Some(dest) = pick_unpack_dest() else {
            self.set_status("unpack cancelled", false);
            return;
        };
        match cart::unpack_cart(&src, &dest) {
            Ok(()) => {
                self.set_status(format!("unpacked to {}", dest.display()), false);
                if let Err(e) = self.open_file(&dest) {
                    self.set_status(format!("{e:#}"), true);
                }
            }
            Err(e) => self.set_status(format!("unpack failed: {e:#}"), true),
        }
    }

    /// Unloads the current cart back to the empty/Browser state.
    fn close_cart(&mut self) {
        self.cart = None;
        self.sources.clear();
        self.active_source = 0;
        self.core.reset_vm();
        self.run_state = RunState::Stopped;
        self.tab = Tab::Browser;
        self.set_status(
            "no cart loaded — caiven-studio edit <project dir | .cav>",
            false,
        );
    }

    fn is_dirty(&self) -> bool {
        self.sources.iter().any(|s| s.dirty)
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
            PendingAction::Open(path) => {
                if wants_unpack_prompt(&path) {
                    self.unpack_prompt = Some(path);
                } else {
                    match self.open_file(&path) {
                        Ok(()) => self.tab = Tab::Code,
                        Err(e) => self.set_status(format!("{e:#}"), true),
                    }
                }
            }
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

    /// Renders the confirmation modal shown when the user tries to open a
    /// distribution `.cav` directly — Studio only edits project
    /// directories, so this explains why and hands off to the unpack
    /// flow (which itself asks where to unpack) instead of opening it.
    fn show_unpack_prompt_modal(&mut self, ctx: &egui::Context) {
        let Some(cav_path) = self.unpack_prompt.clone() else {
            return;
        };
        let name = cav_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| cav_path.display().to_string());

        let modal = egui::Modal::new(egui::Id::new("unpack_prompt")).show(ctx, |ui| {
            ui.set_width(360.0);
            ui.label(format!("{name} is a distribution cartridge (.cav)."));
            ui.label(
                "Caiven Studio only edits project folders. Pick a folder to \
                 unpack it into, then it'll open there for editing.",
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let unpack = ui.button("Choose Folder & Unpack...").clicked();
                let cancel = ui.button("Cancel").clicked();
                (unpack, cancel)
            })
            .inner
        });

        let (unpack, cancel) = modal.inner;
        if unpack {
            self.unpack_prompt = None;
            self.unpack_from(cav_path);
        } else if cancel || modal.should_close() {
            self.unpack_prompt = None;
            self.set_status("open cancelled", false);
        }
    }

    /// Renders the Code tab: a file-tab strip over every open source buffer
    /// (entry first) plus a `+` to add a sibling `.lua` module, then the
    /// code editor for whichever buffer is active.
    fn show_code_tab(&mut self, ui: &mut egui::Ui) -> welcome_panel::WelcomeAction {
        if self.sources.is_empty() {
            return welcome_panel::show(ui, &self.recent);
        }
        if self.active_source >= self.sources.len() {
            self.active_source = 0;
        }

        ui.horizontal_wrapped(|ui| {
            for i in 0..self.sources.len() {
                let src = &self.sources[i];
                let name = src
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| src.path.display().to_string());
                let label = if src.dirty { format!("{name} *") } else { name };
                if ui
                    .selectable_label(i == self.active_source, label)
                    .clicked()
                {
                    self.active_source = i;
                }
            }
            let can_add_module = self.project_dir().is_some();
            if ui
                .add_enabled(can_add_module, egui::Button::new("+"))
                .on_hover_text("New module (require()-able .lua file)")
                .on_disabled_hover_text("Save the cart first — modules need a project dir")
                .clicked()
            {
                self.new_module_name = Some(String::new());
            }
        });
        ui.separator();

        code_panel::show(
            ui,
            &mut self.code,
            &mut self.sources[self.active_source],
            &mut self.debug.dbg,
        );
        welcome_panel::WelcomeAction::None
    }

    /// Renders the "New module name" popup opened by the Code tab's `+`
    /// button, and creates the buffer (not yet written to disk — it's
    /// written like any other dirty buffer on the next Save) on confirm.
    fn show_new_module_modal(&mut self, ctx: &egui::Context) {
        let Some(current) = self.new_module_name.clone() else {
            return;
        };
        let mut field = current;
        let modal = egui::Modal::new(egui::Id::new("new_module")).show(ctx, |ui| {
            ui.set_width(280.0);
            ui.label("New module name (e.g. ui/panel):");
            let resp = ui.text_edit_singleline(&mut field);
            resp.request_focus();
            let submitted = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let create_clicked = ui.button("Create").clicked();
                let cancel_clicked = ui.button("Cancel").clicked();
                (create_clicked || submitted, cancel_clicked)
            })
            .inner
        });
        let (create, cancel) = modal.inner;
        self.new_module_name = Some(field);

        if create {
            if let Some(name) = self.new_module_name.take() {
                self.create_module(&name);
            }
        } else if cancel || modal.should_close() {
            self.new_module_name = None;
        }
    }

    /// Adds a new empty, dirty `.lua` module buffer under the project dir
    /// and switches the Code tab to it. `name` may include `/` for a
    /// subfolder (e.g. `ui/panel`); `.lua` is appended if not already
    /// present.
    fn create_module(&mut self, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let Some(dir) = self.project_dir().map(std::path::Path::to_path_buf) else {
            self.set_status("modules need a project dir — save the cart first", true);
            return;
        };
        let rel = if name.ends_with(".lua") {
            PathBuf::from(name)
        } else {
            PathBuf::from(format!("{name}.lua"))
        };
        let path = dir.join(&rel);
        if self.sources.iter().any(|s| s.path == path) {
            self.set_status(format!("{} is already open", rel.display()), true);
            return;
        }
        self.sources.push(SourceFile {
            path,
            text: String::new(),
            dirty: true,
        });
        self.active_source = self.sources.len() - 1;
    }

    /// Opens a file clicked in the project tree: switches to it if it's
    /// already a loaded buffer (never re-reads an open buffer, so unsaved
    /// edits are never clobbered), loads `.lua` files fresh into a new
    /// buffer, or for a known asset file / `caiven.toml` just switches to
    /// its matching editor tab instead of trying to open it as text.
    fn open_tree_entry(&mut self, path: PathBuf) {
        if let Some(i) = self.sources.iter().position(|s| s.path == path) {
            self.active_source = i;
            self.tab = Tab::Code;
            return;
        }
        if path.extension().and_then(|e| e.to_str()) == Some("lua") {
            match std::fs::read_to_string(&path) {
                Ok(text) => {
                    self.sources.push(SourceFile {
                        path,
                        text,
                        dirty: false,
                    });
                    self.active_source = self.sources.len() - 1;
                    self.tab = Tab::Code;
                }
                Err(e) => self.set_status(format!("failed to open {}: {e}", path.display()), true),
            }
            return;
        }
        if let Some(tab) = asset_tab_for(&path) {
            self.tab = tab;
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
    /// source line, from the game view's error-overlay button. Reported
    /// line numbers only ever resolve against the entry buffer (module
    /// chunks compile separately and don't produce a parseable line — see
    /// `caiven_cart::bundle_lua`), so this always targets `sources[0]`.
    fn jump_to_last_error(&mut self) {
        let Some(line) = self.debug.last_error.as_ref().and_then(|e| e.line) else {
            return;
        };
        self.active_source = 0;
        if let Some(src) = self.sources.first() {
            self.code.goto_line(&src.text, line);
        }
        self.tab = Tab::Code;
    }

    /// Splices `text` into the active code buffer at its last known cursor
    /// position and switches to the Code tab — used by both the API
    /// reference panel and the command palette's "insert builtin" entries.
    fn insert_at_cursor(&mut self, ctx: &egui::Context, text: &str) {
        let Some(src) = self.active_source_mut() else {
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
        if self.cart.is_none() && self.sources.is_empty() {
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
        if self.cart.is_none() && self.sources.is_empty() {
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
            .or_else(|| self.sources.first().map(|s| s.path.as_path()));
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
        if let Some(src) = self.browser.take_pending_unpack() {
            self.unpack_from(src);
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
            menu_bar::MenuAction::ExportCartridge => {
                if let Some(path) = pick_export_cartridge_path(&self.cart_name()) {
                    self.export_cartridge(path);
                }
            }
            menu_bar::MenuAction::UnpackCartridge => self.unpack_cartridge(),
            menu_bar::MenuAction::Close => self.request_close(),
            menu_bar::MenuAction::Exit => self.request_exit(ctx),
            menu_bar::MenuAction::None => {}
        }
        self.show_pending_modal(ctx);
        self.show_unpack_prompt_modal(ctx);
        self.show_new_module_modal(ctx);

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
                command_palette::PaletteAction::ExportCartridge => {
                    if let Some(path) = pick_export_cartridge_path(&self.cart_name()) {
                        self.export_cartridge(path);
                    }
                }
                command_palette::PaletteAction::UnpackCartridge => self.unpack_cartridge(),
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

        let tree_root = self.project_dir().map(std::path::Path::to_path_buf);
        let has_cart = self.cart.is_some();
        let open_paths: Vec<PathBuf> = self.sources.iter().map(|s| s.path.clone()).collect();
        let active_path = self.sources.get(self.active_source).map(|s| s.path.clone());

        let mut tree_clicked = None;
        egui::SidePanel::left("project_tree")
            .resizable(true)
            .default_width(220.0)
            .min_width(140.0)
            .show(ctx, |ui| match &tree_root {
                Some(root) => {
                    let open_refs: Vec<&std::path::Path> =
                        open_paths.iter().map(PathBuf::as_path).collect();
                    tree_clicked = project_tree::show(ui, root, &open_refs, active_path.as_deref());
                }
                None if has_cart => {
                    ui.colored_label(
                        theme::DIM,
                        "Binary cart — no file tree (unpack to browse files)",
                    );
                }
                None => {
                    ui.colored_label(theme::DIM, "No project open");
                }
            });
        if let Some(path) = tree_clicked {
            self.open_tree_entry(path);
        }

        let mut help_insert = None;
        let welcome_action = egui::CentralPanel::default()
            .show(ctx, |ui| match self.tab {
                Tab::Code => self.show_code_tab(ui),
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

/// Maps a project tree click on a known asset file or the manifest to the
/// editor tab that edits it — these aren't opened as text buffers.
fn asset_tab_for(path: &std::path::Path) -> Option<Tab> {
    match path.file_name().and_then(|n| n.to_str())? {
        "sprites.png" | "sprites.hex" | "sprite_flags.hex" => Some(Tab::Sprite),
        "map.png" | "map.hex" => Some(Tab::Map),
        "palette.png" | "palette.hex" => Some(Tab::Palette),
        "sfx.hex" => Some(Tab::Sfx),
        "music.hex" => Some(Tab::Music),
        "caiven.toml" => Some(Tab::Meta),
        _ => None,
    }
}

/// Opens a native "Open" dialog picking a project directory. A `.cav` can
/// still be pointed at via the Browser tab, recent files, or the CLI's
/// `edit`/`unpack` — those funnel through the unpack-prompt modal instead
/// of opening it directly, since Studio only edits project directories.
fn pick_open_path() -> Option<PathBuf> {
    rfd::FileDialog::new().pick_folder()
}

/// Opens a native "Save As" dialog picking a destination project directory.
fn pick_save_as_path(_current_name: &str) -> Option<PathBuf> {
    rfd::FileDialog::new().pick_folder()
}

/// Opens a native "Export Cartridge" dialog filtered to `.cav`, defaulting
/// to the current cart's name — the distribution artifact built from the
/// project directory currently open.
fn pick_export_cartridge_path(current_name: &str) -> Option<PathBuf> {
    let stem = current_name.strip_suffix(".cav").unwrap_or(current_name);
    rfd::FileDialog::new()
        .add_filter("Caiven cart", &["cav"])
        .set_file_name(format!("{stem}.cav"))
        .save_file()
}

/// True for a `.cav` file path that isn't itself a project dir — Studio
/// can't open these directly, only after unpacking.
fn wants_unpack_prompt(path: &std::path::Path) -> bool {
    !caiven_cart::is_project(path) && path.extension().and_then(|e| e.to_str()) == Some("cav")
}

/// Opens a native "Unpack" dialog picking a binary `.cav` to convert into
/// an editable project directory.
fn pick_unpack_source() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Caiven cart", &["cav"])
        .pick_file()
}

/// Opens a native folder dialog for where to write the unpacked project.
fn pick_unpack_dest() -> Option<PathBuf> {
    rfd::FileDialog::new().pick_folder()
}
