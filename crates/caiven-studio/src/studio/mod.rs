//! Caiven Studio: desktop egui editor suite. The console runs headless inside
//! [`caiven_vm::runtime::ConsoleCore`]; its 128x128 output is shown as a
//! nearest-neighbor texture while editors mutate cart RAM directly.

mod app;
mod browser_panel;
pub(crate) mod cart;
mod code_panel;
mod debug_panel;
mod game_panel;
mod map_panel;
mod menu_bar;
mod meta_panel;
mod music_panel;
mod palette_panel;
mod recent;
mod sfx_panel;
mod sheet;
mod sprite_panel;
mod theme;
mod toolbar;

use anyhow::Result;
use std::path::PathBuf;

pub fn run_studio(file: Option<PathBuf>) -> Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Caiven Studio")
            .with_inner_size([1280.0, 760.0])
            .with_min_inner_size([960.0, 600.0]),
        vsync: true,
        ..Default::default()
    };

    eframe::run_native(
        "caiven-studio",
        native_options,
        Box::new(move |cc| {
            app::StudioApp::new(cc, file)
                .map(|a| Box::new(a) as Box<dyn eframe::App>)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.into() })
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))
}
