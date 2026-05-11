#[derive(Clone, Copy)]
pub struct Vec2 {
    x: u32,
    y: u32,
}

impl Vec2 {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }

    pub fn get_x(self) -> u32 {
        self.x
    }

    pub fn set_x(&mut self, x: u32) {
        self.x = x;
    }

    pub fn get_y(self) -> u32 {
        self.y
    }

    pub fn set_y(&mut self, y: u32) {
        self.y = y;
    }

    pub fn set(&mut self, x: u32, y: u32) {
        self.x = x;
        self.y = y;
    }
}
