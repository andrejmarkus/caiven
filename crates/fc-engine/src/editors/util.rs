//! Shared UI helpers for the built-in editors: theme palette, rectangle
//! fills/borders, grid hit-testing, buttons and keyboard-to-char mapping.

use fc_core::{Color, Vec2};
use fc_vm::rendering::text::draw_text;
use fc_vm::rendering::{font::Font, screen::ScreenLayer};
use winit::keyboard::KeyCode;

/// Editor panel width in pixels.
pub const PANEL_W: u32 = 128;
/// Editor panel height in pixels (area above the tab bar).
pub const PANEL_H: u32 = crate::tabs::TAB_BAR_Y;

/// Shared UI color palette for all editors.
pub mod theme {
    use fc_core::Color;

    /// Default panel background.
    pub const BG: Color = Color::new_rgb(15, 15, 15);
    /// Background of the selected row/cell.
    pub const SEL_BG: Color = Color::new_rgb(30, 50, 90);
    /// Foreground of the selected item.
    pub const SELECTED: Color = Color::new_rgb(80, 140, 220);
    /// Active/current element accent.
    pub const ACTIVE: Color = Color::new_rgb(255, 220, 60);
    /// De-emphasized but readable text.
    pub const DIM: Color = Color::new_rgb(160, 160, 160);
    /// Column headers and labels.
    pub const HEADER: Color = Color::new_rgb(200, 200, 200);
    /// Empty/placeholder entries.
    pub const EMPTY: Color = Color::new_rgb(80, 80, 80);
    /// Hint/help text.
    pub const HINT: Color = Color::new_rgb(100, 100, 100);
    /// Error text.
    pub const ERROR: Color = Color::new_rgb(200, 80, 80);
    /// Value bars (e.g. volume).
    pub const BAR: Color = Color::new_rgb(60, 180, 80);
}

/// Fill a solid rectangle.
pub fn fill_rect(layer: &mut ScreenLayer, x: u32, y: u32, w: u32, h: u32, color: Color) {
    for dy in 0..h {
        for dx in 0..w {
            layer.set_pixel(Vec2::new(x + dx, y + dy), color);
        }
    }
}

/// Clear the whole editor panel (area above the tab bar).
pub fn clear_panel(layer: &mut ScreenLayer, color: Color) {
    fill_rect(layer, 0, 0, PANEL_W, PANEL_H, color);
}

/// Draw a 1px rectangle outline.
pub fn rect_border(layer: &mut ScreenLayer, x: u32, y: u32, w: u32, h: u32, color: Color) {
    if w == 0 || h == 0 {
        return;
    }
    for dx in 0..w {
        layer.set_pixel(Vec2::new(x + dx, y), color);
        layer.set_pixel(Vec2::new(x + dx, y + h - 1), color);
    }
    for dy in 0..h {
        layer.set_pixel(Vec2::new(x, y + dy), color);
        layer.set_pixel(Vec2::new(x + w - 1, y + dy), color);
    }
}

/// A `cols x rows` grid of `cell_w x cell_h` pixel cells anchored at `(ox, oy)`,
/// for hit-testing pickers and selector strips.
#[derive(Clone, Copy)]
pub struct Grid {
    pub ox: u32,
    pub oy: u32,
    pub cell_w: u32,
    pub cell_h: u32,
    pub cols: u32,
    pub rows: u32,
}

impl Grid {
    pub const fn new(ox: u32, oy: u32, cell_w: u32, cell_h: u32, cols: u32, rows: u32) -> Self {
        Self {
            ox,
            oy,
            cell_w,
            cell_h,
            cols,
            rows,
        }
    }

    /// Map a screen position to the `(col, row)` cell it falls in,
    /// or `None` when outside the grid.
    pub fn cell_at(&self, x: u32, y: u32) -> Option<(usize, usize)> {
        if x < self.ox || y < self.oy {
            return None;
        }
        let col = (x - self.ox) / self.cell_w;
        let row = (y - self.oy) / self.cell_h;
        if col < self.cols && row < self.rows {
            Some((col as usize, row as usize))
        } else {
            None
        }
    }
}

