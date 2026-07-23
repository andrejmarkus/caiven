//! Palette editor panel: 16 swatches, color picker and hex entry for the
//! selected slot. Edits update the live VM palette and mirror into palette
//! RAM so Ctrl+S persists them.

use super::theme;
use caiven_core::Color;
use caiven_core::memory::{PALETTE_RAM_BASE, PALETTE_SIZE};
use caiven_vm::Vm;
use egui::{Color32, Rect, Sense, Stroke, StrokeKind, Vec2};

pub struct PaletteState {
    pub selected: usize,
    hex: String,
    hex_slot: usize,
    hex_color: [u8; 3],
}

impl Default for PaletteState {
    fn default() -> Self {
        Self {
            selected: 0,
            hex: String::new(),
            hex_slot: usize::MAX,
            hex_color: [0; 3],
        }
    }
}

fn get_rgb(vm: &Vm, slot: usize) -> [u8; 3] {
    match vm.get_palette().get(slot) {
        Some(c) => [c.get_r(), c.get_g(), c.get_b()],
        None => [0; 3],
    }
}

fn set_rgb(vm: &mut Vm, slot: usize, rgb: [u8; 3]) {
    vm.set_palette_color(slot, Color::new_rgb(rgb[0], rgb[1], rgb[2]));
    for (i, b) in rgb.iter().enumerate() {
        vm.poke_memory(PALETTE_RAM_BASE + slot * 3 + i, *b);
    }
}

pub fn show(ui: &mut egui::Ui, state: &mut PaletteState, vm: &mut Vm) {
    ui.add_space(4.0);

    // Swatch grid: 8 × 2, click selects
    let swatch = 40.0;
    let cols = 8;
    let rows = PALETTE_SIZE / cols;
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(cols as f32 * swatch, rows as f32 * swatch),
        Sense::click(),
    );
    let painter = ui.painter_at(rect);
    for i in 0..PALETTE_SIZE {
        let r = Rect::from_min_size(
            rect.min + Vec2::new((i % cols) as f32 * swatch, (i / cols) as f32 * swatch),
            Vec2::splat(swatch),
        );
        let rgb = get_rgb(vm, i);
        painter.rect_filled(
            r.shrink(1.0),
            2.0,
            Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
        );
        if i == state.selected {
            painter.rect_stroke(
                r.shrink(1.0),
                2.0,
                Stroke::new(2.0, theme::ACCENT),
                StrokeKind::Inside,
            );
        }
    }
    if resp.clicked()
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let col = ((pos.x - rect.min.x) / swatch) as usize;
        let row = ((pos.y - rect.min.y) / swatch) as usize;
        if col < cols && row < rows {
            state.selected = row * cols + col;
        }
    }

    ui.add_space(8.0);
    let mut rgb = get_rgb(vm, state.selected);

    // Keep the hex field in sync unless the user is mid-edit on this slot
    if state.hex_slot != state.selected || state.hex_color != rgb {
        state.hex = format!("{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
        state.hex_slot = state.selected;
        state.hex_color = rgb;
    }

    ui.horizontal(|ui| {
        ui.label(format!("COLOR {:02}", state.selected));
        if ui.color_edit_button_srgb(&mut rgb).changed() {
            set_rgb(vm, state.selected, rgb);
        }
        ui.label("#");
        let hex_resp = ui.add(egui::TextEdit::singleline(&mut state.hex).desired_width(70.0));
        if hex_resp.changed()
            && let Some(parsed) = parse_hex(&state.hex)
        {
            set_rgb(vm, state.selected, parsed);
            state.hex_color = parsed;
        }
    });

    ui.add_space(4.0);
    ui.colored_label(
        theme::DIM,
        "changes apply live; Ctrl+S saves them to the cart",
    );
}

fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}
