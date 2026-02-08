use crate::utils::Vec2;

pub struct Camera {
    position: Vec2,
}

impl Camera {
    pub fn new(position: Vec2) -> Self {
        Self { position }
    }

    pub fn get_x(&self) -> u32 {
        self.position.get_x()
    }

    pub fn get_y(&self) -> u32 {
        self.position.get_y()
    }

    pub fn set_position(&mut self, x: u32, y: u32) {
        self.position.set(x, y);
    }

    pub fn move_by(&mut self, dx: i32, dy: i32) {
        let new_x = (self.position.get_x() as i32 + dx).max(0) as u32;
        let new_y = (self.position.get_y() as i32 + dy).max(0) as u32;
        self.position.set(new_x, new_y);
    }
}
