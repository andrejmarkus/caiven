use fc_core::{Color, Vec2};
use fc_vm::rendering::{font::Font, screen::ScreenLayer, text::draw_text};
use fc_vm::vm::Vm;
use winit::keyboard::KeyCode;

use super::Editor;

const MAX_LEN: usize = 32;

fn key_to_char(key: KeyCode) -> Option<char> {
    match key {
        KeyCode::KeyA => Some('A'), KeyCode::KeyB => Some('B'), KeyCode::KeyC => Some('C'),
        KeyCode::KeyD => Some('D'), KeyCode::KeyE => Some('E'), KeyCode::KeyF => Some('F'),
        KeyCode::KeyG => Some('G'), KeyCode::KeyH => Some('H'), KeyCode::KeyI => Some('I'),
        KeyCode::KeyJ => Some('J'), KeyCode::KeyK => Some('K'), KeyCode::KeyL => Some('L'),
        KeyCode::KeyM => Some('M'), KeyCode::KeyN => Some('N'), KeyCode::KeyO => Some('O'),
        KeyCode::KeyP => Some('P'), KeyCode::KeyQ => Some('Q'), KeyCode::KeyR => Some('R'),
        KeyCode::KeyS => Some('S'), KeyCode::KeyT => Some('T'), KeyCode::KeyU => Some('U'),
        KeyCode::KeyV => Some('V'), KeyCode::KeyW => Some('W'), KeyCode::KeyX => Some('X'),
        KeyCode::KeyY => Some('Y'), KeyCode::KeyZ => Some('Z'),
        KeyCode::Digit0 => Some('0'), KeyCode::Digit1 => Some('1'), KeyCode::Digit2 => Some('2'),
        KeyCode::Digit3 => Some('3'), KeyCode::Digit4 => Some('4'), KeyCode::Digit5 => Some('5'),
        KeyCode::Digit6 => Some('6'), KeyCode::Digit7 => Some('7'), KeyCode::Digit8 => Some('8'),
        KeyCode::Digit9 => Some('9'),
        KeyCode::Space => Some(' '),
        KeyCode::Minus => Some('-'),
        KeyCode::Period => Some('.'),
        KeyCode::Comma => Some(','),
        KeyCode::Quote => Some('\''),
        _ => None,
    }
}

pub struct MetaEditor {
    pub title: String,
    pub author: String,
    pub entry_point: u32,
    pub flags: u32,
    focused: u8, // 0 = title, 1 = author
}

impl MetaEditor {
    pub fn new() -> Self {
        MetaEditor {
            title: String::new(),
            author: String::new(),
            entry_point: 0,
            flags: 0,
            focused: 0,
        }
    }

    pub fn set_header(&mut self, title: &str, author: &str, entry_point: u32, flags: u32) {
        self.title = title.to_string();
        self.author = author.to_string();
        self.entry_point = entry_point;
        self.flags = flags;
    }

    fn draw_cursor(layer: &mut ScreenLayer, x: u32, y: u32) {
        let col = Color::new_rgb(220, 220, 220);
        for dy in 0..5u32 {
            layer.set_pixel(Vec2::new(x, y + dy), col);
            if x + 1 < 128 {
                layer.set_pixel(Vec2::new(x + 1, y + dy), col);
            }
        }
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
        let active_c = Color::new_rgb(255, 220, 80);
        let value_c = Color::new_rgb(220, 220, 220);
        let hint_c = Color::new_rgb(70, 70, 70);

        draw_text(font, layer, "CART META", Vec2::new(0, 0), Color::new_rgb(180, 180, 180));

        // Title field
        let title_lc = if self.focused == 0 { active_c } else { label_c };
        draw_text(font, layer, "TITLE", Vec2::new(0, 12), title_lc);
        draw_text(font, layer, &self.title, Vec2::new(0, 20), value_c);
        if self.focused == 0 {
            let cx = (self.title.len() as u32).min(31) * 4;
            Self::draw_cursor(layer, cx, 20);
        }

        // Author field
        let author_lc = if self.focused == 1 { active_c } else { label_c };
        draw_text(font, layer, "AUTHOR", Vec2::new(0, 32), author_lc);
        draw_text(font, layer, &self.author, Vec2::new(0, 40), value_c);
        if self.focused == 1 {
            let cx = (self.author.len() as u32).min(31) * 4;
            Self::draw_cursor(layer, cx, 40);
        }

        // Read-only fields
        draw_text(font, layer, "ENTRY", Vec2::new(0, 52), label_c);
        draw_text(font, layer, &format!("0x{:04X}", self.entry_point), Vec2::new(0, 60), value_c);

        draw_text(font, layer, "FLAGS", Vec2::new(0, 72), label_c);
        draw_text(font, layer, &format!("0x{:04X}", self.flags), Vec2::new(0, 80), value_c);

        draw_text(font, layer, "TAB=SWITCH  CTRL+S=SAVE", Vec2::new(0, 96), hint_c);
        draw_text(font, layer, "TYPE TO EDIT TITLE+AUTHOR", Vec2::new(0, 104), hint_c);
    }

    fn handle_click(&mut self, _x: u32, y: u32, _vm: &mut Vm) {
        if y >= 12 && y < 30 {
            self.focused = 0;
        } else if y >= 32 && y < 50 {
            self.focused = 1;
        }
    }

    fn handle_key(&mut self, key: KeyCode, _vm: &mut Vm) {
        match key {
            KeyCode::Tab => {
                self.focused = 1 - self.focused;
            }
            KeyCode::Backspace => {
                if self.focused == 0 { self.title.pop(); } else { self.author.pop(); }
            }
            _ => {
                if let Some(c) = key_to_char(key) {
                    let field = if self.focused == 0 { &mut self.title } else { &mut self.author };
                    if field.len() < MAX_LEN {
                        field.push(c);
                    }
                }
            }
        }
    }
}
