//! StudioApp: eframe application state — headless console core, cart state,
//! tab selection and per-frame VM stepping + framebuffer texture upload.

use super::{cart, game_panel, theme, toolbar};
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
        };

        if let Some(path) = file {
            match app.open_file(&path) {
                Ok(()) => {
                    app.run_state = RunState::Running;
                    app.set_status(format!("loaded {}", path.display()), false);
                }
                Err(e) => app.set_status(format!("{e:#}"), true),
            }
        }

        Ok(app)
    }

    fn set_status(&mut self, msg: impl Into<String>, is_error: bool) {
        self.status = msg.into();
        self.status_is_error = is_error;
    }

    fn open_file(&mut self, path: &std::path::Path) -> Result<()> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            "rom" => {
                let meta = cart::load_rom(&mut self.core.vm, path)?;
                info!("studio: ROM loaded from {}", path.display());
                self.cart = Some(meta);
                self.source = None;
            }
            "fc" => {
                let text = std::fs::read_to_string(path)?;
                cart::load_fc_source(&mut self.core.vm, path, &text)?;
                info!("studio: fc source loaded from {}", path.display());
                self.source = Some(SourceFile {
                    path: path.to_path_buf(),
                    text,
                });
                self.cart = None;
            }
            _ => anyhow::bail!("unsupported file type: {} (expected .rom or .fc)", ext),
        }
        Ok(())
    }

    fn reset(&mut self) {
        let path = self
            .cart
            .as_ref()
            .map(|c| c.path.clone())
            .or_else(|| self.source.as_ref().map(|s| s.path.clone()));
        let Some(path) = path else {
            self.set_status("nothing to reset", true);
            return;
        };
        match self.open_file(&path) {
            Ok(()) => {
                self.run_state = RunState::Running;
                self.set_status(format!("reset {}", path.display()), false);
            }
            Err(e) => self.set_status(format!("{e:#}"), true),
        }
    }

    fn save(&mut self) {
        match (&self.cart, &self.source) {
            (Some(meta), _) => match rom_io::save(&self.core.vm, meta) {
                Ok(()) => self.set_status(format!("saved {}", meta.path.display()), false),
                Err(e) => self.set_status(format!("save failed: {e:#}"), true),
            },
            (None, Some(src)) => match std::fs::write(&src.path, &src.text) {
                Ok(()) => self.set_status(format!("saved {}", src.path.display()), false),
                Err(e) => self.set_status(format!("save failed: {e:#}"), true),
            },
            (None, None) => self.set_status("nothing to save", true),
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
                if self.cart.is_some() || self.source.is_some() {
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
            ui.add_space(8.0);
            ui.heading(format!("{} EDITOR", self.tab.label()));
            let phase = match self.tab {
                Tab::Code => "P1",
                Tab::Sprite | Tab::Palette => "P2",
                Tab::Map => "P3",
                Tab::Sfx | Tab::Music => "P4",
                Tab::Meta | Tab::Browser => "P5",
            };
            ui.colored_label(theme::DIM, format!("coming in phase {phase}"));
        });

        ctx.request_repaint();
    }
}
