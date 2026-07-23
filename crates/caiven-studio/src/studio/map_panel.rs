//! Map editor panel: scrollable 64×64 tile canvas rendered from the sprite
//! sheet, with pencil/fill/rect tools, RMB tile pick, zoom and full-map undo.

use super::{sheet, theme};
use caiven_core::memory::{
    MAP_H, MAP_LEN, MAP_RAM_BASE, MAP_W, PALETTE_SIZE, SPRITE_FLAGS_RAM_BASE,
};
use caiven_vm::Vm;
use egui::{Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

const TILE_PX: usize = 8;
/// Tiles per 128px screen — grid draws a stronger line on these boundaries.
const SCREEN_TILES: usize = 16;
const ZOOMS: [f32; 3] = [1.0, 2.0, 4.0];
const UNDO_CAP: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Pencil,
    Fill,
    Rect,
}

impl Tool {
    const ALL: [Tool; 3] = [Tool::Pencil, Tool::Fill, Tool::Rect];

    fn label(self) -> &'static str {
        match self {
            Tool::Pencil => "PENCIL",
            Tool::Fill => "FILL",
            Tool::Rect => "RECT",
        }
    }
}

pub struct MapState {
    pub tile: usize,
    pub tool: Tool,
    pub show_flags: bool,
    zoom: usize,
    undo: Vec<Vec<u8>>,
    redo: Vec<Vec<u8>>,
    stroke_before: Option<Vec<u8>>,
    drag_anchor: Option<(usize, usize)>,
    map_tex: Option<egui::TextureHandle>,
    sheet_tex: Option<egui::TextureHandle>,
}

impl Default for MapState {
    fn default() -> Self {
        Self {
            tile: 0,
            tool: Tool::Pencil,
            show_flags: false,
            zoom: 1,
            undo: Vec::new(),
            redo: Vec::new(),
            stroke_before: None,
            drag_anchor: None,
            map_tex: None,
            sheet_tex: None,
        }
    }
}

fn read_map(vm: &Vm) -> Vec<u8> {
    (0..MAP_LEN)
        .map(|i| vm.peek_memory(MAP_RAM_BASE + i))
        .collect()
}

fn write_map(vm: &mut Vm, data: &[u8]) {
    for (i, b) in data.iter().enumerate() {
        vm.poke_memory(MAP_RAM_BASE + i, *b);
    }
}

fn tile_at(vm: &Vm, x: usize, y: usize) -> u8 {
    vm.peek_memory(MAP_RAM_BASE + y * MAP_W + x)
}

fn set_tile(vm: &mut Vm, x: usize, y: usize, tile: u8) {
    vm.poke_memory(MAP_RAM_BASE + y * MAP_W + x, tile);
}

impl MapState {
    fn begin_stroke(&mut self, vm: &Vm) {
        if self.stroke_before.is_none() {
            self.stroke_before = Some(read_map(vm));
        }
    }

    fn end_stroke(&mut self, vm: &Vm) {
        if let Some(before) = self.stroke_before.take()
            && before != read_map(vm)
        {
            self.undo.push(before);
            if self.undo.len() > UNDO_CAP {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
    }

    /// One-shot undoable edit (fill, rect commit).
    fn apply_edit(&mut self, vm: &mut Vm, edit: impl FnOnce(&mut Vm)) {
        self.begin_stroke(vm);
        edit(vm);
        self.end_stroke(vm);
    }

    fn undo(&mut self, vm: &mut Vm) {
        if let Some(data) = self.undo.pop() {
            self.redo.push(read_map(vm));
            write_map(vm, &data);
        }
    }

    fn redo(&mut self, vm: &mut Vm) {
        if let Some(data) = self.redo.pop() {
            self.undo.push(read_map(vm));
            write_map(vm, &data);
        }
    }
}

fn flood_fill(vm: &mut Vm, x: usize, y: usize, fill: u8) {
    let target = tile_at(vm, x, y);
    if target == fill {
        return;
    }
    let mut stack = vec![(x, y)];
    while let Some((x, y)) = stack.pop() {
        if tile_at(vm, x, y) != target {
            continue;
        }
        set_tile(vm, x, y, fill);
        if x > 0 {
            stack.push((x - 1, y));
        }
        if x + 1 < MAP_W {
            stack.push((x + 1, y));
        }
        if y > 0 {
            stack.push((x, y - 1));
        }
        if y + 1 < MAP_H {
            stack.push((x, y + 1));
        }
    }
}

/// All cells of the filled rectangle spanned by two corner cells.
fn rect_cells(a: (usize, usize), b: (usize, usize)) -> Vec<(usize, usize)> {
    let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
    let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
    let mut cells = Vec::new();
    for y in y0..=y1 {
        for x in x0..=x1 {
            cells.push((x, y));
        }
    }
    cells
}

pub fn show(ui: &mut egui::Ui, state: &mut MapState, vm: &mut Vm) {
    handle_shortcuts(ui, state, vm);

    ui.add_space(4.0);
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            show_tool_row(ui, state);
            show_canvas(ui, state, vm);
        });
        ui.add_space(12.0);
        ui.vertical(|ui| {
            ui.label(format!("TILE {:03}", state.tile));
            sheet::show(
                ui,
                &mut state.sheet_tex,
                vm,
                &mut state.tile,
                2.0,
                "map-sheet",
            );
            ui.colored_label(theme::DIM, "RMB on map = pick tile");
            ui.colored_label(theme::DIM, "Ctrl+Z/Y undo/redo");
        });
    });
}

