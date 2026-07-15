//! Sprite editor panel: zoomed pixel canvas with pencil/fill/line/rect
//! tools, palette row, per-sprite flags and a full sprite-sheet picker.
//! All edits go straight to sprite RAM; undo keeps per-sprite snapshots.

use super::theme;
use egui::{Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use fc_core::memory::{
    PALETTE_SIZE, SPRITE_COUNT, SPRITE_FLAGS_RAM_BASE, SPRITE_SHEET_RAM_BASE,
};
use fc_vm::Vm;

const SPRITE_SIZE: usize = 8;
const SPRITE_BYTES: usize = SPRITE_SIZE * SPRITE_SIZE;
const ZOOM: f32 = 32.0;
const SHEET_COLS: usize = 16;
const SHEET_SCALE: f32 = 2.0;
const UNDO_CAP: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Pencil,
    Fill,
    Line,
    Rect,
}

impl Tool {
    const ALL: [Tool; 4] = [Tool::Pencil, Tool::Fill, Tool::Line, Tool::Rect];

    fn label(self) -> &'static str {
        match self {
            Tool::Pencil => "PENCIL",
            Tool::Fill => "FILL",
            Tool::Line => "LINE",
            Tool::Rect => "RECT",
        }
    }
}

type Snapshot = (usize, [u8; SPRITE_BYTES]);

pub struct SpriteState {
    pub sprite: usize,
    pub color: u8,
    pub tool: Tool,
    clipboard: Option<[u8; SPRITE_BYTES]>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    stroke_before: Option<[u8; SPRITE_BYTES]>,
    drag_anchor: Option<(usize, usize)>,
    sheet_tex: Option<egui::TextureHandle>,
}

impl Default for SpriteState {
    fn default() -> Self {
        Self {
            sprite: 0,
            color: 7,
            tool: Tool::Pencil,
            clipboard: None,
            undo: Vec::new(),
            redo: Vec::new(),
            stroke_before: None,
            drag_anchor: None,
            sheet_tex: None,
        }
    }
}

fn sprite_base(sprite: usize) -> usize {
    SPRITE_SHEET_RAM_BASE + sprite * SPRITE_BYTES
}

fn read_sprite(vm: &Vm, sprite: usize) -> [u8; SPRITE_BYTES] {
    let base = sprite_base(sprite);
    std::array::from_fn(|i| vm.peek_memory(base + i))
}

fn write_sprite(vm: &mut Vm, sprite: usize, data: &[u8; SPRITE_BYTES]) {
    let base = sprite_base(sprite);
    for (i, b) in data.iter().enumerate() {
        vm.poke_memory(base + i, *b);
    }
}

fn palette_color32(vm: &Vm, index: u8) -> Color32 {
    match vm.get_palette().get(index as usize) {
        Some(c) => Color32::from_rgb(c.get_r(), c.get_g(), c.get_b()),
        None => Color32::BLACK,
    }
}

impl SpriteState {
    fn begin_stroke(&mut self, vm: &Vm) {
        if self.stroke_before.is_none() {
            self.stroke_before = Some(read_sprite(vm, self.sprite));
        }
    }

    fn end_stroke(&mut self, vm: &Vm) {
        if let Some(before) = self.stroke_before.take() {
            if before != read_sprite(vm, self.sprite) {
                self.undo.push((self.sprite, before));
                if self.undo.len() > UNDO_CAP {
                    self.undo.remove(0);
                }
                self.redo.clear();
            }
        }
    }

    /// One-shot undoable edit (fill, paste, line/rect commit).
    fn apply_edit(&mut self, vm: &mut Vm, edit: impl FnOnce(&mut Vm, usize)) {
        self.begin_stroke(vm);
        edit(vm, self.sprite);
        self.end_stroke(vm);
    }

    fn undo(&mut self, vm: &mut Vm) {
        if let Some((sprite, data)) = self.undo.pop() {
            self.redo.push((sprite, read_sprite(vm, sprite)));
            write_sprite(vm, sprite, &data);
            self.sprite = sprite;
        }
    }

