use caiven_core::Color;

/// Default 16-color palette: four hue ramps of three shades, plus black,
/// white and two accents.
///
/// The layout is the teaching aid. Slots 1-12 are four ramps in dark → mid →
/// light order, so shading a sprite is "step to the next slot" and never a
/// color-theory decision. Slot `n` is a shadow when `(n - 1) % 3 == 0` and a
/// highlight when `(n - 1) % 3 == 2`, across every ramp. Black is the darkest
/// slot and white the lightest, so index order also reads as value order at
/// the two ends.
pub const DEFAULT_COLORS: [(u8, u8, u8); 16] = [
    (16, 16, 26),    // 0  black
    (110, 31, 46),   // 1  ember dark    — fire, blood, brick
    (194, 55, 47),   // 2  ember mid
    (242, 128, 60),  // 3  ember light
    (30, 58, 42),    // 4  moss dark     — foliage, grass
    (62, 138, 74),   // 5  moss mid
    (134, 207, 98),  // 6  moss light
    (35, 52, 94),    // 7  sky dark      — water, sky, cold metal
    (61, 109, 196),  // 8  sky mid
    (116, 192, 232), // 9  sky light
    (58, 51, 64),    // 10 stone dark    — ground, walls, wood, skin
    (122, 110, 114), // 11 stone mid
    (195, 181, 168), // 12 stone light
    (245, 197, 66),  // 13 gold accent   — coins, highlights, sun
    (224, 96, 160),  // 14 magenta accent — magic, focus, alarm
    (244, 241, 230), // 15 white
];

pub struct Palette {
    colors: Vec<Color>,
}

impl Palette {
    pub fn new(palette_size: usize) -> Self {
        let colors = (0..palette_size)
            .map(|i| match DEFAULT_COLORS.get(i) {
                Some(&(r, g, b)) => Color::new_rgb(r, g, b),
                None => Color::new_rgb(i as u8, i as u8, i as u8),
            })
            .collect();
        Self { colors }
    }

    pub fn get_colors(&self) -> &[Color] {
        &self.colors
    }

    pub fn get_color(&self, index: usize) -> Color {
        if index < self.colors.len() {
            self.colors[index]
        } else {
            Color::new_rgb(0, 0, 0)
        }
    }

    pub fn set_color(&mut self, index: usize, color: Color) {
        if index < self.colors.len() {
            self.colors[index] = color;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DEFAULT_COLORS;

    fn luminance((r, g, b): (u8, u8, u8)) -> f32 {
        0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32
    }

    /// Slots 1-12 are four ramps of three shades in dark → mid → light order.
    /// A cart shades by stepping to the next slot, so a ramp that is not
    /// monotone in brightness would silently teach the wrong habit.
    #[test]
    fn hue_ramps_climb_in_brightness() {
        for ramp in 0..4 {
            let base = 1 + ramp * 3;
            let shades: Vec<f32> = (0..3)
                .map(|i| luminance(DEFAULT_COLORS[base + i]))
                .collect();
            assert!(
                shades[0] < shades[1] && shades[1] < shades[2],
                "ramp at slot {base} is not dark → mid → light: {shades:?}"
            );
        }
    }

    /// Black is the darkest slot and white the lightest, so the two ends of the
    /// index range also read as the two ends of the value range.
    #[test]
    fn black_and_white_bound_the_palette() {
        let darkest = luminance(DEFAULT_COLORS[0]);
        let lightest = luminance(DEFAULT_COLORS[15]);
        for (index, &color) in DEFAULT_COLORS.iter().enumerate().skip(1).take(14) {
            let value = luminance(color);
            assert!(value > darkest, "slot {index} is not brighter than black");
            assert!(value < lightest, "slot {index} is not darker than white");
        }
    }

    /// Every shade tier sits at the same slot offset in each ramp, which is what
    /// makes "slot + 1 is the highlight" true no matter which hue a cart picked.
    #[test]
    fn shade_tiers_stay_separated_across_ramps() {
        let tier = |offset: usize| -> Vec<f32> {
            (0..4)
                .map(|ramp| luminance(DEFAULT_COLORS[1 + ramp * 3 + offset]))
                .collect()
        };
        let (shadows, mids, highlights) = (tier(0), tier(1), tier(2));
        let max = |values: &[f32]| values.iter().copied().fold(f32::MIN, f32::max);
        let min = |values: &[f32]| values.iter().copied().fold(f32::MAX, f32::min);
        assert!(
            max(&shadows) < min(&mids),
            "a shadow is brighter than a mid"
        );
        assert!(
            max(&mids) < min(&highlights),
            "a mid is brighter than a highlight"
        );
    }
}