fn handle_shortcuts(ui: &mut egui::Ui, state: &mut MapState, vm: &mut Vm) {
    if ui.ctx().wants_keyboard_input() {
        return;
    }
    let (undo, redo) = ui.input_mut(|i| {
        (
            i.consume_key(egui::Modifiers::CTRL, egui::Key::Z),
            i.consume_key(egui::Modifiers::CTRL, egui::Key::Y),
        )
    });
    if undo {
        state.undo(vm);
    }
    if redo {
        state.redo(vm);
    }
}

fn show_tool_row(ui: &mut egui::Ui, state: &mut MapState) {
    ui.horizontal(|ui| {
        for tool in Tool::ALL {
            ui.selectable_value(&mut state.tool, tool, tool.label());
        }
        ui.separator();
        for (i, z) in ZOOMS.iter().enumerate() {
            ui.selectable_value(&mut state.zoom, i, format!("{z}\u{d7}"));
        }
        ui.separator();
        ui.checkbox(&mut state.show_flags, "FLAGS");
    });
}

/// Blended color for a tile's sprite-flag byte, one fixed hue per bit,
/// averaged across set bits. `None` when no flags are set.
fn flag_overlay_color(flags: u8) -> Option<Color32> {
    const BIT_COLORS: [(u32, u32, u32); 8] = [
        (237, 28, 36),
        (255, 127, 39),
        (255, 242, 0),
        (34, 177, 76),
        (0, 162, 232),
        (63, 72, 204),
        (163, 73, 164),
        (255, 174, 201),
    ];
    let mut sum = (0u32, 0u32, 0u32);
    let mut n = 0u32;
    for (bit, (r, g, b)) in BIT_COLORS.iter().enumerate() {
        if flags & (1 << bit) != 0 {
            sum.0 += r;
            sum.1 += g;
            sum.2 += b;
            n += 1;
        }
    }
    (n > 0).then(|| Color32::from_rgb((sum.0 / n) as u8, (sum.1 / n) as u8, (sum.2 / n) as u8))
}

fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
}

/// Renders the whole 512×512 map image from tile + sprite RAM. When
/// `show_flags` is set, each tile is tinted by its sprite-flag byte.
fn build_map_image(vm: &Vm, show_flags: bool) -> egui::ColorImage {
    let w = MAP_W * TILE_PX;
    let h = MAP_H * TILE_PX;
    let pal: [Color32; PALETTE_SIZE] = std::array::from_fn(|i| sheet::palette_color32(vm, i as u8));
    let mut rgba = vec![0u8; w * h * 4];
    for ty in 0..MAP_H {
        for tx in 0..MAP_W {
            let tile = tile_at(vm, tx, ty) as usize;
            let base = sheet::sprite_base(tile);
            let overlay = show_flags
                .then(|| flag_overlay_color(vm.peek_memory(SPRITE_FLAGS_RAM_BASE + tile)))
                .flatten();
            for py in 0..TILE_PX {
                for px in 0..TILE_PX {
                    let mut c =
                        pal[(vm.peek_memory(base + py * TILE_PX + px) as usize) % PALETTE_SIZE];
                    if let Some(ov) = overlay {
                        c = blend(c, ov, 0.55);
                    }
                    let off = ((ty * TILE_PX + py) * w + tx * TILE_PX + px) * 4;
                    rgba[off..off + 4].copy_from_slice(&[c.r(), c.g(), c.b(), 255]);
                }
            }
        }
    }
    egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba)
}