    fn redo(&mut self, vm: &mut Vm) {
        if let Some((sprite, data)) = self.redo.pop() {
            self.undo.push((sprite, read_sprite(vm, sprite)));
            write_sprite(vm, sprite, &data);
            self.sprite = sprite;
        }
    }
}

fn flood_fill(vm: &mut Vm, sprite: usize, px: usize, py: usize, fill: u8) {
    let base = sprite_base(sprite);
    let target = vm.peek_memory(base + py * SPRITE_SIZE + px);
    if target == fill {
        return;
    }
    let mut stack = vec![(px, py)];
    while let Some((x, y)) = stack.pop() {
        let off = base + y * SPRITE_SIZE + x;
        if vm.peek_memory(off) != target {
            continue;
        }
        vm.poke_memory(off, fill);
        if x > 0 {
            stack.push((x - 1, y));
        }
        if x + 1 < SPRITE_SIZE {
            stack.push((x + 1, y));
        }
        if y > 0 {
            stack.push((x, y - 1));
        }
        if y + 1 < SPRITE_SIZE {
            stack.push((x, y + 1));
        }
    }
}

/// Cells of a straight line between two canvas cells (Bresenham).
fn line_cells(a: (usize, usize), b: (usize, usize)) -> Vec<(usize, usize)> {
    let (mut x0, mut y0) = (a.0 as i32, a.1 as i32);
    let (x1, y1) = (b.0 as i32, b.1 as i32);
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut cells = Vec::new();
    loop {
        cells.push((x0 as usize, y0 as usize));
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
    cells
}

/// Cells of a rectangle outline between two corner cells.
fn rect_cells(a: (usize, usize), b: (usize, usize)) -> Vec<(usize, usize)> {
    let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
    let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
    let mut cells = Vec::new();
    for x in x0..=x1 {
        cells.push((x, y0));
        cells.push((x, y1));
    }
    for y in y0..=y1 {
        cells.push((x0, y));
        cells.push((x1, y));
    }
    cells
}

pub fn show(ui: &mut egui::Ui, state: &mut SpriteState, vm: &mut Vm) {
    handle_shortcuts(ui, state, vm);

    ui.add_space(4.0);
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            show_tool_row(ui, state);
            show_canvas(ui, state, vm);
            show_palette_row(ui, state, vm);
            show_flags_row(ui, state, vm);
        });
        ui.add_space(12.0);
        ui.vertical(|ui| {
            ui.label(format!("SPR {:03}", state.sprite));
            show_sheet_picker(ui, state, vm);
            ui.colored_label(theme::DIM, "RMB on canvas = pick color");
            ui.colored_label(theme::DIM, "Ctrl+Z/Y undo/redo, Ctrl+C/V copy/paste");
        });
    });
}

fn handle_shortcuts(ui: &mut egui::Ui, state: &mut SpriteState, vm: &mut Vm) {
    if ui.ctx().wants_keyboard_input() {
        return;
    }
    let (undo, redo, copy, paste) = ui.input_mut(|i| {
        (
            i.consume_key(egui::Modifiers::CTRL, egui::Key::Z),
            i.consume_key(egui::Modifiers::CTRL, egui::Key::Y),
            i.consume_key(egui::Modifiers::CTRL, egui::Key::C),
            i.consume_key(egui::Modifiers::CTRL, egui::Key::V),
        )
    });
    if undo {
        state.undo(vm);
    }
    if redo {
        state.redo(vm);
    }
    if copy {
        state.clipboard = Some(read_sprite(vm, state.sprite));
    }
    if paste {
        if let Some(data) = state.clipboard {
            state.apply_edit(vm, |vm, sprite| write_sprite(vm, sprite, &data));
        }
    }
}

fn show_tool_row(ui: &mut egui::Ui, state: &mut SpriteState) {
    ui.horizontal(|ui| {
        for tool in Tool::ALL {
            ui.selectable_value(&mut state.tool, tool, tool.label());
        }
    });
}

