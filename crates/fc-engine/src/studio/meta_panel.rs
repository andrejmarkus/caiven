//! Cart metadata panel: title/author editing for .rom carts (written into
//! the ROM header on Ctrl+S), entry point and flags shown read-only.

use super::theme;
use crate::app::rom_io::CartMeta;
use crate::studio::app::SourceFile;

/// RomHeader stores title/author as fixed 32-byte fields.
const FIELD_MAX: usize = 32;

pub fn show(ui: &mut egui::Ui, cart: Option<&mut CartMeta>, source: Option<&SourceFile>) {
    ui.add_space(8.0);
    ui.heading("CART META");
    ui.add_space(8.0);

    if let Some(meta) = cart {
        egui::Grid::new("meta_grid")
            .num_columns(2)
            .spacing([16.0, 10.0])
            .show(ui, |ui| {
                ui.label("TITLE");
                ui.add(
                    egui::TextEdit::singleline(&mut meta.header.title)
                        .char_limit(FIELD_MAX)
                        .desired_width(320.0),
                );
                ui.end_row();

                ui.label("AUTHOR");
                ui.add(
                    egui::TextEdit::singleline(&mut meta.header.author)
                        .char_limit(FIELD_MAX)
                        .desired_width(320.0),
                );
                ui.end_row();

                ui.label("ENTRY");
                ui.colored_label(theme::DIM, format!("0x{:04X}", meta.header.entry_point));
                ui.end_row();

                ui.label("FLAGS");
                ui.colored_label(theme::DIM, format!("0x{:04X}", meta.header.flags));
                ui.end_row();

                ui.label("FILE");
                ui.colored_label(theme::DIM, meta.path.display().to_string());
                ui.end_row();
            });
        ui.add_space(8.0);
        ui.colored_label(theme::DIM, "Ctrl+S writes title/author into the ROM header");
        return;
    }

    if let Some(src) = source {
        let stem = src
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("cart");
        egui::Grid::new("meta_grid_fc")
            .num_columns(2)
            .spacing([16.0, 10.0])
            .show(ui, |ui| {
                ui.label("FILE");
                ui.colored_label(theme::DIM, src.path.display().to_string());
                ui.end_row();

                ui.label("TITLE");
                ui.colored_label(theme::DIM, stem);
                ui.end_row();
            });
        ui.add_space(8.0);
        ui.colored_label(
            theme::DIM,
            ".fc sources have no header — `fc-engine build` derives the title from the file name",
        );
        return;
    }

    ui.colored_label(theme::DIM, "no cart loaded");
}
