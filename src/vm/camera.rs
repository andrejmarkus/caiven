pub struct Camera {
    pub x: u32,
    pub y: u32,
}

impl Camera {
    pub fn new() -> Self {
        Self { x: 0, y: 0 }
    }

    pub fn get_x(&self) -> u32 {
        self.x
    }

    pub fn get_y(&self) -> u32 {
        self.y
    }

    pub fn set_position(&mut self, x: u32, y: u32) {
        self.x = x;
        self.y = y;
    }

    pub fn move_by(&mut self, dx: i32, dy: i32) {
        let new_x = (self.x as i32 + dx).max(0) as u32;
        let new_y = (self.y as i32 + dy).max(0) as u32;
        self.x = new_x;
        self.y = new_y;
    }
}