fn show_canvas(ui: &mut egui::Ui, state: &mut SpriteState, vm: &mut Vm) {
    let side = SPRITE_SIZE as f32 * ZOOM;
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(side), Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    let cell_at = |pos: Pos2| -> Option<(usize, usize)> {
        if !rect.contains(pos) {
            return None;
        }
        let x = ((pos.x - rect.min.x) / ZOOM) as usize;
        let y = ((pos.y - rect.min.y) / ZOOM) as usize;
        (x < SPRITE_SIZE && y < SPRITE_SIZE).then_some((x, y))
    };
    let cell_rect = |(x, y): (usize, usize)| {
        Rect::from_min_size(
            rect.min + Vec2::new(x as f32 * ZOOM, y as f32 * ZOOM),
            Vec2::splat(ZOOM),
        )
    };

    let hover = resp.hover_pos().and_then(cell_at);
    let pointer_cell = resp.interact_pointer_pos().and_then(cell_at);

    // Draw sprite pixels
    let base = sprite_base(state.sprite);
    for y in 0..SPRITE_SIZE {
        for x in 0..SPRITE_SIZE {
            let color = palette_color32(vm, vm.peek_memory(base + y * SPRITE_SIZE + x));
            painter.rect_filled(cell_rect((x, y)), 0.0, color);
        }
    }

    // Tool input
    match state.tool {
        Tool::Pencil => {
            if resp.is_pointer_button_down_on()
                && ui.input(|i| i.pointer.primary_down())
            {
                if let Some((x, y)) = pointer_cell {
                    state.begin_stroke(vm);
                    vm.poke_memory(base + y * SPRITE_SIZE + x, state.color);
                }
            } else {
                state.end_stroke(vm);
            }
        }
        Tool::Fill => {
            if resp.clicked() {
                if let Some((x, y)) = pointer_cell {
                    let color = state.color;
                    state.apply_edit(vm, |vm, sprite| flood_fill(vm, sprite, x, y, color));
                }
            }
        }
        Tool::Line | Tool::Rect => {
            let tool = state.tool;
            let shape = move |a, b| match tool {
                Tool::Line => line_cells(a, b),
                _ => rect_cells(a, b),
            };
            if resp.drag_started() {
                state.drag_anchor = pointer_cell;
            }
            if let (Some(anchor), Some(cur)) = (state.drag_anchor, pointer_cell) {
                if resp.dragged() {
                    // Preview overlay only; committed on release.
                    let preview = palette_color32(vm, state.color);
                    for cell in shape(anchor, cur) {
                        painter.rect_filled(cell_rect(cell), 0.0, preview);
                    }
                } else if resp.drag_stopped() {
                    let color = state.color;
                    state.apply_edit(vm, |vm, sprite| {
                        let base = sprite_base(sprite);
                        for (x, y) in shape(anchor, cur) {
                            vm.poke_memory(base + y * SPRITE_SIZE + x, color);
                        }
                    });
                    state.drag_anchor = None;
                }
            }
            if resp.drag_stopped() {
                state.drag_anchor = None;
            }
        }
    }

    // Eyedropper on right click
    if resp.secondary_clicked() {
        if let Some((x, y)) = pointer_cell {
            state.color = vm.peek_memory(base + y * SPRITE_SIZE + x).min(15);
        }
    }

    // Grid + hover highlight
    let grid = Stroke::new(1.0, Color32::from_rgb(45, 45, 55));
    for i in 0..=SPRITE_SIZE {
        let t = i as f32 * ZOOM;
        painter.line_segment(
            [rect.min + Vec2::new(t, 0.0), rect.min + Vec2::new(t, side)],
            grid,
        );
        painter.line_segment(
            [rect.min + Vec2::new(0.0, t), rect.min + Vec2::new(side, t)],
            grid,
        );
    }
    if let Some(cell) = hover {
        painter.rect_stroke(
            cell_rect(cell),
            0.0,
            Stroke::new(2.0, theme::ACCENT),
            StrokeKind::Inside,
        );
    }
}