fn show_canvas(ui: &mut egui::Ui, state: &mut MapState, vm: &mut Vm) {
    let image = build_map_image(vm, state.show_flags);
    let tex = match &mut state.map_tex {
        Some(tex) => {
            tex.set(image, egui::TextureOptions::NEAREST);
            tex.clone()
        }
        None => {
            let tex = ui
                .ctx()
                .load_texture("map-canvas", image, egui::TextureOptions::NEAREST);
            state.map_tex = Some(tex.clone());
            tex
        }
    };

    let cell_px = TILE_PX as f32 * ZOOMS[state.zoom];
    let mut hover_info: Option<(usize, usize, u8)> = None;

    egui::ScrollArea::both()
        .id_salt("map-scroll")
        .max_height(ui.available_height() - 24.0)
        .show(ui, |ui| {
            let size = Vec2::new(MAP_W as f32, MAP_H as f32) * cell_px;
            let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
            let painter = ui.painter_at(rect);
            painter.image(
                tex.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );

            let cell_at = |pos: Pos2| -> Option<(usize, usize)> {
                if !rect.contains(pos) {
                    return None;
                }
                let x = ((pos.x - rect.min.x) / cell_px) as usize;
                let y = ((pos.y - rect.min.y) / cell_px) as usize;
                (x < MAP_W && y < MAP_H).then_some((x, y))
            };
            let cell_rect = |(x, y): (usize, usize)| {
                Rect::from_min_size(
                    rect.min + Vec2::new(x as f32 * cell_px, y as f32 * cell_px),
                    Vec2::splat(cell_px),
                )
            };

            let hover = resp.hover_pos().and_then(cell_at);
            let pointer_cell = resp.interact_pointer_pos().and_then(cell_at);
            if let Some((x, y)) = hover {
                hover_info = Some((x, y, tile_at(vm, x, y)));
            }

            match state.tool {
                Tool::Pencil => {
                    if resp.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_down()) {
                        if let Some((x, y)) = pointer_cell {
                            state.begin_stroke(vm);
                            set_tile(vm, x, y, state.tile as u8);
                        }
                    } else {
                        state.end_stroke(vm);
                    }
                }
                Tool::Fill => {
                    if resp.clicked()
                        && let Some((x, y)) = pointer_cell
                    {
                        let tile = state.tile as u8;
                        state.apply_edit(vm, |vm| flood_fill(vm, x, y, tile));
                    }
                }
                Tool::Rect => {
                    if resp.drag_started() {
                        state.drag_anchor = pointer_cell;
                    }
                    if let (Some(anchor), Some(cur)) = (state.drag_anchor, pointer_cell) {
                        if resp.dragged() {
                            // Preview overlay only; committed on release.
                            let preview = Color32::from_rgba_unmultiplied(255, 220, 60, 90);
                            for cell in rect_cells(anchor, cur) {
                                painter.rect_filled(cell_rect(cell), 0.0, preview);
                            }
                        } else if resp.drag_stopped() {
                            let tile = state.tile as u8;
                            state.apply_edit(vm, |vm| {
                                for (x, y) in rect_cells(anchor, cur) {
                                    set_tile(vm, x, y, tile);
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

            if resp.secondary_clicked()
                && let Some((x, y)) = pointer_cell
            {
                state.tile = tile_at(vm, x, y) as usize;
            }

            draw_grid(&painter, rect, cell_px, state.zoom);

            if let Some(cell) = hover {
                painter.rect_stroke(
                    cell_rect(cell),
                    0.0,
                    Stroke::new(2.0, theme::ACCENT),
                    StrokeKind::Inside,
                );
            }
        });

    match hover_info {
        Some((x, y, t)) => {
            ui.colored_label(theme::DIM, format!("{x},{y} = tile {t:03}"));
        }
        None => {
            ui.colored_label(theme::DIM, format!("map {MAP_W}\u{d7}{MAP_H}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_flags_means_no_overlay() {
        assert_eq!(flag_overlay_color(0), None);
    }

    #[test]
    fn single_flag_uses_its_own_color() {
        assert_eq!(flag_overlay_color(1), Some(Color32::from_rgb(237, 28, 36)));
    }

    #[test]
    fn multiple_flags_average_their_colors() {
        // bits 0 and 2 -> average of (237,28,36) and (255,242,0)
        let c = flag_overlay_color(0b101).unwrap();
        assert_eq!(c, Color32::from_rgb(246, 135, 18));
    }

    #[test]
    fn blend_at_zero_keeps_original() {
        let a = Color32::from_rgb(10, 20, 30);
        let b = Color32::from_rgb(200, 200, 200);
        assert_eq!(blend(a, b, 0.0), a);
    }

    #[test]
    fn blend_at_one_gives_target() {
        let a = Color32::from_rgb(10, 20, 30);
        let b = Color32::from_rgb(200, 200, 200);
        assert_eq!(blend(a, b, 1.0), b);
    }
}

/// Per-tile grid at 2×+ zoom, screen-boundary lines (every 16 tiles) always.
fn draw_grid(painter: &egui::Painter, rect: Rect, cell_px: f32, zoom: usize) {
    let fine = Stroke::new(1.0, Color32::from_rgba_unmultiplied(45, 45, 55, 120));
    let screen = Stroke::new(1.0, Color32::from_rgba_unmultiplied(120, 120, 140, 160));
    let (w, h) = (rect.width(), rect.height());
    for i in 0..=MAP_W {
        let on_screen_edge = i % SCREEN_TILES == 0;
        if zoom == 0 && !on_screen_edge {
            continue;
        }
        let stroke = if on_screen_edge { screen } else { fine };
        let t = i as f32 * cell_px;
        painter.line_segment(
            [rect.min + Vec2::new(t, 0.0), rect.min + Vec2::new(t, h)],
            stroke,
        );
        painter.line_segment(
            [rect.min + Vec2::new(0.0, t), rect.min + Vec2::new(w, t)],
            stroke,
        );
    }
}
