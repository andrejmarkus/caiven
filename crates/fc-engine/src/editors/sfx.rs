use fc_core::{Color, Vec2};
use fc_vm::rendering::{font::Font, screen::ScreenLayer, text::draw_text};
use fc_vm::vm::Vm;
use winit::keyboard::KeyCode;

use super::Editor;

pub struct SfxEditor;

impl SfxEditor {
    pub fn new() -> Self {
        SfxEditor
    }
}

impl Editor for SfxEditor {
    fn render(&self, layer: &mut ScreenLayer, _vm: &Vm, font: &Font, _cursor: (u32, u32)) {
        draw_text(
            font,
            layer,
            "SFX EDITOR",
            Vec2::new(4, 4),
            Color::new_rgb(200, 200, 200),
        );
        draw_text(
            font,
            layer,
            "COMING SOON",
            Vec2::new(4, 12),
            Color::new_rgb(100, 100, 100),
        );
    }

    fn handle_click(&mut self, _x: u32, _y: u32, _vm: &mut Vm) {}

    fn handle_key(&mut self, _key: KeyCode, _vm: &mut Vm) {}
}