fn show_palette_row(ui: &mut egui::Ui, state: &mut SpriteState, vm: &Vm) {
    let swatch = 16.0;
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(swatch * PALETTE_SIZE as f32, swatch),
        Sense::click(),
    );
    let painter = ui.painter_at(rect);
    for i in 0..PALETTE_SIZE {
        let r = Rect::from_min_size(
            rect.min + Vec2::new(i as f32 * swatch, 0.0),
            Vec2::splat(swatch),
        );
        painter.rect_filled(r, 0.0, palette_color32(vm, i as u8));
        if i == state.color as usize {
            painter.rect_stroke(r, 0.0, Stroke::new(2.0, Color32::WHITE), StrokeKind::Inside);
        }
    }
    if resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let i = ((pos.x - rect.min.x) / swatch) as usize;
            if i < PALETTE_SIZE {
                state.color = i as u8;
            }
        }
    }
}

fn show_flags_row(ui: &mut egui::Ui, state: &SpriteState, vm: &mut Vm) {
    let addr = SPRITE_FLAGS_RAM_BASE + state.sprite;
    let mut flags = vm.peek_memory(addr);
    ui.horizontal(|ui| {
        ui.label("FLAGS");
        let mut changed = false;
        for bit in 0..8u8 {
            let mut on = flags & (1 << bit) != 0;
            if ui.checkbox(&mut on, "").changed() {
                flags = (flags & !(1 << bit)) | ((on as u8) << bit);
                changed = true;
            }
        }
        ui.colored_label(theme::DIM, format!("0x{flags:02X}"));
        if changed {
            vm.poke_memory(addr, flags);
        }
    });
}

fn show_sheet_picker(ui: &mut egui::Ui, state: &mut SpriteState, vm: &Vm) {
    let rows = SPRITE_COUNT / SHEET_COLS;
    let w = SHEET_COLS * SPRITE_SIZE;
    let h = rows * SPRITE_SIZE;

    let mut rgba = Vec::with_capacity(w * h * 4);
    for py in 0..h {
        for px in 0..w {
            let sprite = (py / SPRITE_SIZE) * SHEET_COLS + px / SPRITE_SIZE;
            let off =
                sprite_base(sprite) + (py % SPRITE_SIZE) * SPRITE_SIZE + px % SPRITE_SIZE;
            let c = palette_color32(vm, vm.peek_memory(off));
            rgba.extend_from_slice(&[c.r(), c.g(), c.b(), 255]);
        }
    }
    let image = egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba);
    let tex = match &mut state.sheet_tex {
        Some(tex) => {
            tex.set(image, egui::TextureOptions::NEAREST);
            tex.clone()
        }
        None => {
            let tex = ui
                .ctx()
                .load_texture("sprite-sheet", image, egui::TextureOptions::NEAREST);
            state.sheet_tex = Some(tex.clone());
            tex
        }
    };

    let size = Vec2::new(w as f32, h as f32) * SHEET_SCALE;
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.image(
        tex.id(),
        rect,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );

    if resp.is_pointer_button_down_on() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let cell_px = SPRITE_SIZE as f32 * SHEET_SCALE;
            let col = ((pos.x - rect.min.x) / cell_px) as usize;
            let row = ((pos.y - rect.min.y) / cell_px) as usize;
            if col < SHEET_COLS && row < rows {
                state.sprite = row * SHEET_COLS + col;
            }
        }
    }

    let cell_px = SPRITE_SIZE as f32 * SHEET_SCALE;
    let sel = Rect::from_min_size(
        rect.min
            + Vec2::new(
                (state.sprite % SHEET_COLS) as f32 * cell_px,
                (state.sprite / SHEET_COLS) as f32 * cell_px,
            ),
        Vec2::splat(cell_px),
    );
    painter.rect_stroke(sel, 0.0, Stroke::new(2.0, theme::ACCENT), StrokeKind::Outside);
}