/// Draw a small labeled button. Width = label.len()*4 + 4, height = 7.
pub fn draw_button(
    layer: &mut ScreenLayer,
    font: &Font,
    bx: u32,
    by: u32,
    label: &str,
    active: bool,
) {
    let w = label.len() as u32 * 4 + 4;
    let h = 7u32;
    let bg = if active {
        Color::new_rgb(60, 100, 180)
    } else {
        Color::new_rgb(35, 35, 35)
    };
    let border = if active {
        Color::new_rgb(100, 160, 220)
    } else {
        Color::new_rgb(70, 70, 70)
    };
    let fg = if active {
        Color::new_rgb(255, 255, 255)
    } else {
        Color::new_rgb(140, 140, 140)
    };
    fill_rect(layer, bx, by, w, h, bg);
    rect_border(layer, bx, by, w, h, border);
    draw_text(font, layer, label, Vec2::new(bx + 2, by + 1), fg);
}

/// Hit-test a button drawn by draw_button.
pub fn button_hit(bx: u32, by: u32, label: &str, x: u32, y: u32) -> bool {
    let w = label.len() as u32 * 4 + 4;
    x >= bx && x < bx + w && y >= by && y < by + 7
}

/// Translate a key press to the character it produces on a US layout.
pub fn key_to_char(key: KeyCode, shift: bool) -> Option<char> {
    Some(match (key, shift) {
        (KeyCode::Space, _) => ' ',
        (KeyCode::KeyA, false) => 'a',
        (KeyCode::KeyA, true) => 'A',
        (KeyCode::KeyB, false) => 'b',
        (KeyCode::KeyB, true) => 'B',
        (KeyCode::KeyC, false) => 'c',
        (KeyCode::KeyC, true) => 'C',
        (KeyCode::KeyD, false) => 'd',
        (KeyCode::KeyD, true) => 'D',
        (KeyCode::KeyE, false) => 'e',
        (KeyCode::KeyE, true) => 'E',
        (KeyCode::KeyF, false) => 'f',
        (KeyCode::KeyF, true) => 'F',
        (KeyCode::KeyG, false) => 'g',
        (KeyCode::KeyG, true) => 'G',
        (KeyCode::KeyH, false) => 'h',
        (KeyCode::KeyH, true) => 'H',
        (KeyCode::KeyI, false) => 'i',
        (KeyCode::KeyI, true) => 'I',
        (KeyCode::KeyJ, false) => 'j',
        (KeyCode::KeyJ, true) => 'J',
        (KeyCode::KeyK, false) => 'k',
        (KeyCode::KeyK, true) => 'K',
        (KeyCode::KeyL, false) => 'l',
        (KeyCode::KeyL, true) => 'L',
        (KeyCode::KeyM, false) => 'm',
        (KeyCode::KeyM, true) => 'M',
        (KeyCode::KeyN, false) => 'n',
        (KeyCode::KeyN, true) => 'N',
        (KeyCode::KeyO, false) => 'o',
        (KeyCode::KeyO, true) => 'O',
        (KeyCode::KeyP, false) => 'p',
        (KeyCode::KeyP, true) => 'P',
        (KeyCode::KeyQ, false) => 'q',
        (KeyCode::KeyQ, true) => 'Q',
        (KeyCode::KeyR, false) => 'r',
        (KeyCode::KeyR, true) => 'R',
        (KeyCode::KeyS, false) => 's',
        (KeyCode::KeyS, true) => 'S',
        (KeyCode::KeyT, false) => 't',
        (KeyCode::KeyT, true) => 'T',
        (KeyCode::KeyU, false) => 'u',
        (KeyCode::KeyU, true) => 'U',
        (KeyCode::KeyV, false) => 'v',
        (KeyCode::KeyV, true) => 'V',
        (KeyCode::KeyW, false) => 'w',
        (KeyCode::KeyW, true) => 'W',
        (KeyCode::KeyX, false) => 'x',
        (KeyCode::KeyX, true) => 'X',
        (KeyCode::KeyY, false) => 'y',
        (KeyCode::KeyY, true) => 'Y',
        (KeyCode::KeyZ, false) => 'z',
        (KeyCode::KeyZ, true) => 'Z',
        (KeyCode::Digit0, false) => '0',
        (KeyCode::Digit0, true) => ')',
        (KeyCode::Digit1, false) => '1',
        (KeyCode::Digit1, true) => '!',
        (KeyCode::Digit2, false) => '2',
        (KeyCode::Digit2, true) => '@',
        (KeyCode::Digit3, false) => '3',
        (KeyCode::Digit3, true) => '#',
        (KeyCode::Digit4, false) => '4',
        (KeyCode::Digit4, true) => '$',
        (KeyCode::Digit5, false) => '5',
        (KeyCode::Digit5, true) => '%',
        (KeyCode::Digit6, false) => '6',
        (KeyCode::Digit6, true) => '^',
        (KeyCode::Digit7, false) => '7',
        (KeyCode::Digit7, true) => '&',
        (KeyCode::Digit8, false) => '8',
        (KeyCode::Digit8, true) => '*',
        (KeyCode::Digit9, false) => '9',
        (KeyCode::Digit9, true) => '(',
        (KeyCode::Minus, false) => '-',
        (KeyCode::Minus, true) => '_',
        (KeyCode::Equal, false) => '=',
        (KeyCode::Equal, true) => '+',
        (KeyCode::BracketLeft, false) => '[',
        (KeyCode::BracketLeft, true) => '{',
        (KeyCode::BracketRight, false) => ']',
        (KeyCode::BracketRight, true) => '}',
        (KeyCode::Semicolon, false) => ';',
        (KeyCode::Semicolon, true) => ':',
        (KeyCode::Quote, false) => '\'',
        (KeyCode::Quote, true) => '"',
        (KeyCode::Backquote, false) => '`',
        (KeyCode::Backquote, true) => '~',
        (KeyCode::Backslash, false) => '\\',
        (KeyCode::Backslash, true) => '|',
        (KeyCode::Slash, false) => '/',
        (KeyCode::Slash, true) => '?',
        (KeyCode::Period, false) => '.',
        (KeyCode::Period, true) => '>',
        (KeyCode::Comma, false) => ',',
        (KeyCode::Comma, true) => '<',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_cell_inside_and_outside() {
        // 8x8 cells, 4 cols x 8 rows anchored at (96, 0)
        let g = Grid::new(96, 0, 8, 8, 4, 8);
        assert_eq!(g.cell_at(96, 0), Some((0, 0)));
        assert_eq!(g.cell_at(127, 63), Some((3, 7)));
        assert_eq!(g.cell_at(95, 0), None); // left of grid
        assert_eq!(g.cell_at(96, 64), None); // below grid
        assert_eq!(g.cell_at(128, 0), None); // right of grid
    }

    #[test]
    fn key_to_char_shift_variants() {
        assert_eq!(key_to_char(KeyCode::KeyA, false), Some('a'));
        assert_eq!(key_to_char(KeyCode::KeyA, true), Some('A'));
        assert_eq!(key_to_char(KeyCode::Digit1, true), Some('!'));
        assert_eq!(key_to_char(KeyCode::F1, false), None);
    }

    #[test]
    fn button_hit_bounds() {
        // "RUN" -> width 3*4+4 = 16, height 7
        assert!(button_hit(112, 0, "RUN", 112, 0));
        assert!(button_hit(112, 0, "RUN", 127, 6));
        assert!(!button_hit(112, 0, "RUN", 111, 0));
        assert!(!button_hit(112, 0, "RUN", 112, 7));
    }
}
