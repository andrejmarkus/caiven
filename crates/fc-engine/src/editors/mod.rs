pub mod browser;
pub mod code;
pub mod map;
pub mod meta;
pub mod music;
pub mod palette;
pub mod sfx;
pub mod sprite;

pub use browser::BrowserEditor;
pub use code::{CodeEditor, CodeEditorAction};
pub use map::MapEditor;
pub use meta::MetaEditor;
pub use music::MusicEditor;
pub use palette::PaletteEditor;
pub use sfx::SfxEditor;
pub use sprite::SpriteEditor;

use fc_core::{Color, Vec2};
use fc_vm::rendering::text::draw_text;
use fc_vm::rendering::{font::Font, screen::ScreenLayer};
use fc_vm::vm::Vm;
use winit::keyboard::KeyCode;

pub trait Editor {
    fn render(&self, layer: &mut ScreenLayer, vm: &Vm, font: &Font, cursor: (u32, u32));
    fn handle_click(&mut self, x: u32, y: u32, vm: &mut Vm);
    fn handle_drag(&mut self, x: u32, y: u32, vm: &mut Vm) {
        self.handle_click(x, y, vm);
    }
    fn handle_mouse_up(&mut self, _x: u32, _y: u32, _vm: &mut Vm) {}
    fn handle_key(&mut self, _key: KeyCode, _vm: &mut Vm) {}
    fn handle_right_click(&mut self, _x: u32, _y: u32, _vm: &mut Vm) {}
    fn handle_right_drag(&mut self, x: u32, y: u32, vm: &mut Vm) {
        self.handle_right_click(x, y, vm);
    }
    fn handle_scroll(&mut self, _dx: f32, _dy: f32, _vm: &mut Vm) {}
    #[allow(dead_code)]
    fn tick(&mut self, _vm: &mut Vm) {}
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
    for dy in 0..h {
        for dx in 0..w {
            layer.set_pixel(Vec2::new(bx + dx, by + dy), bg);
        }
    }
    for dx in 0..w {
        layer.set_pixel(Vec2::new(bx + dx, by), border);
        layer.set_pixel(Vec2::new(bx + dx, by + h - 1), border);
    }
    for dy in 0..h {
        layer.set_pixel(Vec2::new(bx, by + dy), border);
        layer.set_pixel(Vec2::new(bx + w - 1, by + dy), border);
    }
    draw_text(font, layer, label, Vec2::new(bx + 2, by + 1), fg);
}

/// Hit-test a button drawn by draw_button.
pub fn button_hit(bx: u32, by: u32, label: &str, x: u32, y: u32) -> bool {
    let w = label.len() as u32 * 4 + 4;
    x >= bx && x < bx + w && y >= by && y < by + 7
}
