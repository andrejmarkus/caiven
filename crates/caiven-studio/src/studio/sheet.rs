//! Shared sprite-sheet picker: the full 256-sprite sheet rendered as one
//! texture with click/drag selection. Used by the sprite and map editors.

use super::theme;
use caiven_core::memory::{SPRITE_BYTES, SPRITE_COUNT, SPRITE_SHEET_RAM_BASE};
use caiven_vm::Vm;
use egui::{Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

pub const SHEET_COLS: usize = 16;
const SPRITE_SIZE: usize = 8;

pub fn sprite_base(sprite: usize) -> usize {
    SPRITE_SHEET_RAM_BASE + sprite * SPRITE_BYTES
}

pub fn palette_color32(vm: &Vm, index: u8) -> Color32 {
    match vm.get_palette().get(index as usize) {
        Some(c) => Color32::from_rgb(c.get_r(), c.get_g(), c.get_b()),
        None => Color32::BLACK,
    }
}

pub fn show(
    ui: &mut egui::Ui,
    tex_slot: &mut Option<egui::TextureHandle>,
    vm: &Vm,
    selected: &mut usize,
    scale: f32,
    tex_name: &str,
) {
    let rows = SPRITE_COUNT / SHEET_COLS;
    let w = SHEET_COLS * SPRITE_SIZE;
    let h = rows * SPRITE_SIZE;

    let mut rgba = Vec::with_capacity(w * h * 4);
    for py in 0..h {
        for px in 0..w {
            let sprite = (py / SPRITE_SIZE) * SHEET_COLS + px / SPRITE_SIZE;
            let off = sprite_base(sprite) + (py % SPRITE_SIZE) * SPRITE_SIZE + px % SPRITE_SIZE;
            let c = palette_color32(vm, vm.peek_memory(off));
            rgba.extend_from_slice(&[c.r(), c.g(), c.b(), 255]);
        }
    }
    let image = egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba);
    let tex = match tex_slot {
        Some(tex) => {
            tex.set(image, egui::TextureOptions::NEAREST);
            tex.clone()
        }
        None => {
            let tex = ui
                .ctx()
                .load_texture(tex_name, image, egui::TextureOptions::NEAREST);
            *tex_slot = Some(tex.clone());
            tex
        }
    };

    let size = Vec2::new(w as f32, h as f32) * scale;
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.image(
        tex.id(),
        rect,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );

    if resp.is_pointer_button_down_on()
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let cell_px = SPRITE_SIZE as f32 * scale;
        let col = ((pos.x - rect.min.x) / cell_px) as usize;
        let row = ((pos.y - rect.min.y) / cell_px) as usize;
        if col < SHEET_COLS && row < rows {
            *selected = row * SHEET_COLS + col;
        }
    }

    let cell_px = SPRITE_SIZE as f32 * scale;
    let sel = Rect::from_min_size(
        rect.min
            + Vec2::new(
                (*selected % SHEET_COLS) as f32 * cell_px,
                (*selected / SHEET_COLS) as f32 * cell_px,
            ),
        Vec2::splat(cell_px),
    );
    painter.rect_stroke(
        sel,
        0.0,
        Stroke::new(2.0, theme::ACCENT),
        StrokeKind::Outside,
    );
}
