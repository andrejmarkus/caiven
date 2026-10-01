#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Vec2 {
    pub x: u32,
    pub y: u32,
}

impl Vec2 {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}
