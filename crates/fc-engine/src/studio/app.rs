//! StudioApp: eframe application state — headless console core, cart state,
//! tab selection and per-frame VM stepping + framebuffer texture upload.

use super::{cart, code_panel, game_panel, map_panel, palette_panel, sprite_panel, theme, toolbar};
use crate::app::rom_io::{self, CartMeta};
use anyhow::Result;
use fc_vm::input::Button;
use fc_vm::runtime::ConsoleCore;
use log::info;
use std::path::PathBuf;
use std::time::Instant;

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
}

impl Tab {
    pub const ALL: [Tab; 8] = [
        Tab::Code,
        Tab::Sprite,
        Tab::Map,
        Tab::Sfx,
        Tab::Music,
        Tab::Palette,
        Tab::Meta,
        Tab::Browser,
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
            status: "no cart loaded — fc-engine edit <file.rom|file.fc>".into(),
            status_is_error: false,
            code: code_panel::CodeState::default(),
            sprite: sprite_panel::SpriteState::default(),
            map: map_panel::MapState::default(),
            palette: palette_panel::PaletteState::default(),
        };

        if let Some(path) = file {
            if let Err(e) = app.open_file(&path) {
                app.set_status(format!("{e:#}"), true);
            }
        }

        Ok(app)
    }

    fn set_status(&mut self, msg: impl Into<String>, is_error: bool) {
        self.status = msg.into();
        self.status_is_error = is_error;
    }

    /// Opens a cart file. Returns `Err` only for hard failures (I/O, bad ROM);
    /// a `.fc` file that fails to compile still opens in the editor with the
    /// error shown, so it can be fixed in place.
    fn open_file(&mut self, path: &std::path::Path) -> Result<()> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            "rom" => {
                let meta = cart::load_rom(&mut self.core.vm, path)?;
                info!("studio: ROM loaded from {}", path.display());
                self.cart = Some(meta);
                self.source = None;
                self.code.error = None;
                self.run_state = RunState::Running;
                self.set_status(format!("loaded {}", path.display()), false);
            }
            "fc" => {
                let text = std::fs::read_to_string(path)?;
                // Editor buffer holds only the code part; asset blocks live
                // in VM RAM (mutated by the sprite/map/... editors) and are
                // re-embedded on save.
                let (code, sections) =
                    fc_rom::text::split_source(&text).map_err(anyhow::Error::msg)?;
                info!("studio: fc source loaded from {}", path.display());
                cart::apply_sections(&mut self.core.vm, &sections);
                if !sections
                    .iter()
                    .any(|(k, _)| *k == fc_rom::SectionKind::Palette)
                {
                    cart::sync_palette_to_ram(&mut self.core.vm);
                }
                self.source = Some(SourceFile {
                    path: path.to_path_buf(),
                    text: code,
                    dirty: false,
                });
                self.cart = None;
                self.run_source();
            }
            _ => anyhow::bail!("unsupported file type: {} (expected .rom or .fc)", ext),
        }
        Ok(())
    }

    /// Compiles the current editor buffer and (re)starts the game.
    fn run_source(&mut self) {
        let Some(src) = &self.source else {
            self.set_status("no .fc source loaded", true);
            return;
        };
        match cart::compile_into_vm(&mut self.core.vm, &src.text) {
            Ok(()) => {
                let name = src.path.display().to_string();
                self.code.error = None;
                self.run_state = RunState::Running;
                self.set_status(format!("compiled {name}"), false);
            }
            Err(e) => {
                let first = e.message.lines().next().unwrap_or("compile error").to_string();
                self.code.error = Some(e);
                self.run_state = RunState::Stopped;
                self.set_status(format!("compile error: {first}"), true);
            }
        }
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
        if let Some(meta) = &self.cart {
            match rom_io::save(&self.core.vm, meta) {
                Ok(()) => self.set_status(format!("saved {}", meta.path.display()), false),
                Err(e) => self.set_status(format!("save failed: {e:#}"), true),
            }
            return;
        }
        if let Some(src) = &mut self.source {
            let sections = cart::collect_ram_sections(&self.core.vm);
            let text = fc_rom::text::join_source(&src.text, &sections);
            let result = std::fs::write(&src.path, text);
            let path = src.path.display().to_string();
            if result.is_ok() {
                src.dirty = false;
            }
            match result {
                Ok(()) => self.set_status(format!("saved {path}"), false),
                Err(e) => self.set_status(format!("save failed: {e:#}"), true),
            }
            return;
        }
        self.set_status("nothing to save", true);
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
        if self.run_state == RunState::Running {
            let steps = self.core.frame_steps();
            for _ in 0..steps {
                self.core.run_frame();
            }
        } else {
            self.core.last_tick = Instant::now();
        }
    }

    fn update_game_texture(&mut self, ctx: &egui::Context) {
        let w = self.core.config.width as usize;
        let h = self.core.config.height as usize;
        self.compose_buf.resize(w * h * 4, 0);
        self.core.screen.construct(
            &mut self.compose_buf,
            self.core.vm.world_pixels(),
            self.core.vm.ui_pixels(),
        );
        for px in self.compose_buf.chunks_exact_mut(4) {
            px[3] = 255;
        }
        let image = egui::ColorImage::from_rgba_unmultiplied([w, h], &self.compose_buf);
        match &mut self.game_tex {
            Some(tex) => tex.set(image, egui::TextureOptions::NEAREST),
            None => {
                self.game_tex = Some(ctx.load_texture("game", image, egui::TextureOptions::NEAREST))
            }
        }
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
        let save = ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::CTRL, egui::Key::S)
        });
        if save {
            self.save();
        }
        let run = ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::CTRL, egui::Key::R)
        });
        if run {
            self.run_source();
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
        self.step_vm();
        self.update_game_texture(ctx);

        let fps = ctx.input(|i| 1.0 / i.stable_dt.max(1e-6));
        let action = toolbar::show(ctx, &self.cart_name(), self.run_state, fps);
        match action {
            toolbar::ToolbarAction::Run => {
                if self.source.is_some() && self.run_state == RunState::Stopped {
                    // Stopped source means never compiled or compile failed —
                    // recompile instead of resuming a stale program.
                    self.run_source();
                } else if self.cart.is_some() || self.source.is_some() {
                    self.run_state = RunState::Running;
                } else {
                    self.set_status("no cart loaded", true);
                }
            }
            toolbar::ToolbarAction::Pause => self.run_state = RunState::Paused,
            toolbar::ToolbarAction::Reset => self.reset(),
            toolbar::ToolbarAction::Save => self.save(),
            toolbar::ToolbarAction::None => {}
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
                game_panel::show(
                    ui,
                    self.game_tex.as_ref(),
                    self.core.config.width as f32,
                    self.run_state,
                );
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.tab {
                Tab::Code => {
                    match &mut self.source {
                        Some(src) => code_panel::show(ui, &mut self.code, src),
                        None => {
                            ui.add_space(8.0);
                            ui.heading("CODE EDITOR");
                            ui.colored_label(
                                theme::DIM,
                                "no .fc source open — fc-engine edit <file.fc>",
                            );
                        }
                    }
                    return;
                }
                Tab::Sprite => {
                    sprite_panel::show(ui, &mut self.sprite, &mut self.core.vm);
                    return;
                }
                Tab::Map => {
                    map_panel::show(ui, &mut self.map, &mut self.core.vm);
                    return;
                }
                Tab::Palette => {
                    palette_panel::show(ui, &mut self.palette, &mut self.core.vm);
                    return;
                }
                _ => {}
            }
            ui.add_space(8.0);
            ui.heading(format!("{} EDITOR", self.tab.label()));
            let phase = match self.tab {
                Tab::Code | Tab::Sprite | Tab::Map | Tab::Palette => unreachable!(),
                Tab::Sfx | Tab::Music => "P4",
                Tab::Meta | Tab::Browser => "P5",
            };
            ui.colored_label(theme::DIM, format!("coming in phase {phase}"));
        });

        ctx.request_repaint();
    }
}
