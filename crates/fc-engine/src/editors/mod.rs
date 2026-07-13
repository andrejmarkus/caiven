pub mod browser;
pub mod code;
pub mod map;
pub mod meta;
pub mod music;
pub mod palette;
pub mod sfx;
pub mod sprite;
pub mod util;

pub use browser::BrowserEditor;
pub use code::{CodeEditor, CodeEditorAction};
pub use map::MapEditor;
pub use meta::MetaEditor;
pub use music::MusicEditor;
pub use palette::PaletteEditor;
pub use sfx::SfxEditor;
pub use sprite::SpriteEditor;
pub use util::{button_hit, draw_button};

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
