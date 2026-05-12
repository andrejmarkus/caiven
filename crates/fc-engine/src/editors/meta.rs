use fc_core::{Color, Vec2};
use fc_vm::rendering::{font::Font, screen::ScreenLayer, text::draw_text};
use fc_vm::vm::Vm;
use winit::keyboard::KeyCode;

use super::Editor;

pub struct MetaEditor {
    pub title: String,
    pub author: String,
    pub entry_point: u32,
    pub flags: u32,
}

impl MetaEditor {
    pub fn new() -> Self {
        MetaEditor {
            title: String::new(),
            author: String::new(),
            entry_point: 0,
            flags: 0,
        }
    }

    pub fn set_header(&mut self, title: &str, author: &str, entry_point: u32, flags: u32) {
        self.title = title.to_string();
        self.author = author.to_string();
        self.entry_point = entry_point;
        self.flags = flags;
    }
}

impl Editor for MetaEditor {
    fn render(&self, layer: &mut ScreenLayer, _vm: &Vm, font: &Font, _cursor: (u32, u32)) {
        let bg = Color::new_rgb(15, 15, 15);
        for y in 0..120u32 {
            for x in 0..128u32 {
                layer.set_pixel(Vec2::new(x, y), bg);
            }
        }

        let label_c = Color::new_rgb(140, 140, 140);
        let value_c = Color::new_rgb(220, 220, 220);
        let hint_c = Color::new_rgb(70, 70, 70);

        draw_text(font, layer, "CART META", Vec2::new(0, 0), Color::new_rgb(180, 180, 180));

        draw_text(font, layer, "TITLE", Vec2::new(0, 12), label_c);
        let title_display = if self.title.is_empty() { "(none)" } else { &self.title };
        draw_text(font, layer, title_display, Vec2::new(0, 20), value_c);

        draw_text(font, layer, "AUTHOR", Vec2::new(0, 32), label_c);
        let author_display = if self.author.is_empty() { "(none)" } else { &self.author };
        draw_text(font, layer, author_display, Vec2::new(0, 40), value_c);

        draw_text(font, layer, "ENTRY", Vec2::new(0, 52), label_c);
        let entry = format!("0x{:04X}", self.entry_point);
        draw_text(font, layer, &entry, Vec2::new(0, 60), value_c);

        draw_text(font, layer, "FLAGS", Vec2::new(0, 72), label_c);
        let flags = format!("0x{:04X}", self.flags);
        draw_text(font, layer, &flags, Vec2::new(0, 80), value_c);

        draw_text(font, layer, "EDIT .ASM TO CHANGE", Vec2::new(0, 96), hint_c);
        draw_text(font, layer, "CTRL+S SAVES HEADER", Vec2::new(0, 104), hint_c);
    }

    fn handle_click(&mut self, _x: u32, _y: u32, _vm: &mut Vm) {}

    fn handle_key(&mut self, _key: KeyCode, _vm: &mut Vm) {}
}
