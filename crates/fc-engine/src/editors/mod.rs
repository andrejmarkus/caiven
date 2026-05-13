pub mod browser;
pub mod map;
pub mod meta;
pub mod music;
pub mod palette;
pub mod sfx;
pub mod sprite;

pub use browser::BrowserEditor;
pub use map::MapEditor;
pub use meta::MetaEditor;
pub use music::MusicEditor;
pub use palette::PaletteEditor;
pub use sfx::SfxEditor;
pub use sprite::SpriteEditor;

use fc_vm::rendering::{font::Font, screen::ScreenLayer};
use fc_vm::vm::Vm;
use winit::keyboard::KeyCode;

pub trait Editor {
    fn render(&self, layer: &mut ScreenLayer, vm: &Vm, font: &Font, cursor: (u32, u32));
    fn handle_click(&mut self, x: u32, y: u32, vm: &mut Vm);
    fn handle_key(&mut self, _key: KeyCode, _vm: &mut Vm) {}
    fn tick(&mut self, _vm: &mut Vm) {}
}
