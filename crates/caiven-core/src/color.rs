#[derive(Clone, Copy, Debug)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Color {
    pub const fn new_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn new_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn to_rgba(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn to_rgb(self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }

    pub fn get_r(self) -> u8 {
        self.r
    }

    pub fn get_g(self) -> u8 {
        self.g
    }

    pub fn get_b(self) -> u8 {
        self.b
    }

    pub fn get_a(self) -> u8 {
        self.a
    }
}

#[cfg(test)]
mod tests {
    use super::Color;

    #[test]
    fn new_rgb_defaults_alpha_to_opaque() {
        let c = Color::new_rgb(10, 20, 30);
        assert_eq!(c.to_rgba(), [10, 20, 30, 255]);
    }

    #[test]
    fn new_rgba_and_getters_roundtrip() {
        let c = Color::new_rgba(1, 2, 3, 4);
        assert_eq!((c.get_r(), c.get_g(), c.get_b(), c.get_a()), (1, 2, 3, 4));
    }

    #[test]
    fn to_rgb_drops_alpha() {
        let c = Color::new_rgba(5, 6, 7, 8);
        assert_eq!(c.to_rgb(), [5, 6, 7]);
    }
}
