use crate::screen::Screen;

pub trait Cartridge {
    fn update(&mut self, screen: &mut Screen);
}
