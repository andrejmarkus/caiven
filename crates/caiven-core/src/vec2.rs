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

#[cfg(test)]
mod tests {
    use super::Vec2;

    #[test]
    fn new_sets_both_components() {
        let v = Vec2::new(3, 4);
        assert_eq!((v.get_x(), v.get_y()), (3, 4));
    }

    #[test]
    fn setters_update_independently() {
        let mut v = Vec2::new(0, 0);
        v.set_x(7);
        v.set_y(8);
        assert_eq!((v.get_x(), v.get_y()), (7, 8));
    }

    #[test]
    fn set_updates_both_at_once() {
        let mut v = Vec2::new(1, 1);
        v.set(9, 10);
        assert_eq!((v.get_x(), v.get_y()), (9, 10));
    }
}
